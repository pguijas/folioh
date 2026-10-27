//! Unix command shutdown: handlers set atomics; normal code stops children,
//! drops workspace/terminal guards and returns the conventional exit code.
//! Windows keeps its existing console handling.

use std::io;
use std::process::{Child, ExitStatus};

#[cfg(unix)]
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// Scoped actions for one init/serve command. Unregister only at shutdown:
/// signal-hook keeps the OS dispatcher installed after its actions are gone.
pub(crate) struct StopSignals {
    #[cfg(unix)]
    pending: Arc<AtomicUsize>,
    #[cfg(unix)]
    registrations: Vec<signal_hook::SigId>,
}

impl StopSignals {
    pub fn install() -> io::Result<Self> {
        let guard = Self {
            #[cfg(unix)]
            pending: Arc::new(AtomicUsize::new(0)),
            #[cfg(unix)]
            registrations: Vec::new(),
        };
        #[cfg(unix)]
        let mut guard = guard;
        #[cfg(unix)]
        for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP, libc::SIGQUIT] {
            guard.registrations.push(signal_hook::flag::register_usize(
                signal,
                Arc::clone(&guard.pending),
                signal as usize,
            )?);
        }
        Ok(guard)
    }

    pub fn exit_code(&self) -> Option<u8> {
        #[cfg(unix)]
        return match self.pending.load(Ordering::Relaxed) {
            0 => None,
            signal => Some((128 + signal) as u8),
        };
        #[cfg(not(unix))]
        None
    }
}

impl Drop for StopSignals {
    fn drop(&mut self) {
        #[cfg(unix)]
        for id in &self.registrations {
            signal_hook::low_level::unregister(*id);
        }
    }
}

/// Stop only the server group created by NextRuntime, then reap its direct
/// child. Keeping that child unreaped reserves the group ID through the TERM
/// grace period, including when grandchildren ignore TERM.
pub(crate) fn stop_child(child: &mut Child) -> io::Result<ExitStatus> {
    #[cfg(unix)]
    {
        let group = libc::pid_t::try_from(child.id()).map_err(io::Error::other)?;
        let send = |signal| {
            // NextRuntime uses process_group(0); the positive PID belongs
            // to the held Child, and the negative ID addresses only its group.
            if unsafe { libc::kill(-group, signal) } == 0 {
                return Ok(());
            }
            let err = io::Error::last_os_error();
            // Darwin skips zombies when signalling groups, returning EPERM
            // when none can receive a signal. Verify that state rather than
            // swallowing a permission error on a live member.
            #[cfg(target_os = "macos")]
            if err.raw_os_error() == Some(libc::EPERM) && macos_group_stopped(group) {
                return Ok(());
            }
            if err.raw_os_error() == Some(libc::ESRCH) {
                Ok(())
            } else {
                Err(err)
            }
        };
        let term = send(libc::SIGTERM);
        std::thread::sleep(std::time::Duration::from_millis(200));
        let force = send(libc::SIGKILL);
        if let Err(group_error) = force {
            // Do not wait forever if the OS also refuses the direct stop.
            if child.kill().is_ok() {
                let _ = child.wait();
            }
            return Err(group_error);
        }
        term.and(child.wait())
    }
    #[cfg(not(unix))]
    {
        child.kill()?;
        child.wait()
    }
}

/// Called only while the direct child is still unreaped, reserving the group
/// identity. A failed/truncated query never turns EPERM into success.
#[cfg(target_os = "macos")]
// is_multiple_of requires Rust 1.87; the workspace supports Rust 1.85.
#[allow(clippy::manual_is_multiple_of)]
fn macos_group_stopped(group: libc::pid_t) -> bool {
    // Apple's proc_info.h declares PROC_PGRP_ONLY = 2; libc exposes the API
    // and its record types but not this selector.
    const PROC_PGRP_ONLY: u32 = 2;
    let needed =
        unsafe { libc::proc_listpids(PROC_PGRP_ONLY, group as u32, std::ptr::null_mut(), 0) };
    if needed <= 0 {
        return false;
    }
    let width = std::mem::size_of::<libc::pid_t>();
    let mut pids = vec![0 as libc::pid_t; needed as usize / width + 8];
    let Ok(capacity) = i32::try_from(std::mem::size_of_val(pids.as_slice())) else {
        return false;
    };
    // The initialized, aligned buffer has exactly the advertised capacity.
    let bytes = unsafe {
        libc::proc_listpids(
            PROC_PGRP_ONLY,
            group as u32,
            pids.as_mut_ptr().cast(),
            capacity,
        )
    };
    if bytes <= 0 || bytes >= capacity || bytes as usize % width != 0 {
        return false;
    }
    let mut found = false;
    for pid in pids
        .into_iter()
        .take(bytes as usize / width)
        .filter(|pid| *pid > 0)
    {
        found = true;
        let mut info = std::mem::MaybeUninit::<libc::proc_bsdshortinfo>::uninit();
        let size = std::mem::size_of_val(&info) as i32;
        let read = unsafe {
            libc::proc_pidinfo(
                pid,
                libc::PROC_PIDT_SHORTBSDINFO,
                0,
                info.as_mut_ptr().cast(),
                size,
            )
        };
        if read == size {
            // The kernel wrote the complete C record. A live member, even
            // one we cannot signal, must keep the original permission error.
            if unsafe { info.assume_init() }.pbsi_status != libc::SZOMB {
                return false;
            }
        } else if read != 0 || io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
            return false;
        }
        // proc_pidinfo returns ESRCH for a zombie or an already-gone PID.
    }
    found
}

#[cfg(all(test, target_os = "macos"))]
#[path = "signals_tests.rs"]
mod tests;
