//! Black-box runs of the built binary: the help surface, `--version`, the
//! usage-error exit code, `init` end to end, `clean`, the hidden
//! `github-pages` steps and the missing-config errors of the pipeline commands.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const BANNER_LINE: &str = "████████╗ ██████╗ ██╗";

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

impl Run {
    /// Stderr without the no-op runtime's own warning: what this command
    /// reported.
    fn diagnostics(&self) -> String {
        self.stderr
            .lines()
            .filter(|l| !l.contains("FOLIOH_FRONTEND_RUNTIME=noop"))
            .map(|l| format!("{l}\n"))
            .collect()
    }
}

#[path = "common/env.rs"]
mod scrub;

fn folioh(cwd: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_folioh"));
    // The surface tests pin exact stdout; the scrub keeps them free of the
    // machine's Folioh and Node state.
    scrub::scrub(cmd.current_dir(cwd));
    cmd
}

/// Run with `stdin` piped in (`None` closes stdin at once).
fn run(cmd: &mut Command, stdin: Option<&str>) -> Run {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.stdin(if stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    let mut child = cmd.spawn().expect("folioh binary");
    if let Some(input) = stdin {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
    }
    let output = child.wait_with_output().unwrap();
    Run {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}

fn folioh_run(cwd: &Path, args: &[&str], stdin: Option<&str>) -> Run {
    run(folioh(cwd).args(args), stdin)
}

fn yaml(path: &Path) -> serde_yaml_ng::Value {
    serde_yaml_ng::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn news_line_present(output: &str) -> bool {
    output.lines().any(|line| {
        let line = line.trim();
        line.starts_with("· ") && line.ends_with(" ·") && line.len() > 4
    })
}

#[test]
fn version_help_and_usage_errors() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path();

    let version = folioh_run(cwd, &["--version"], None);
    assert_eq!(
        (version.code, version.stdout.as_str()),
        (0, format!("folioh {VERSION}\n").as_str())
    );
    let short = folioh_run(cwd, &["-V"], None);
    assert_eq!(short.stdout, format!("folioh {VERSION}\n"));
    // Eager: the version wins over a subcommand that follows it.
    let eager = folioh_run(cwd, &["--version", "build"], None);
    assert_eq!(
        (eager.code, eager.stdout.as_str()),
        (0, format!("folioh {VERSION}\n").as_str())
    );

    let help = folioh_run(cwd, &["--help"], None);
    assert_eq!(help.code, 0, "{}", help.stderr);
    assert!(help.stdout.contains("Usage: folioh"));
    assert!(help
        .stdout
        .contains("Build documentation from source and guides."));
    assert!(help.stdout.contains("-V, --version"));
    assert!(help.stdout.contains("Show version"));
    assert!(help.stdout.contains("--update"));
    assert!(help
        .stdout
        .contains("Update folioh to the latest release and exit"));
    let positions: Vec<usize> = [
        "\n  init ",
        "\n  build ",
        "\n  serve ",
        "\n  coverage ",
        "\n  clean ",
        "\n  roadmap ",
    ]
    .iter()
    .map(|name| {
        help.stdout
            .find(name)
            .unwrap_or_else(|| panic!("{name:?} listed in\n{}", help.stdout))
    })
    .collect();
    assert!(
        positions.windows(2).all(|w| w[0] < w[1]),
        "commands in the guide's order"
    );
    assert!(!help.stdout.contains("build-versions"));
    assert!(!help.stdout.contains("github-pages"));
    assert!(!help.stdout.contains("completion"));
    assert!(
        !help.stdout.contains("\n  board "),
        "the docs CLI ships no board command"
    );

    let bare = folioh_run(cwd, &[], None);
    assert_eq!(bare.code, 2);
    assert!(bare.stdout.contains("Usage: folioh"), "{}", bare.stdout);
    assert!(
        bare.stdout.contains("\n  init "),
        "help on stdout, as typer printed it"
    );
    assert!(bare.stderr.is_empty(), "{}", bare.stderr);

    let unknown = folioh_run(cwd, &["nope"], None);
    assert_eq!(unknown.code, 2);
    assert!(unknown.stderr.contains("nope"));
    let bad_value = folioh_run(cwd, &["serve", "--port", "abc"], None);
    assert_eq!(bad_value.code, 2);
    assert!(bad_value.stderr.contains("abc"));
    let bad_flag = folioh_run(cwd, &["build", "--no-clean"], None);
    assert_eq!(bad_flag.code, 2);
    // `--port` only means something with `--open`; alone it is refused.
    let lone_port = folioh_run(cwd, &["build", "--port", "9000"], None);
    assert_eq!(lone_port.code, 2);
    assert!(lone_port.stderr.contains("--open"), "{}", lone_port.stderr);

    let init_help = folioh_run(cwd, &["init", "--help"], None);
    assert_eq!(init_help.code, 0);
    assert!(init_help
        .stdout
        .contains("Initialize a new Folioh documentation project."));
    assert!(init_help.stdout.contains("[DIRECTORY]"));
    assert!(init_help
        .stdout
        .contains("Project directory (defaults to cwd)"));
    assert!(init_help.stdout.contains("-y, --yes"));
    assert!(init_help
        .stdout
        .contains("Skip prompts, use detected defaults"));

    let build_help = folioh_run(cwd, &["build", "--help"], None);
    assert!(build_help.stdout.contains("-o, --open"));
    assert!(build_help
        .stdout
        .contains("Starts a static preview in the browser and blocks until interrupted"));
    assert!(build_help.stdout.contains("--project-dir"));
    assert!(build_help
        .stdout
        .contains("Compatibility option for scripts that prefer named arguments"));
    assert!(build_help.stdout.contains("[default: docs.yaml]"));
    let serve_help = folioh_run(cwd, &["serve", "--help"], None);
    assert!(serve_help.stdout.contains("-p, --port"));
    assert!(serve_help.stdout.contains("[default: 4321]"));
    assert!(serve_help.stdout.contains("--kill-existing"));
    assert!(serve_help.stdout.contains("--previews"));
    assert!(!serve_help.stdout.contains("--versions"), "hidden option");
    let coverage_help = folioh_run(cwd, &["coverage", "--help"], None);
    assert!(coverage_help
        .stdout
        .contains("Analyze documentation coverage of Python source files."));
    assert!(coverage_help
        .stdout
        .contains("Minimum coverage percentage (exit 1 if below)"));
    assert!(coverage_help
        .stdout
        .contains("List each undocumented symbol"));
    let roadmap_help = folioh_run(cwd, &["roadmap", "--help"], None);
    assert!(roadmap_help
        .stdout
        .contains("Preview source-defined roadmap phases."));
    let versions_help = folioh_run(cwd, &["build-versions", "--help"], None);
    assert_eq!(versions_help.code, 0);
    assert!(versions_help
        .stdout
        .contains("Build docs for all configured versions."));

    let pages_bare = folioh_run(cwd, &["github-pages"], None);
    assert_eq!(pages_bare.code, 2);
    assert!(
        pages_bare.stdout.contains("compute-preview-path"),
        "{}",
        pages_bare.stdout
    );
    assert!(pages_bare.stderr.is_empty());
    let pages_help = folioh_run(cwd, &["github-pages", "--help"], None);
    assert_eq!(pages_help.code, 0);
    for step in [
        "compute-preview-path",
        "preserve-previews",
        "prepare-artifact",
        "copy-branch-preview",
        "write-preview-metadata",
        "write-previews-data",
        "prune-previews",
        "save-state",
        "verify-url",
        "comment-preview",
    ] {
        // Every step is listed with its one-line description.
        let row = pages_help
            .stdout
            .lines()
            .find(|line| line.trim_start().starts_with(&format!("{step} ")))
            .unwrap_or_else(|| panic!("{step} in\n{}", pages_help.stdout));
        assert!(
            row.len() > step.len() + 10,
            "{step} has no description: {row:?}"
        );
    }
    assert!(pages_help
        .stdout
        .starts_with("Steps of the GitHub Pages workflows that folioh init writes."));
    assert!(!pages_help.stdout.contains('`'), "no raw doc comment");
    let missing_option = folioh_run(
        cwd,
        &["github-pages", "compute-preview-path", "--head-ref", "x"],
        None,
    );
    assert_eq!(missing_option.code, 2);
}

#[test]
fn init_yes_scaffolds_an_empty_directory_and_prints_the_ready_panel() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path();
    fs::create_dir(cwd.join("demo")).unwrap();

    let result = folioh_run(cwd, &["init", "demo", "--yes"], None);
    assert_eq!(result.code, 0, "{}{}", result.stdout, result.stderr);
    assert!(result.stderr.is_empty());
    let out = &result.stdout;

    // Banner: blank, six centred art lines with the version, blank, news.
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "");
    assert!(
        lines[1].starts_with(&format!("{} {BANNER_LINE}", " ".repeat(14))),
        "{:?}",
        lines[1]
    );
    assert!(
        lines[6].ends_with(&format!("╚═╝ v{VERSION}")),
        "{:?}",
        lines[6]
    );
    assert_eq!(lines[7], "");
    assert!(news_line_present(out));
    assert!(!out.contains(&format!("Folioh v{VERSION}")));

    // Detected panel, centred, seven rows.
    let margin = " ".repeat(16);
    assert!(out.contains(&format!(
        "{margin}╭{} Detected {}╮\n",
        "─".repeat(17),
        "─".repeat(18)
    )));
    assert!(out.contains(&format!("{margin}│ Target     demo{} │\n", " ".repeat(28))));
    assert!(out.contains("│ Project    demo 0.1.0 "));
    assert!(out.contains("│ Source     no sources detected "));
    assert!(out.contains("│ Repo       not detected "));
    assert!(out.contains("│ Python     not detected "));
    assert!(out.contains("│ Framework  not detected "));
    assert!(out.contains("│ Status     Documentation scaffold not found │"));

    // Ready panel: hugs its widest line (56 columns here), left aligned.
    assert!(out.contains(&format!(
        "\n╭{} Ready {}╮\n",
        "─".repeat(23),
        "─".repeat(24)
    )));
    assert!(out.contains(&format!(
        "│ ✓ Documentation project ready.{} │\n",
        " ".repeat(22)
    )));
    assert!(out.contains("│ Files "));
    assert!(out.contains("│   Created demo/docs.yaml "));
    assert!(out.contains("│   Created demo/docs/index.md "));
    assert!(out.contains("│   Created demo/.github/workflows/pages.yml "));
    assert!(out.contains("│   Created demo/.github/workflows/branch-previews.yml "));
    assert!(out.contains("│ Next commands "));
    assert!(out.contains("│   folioh serve demo "));
    assert!(out.contains("│   folioh build demo "));
    assert!(
        !out.contains("folioh coverage"),
        "coverage reads Python only; none here"
    );
    assert!(out.ends_with(&format!("╰{}╯\n", "─".repeat(54))));
    for stale in [
        "Project scan",
        "Initialize docs from your Python project",
        "Folioh will write docs.yaml",
        "______ ____",
        "Docstring style",
    ] {
        assert!(!out.contains(stale), "{stale}");
    }

    let expected = r#"# Folioh configuration
# Full reference: https://folioh.site/docs/configuration

project:
  name: "demo"
  version: "0.1.0"
  # repo: "https://github.com/owner/repo"

source:
  # No Python sources detected; uncomment to publish an API reference.
  # python:
  #   paths:
  #     - "src/"
  docs:
    - "docs/"

output: "_site"

theme:
  preset: "pastel"
  dark_mode: true
  # logo: "docs/logo.png"
  # favicon: "docs/favicon.ico"

llm:
  generate_llms_txt: true
  generate_llms_full_txt: true
"#;
    assert_eq!(
        fs::read_to_string(cwd.join("demo/docs.yaml")).unwrap(),
        expected
    );
    assert_eq!(
        fs::read_to_string(cwd.join("demo/docs/index.md")).unwrap(),
        "# demo\n\nWelcome to the documentation.\n"
    );
    let pages = fs::read_to_string(cwd.join("demo/.github/workflows/pages.yml")).unwrap();
    assert!(pages.starts_with("name: Deploy Docs\n"));
    assert!(pages.contains("curl -LsSf https://folioh.site/install.sh | sh"));
    assert!(!pages.contains("uv "));
    let previews =
        fs::read_to_string(cwd.join("demo/.github/workflows/branch-previews.yml")).unwrap();
    assert!(previews.starts_with("name: Deploy Branch Preview Docs\n"));
    assert!(previews.contains("folioh github-pages comment-preview"));
    assert!(!cwd.join("demo/.github/workflows/folioh-pages.yml").exists());
    assert!(!cwd
        .join("demo/.github/workflows/branch-preview-cleanup.yml")
        .exists());
    let parsed = yaml(&cwd.join("demo/docs.yaml"));
    assert!(parsed.get("landing").is_none());
    assert!(parsed.get("plugins").is_none());

    // The cwd itself: Target is `.`, paths and commands carry no suffix.
    let here = tempfile::tempdir().unwrap();
    let result = folioh_run(here.path(), &["init", "--yes"], None);
    assert_eq!(result.code, 0, "{}", result.stdout);
    assert!(result.stdout.contains("│ Target     . "));
    assert!(result.stdout.contains("│   Created docs.yaml "));
    assert!(result.stdout.contains("│   folioh serve  "));
    assert!(here.path().join("docs.yaml").is_file());
}

