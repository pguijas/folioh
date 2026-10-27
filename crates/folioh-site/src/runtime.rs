//! The Node toolchain behind the `FrontendRuntime` seam: preflight, `pnpm
//! install` skipped by lockfile hash, the two Nextra patches, `pnpm run build`
//! streamed to a log, and `next dev` with port handling. `NoopRuntime` stands
//! in for tests.

use std::io::{BufRead, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::fs::{hash_file, remove_dir_all_if_exists};
use crate::{Result, SiteError};

/// Next.js 16 needs Node 20.19.
pub const MIN_NODE_VERSION: (u32, u32) = (20, 19);
/// The template lockfile is pnpm 10.
pub const MIN_PNPM_VERSION: (u32, u32) = (10, 0);

/// What the site builder needs from the frontend toolchain.
pub trait FrontendRuntime {
    /// Install dependencies when needed, returning whether an install ran and
    /// what it printed (the binary shows it under `--verbose`). `on_phase`
    /// names what the step is doing as it does it: the check is quick and the
    /// install is not, and a row that says only "checking pnpm" for a minute
    /// tells the reader nothing about which of the two they are waiting on.
    fn install_deps(
        &self,
        template_dir: &Path,
        build_dir: &Path,
        on_phase: &mut dyn FnMut(&str),
    ) -> Result<InstallResult>;
    /// Run the static export, streaming every output line to `log_path` and `on_line`.
    fn build(&self, build_dir: &Path, log_path: &Path, on_line: &mut dyn FnMut(&str))
        -> Result<()>;
    /// Start the dev server; `None` when the runtime spawns nothing. The
    /// child's stdout and stderr are piped and relayed line by line to
    /// `on_line` from detached threads for the child's lifetime, so the caller
    /// never reads its stdio (an undrained pipe would block `next dev`).
    fn serve(
        &self,
        build_dir: &Path,
        port: u16,
        kill_existing: bool,
        on_line: Box<dyn FnMut(&str) + Send>,
    ) -> Result<Option<Child>>;
}

/// What `install_deps` did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstallResult {
    /// `pnpm install` ran (the step row says `installed`, else `up to date`).
    pub installed: bool,
    /// The install's stdout then stderr, as lines; empty when nothing ran.
    pub output: Vec<String>,
}

/// A runtime that installs, builds and serves nothing.
pub struct NoopRuntime;

impl FrontendRuntime for NoopRuntime {
    fn install_deps(&self, _: &Path, _: &Path, _: &mut dyn FnMut(&str)) -> Result<InstallResult> {
        Ok(InstallResult::default())
    }
    fn build(&self, _: &Path, _: &Path, _: &mut dyn FnMut(&str)) -> Result<()> {
        Ok(())
    }
    fn serve(
        &self,
        _: &Path,
        _: u16,
        _: bool,
        _: Box<dyn FnMut(&str) + Send>,
    ) -> Result<Option<Child>> {
        Ok(None)
    }
}

/// `v20.19.1-rc+x` -> `[20, 19, 1]`: numeric dot parts up to the first non-number.
pub fn parse_version(raw: &str) -> Vec<u32> {
    let core = raw.trim().trim_start_matches('v');
    let core = core
        .split('-')
        .next()
        .unwrap_or("")
        .split('+')
        .next()
        .unwrap_or("");
    core.split('.')
        .map_while(|part| part.parse().ok())
        .collect()
}

fn tool_version(command: &str) -> String {
    match run_with_timeout(
        Command::new(command).arg("--version"),
        Duration::from_secs(15),
    ) {
        Some(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => String::new(),
    }
}

fn on_path(command: &str) -> bool {
    if Path::new(command).components().count() > 1 {
        return Path::new(command).exists();
    }
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                dir.join(command).is_file() || dir.join(format!("{command}.cmd")).is_file()
            })
        })
        .unwrap_or(false)
}

fn too_old(version: &[u32], minimum: (u32, u32)) -> bool {
    !version.is_empty() && (version[0], version.get(1).copied().unwrap_or(0)) < minimum
}

/// Verify Node and pnpm exist at usable versions; every problem listed at once.
pub fn preflight_check() -> Result<()> {
    preflight_check_with("node", "pnpm")
}

