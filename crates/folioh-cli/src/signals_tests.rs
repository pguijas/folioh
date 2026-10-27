use super::*;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

struct Group(Child);
impl Drop for Group {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            unsafe {
                libc::kill(-(self.0.id() as i32), libc::SIGKILL);
            }
        }
        let _ = self.0.wait();
    }
}

#[test]
fn a_live_group_is_not_mistaken_for_unreaped_zombies() {
    let child = Command::new("sleep")
        .arg("300")
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let group = Group(child);
    let id = group.0.id() as libc::pid_t;
    assert!(!macos_group_stopped(id));
    assert_eq!(unsafe { libc::kill(-id, libc::SIGTERM) }, 0);
    let deadline = Instant::now() + Duration::from_secs(3);
    while !macos_group_stopped(id) {
        assert!(
            Instant::now() < deadline,
            "terminated group still appears live"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    // The held child stays unreaped until Group drops; its PID still reserves
    // this group's identity during both queries.
}