#[test]
fn init_only_asks_about_docstrings_when_python_is_detected() {
    for (language, files, answers, has_python) in [
        (
            "rust",
            vec![
                (
                    "Cargo.toml",
                    "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n",
                ),
                ("src/lib.rs", "//! Demo.\n"),
            ],
            "2\n",
            false,
        ),
        (
            "javascript",
            vec![
                ("package.json", "{\"name\":\"demo\"}"),
                ("src/index.js", "export const x = 1;\n"),
            ],
            "2\n",
            false,
        ),
        ("python", vec![("src/demo.py", "x = 1\n")], "2\n2\n", true),
        (
            "mixed",
            vec![
                ("src/demo.py", "x = 1\n"),
                (
                    "Cargo.toml",
                    "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n",
                ),
                ("src/lib.rs", "//! Demo.\n"),
            ],
            "2\n2\n",
            true,
        ),
        ("empty", vec![], "2\n", false),
    ] {
        let dir = tempfile::tempdir().unwrap();
        for (name, contents) in files {
            let path = dir.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
        let result = folioh_run(dir.path(), &["init"], Some(answers));
        assert_eq!(
            result.code, 0,
            "{language}: {}{}",
            result.stdout, result.stderr
        );
        assert_eq!(
            result.stdout.contains("Docstring style"),
            has_python,
            "{language}"
        );
        let config = yaml(&dir.path().join("docs.yaml"));
        assert_eq!(
            config["theme"]["preset"].as_str(),
            Some("beacon"),
            "{language}"
        );
        assert_eq!(
            config["source"]["python"]["docstring_style"].as_str(),
            has_python.then_some("numpy"),
            "{language}"
        );

        fs::remove_file(dir.path().join("docs.yaml")).unwrap();
        let defaults = folioh_run(dir.path(), &["init", "--yes"], None);
        assert_eq!(
            defaults.code, 0,
            "{language}: {}{}",
            defaults.stdout, defaults.stderr
        );
        assert!(!defaults.stdout.contains("Docstring style"));
        let config = yaml(&dir.path().join("docs.yaml"));
        assert_eq!(config["theme"]["preset"].as_str(), Some("pastel"));
        assert!(config["source"]["python"].get("docstring_style").is_none());
    }
}

#[test]
fn init_with_piped_answers_uses_the_numbered_prompts() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path();

    // Without Python, Enter accepts only the visual preset default.
    fs::create_dir(cwd.join("a")).unwrap();
    let result = folioh_run(cwd, &["init", "a"], Some("\n"));
    assert_eq!(result.code, 0, "{}{}", result.stdout, result.stderr);
    let out = &result.stdout;
    assert!(out.contains(BANNER_LINE));
    assert!(news_line_present(out));
    assert!(out.contains(&format!("v{VERSION}")));
    for row in [
        "Detected",
        "Target",
        "Project",
        "Source",
        "Repo",
        "Python",
        "Framework",
        "Status",
        "Documentation scaffold not found",
    ] {
        assert!(out.contains(row), "{row}");
    }
    let prompts = "\n  Visual preset\n  1. Roller\n     Fine headings, generous spacing, and cobalt ink.\n  2. Beacon\n     Bright product docs with clear contrast.\n  3. Letterpress\n     Bold serif headings on a paper surface.\n  4. Notebook\n     Framed pages with warm surfaces and green accents.\n  5. Folioh Pastel (default)\n     Pen-blue ink, soft colours, and organic shapes.\n  Select (5): \n  ✔ Created a/docs.yaml\n  ✔ Created a/docs/index.md\n  ✔ Created a/.github/workflows/pages.yml\n  ✔ Created a/.github/workflows/branch-previews.yml\n  Next folioh serve a\n";
    assert!(out.ends_with(prompts), "{out}");
    assert!(!out.contains("Ready"), "compact output, no panel");
    assert!(!out.contains("🏠 Starter landing page"));
    let parsed = yaml(&cwd.join("a/docs.yaml"));
    assert_eq!(parsed["theme"]["preset"].as_str(), Some("pastel"));
    assert!(!fs::read_to_string(cwd.join("a/docs.yaml"))
        .unwrap()
        .contains("docstring_style"));

    // Explicit picks by number, with a Python source root present.
    fs::create_dir_all(cwd.join("b/src")).unwrap();
    fs::write(cwd.join("b/src/mod.py"), "x = 1\n").unwrap();
    let result = folioh_run(cwd, &["init", "b"], Some("2\n2\n"));
    assert_eq!(result.code, 0, "{}", result.stdout);
    let config = fs::read_to_string(cwd.join("b/docs.yaml")).unwrap();
    assert!(config
        .contains("  python:\n    paths:\n      - \"src/\"\n    docstring_style: \"numpy\"\n"));
    assert!(config.contains("\"API Reference\""));
    assert_eq!(
        yaml(&cwd.join("b/docs.yaml"))["theme"]["preset"].as_str(),
        Some("beacon")
    );
    assert!(result.stdout.contains("│ Source     src/ "));

    // An answer by value, and an invalid answer that is asked again.
    fs::create_dir(cwd.join("c")).unwrap();
    let result = folioh_run(cwd, &["init", "c"], Some("nope\nworkshop\n"));
    assert_eq!(result.code, 0, "{}", result.stdout);
    assert!(result
        .stdout
        .contains("  Select (5): Please select one of the available options\n  Select (5): "));
    let parsed = yaml(&cwd.join("c/docs.yaml"));
    assert_eq!(parsed["theme"]["preset"].as_str(), Some("workshop"));
    assert!(
        fs::read_to_string(cwd.join("c/docs.yaml"))
            .unwrap()
            .contains("# python:"),
        "no sources: no docstring_style line"
    );

    // Roller remains available as an explicit choice at its original number.
    fs::create_dir(cwd.join("roller")).unwrap();
    let result = folioh_run(cwd, &["init", "roller"], Some("1\n"));
    assert_eq!(result.code, 0, "{}{}", result.stdout, result.stderr);
    assert_eq!(
        yaml(&cwd.join("roller/docs.yaml"))["theme"]["preset"].as_str(),
        Some("organic-editorial")
    );

    // EOF on stdin cancels before anything is written.
    fs::create_dir(cwd.join("d")).unwrap();
    let result = folioh_run(cwd, &["init", "d"], None);
    assert_eq!(result.code, 1);
    assert!(
        result
            .stdout
            .ends_with("  Select (5): \nInit aborted: stdin ended before an answer. No files changed; rerun with --yes to take the defaults.\n"),
        "{}",
        result.stdout
    );
    assert!(!cwd.join("d/docs.yaml").exists());
    assert!(!cwd.join("d/docs").exists());

    // `CI` set with a terminal would also take this path; piped stdin already does.
    fs::create_dir(cwd.join("e")).unwrap();
    let result = run(folioh(cwd).args(["init", "e"]).env("CI", "1"), Some("2\n"));
    assert_eq!(result.code, 0);
    assert_eq!(
        yaml(&cwd.join("e/docs.yaml"))["theme"]["preset"].as_str(),
        Some("beacon")
    );
}