/// `preflight_check` with explicit command names.
pub fn preflight_check_with(node: &str, pnpm: &str) -> Result<()> {
    let mut problems = Vec::new();
    if !on_path(node) {
        problems.push("Node.js was not found. The generated site is a Next.js app; install Node >= 20.19 from https://nodejs.org/".to_string());
    } else {
        let raw = tool_version(node);
        if too_old(&parse_version(&raw), MIN_NODE_VERSION) {
            problems.push(format!("Node.js {raw} is too old - the bundled template (Next.js 16) requires Node >= 20.19. Upgrade from https://nodejs.org/"));
        }
    }
    if !on_path(pnpm) {
        problems.push("pnpm was not found. Install it with: npm install -g pnpm".to_string());
    } else {
        let raw = tool_version(pnpm);
        if too_old(&parse_version(&raw), MIN_PNPM_VERSION) {
            problems.push(format!("pnpm {raw} is too old - the template lockfile requires pnpm >= 10. Upgrade with: npm install -g pnpm@10"));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(SiteError::Runtime(format!(
            "Environment check failed:\n  - {}",
            problems.join("\n  - ")
        )))
    }
}

/// Feed every line (newline kept, UTF-8 with replacement) of `reader` to `emit`.
fn relay_lines(reader: impl std::io::Read, mut emit: impl FnMut(String)) {
    let mut reader = std::io::BufReader::new(reader);
    let mut buffer = Vec::new();
    while let Ok(read) = reader.read_until(b'\n', &mut buffer) {
        if read == 0 {
            break;
        }
        emit(String::from_utf8_lossy(&buffer).into_owned());
        buffer.clear();
    }
}

fn run_with_timeout(command: &mut Command, timeout: Duration) -> Option<std::process::Output> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().ok(),
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return None,
        }
    }
}

/// Whether something accepts TCP connections on `localhost:port`.
pub fn is_port_in_use(port: u16) -> bool {
    ("localhost", port)
        .to_socket_addrs()
        .map(|addrs| {
            addrs
                .into_iter()
                .any(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok())
        })
        .unwrap_or(false)
}

/// Kill the server listening on `port` (`lsof -ti tcp:port -sTCP:LISTEN` +
/// SIGKILL); `true` when a process was killed. Only listening sockets are
/// selected: a browser tab, a `curl` or this process holding a connection to
/// the port is a client and is left alone. Always `false` off Unix.
pub fn kill_port(port: u16) -> bool {
    #[cfg(unix)]
    {
        let Ok(output) = Command::new("lsof")
            .args(["-ti", &format!("tcp:{port}"), "-sTCP:LISTEN"])
            .output()
        else {
            return false;
        };
        let pids: Vec<&str> = std::str::from_utf8(&output.stdout)
            .unwrap_or("")
            .split_whitespace()
            .collect();
        if pids.is_empty() {
            return false;
        }
        for pid in pids {
            if Command::new("kill")
                .args(["-9", pid])
                .status()
                .map(|s| !s.success())
                .unwrap_or(true)
            {
                return false;
            }
        }
        true
    }
    #[cfg(not(unix))]
    {
        let _ = port;
        false
    }
}

/// Remove a stale export and dev cache (they break Tailwind class hashing across modes).
pub fn remove_stale_build_artifacts(build_dir: &Path) -> std::io::Result<()> {
    remove_dir_all_if_exists(&build_dir.join("out"))?;
    remove_dir_all_if_exists(&build_dir.join(".next").join("dev"))
}

/// The real pnpm/next runtime.
pub struct NextRuntime {
    /// The pnpm executable; tests point it at a shim.
    pub pnpm: String,
    /// Run the Node/pnpm preflight before installing.
    pub preflight: bool,
}

impl Default for NextRuntime {
    fn default() -> Self {
        NextRuntime {
            pnpm: "pnpm".to_string(),
            preflight: true,
        }
    }
}

impl NextRuntime {
    fn pnpm(&self, build_dir: &Path) -> Command {
        let mut command = Command::new(&self.pnpm);
        command.current_dir(build_dir);
        command
    }

