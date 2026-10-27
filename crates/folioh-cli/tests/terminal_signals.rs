//! Real process/terminal regressions; no Node installation is needed for the
//! controlled dev-server tree. The own-site check also exercises real Next.
#![cfg(unix)]

use std::fs::{self, File};
use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

mod common;

struct Process {
    child: Child,
    log: PathBuf,
    server_pids: Option<PathBuf>,
}

impl Process {
    fn output(&self) -> String {
        fs::read_to_string(&self.log).unwrap()
    }

    fn wait_for(&mut self, text: &str) {
        until(|| {
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "exited before {text}: {}",
                self.output()
            );
            self.output().contains(text)
        });
    }

    fn signal(&self, signal: i32) {
        assert_eq!(unsafe { libc::kill(self.child.id() as i32, signal) }, 0);
    }

    fn wait(&mut self) -> ExitStatus {
        let mut status = None;
        until(|| {
            status = self.child.try_wait().unwrap();
            status.is_some()
        });
        status.unwrap()
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if let Some(path) = &self.server_pids {
            if let Ok(text) = fs::read_to_string(path) {
                let pids: Vec<i32> = text
                    .split_whitespace()
                    .filter_map(|p| p.parse().ok())
                    .collect();
                if let Some(group) = pids.first() {
                    // Only a still-present member of this fixture's group
                    // permits cleanup. Never signal the test runner's group.
                    if pids
                        .iter()
                        .any(|pid| unsafe { libc::getpgid(*pid) } == *group)
                    {
                        unsafe {
                            libc::kill(-*group, libc::SIGKILL);
                        }
                    }
                }
            }
        }
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

fn until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(25);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out");
        thread::sleep(Duration::from_millis(25));
    }
}

fn spawn(project: &Path, args: &[&str], stdin: Stdio, setup: impl FnOnce(&mut Command)) -> Process {
    let log = project.join("process.log");
    let file = File::create(&log).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_folioh"));
    common::env::scrub(&mut command)
        .current_dir(project)
        .args(args)
        .stdin(stdin)
        .stdout(file.try_clone().unwrap())
        .stderr(file);
    setup(&mut command);
    Process {
        child: command.spawn().unwrap(),
        log,
        server_pids: None,
    }
}

fn executable(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn running(pid: i32) -> bool {
    let output = Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .unwrap();
    let state = String::from_utf8_lossy(&output.stdout);
    output.status.success() && !state.trim().is_empty() && !state.trim_start().starts_with('Z')
}

#[test]
fn serve_signals_stop_the_owned_tree_and_release_the_workspace() {
    for (signal, code, ignore_term) in [
        (libc::SIGINT, 130, true),
        (libc::SIGTERM, 143, true),
        (libc::SIGHUP, 129, true),
        (libc::SIGQUIT, 131, true),
        (libc::SIGTERM, 143, false),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path();
        fs::create_dir(project.join("docs")).unwrap();
        fs::write(project.join("docs/index.md"), "# Signals\n").unwrap();
        fs::write(
            project.join("docs.yaml"),
            "project: {name: Signals}\nsource:\n  docs: [docs]\n",
        )
        .unwrap();
        let bin = project.join("bin");
        fs::create_dir(&bin).unwrap();
        executable(&bin.join("node"), "#!/bin/sh\necho v22.0.0\n");
        executable(
            &bin.join("pnpm"),
            r#"#!/bin/sh
set -eu
case "$*" in
  --version) echo 10.0.0 ;;
  install*) mkdir -p node_modules/.bin; touch node_modules/.bin/next ;;
  'exec next --version') echo 16.3.3 ;;
  'exec next dev'*)
    if [ "$FOLIOH_TEST_IGNORE_TERM" = 1 ]; then trap '' TERM INT; fi
    sleep 300 &
    # Readers must see the complete PID pair as soon as the ready file exists.
    echo "$$ $!" > "${FOLIOH_SIGNAL_PIDS}.pending"
    mv "${FOLIOH_SIGNAL_PIDS}.pending" "$FOLIOH_SIGNAL_PIDS"
    echo 'Ready in 0.1s'
    wait ;;
  *) echo "unexpected pnpm arguments: $*" >&2; exit 2 ;;