#[test]
fn init_prints_created_paths_relative_to_the_current_directory() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path();
    fs::create_dir_all(cwd.join("tmp/sample-init-check")).unwrap();

    let result = folioh_run(cwd, &["init", "tmp/sample-init-check"], Some("2\n"));
    assert_eq!(result.code, 0, "{}", result.stdout);
    assert!(result
        .stdout
        .contains("✔ Created tmp/sample-init-check/docs.yaml"));
    assert!(!result.stdout.contains("✔ Created docs.yaml"));
    assert!(result
        .stdout
        .contains("Next folioh serve tmp/sample-init-check"));
    assert!(result
        .stdout
        .contains("│ Target     tmp/sample-init-check "));

    // A name that needs quoting in a shell is quoted in the suffix.
    fs::create_dir_all(cwd.join("my project")).unwrap();
    let result = folioh_run(cwd, &["init", "my project", "--yes"], None);
    assert_eq!(result.code, 0, "{}", result.stdout);
    assert!(result.stdout.contains("│   folioh serve 'my project' "));
    assert!(result.stdout.contains("│   Created my project/docs.yaml "));
}

#[test]
fn init_reads_pyproject_git_and_the_source_layout() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path();

    // Name, version, framework, requires-python, src/<pkg>, git remote.
    let aurora = cwd.join("aurora");
    fs::create_dir_all(aurora.join("src/aurora_labs")).unwrap();
    fs::write(
        aurora.join("pyproject.toml"),
        "[project]\nname = \"aurora-labs\"\nversion = \"1.8.4\"\nrequires-python = \">=3.10\"\ndependencies = [\"typer>=0.9\"]\n",
    )
    .unwrap();
    let git = |args: &[&str]| {
        let status = Command::new("git")
            .args(args)
            .current_dir(&aurora)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    };
    git(&["init", "-q"]);
    git(&[
        "remote",
        "add",
        "origin",
        "git@github.com:acme/aurora-labs.git",
    ]);
    let result = folioh_run(cwd, &["init", "aurora"], Some("\n\n"));
    assert_eq!(result.code, 0, "{}", result.stdout);
    let parsed = yaml(&aurora.join("docs.yaml"));
    assert_eq!(parsed["project"]["name"].as_str(), Some("aurora-labs"));
    assert_eq!(parsed["project"]["version"].as_str(), Some("1.8.4"));
    assert_eq!(
        parsed["project"]["repo"].as_str(),
        Some("https://github.com/acme/aurora-labs")
    );
    assert_eq!(
        parsed["source"]["python"]["paths"],
        serde_yaml_ng::to_value(vec!["src/aurora_labs"]).unwrap()
    );
    assert!(result.stdout.contains("│ Project    aurora-labs 1.8.4 "));
    assert!(result
        .stdout
        .contains("│ Source     src/aurora_labs (Python) "));
    assert!(result
        .stdout
        .contains("│ Repo       https://github.com/acme/aurora-labs "));
    assert!(result.stdout.contains("│ Python     >=3.10 "));
    assert!(result.stdout.contains("│ Framework  Typer package "));
    assert!(!result.stdout.contains("Project name"));
    assert!(!result.stdout.contains("Use detected project settings"));

    // Hostile metadata is quoted, never interpreted as YAML structure.
    let hostile = "x\"\nplugins: [\"docs/evil.py\"]\ntrailing:\n  key: \"y";
    for field in ["name", "version"] {
        let project = cwd.join(format!("hostile-{field}"));
        fs::create_dir(&project).unwrap();
        let (name, version) = if field == "name" {
            (hostile, "0.1.0")
        } else {
            ("safe-name", hostile)
        };
        fs::write(
            project.join("pyproject.toml"),
            format!(
                "[project]\nname = {}\nversion = {}\n",
                serde_json::to_string(name).unwrap(),
                serde_json::to_string(version).unwrap()
            ),
        )
        .unwrap();
        let result = folioh_run(
            cwd,
            &[
                "init",
                project.file_name().unwrap().to_str().unwrap(),
                "--yes",
            ],
            None,
        );
        assert_eq!(result.code, 0, "{}", result.stdout);
        let parsed = yaml(&project.join("docs.yaml"));
        assert!(parsed.get("plugins").is_none());
        assert!(parsed.get("trailing").is_none());
        assert_eq!(parsed["project"][field].as_str(), Some(hostile));
    }

    // Wrong types warn and keep the defaults.
    let typed = cwd.join("typed");
    fs::create_dir(&typed).unwrap();
    fs::write(
        typed.join("pyproject.toml"),
        "[project]\nname = 123\nversion = true\n",
    )
    .unwrap();
    let result = folioh_run(cwd, &["init", "typed", "--yes"], None);
    assert_eq!(result.code, 0, "{}", result.stdout);
    assert!(result.diagnostics().starts_with(
        "Warning: Could not read project metadata from pyproject.toml: project.name must be a string\n\
         Warning: Could not read project metadata from pyproject.toml: project.version must be a string\n"
    ));
    assert!(fs::read_to_string(typed.join("docs.yaml"))
        .unwrap()
        .contains("name: \"typed\"\n"));
    assert!(result.stdout.contains("│ Framework  Python package "));

    // Unparsable TOML warns and init still succeeds.
    let broken = cwd.join("broken");
    fs::create_dir(&broken).unwrap();
    fs::write(broken.join("pyproject.toml"), "[project\n").unwrap();
    let result = folioh_run(cwd, &["init", "broken", "--yes"], None);
    assert_eq!(result.code, 0, "{}", result.stdout);
    assert!(result
        .diagnostics()
        .starts_with("Warning: Could not read project metadata from pyproject.toml: "));
    assert!(broken.join("docs.yaml").is_file());

    // `[project]` that is not a table.
    let scalar = cwd.join("scalar");
    fs::create_dir(&scalar).unwrap();
    fs::write(scalar.join("pyproject.toml"), "project = 1\n").unwrap();
    let result = folioh_run(cwd, &["init", "scalar", "--yes"], None);
    assert!(result.diagnostics().starts_with(
        "Warning: Could not read project metadata from pyproject.toml: [project] must be a table\n"
    ));

    // Flat layouts: `<pkg>/` and `<name>/`.
    let flat = cwd.join("flat-pkg");
    fs::create_dir_all(flat.join("flat_pkg")).unwrap();
    folioh_run(cwd, &["init", "flat-pkg", "--yes"], None);
    assert!(fs::read_to_string(flat.join("docs.yaml"))
        .unwrap()
        .contains("      - \"flat_pkg/\"\n"));
    let named = cwd.join("named");
    fs::create_dir_all(named.join("named")).unwrap();
    folioh_run(cwd, &["init", "named", "--yes"], None);
    assert!(fs::read_to_string(named.join("docs.yaml"))
        .unwrap()
        .contains("      - \"named/\"\n"));

    // git missing from PATH: a warning, and init still completes.
    let nogit = cwd.join("nogit");
    fs::create_dir(&nogit).unwrap();
    let result = run(
        folioh(cwd)
            .args(["init", "nogit", "--yes"])
            .env("PATH", "")
            .env_remove("FOLIOH_FRONTEND_RUNTIME"),
        None,
    );
    assert_eq!(result.code, 0, "{}{}", result.stdout, result.stderr);
    // Without a PATH (and with the real runtime selected) the toolchain
    // preflight warns first and init goes on.
    assert!(
        result
            .diagnostics()
            .starts_with("Environment check failed:\n  - Node.js was not found."),
        "{}",
        result.stderr
    );
    assert!(
        result
            .diagnostics()
            .contains("\nWarning: Could not detect git remote: "),
        "{}",
        result.stderr
    );
    assert!(nogit.join("docs.yaml").is_file());
}