    fn has_working_next(&self, build_dir: &Path) -> bool {
        let binary = if cfg!(windows) { "next.cmd" } else { "next" };
        if !build_dir
            .join("node_modules")
            .join(".bin")
            .join(binary)
            .exists()
        {
            return false;
        }
        run_with_timeout(
            self.pnpm(build_dir).args(["exec", "next", "--version"]),
            Duration::from_secs(20),
        )
        .map(|output| output.status.success())
        .unwrap_or(false)
    }

    /// Files named `rel` under `node_modules/<pkg>` in the npm and pnpm layouts.
    fn package_files(node_modules: &Path, package: &str, rel: &str) -> Vec<PathBuf> {
        // ponytail: probes the npm and pnpm store layouts instead of walking
        // node_modules; a full walk is the upgrade if another manager appears.
        let mut found = Vec::new();
        let direct = node_modules.join(package).join(rel);
        if direct.is_file() {
            found.push(direct);
        }
        if let Ok(store) = std::fs::read_dir(node_modules.join(".pnpm")) {
            for entry in store.flatten() {
                let path = entry.path().join("node_modules").join(package).join(rel);
                if path.is_file() {
                    found.push(path);
                }
            }
        }
        found
    }

    fn patch_nextra_schema(build_dir: &Path) -> std::io::Result<()> {
        for path in Self::package_files(
            &build_dir.join("node_modules"),
            "nextra-theme-docs",
            "dist/schemas.js",
        ) {
            let content = std::fs::read_to_string(&path)?;
            let patched =
                content.replacen("children: reactNode,", "children: reactNode.optional(),", 1);
            if patched != content {
                std::fs::write(&path, patched)?;
            }
        }
        Ok(())
    }

    /// Pages carry a commit time only in a build, and generated content never
    /// does. In dev Nextra stamps every page with the time its loader loaded,
    /// and the theme then clones its LastUpdated element; React can still hold
    /// that element as a lazy reference while the dev debug channel streams,
    /// and the clone of a lazy reference has no type.
    fn patch_nextra_generated_content_timestamps(build_dir: &Path) -> std::io::Result<()> {
        let target =
            "const lastCommitTime = IS_PRODUCTION ? await getLastCommitTime(resourcePath) : NOW;";
        let current = "const lastCommitTime = IS_PRODUCTION && !isGeneratedFoliohContent ? await getLastCommitTime(resourcePath) : void 0;";
        let replacement = format!(
            "const isGeneratedFoliohContent = resourcePath.includes(`${{CWD}}/content/`);\n  {current}"
        );
        // A loader patched by 0.3.0-a1 still stamps dev pages.
        let earlier = "const lastCommitTime = IS_PRODUCTION ? isGeneratedFoliohContent ? void 0 : await getLastCommitTime(resourcePath) : NOW;";
        for path in Self::package_files(
            &build_dir.join("node_modules"),
            "nextra",
            "dist/server/loader.js",
        ) {
            let content = std::fs::read_to_string(&path)?;
            let patched = if content.contains("isGeneratedFoliohContent") {
                content.replacen(earlier, current, 1)
            } else {
                content.replacen(target, &replacement, 1)
            };
            if patched != content {
                std::fs::write(&path, patched)?;
            }
        }
        Ok(())
    }

