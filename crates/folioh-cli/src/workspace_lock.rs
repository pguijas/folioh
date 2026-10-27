//! Exclusive ownership of a generated workspace, including the whole serve
//! session. The lock inode stays outside the directory that `--clean` removes.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use folioh_config::canonicalize_lenient;
use fs4::TryLockError;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Owner {
    pid: u32,
    command: String,
    port: Option<u16>,
}

impl Owner {
    fn description(&self) -> String {
        match self.port {
            Some(port) => format!("PID {} (folioh {}, port {port})", self.pid, self.command),
            None => format!("PID {} (folioh {})", self.pid, self.command),
        }
    }
}

/// The OS releases ownership even after an abrupt exit. Metadata is separate
/// because Windows can deny reads of the locked file by competing processes.
pub(crate) struct WorkspaceLease {
    _file: File,
    owner_path: PathBuf,
    pub recovered: Option<String>,
}

fn sibling(path: &Path, suffix: &str) -> Result<PathBuf, String> {
    let mut name = path
        .file_name()
        .ok_or_else(|| format!("Invalid workspace path: {}", path.display()))?
        .to_os_string();
    name.push(suffix);
    Ok(path.with_file_name(name))
}

fn read_owner(path: &Path) -> Option<Owner> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

impl WorkspaceLease {
    pub fn acquire(build_dir: &Path, command: &str, port: Option<u16>) -> Result<Self, String> {
        let build_dir = canonicalize_lenient(build_dir);
        let lock_path = sibling(&build_dir, ".lock")?;
        let owner_path = sibling(&build_dir, ".owner.json")?;
        let io_error = |err| format!("Cannot claim workspace {}: {err}", build_dir.display());
        fs::create_dir_all(lock_path.parent().expect("workspace has a parent"))
            .map_err(io_error)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(io_error)?;
        match fs4::FileExt::try_lock(&file) {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                // A new owner may still be publishing its diagnostic record.
                let mut owner = read_owner(&owner_path);
                for _ in 0..5 {
                    if owner.is_some() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                    owner = read_owner(&owner_path);
                }
                let holder = owner
                    .map(|owner| owner.description())
                    .unwrap_or_else(|| "another Folioh process (owner details unavailable)".into());
                return Err(format!(
                    "Workspace {} is already in use by {holder}. Stop that process or use a different checkout.",
                    build_dir.display()
                ));
            }
            Err(TryLockError::Error(err)) => return Err(io_error(err)),
        }
        // Acquiring the OS lock is the authority, not a PID probe: a reused
        // PID or an incomplete owner record cannot strand the workspace.
        let recovered = owner_path.exists().then(|| {
            let previous = read_owner(&owner_path)
                .map(|owner| owner.description())
                .unwrap_or_else(|| "an interrupted process".into());
            format!(
                "Recovered stale workspace claim left by {previous} at {}.",
                build_dir.display()
            )
        });
        let owner = Owner {
            pid: std::process::id(),
            command: command.to_string(),
            port,
        };
        let json = serde_json::to_vec(&owner).expect("workspace owner serializes");
        fs::write(&owner_path, json).map_err(io_error)?;
        Ok(Self {
            _file: file,
            owner_path,
            recovered,
        })
    }
}

impl Drop for WorkspaceLease {
    fn drop(&mut self) {
        // Remove only the diagnostic record, while the lock is still held.
        // Never unlink the lock inode: waiters may already have opened it.
        let _ = fs::remove_file(&self.owner_path);
    }
}