#[test]
fn init_detects_rust_and_javascript_projects() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path();

    // A crate: its name, version and `src/`; `src/` is not claimed for Python.
    let krate = cwd.join("krate");
    fs::create_dir_all(krate.join("src")).unwrap();
    fs::write(
        krate.join("Cargo.toml"),
        "[package]\nname = \"demo-crate\"\nversion = \"0.4.2\"\n",
    )
    .unwrap();
    fs::write(krate.join("src/lib.rs"), "//! Demo.\n").unwrap();
    let result = folioh_run(cwd, &["init", "krate", "--yes"], None);
    assert_eq!(result.code, 0, "{}{}", result.stdout, result.stderr);
    let parsed = yaml(&krate.join("docs.yaml"));
    assert_eq!(parsed["project"]["name"].as_str(), Some("demo-crate"));
    assert_eq!(parsed["project"]["version"].as_str(), Some("0.4.2"));
    assert_eq!(
        parsed["source"]["rust"]["paths"],
        serde_yaml_ng::to_value(vec!["src/"]).unwrap()
    );
    assert!(parsed["source"].get("python").is_none());
    assert_eq!(
        parsed["nav"],
        serde_yaml_ng::to_value(vec!["API Reference"]).unwrap()
    );
    let raw = fs::read_to_string(krate.join("docs.yaml")).unwrap();
    assert!(!raw.contains("# python:"), "{raw}");
    assert!(result.stdout.contains("│ Project    demo-crate 0.4.2 "));
    assert!(result.stdout.contains("│ Source     src/ (Rust) "));
    assert!(result.stdout.contains("│ Framework  Rust crate "));
    assert!(!result.stdout.contains("folioh coverage"));

    // A workspace is read from its root.
    let workspace = cwd.join("workspace");
    fs::create_dir_all(workspace.join("crates/core/src")).unwrap();
    fs::write(
        workspace.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n[workspace.package]\nversion = \"2.0.0\"\n",
    )
    .unwrap();
    let result = folioh_run(cwd, &["init", "workspace", "--yes"], None);
    assert_eq!(result.code, 0, "{}", result.stdout);
    let parsed = yaml(&workspace.join("docs.yaml"));
    assert_eq!(parsed["project"]["name"].as_str(), Some("workspace"));
    assert_eq!(parsed["project"]["version"].as_str(), Some("2.0.0"));
    assert_eq!(
        parsed["source"]["rust"]["paths"],
        serde_yaml_ng::to_value(vec!["./"]).unwrap()
    );
    assert!(result.stdout.contains("│ Framework  Rust workspace "));

    // A package.json with JavaScript under `src/`.
    let web = cwd.join("web");
    fs::create_dir_all(web.join("src")).unwrap();
    fs::create_dir_all(web.join("node_modules/dep")).unwrap();
    fs::write(web.join("node_modules/dep/index.js"), "").unwrap();
    fs::write(web.join("src/index.js"), "export const x = 1;\n").unwrap();
    fs::write(
        web.join("package.json"),
        r#"{"name": "@acme/web", "version": "3.1.0"}"#,
    )
    .unwrap();
    let result = folioh_run(cwd, &["init", "web", "--yes"], None);
    assert_eq!(result.code, 0, "{}", result.stdout);
    let parsed = yaml(&web.join("docs.yaml"));
    assert_eq!(parsed["project"]["name"].as_str(), Some("@acme/web"));
    assert_eq!(parsed["project"]["version"].as_str(), Some("3.1.0"));
    assert_eq!(
        parsed["source"]["javascript"]["paths"],
        serde_yaml_ng::to_value(vec!["src/"]).unwrap()
    );
    assert!(parsed["source"].get("python").is_none());
    assert!(result.stdout.contains("│ Source     src/ (JavaScript) "));
    assert!(result.stdout.contains("│ Framework  JavaScript package "));

    // Mixed: a Python package beside a crate gets both sources.
    let mixed = cwd.join("mixed");
    fs::create_dir_all(mixed.join("mixed")).unwrap();
    fs::write(mixed.join("mixed/__init__.py"), "").unwrap();
    fs::create_dir_all(mixed.join("src")).unwrap();
    fs::write(mixed.join("src/lib.rs"), "").unwrap();
    fs::write(
        mixed.join("pyproject.toml"),
        "[project]\nname = \"mixed\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    fs::write(
        mixed.join("Cargo.toml"),
        "[package]\nname = \"mixed-core\"\nversion = \"9.9.9\"\n",
    )
    .unwrap();
    let result = folioh_run(cwd, &["init", "mixed", "--yes"], None);
    assert_eq!(result.code, 0, "{}", result.stdout);
    let parsed = yaml(&mixed.join("docs.yaml"));
    assert_eq!(
        parsed["project"]["name"].as_str(),
        Some("mixed"),
        "pyproject.toml wins"
    );
    assert_eq!(parsed["project"]["version"].as_str(), Some("1.0.0"));
    assert_eq!(
        parsed["source"]["python"]["paths"],
        serde_yaml_ng::to_value(vec!["mixed/"]).unwrap()
    );
    assert_eq!(
        parsed["source"]["rust"]["paths"],
        serde_yaml_ng::to_value(vec!["src/"]).unwrap()
    );
    assert!(result
        .stdout
        .contains("│ Source     mixed/ (Python), src/ (Rust) "));
    assert!(result
        .stdout
        .contains("│ Framework  Python package + Rust crate "));
    assert!(result.stdout.contains("folioh coverage mixed"));

    // A manifest that does not parse warns and init goes on.
    let broken = cwd.join("broken-json");
    fs::create_dir(&broken).unwrap();
    fs::write(broken.join("package.json"), "{").unwrap();
    let result = folioh_run(cwd, &["init", "broken-json", "--yes"], None);
    assert_eq!(result.code, 0, "{}", result.stdout);
    assert!(
        result
            .diagnostics()
            .starts_with("Warning: Could not read project metadata from package.json: "),
        "{}",
        result.diagnostics()
    );
}

