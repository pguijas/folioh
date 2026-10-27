//! Competing commands must refuse before touching a live serve's workspace;
//! a killed holder must not leave a permanent claim behind.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod common;

struct Holder(Child);

impl Drop for Holder {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn serve(project: &Path, port: u16) -> Holder {
    let mut command = Command::new(env!("CARGO_BIN_EXE_folioh"));
    let mut child = common::env::scrub(&mut command)
        .current_dir(project)
        .args(["serve", "--port", &port.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let pipe = child.stdout.take().unwrap();
    let holder = Holder(child);
    let (sender, ready) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(pipe).lines().map_while(Result::ok) {
            if line.contains("Watching for file changes") {
                let _ = sender.send(());
            }
        }
    });
    ready
        .recv_timeout(Duration::from_secs(30))
        .expect("stubbed serve becomes ready");
    holder
}

#[test]
fn a_live_serve_refuses_competing_commands_and_a_killed_holder_recovers() {
    let (_dir, project) = common::example_project();
    let port = common::free_port();
    let mut holder = serve(&project, port);
    let pid = holder.0.id();
    let sentinel = project.join(".build/holder-sentinel");
    fs::write(&sentinel, "do not remove").unwrap();
    let content = project.join(".build/content/index.mdx");
    let before = fs::read(&content).unwrap();

    let other_port = common::free_port().to_string();
    for args in [
        vec!["build", "--clean"],
        vec!["clean"],
        vec!["serve", "--port", &other_port, "--clean"],
        vec!["serve", "--port", &other_port, "--clean", "--kill-existing"],
    ] {
        let run = common::folioh(&project, &args, &[]);
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert!(run.stderr.contains("Workspace "), "{}", run.stderr);
        assert!(
            run.stderr
                .contains(&format!("PID {pid} (folioh serve, port {port})")),
            "{}",
            run.stderr
        );
        assert!(sentinel.exists(), "a refused --clean touched the workspace");
        assert_eq!(fs::read(&content).unwrap(), before);
        assert!(holder.0.try_wait().unwrap().is_none());
    }

    // Ownership is scoped to this workspace, not a machine-wide mutex.
    let (_other_dir, other) = common::example_project();
    common::folioh(&other, &["build"], &[]).ok();

    holder.0.kill().unwrap();
    holder.0.wait().unwrap();
    assert!(project.join(".build.owner.json").exists());
    let recovered = common::folioh(&project, &["build", "--clean"], &[]);
    recovered.ok();
    assert!(
        recovered.stdout.contains(&format!(
            "Recovered stale workspace claim left by PID {pid} (folioh serve, port {port})"
        )),
        "{}",
        recovered.stdout
    );
    assert!(!sentinel.exists(), "the recovered --clean must now run");
    assert!(!project.join(".build.owner.json").exists());
    let warm = common::folioh(&project, &["build"], &[]);
    warm.ok();
    assert!(!warm.stdout.contains("Recovered stale workspace"));
}

#[test]
fn an_incomplete_owner_record_cannot_strand_an_unlocked_workspace() {
    let (_dir, project) = common::example_project();
    // Even a valid record naming a currently live PID is only diagnostic:
    // it cannot block a workspace when nobody owns its OS lock.
    let live_pid = serde_json::json!({
        "pid": std::process::id(), "command": "serve", "port": 4399
    })
    .to_string();
    for record in ["{interrupted", &live_pid] {
        fs::write(project.join(".build.owner.json"), record).unwrap();
        let run = common::folioh(&project, &["build"], &[]);
        run.ok();
        assert!(
            run.stdout.contains("Recovered stale workspace claim"),
            "{}",
            run.stdout
        );
        assert!(!project.join(".build.owner.json").exists());
    }
}