    /// `serve` with injectable port probe and killer.
    pub fn serve_with(
        &self,
        build_dir: &Path,
        port: u16,
        kill_existing: bool,
        in_use: &dyn Fn(u16) -> bool,
        kill: &dyn Fn(u16) -> bool,
        on_line: Box<dyn FnMut(&str) + Send>,
    ) -> Result<Child> {
        if in_use(port) {
            if kill_existing {
                kill(port);
            } else {
                return Err(SiteError::Runtime(format!("Port {port} is already in use. Stop the existing process or rerun with --kill-existing.")));
            }
        }
        remove_stale_build_artifacts(build_dir).map_err(|e| SiteError::io(build_dir, e))?;
        let mut command = self.pnpm(build_dir);
        // The CLI owns this server tree. A terminal signal reaches the CLI,
        // which shuts down this separate group and reaps its direct child.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command
            .args([
                "exec",
                "next",
                "dev",
                "--turbopack",
                "--port",
                &port.to_string(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SiteError::Runtime(format!("failed to start next dev: {e}")))?;
        let sink = std::sync::Arc::new(std::sync::Mutex::new(on_line));
        for reader in [
            child
                .stdout
                .take()
                .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
            child
                .stderr
                .take()
                .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
        ]
        .into_iter()
        .flatten()
        {
            let sink = std::sync::Arc::clone(&sink);
            std::thread::spawn(move || {
                relay_lines(reader, |line| {
                    if let Ok(mut emit) = sink.lock() {
                        emit(&line);
                    }
                })
            });
        }
        Ok(child)
    }
}

impl FrontendRuntime for NextRuntime {
    fn install_deps(
        &self,
        template_dir: &Path,
        build_dir: &Path,
        on_phase: &mut dyn FnMut(&str),
    ) -> Result<InstallResult> {
        if self.preflight {
            on_phase("checking node and pnpm");
            preflight_check_with("node", &self.pnpm)?;
        }
        on_phase("comparing the lockfile");
        let node_modules = build_dir.join("node_modules");
        let complete = node_modules.exists() && self.has_working_next(build_dir);
        let installed = !complete
            || hash_file(&template_dir.join("pnpm-lock.yaml"))
                != hash_file(&build_dir.join("pnpm-lock.yaml"));
        let mut output = Vec::new();
        if installed {
            if node_modules.exists() && !complete {
                on_phase("clearing an incomplete node_modules");
                remove_dir_all_if_exists(&node_modules)
                    .map_err(|e| SiteError::io(&node_modules, e))?;
            }
            on_phase(if node_modules.exists() {
                "pnpm install, updating node_modules"
            } else {
                "pnpm install, first run in this workspace"
            });
            let result = self
                .pnpm(build_dir)
                .args(["install", "--frozen-lockfile"])
                .stdin(Stdio::null())
                .output()
                .map_err(|e| SiteError::Runtime(format!("pnpm install failed:\n{e}\n")))?;
            let stdout = String::from_utf8_lossy(&result.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
            if !result.status.success() {
                return Err(SiteError::Runtime(format!(
                    "pnpm install failed:\n{stderr}\n{stdout}"
                )));
            }
            output.extend(stdout.lines().chain(stderr.lines()).map(str::to_string));
        }
        Self::patch_nextra_schema(build_dir).map_err(|e| SiteError::io(build_dir, e))?;
        Self::patch_nextra_generated_content_timestamps(build_dir)
            .map_err(|e| SiteError::io(build_dir, e))?;
        Ok(InstallResult { installed, output })
    }

    fn build(
        &self,
        build_dir: &Path,
        log_path: &Path,
        on_line: &mut dyn FnMut(&str),
    ) -> Result<()> {
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SiteError::io(parent, e))?;
        }
        remove_stale_build_artifacts(build_dir).map_err(|e| SiteError::io(build_dir, e))?;
        let mut log = std::fs::File::create(log_path).map_err(|e| SiteError::io(log_path, e))?;
        let mut child = self
            .pnpm(build_dir)
            .args(["run", "build"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SiteError::Runtime(format!("pnpm build failed:\n{e}\n")))?;
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let (sender, receiver) = std::sync::mpsc::channel::<String>();
        let mut lines = Vec::new();
        std::thread::scope(|scope| {
            let out_sender = sender.clone();
            scope.spawn(move || relay_lines(stdout, |line| drop(out_sender.send(line))));
            scope.spawn(move || relay_lines(stderr, |line| drop(sender.send(line))));
            for line in receiver {
                let _ = log.write_all(line.as_bytes());
                let _ = log.flush();
                on_line(&line);
                lines.push(line);
            }
        });
        let status = child
            .wait()
            .map_err(|e| SiteError::Runtime(format!("pnpm build failed:\n{e}\n")))?;
        if !status.success() {
            return Err(SiteError::Build {
                output: lines,
                log_path: log_path.to_path_buf(),
            });
        }
        Ok(())
    }

    fn serve(
        &self,
        build_dir: &Path,
        port: u16,
        kill_existing: bool,
        on_line: Box<dyn FnMut(&str) + Send>,
    ) -> Result<Option<Child>> {
        self.serve_with(
            build_dir,
            port,
            kill_existing,
            &is_port_in_use,
            &kill_port,
            on_line,
        )
        .map(Some)
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