#[test]
fn init_leaves_existing_files_alone_and_reports_a_missing_directory() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path();

    let existing = cwd.join("existing");
    fs::create_dir(&existing).unwrap();
    fs::write(existing.join("docs.yaml"), "existing").unwrap();
    let result = folioh_run(cwd, &["init", "existing"], None);
    assert_eq!(result.code, 0);
    assert_eq!(result.stdout, "docs.yaml already exists, skipping.\n");
    assert_eq!(
        fs::read_to_string(existing.join("docs.yaml")).unwrap(),
        "existing"
    );
    assert!(!existing.join("docs").exists());

    let custom = cwd.join("custom");
    fs::create_dir_all(custom.join(".github/workflows")).unwrap();
    fs::create_dir_all(custom.join("docs")).unwrap();
    fs::write(
        custom.join(".github/workflows/pages.yml"),
        "custom caller\n",
    )
    .unwrap();
    fs::write(
        custom.join(".github/workflows/branch-previews.yml"),
        "custom preview caller\n",
    )
    .unwrap();
    let result = folioh_run(cwd, &["init", "custom", "--yes"], None);
    assert_eq!(result.code, 0, "{}", result.stdout);
    assert_eq!(
        fs::read_to_string(custom.join(".github/workflows/pages.yml")).unwrap(),
        "custom caller\n"
    );
    assert_eq!(
        fs::read_to_string(custom.join(".github/workflows/branch-previews.yml")).unwrap(),
        "custom preview caller\n"
    );
    assert!(!custom.join(".github/workflows/folioh-pages.yml").exists());
    // An existing docs/ without Markdown gets the index page it needs.
    assert_eq!(
        fs::read_to_string(custom.join("docs/index.md")).unwrap(),
        "# custom\n\nWelcome to the documentation.\n"
    );
    assert!(result.stdout.contains("│   Created custom/docs.yaml "));
    assert!(result.stdout.contains("│   Created custom/docs/index.md "));
    assert!(!result.stdout.contains("pages.yml"));
    assert!(result
        .stdout
        .contains("│ Status     docs/ found, docs.yaml missing "));

    // A docs/ that already holds Markdown is never touched.
    let written = cwd.join("written");
    fs::create_dir_all(written.join("docs/guide")).unwrap();
    fs::write(written.join("docs/guide/start.md"), "# Start\n").unwrap();
    let result = folioh_run(cwd, &["init", "written", "--yes"], None);
    assert_eq!(result.code, 0, "{}", result.stdout);
    assert!(!written.join("docs/index.md").exists());
    assert!(!result.stdout.contains("Created written/docs/index.md"));

    let result = folioh_run(cwd, &["init", "missing/dir", "--yes"], None);
    assert_eq!(result.code, 1);
    assert_eq!(
        result.diagnostics(),
        "Error: Directory not found: missing/dir\n"
    );
    assert!(result.stdout.is_empty());
}