esac
"#,
        );
        let pids_path = project.join("server.pids");
        let mut paths = vec![bin];
        paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
        let path = std::env::join_paths(paths).unwrap();
        let port = common::free_port().to_string();
        let mut process = spawn(project, &["serve", "--port", &port], Stdio::null(), |cmd| {
            cmd.env_remove("FOLIOH_FRONTEND_RUNTIME")
                .env("PATH", path)
                .env("FOLIOH_SIGNAL_PIDS", &pids_path)
                .env(
                    "FOLIOH_TEST_IGNORE_TERM",
                    if ignore_term { "1" } else { "0" },
                );
        });
        process.server_pids = Some(pids_path.clone());
        process.wait_for("Watching for file changes");
        until(|| pids_path.is_file());
        let pids: Vec<i32> = fs::read_to_string(&pids_path)
            .unwrap()
            .split_whitespace()
            .map(|p| p.parse().unwrap())
            .collect();
        assert_eq!(pids.len(), 2);
        assert!(pids
            .iter()
            .all(|pid| unsafe { libc::getpgid(*pid) } == pids[0]));
        process.signal(signal);
        assert_eq!(process.wait().code(), Some(code), "{}", process.output());
        until(|| pids.iter().all(|pid| !running(*pid)));
        process.server_pids = None;
        assert_eq!(process.output().matches("Server stopped.").count(), 1);
        assert!(!process.output().contains("Build failed"));
        assert!(!project.join(".build.owner.json").exists());
        // A subsequent build claims the same workspace normally.
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_folioh"));
        let rebuilt = common::env::scrub(&mut cmd)
            .current_dir(project)
            .arg("build")
            .output()
            .unwrap();
        assert!(
            rebuilt.status.success(),
            "{}",
            String::from_utf8_lossy(&rebuilt.stderr)
        );
        assert!(!String::from_utf8_lossy(&rebuilt.stderr).contains("Recovered stale"));
    }
}

#[test]
fn init_interrupts_an_unfinished_piped_answer_without_writing_files() {
    for (signal, code) in [(libc::SIGINT, 130), (libc::SIGTERM, 143)] {
        let tmp = tempfile::tempdir().unwrap();
        let mut process = spawn(tmp.path(), &["init"], Stdio::piped(), |_| {});
        let mut input = process.child.stdin.take().unwrap();
        input.write_all(b"au").unwrap(); // no newline, and keep the pipe open
        process.wait_for("Select");
        process.signal(signal);
        assert_eq!(process.wait().code(), Some(code), "{}", process.output());
        assert!(process.output().contains("Init aborted."));
        assert!(!tmp.path().join("docs.yaml").exists());
    }
}

fn termios(fd: i32) -> libc::termios {
    let mut state = std::mem::MaybeUninit::uninit();
    assert_eq!(unsafe { libc::tcgetattr(fd, state.as_mut_ptr()) }, 0);
    // tcgetattr initialized the complete value on success.
    unsafe { state.assume_init() }
}

#[test]
fn init_os_signal_restores_the_terminal_after_a_numbered_prompt() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut master, mut slave) = (-1, -1);
    let mut size = libc::winsize {
        ws_row: 24,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    assert_eq!(
        unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::addr_of_mut!(size),
            )
        },
        0
    );
    for fd in [master, slave] {
        assert_eq!(
            unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) },
            0
        );
    }
    // Own each returned descriptor once; duplicate only the child's stdin.
    let (_master, slave) = unsafe { (File::from_raw_fd(master), File::from_raw_fd(slave)) };
    let fd = slave.as_raw_fd();
    let before = termios(fd);
    let input = slave.try_clone().unwrap();
    let mut process = spawn(tmp.path(), &["init"], Stdio::from(input), |cmd| {
        cmd.env("CI", "1");
    });
    process.wait_for("Select");
    until(|| termios(fd).c_lflag & libc::ICANON == 0);
    process.signal(libc::SIGINT);
    assert_eq!(process.wait().code(), Some(130), "{}", process.output());
    let restored = termios(fd);
    assert_eq!(
        restored.c_lflag & (libc::ICANON | libc::ECHO),
        before.c_lflag & (libc::ICANON | libc::ECHO)
    );
    assert!(process.output().contains("Init aborted."));
    assert!(!tmp.path().join("docs.yaml").exists());
}