#[test]
fn clean_reads_the_output_directory_best_effort() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path();

    // Unknown keys are irrelevant: the raw `output` wins.
    let raw = cwd.join("raw");
    fs::create_dir_all(raw.join(".build")).unwrap();
    fs::create_dir_all(raw.join("public")).unwrap();
    fs::write(
        raw.join("docs.yaml"),
        "project:\n  name: \"Demo\"\nplugins:\n  - \"missing_plugin\"\noutput: \"public\"\n",
    )
    .unwrap();
    let result = folioh_run(cwd, &["clean", "--project-dir", "raw"], None);
    assert_eq!(result.code, 0, "{}", result.stdout);
    assert_eq!(result.stdout, "Cleaned: .build, public\n");
    assert!(!raw.join(".build").exists());
    assert!(!raw.join("public").exists());

    // An output outside the project is refused; `.build` still goes.
    let inside = cwd.join("inside");
    let outside = cwd.join("inside-outside-output");
    fs::create_dir_all(inside.join(".build")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("keep.txt"), "keep").unwrap();
    fs::write(
        inside.join("docs.yaml"),
        "project:\n  name: \"Demo\"\noutput: \"../inside-outside-output\"\n",
    )
    .unwrap();
    let result = folioh_run(cwd, &["clean", "inside"], None);
    assert_eq!(result.code, 0, "{}", result.stdout);
    assert_eq!(
        format!("{}{}", result.diagnostics(), result.stdout),
        "Warning: Ignoring unsafe output in inside/docs.yaml: Output directory must stay within the project directory\nCleaned: .build\n"
    );
    assert!(!inside.join(".build").exists());
    assert!(outside.join("keep.txt").exists());

    // Nothing to clean, and the `_site` default without a config file.
    let empty = cwd.join("empty");
    fs::create_dir(&empty).unwrap();
    let result = folioh_run(cwd, &["clean", "empty"], None);
    assert_eq!(result.stdout, "Nothing to clean.\n");
    fs::create_dir_all(empty.join("_site")).unwrap();
    let result = folioh_run(cwd, &["clean", "empty"], None);
    assert_eq!(result.stdout, "Cleaned: _site\n");

    // An unparsable file warns and falls back to `_site`; exit stays 0.
    let broken = cwd.join("broken");
    fs::create_dir_all(broken.join("_site")).unwrap();
    fs::write(broken.join("docs.yaml"), "output: [\n").unwrap();
    let result = folioh_run(cwd, &["clean", "broken"], None);
    assert_eq!(result.code, 0, "{}", result.stdout);
    assert!(
        result
            .stdout
            .starts_with("Warning: Could not read output from broken/docs.yaml: "),
        "{}",
        result.stdout
    );
    assert!(result.stdout.ends_with("Cleaned: _site\n"));

    // `output: ".git"` never deletes the repository; `.build` still goes.
    let repo = cwd.join("repo");
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::write(repo.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    fs::create_dir_all(repo.join(".build")).unwrap();
    fs::write(repo.join("docs.yaml"), "output: \".git\"\n").unwrap();
    let result = folioh_run(cwd, &["clean", "repo"], None);
    assert_eq!(result.code, 0, "{}", result.stdout);
    assert_eq!(
        format!("{}{}", result.diagnostics(), result.stdout),
        "Warning: Ignoring unsafe output in repo/docs.yaml: Output directory '.git' must not contain the repository's .git directory; the build removes the output directory before writing to it\nCleaned: .build\n"
    );
    assert!(repo.join(".git/HEAD").is_file());
    assert!(!repo.join(".build").exists());

    // An output inside `.build/` goes with it: one removal, no error.
    let nested = cwd.join("nested");
    fs::create_dir_all(nested.join(".build/site")).unwrap();
    fs::write(nested.join("docs.yaml"), "output: \".build/site\"\n").unwrap();
    let result = folioh_run(cwd, &["clean", "nested"], None);
    assert_eq!(result.code, 0, "{}", result.stdout);
    assert_eq!(result.stdout, "Cleaned: .build\n");
    assert!(!nested.join(".build").exists());

    // An output that is, or contains, a source root is never deleted.
    let sources = cwd.join("sources");
    fs::create_dir_all(sources.join("docs")).unwrap();
    fs::write(sources.join("docs/index.md"), "# Guide\n").unwrap();
    fs::create_dir_all(sources.join("src/pkg")).unwrap();
    fs::write(sources.join("src/pkg/__init__.py"), "").unwrap();
    fs::create_dir_all(sources.join(".build")).unwrap();
    for (output, source) in [("docs", "docs/"), ("src", "src/pkg")] {
        fs::write(
            sources.join("other.yaml"),
            format!(
                "output: \"{output}\"\nsource:\n  python:\n    paths: [\"src/pkg\"]\n  docs: [\"docs/\"]\n"
            ),
        )
        .unwrap();
        let result = folioh_run(cwd, &["clean", "sources", "-c", "other.yaml"], None);
        assert_eq!(result.code, 0, "{}", result.stdout);
        assert!(
            result.stdout.starts_with(&format!(
                "Warning: Ignoring unsafe output in sources/other.yaml: Output directory '{output}' would delete the source directory '{source}'"
            )),
            "{}",
            result.stdout
        );
        assert!(sources.join("docs/index.md").is_file());
        assert!(sources.join("src/pkg/__init__.py").is_file());
    }

    // An output that names a file is refused before anything is removed.
    let file = cwd.join("file");
    fs::create_dir_all(file.join(".build")).unwrap();
    fs::write(file.join("docs.yaml"), "output: \"docs.yaml\"\n").unwrap();
    let result = folioh_run(cwd, &["clean", "file"], None);
    assert_eq!(result.code, 1, "{}", result.stdout);
    assert_eq!(
        result.diagnostics(),
        "Error: docs.yaml is not a directory; folioh clean only removes directories. Check output in file/docs.yaml.\n"
    );
    assert!(file.join("docs.yaml").is_file());
    assert!(file.join(".build").is_dir());

    // Both directory forms naming different places is an error.
    let result = folioh_run(cwd, &["clean", "raw", "--project-dir", "empty"], None);
    assert_eq!(result.code, 1);
    assert_eq!(
        result.diagnostics(),
        "Error: Pass the project directory either as an argument or --project-dir, not both.\n"
    );

    // A project directory that does not exist is an error, not "Nothing to clean."
    let result = folioh_run(cwd, &["clean", "missing"], None);
    assert_eq!(result.code, 1, "{}", result.stdout);
    assert_eq!(
        result.diagnostics(),
        "Error: Directory not found: missing\n"
    );
    assert_eq!(result.stdout, "");
}

#[test]
fn pipeline_commands_parse_and_report_their_missing_config() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path();
    let root = cwd.canonicalize().unwrap();
    let missing = folioh_run(cwd, &["build", "--clean", "-v", "-o"], None);
    assert_eq!(missing.code, 1);
    assert_eq!(
        missing.diagnostics(),
        format!(
            "Error: Config file not found: {}\n",
            root.join("docs.yaml").display()
        )
    );
    let coverage = folioh_run(cwd, &["coverage", "--min", "80.5", "-v"], None);
    assert_eq!(coverage.code, 1);
    assert_eq!(
        coverage.diagnostics(),
        format!(
            "Error: Config file not found: {}\n",
            root.join("docs.yaml").display()
        )
    );
    let roadmap = folioh_run(cwd, &["roadmap", "-c", "other.yaml"], None);
    assert_eq!(roadmap.code, 1);
    assert_eq!(
        roadmap.diagnostics(),
        format!(
            "Error: Config file not found: {}\n",
            root.join("other.yaml").display()
        )
    );
    let serve = folioh_run(cwd, &["serve", "-p", "5678", "--kill-existing"], None);
    assert_eq!(serve.code, 1);
    assert_eq!(
        serve.diagnostics(),
        format!(
            "Error: Config file not found: {}\n",
            root.join("docs.yaml").display()
        )
    );
    let versions = folioh_run(cwd, &["serve", "-p", "5678", "--versions"], None);
    assert_eq!(versions.code, 1);
    assert_eq!(
        versions.stdout,
        "The 'versions' feature is not available in this release.\n"
    );
    let gated = folioh_run(cwd, &["build-versions", "--clean"], None);
    assert_eq!(gated.code, 1);
    assert_eq!(
        gated.stdout,
        "The 'versions' feature is not available in this release.\n"
    );
    let result = folioh_run(cwd, &["build", "a", "--project-dir", "b"], None);
    assert_eq!(result.code, 1);
    assert_eq!(
        result.diagnostics(),
        "Error: Pass the project directory either as an argument or --project-dir, not both.\n"
    );
}

#[test]
fn github_pages_steps_write_github_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path();
    let output_path = cwd.join("github-output");

    let args = [
        "github-pages",
        "compute-preview-path",
        "--head-ref",
        "Feature/Foo",
        "--pr-number",
        "17",
        "--pages-base-path",
        "/folioh/",
        "--pages-base-url",
        "https://folioh.site/",
    ];
    let result = run(
        folioh(cwd).args(args).env("GITHUB_OUTPUT", &output_path),
        None,
    );
    assert_eq!(result.code, 0, "{}{}", result.stdout, result.stderr);
    assert!(result.stdout.is_empty());
    let expected = "safe_branch=pr-17-feature-foo\nbase_path=/folioh/previews/pr-17-feature-foo\nurl=https://folioh.site/previews/pr-17-feature-foo/\n";
    assert_eq!(fs::read_to_string(&output_path).unwrap(), expected);
    // Appended, not overwritten; printed instead when the variable is unset.
    run(
        folioh(cwd).args(args).env("GITHUB_OUTPUT", &output_path),
        None,
    );
    assert_eq!(
        fs::read_to_string(&output_path).unwrap(),
        format!("{expected}{expected}")
    );
    let result = folioh_run(cwd, &args, None);
    assert_eq!(result.stdout, expected);

    let previews = cwd.join("previews");
    for name in ["pr-1-foo", "pr-2-bar"] {
        fs::create_dir_all(previews.join(name)).unwrap();
        fs::write(previews.join(name).join("index.html"), "x").unwrap();
    }
    let prune_output = cwd.join("prune-output");
    let result = run(
        folioh(cwd)
            .args([
                "github-pages",
                "prune-previews",
                "--previews-dir",
                "previews",
                "--open-prs-json",
                "[{\"number\": 1, \"headRefName\": \"foo\"}]",
            ])
            .env("GITHUB_OUTPUT", &prune_output),
        None,
    );
    assert_eq!(result.code, 0, "{}{}", result.stdout, result.stderr);
    assert_eq!(result.stdout, "Removed stale preview: pr-2-bar\n");
    assert_eq!(fs::read_to_string(&prune_output).unwrap(), "removed=1\n");
    assert!(previews.join("pr-1-foo").is_dir());
    assert!(!previews.join("pr-2-bar").exists());

    let result = folioh_run(
        cwd,
        &[
            "github-pages",
            "write-previews-data",
            "--previews-dir",
            "previews",
        ],
        None,
    );
    assert_eq!(result.stdout, "Wrote metadata for 1 preview(s).\n");
    assert!(fs::read_to_string(previews.join("previews.json"))
        .unwrap()
        .contains("\"name\": \"pr-1-foo\""));

    let result = folioh_run(
        cwd,
        &[
            "github-pages",
            "copy-branch-preview",
            "--preview-site",
            "nope",
            "--artifact-dir",
            "art",
            "--preview-id",
            "../x",
        ],
        None,
    );
    assert_eq!(result.code, 1);
    assert_eq!(result.diagnostics(), "Error: Invalid preview id: ../x\n");
    let result = folioh_run(
        cwd,
        &[
            "github-pages",
            "copy-branch-preview",
            "--preview-site",
            "nope",
            "--artifact-dir",
            "art",
            "--preview-id",
            "pr-1",
        ],
        None,
    );
    assert_eq!(result.code, 1);
    assert!(result
        .diagnostics()
        .starts_with("Error: Source directory not found: "));

    // Free-text values may begin with a hyphen (a branch `---`, a PR title `-wip`).
    let result = folioh_run(
        cwd,
        &[
            "github-pages",
            "compute-preview-path",
            "--head-ref",
            "---",
            "--pr-number",
            "23",
            "--pages-base-path",
            "/",
            "--pages-base-url",
            "https://example.com/project",
        ],
        None,
    );
    assert_eq!(result.code, 0, "{}", result.stderr);
    assert_eq!(result.stdout, "safe_branch=pr-23\nbase_path=/previews/pr-23\nurl=https://example.com/project/previews/pr-23/\n");
    fs::create_dir_all(cwd.join("art/previews/pr-1")).unwrap();
    let result = folioh_run(
        cwd,
        &[
            "github-pages",
            "write-preview-metadata",
            "--artifact-dir",
            "art",
            "--preview-id",
            "pr-1",
            "--pr-number",
            "1",
            "--updated-at",
            "2026-07-01T12:00:00Z",
            "--title",
            "-wip",
            "--branch",
            "-b",
        ],
        None,
    );
    assert_eq!(result.code, 0, "{}{}", result.stdout, result.stderr);
    let sidecar = fs::read_to_string(cwd.join("art/previews/pr-1/.folioh-preview.json")).unwrap();
    assert!(sidecar.contains("\"title\": \"-wip\""));
    assert!(sidecar.contains("\"branch\": \"-b\""));
}
