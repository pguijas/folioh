//! Persistent runner workspaces must not carry files into the next build.

#[test]
fn private_workflows_select_the_nas_without_exposing_public_or_fork_jobs() {
    use serde_json::json;
    use serde_yaml_ng::Value;
    use std::{fs, path::Path, process::Command};

    if Command::new("node").arg("--version").output().is_err() {
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut cases = Vec::new();
    for file in ["rust.yml", "branch-previews.yml", "pages.yml"] {
        let workflow: Value = serde_yaml_ng::from_str(
            &fs::read_to_string(root.join(".github/workflows").join(file)).unwrap(),
        )
        .unwrap();
        if file == "rust.yml" {
            assert_eq!(
                workflow["jobs"]["test"]["strategy"]["matrix"]["os"],
                serde_yaml_ng::to_value(["ubuntu-latest"]).unwrap(),
                "preserve the required test (ubuntu-latest) check name"
            );
        }
        for (job_name, job) in workflow["jobs"].as_mapping().unwrap() {
            let expression = job["runs-on"].as_str().unwrap();
            for (private, event, head) in [
                (true, "push", None),
                (true, "workflow_dispatch", None),
                (true, "pull_request", Some("example/docs")),
                (true, "pull_request_target", Some("example/docs")),
                (true, "pull_request", Some("contributor/docs")),
                (true, "pull_request_target", Some("contributor/docs")),
                (false, "push", None),
                (false, "pull_request", Some("example/docs")),
            ] {
                let expected = if private && head.is_none_or(|head| head == "example/docs") {
                    json!(["self-hosted", "nas"])
                } else {
                    json!("ubuntu-latest")
                };
                cases.push(json!({
                    "label": format!("{file}/{}: {private}/{event}/{head:?}", job_name.as_str().unwrap()),
                    "expression": expression,
                    "github": {
                        "repository": "example/docs",
                        "event_name": event,
                        "event": {
                            "repository": { "private": private },
                            "pull_request": head.map(|head| json!({"head": {"repo": {"full_name": head}}}))
                        }
                    },
                    "matrix": { "os": "ubuntu-latest" },
                    "expected": expected
                }));
            }
            if job_name.as_str() == Some("test") {
                let mut case = cases.last().unwrap().clone();
                case["label"] = json!(format!("{file}/test: private macOS stays hosted"));
                case["github"]["event"]["repository"]["private"] = json!(true);
                case["matrix"]["os"] = json!("macos-latest");
                case["expected"] = json!("macos-latest");
                cases.push(case);
            }
        }
    }
    // These selectors use the shared JS/GitHub boolean, property-access and
    // JSON subset. Execute their actual expressions across the trust contexts.
    let output = Command::new("node")
        .args([
            "-e",
            r#"
const { deepStrictEqual } = require('node:assert')
const { runInNewContext } = require('node:vm')
for (const { label, expression, github, matrix, expected } of JSON.parse(process.argv[1])) {
  const source = expression.trim().slice(3, -2)
  const actual = runInNewContext(source, { github, matrix, fromJSON: JSON.parse })
  deepStrictEqual(JSON.parse(JSON.stringify(actual)), expected, label)
}
"#,
        ])
        .arg(serde_json::to_string(&cases).unwrap())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for file in [
        ".github/workflows/release.yml",
        "crates/folioh-cli/src/workflows/pages.yml",
        "crates/folioh-cli/src/workflows/branch-previews.yml",
    ] {
        assert!(
            !fs::read_to_string(root.join(file))
                .unwrap()
                .contains("self-hosted"),
            "{file} stays hosted"
        );
    }
}

#[cfg(unix)]
#[test]
fn pages_jobs_clear_their_own_workspace_without_touching_other_jobs() {
    use serde_yaml_ng::Value;
    use std::{fs, path::Path, process::Command};

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directories = [
        "production-source",
        "preview-source",
        "trusted-source",
        "_preview-site",
        "_pages-artifact",
        "_pages-state",
        "unrelated-job",
    ];
    for (file, job, removed) in [
        ("pages.yml", "deploy", &["production-source"][..]),
        (
            "branch-previews.yml",
            "build-preview",
            &["preview-source"][..],
        ),
        (
            "branch-previews.yml",
            "deploy",
            &[
                "trusted-source",
                "_preview-site",
                "_pages-artifact",
                "_pages-state",
            ][..],
        ),
    ] {
        let workflow: Value = serde_yaml_ng::from_str(
            &fs::read_to_string(root.join(".github/workflows").join(file)).unwrap(),
        )
        .unwrap();
        let steps = workflow["jobs"][job]["steps"].as_sequence().unwrap();
        let reset = steps
            .iter()
            .position(|step| step["name"].as_str() == Some("Clear job workspace"))
            .expect("clear the persistent workspace before checkout or download");
        let checkout = steps
            .iter()
            .position(|step| step["uses"].as_str() == Some("actions/checkout@v4"))
            .unwrap();
        assert!(reset < checkout);

        let workspace = tempfile::tempdir().unwrap();
        for directory in directories {
            let path = workspace.path().join(directory).join("nested");
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("stale.html"), "previous run").unwrap();
        }
        let status = Command::new("bash")
            .args([
                "-euo",
                "pipefail",
                "-c",
                steps[reset]["run"].as_str().unwrap(),
            ])
            .env("GITHUB_WORKSPACE", workspace.path())
            .current_dir(workspace.path())
            .status()
            .unwrap();
        assert!(status.success(), "{file}: {job} cleanup failed");
        for directory in directories {
            assert_eq!(
                workspace.path().join(directory).exists(),
                !removed.contains(&directory),
                "{file}: {job} must only clear its own {removed:?}, checked {directory}"
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn preview_build_paths_follow_pages_configuration_for_custom_and_project_domains() {
    use serde_yaml_ng::Value;
    use std::{fs, path::Path, process::Command};

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = Path::new(env!("CARGO_BIN_EXE_folioh"));
    let path = format!(
        "{}:{}",
        binary.parent().unwrap().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    for file in [
        ".github/workflows/branch-previews.yml",
        "crates/folioh-cli/src/workflows/branch-previews.yml",
    ] {
        let workflow: Value =
            serde_yaml_ng::from_str(&fs::read_to_string(root.join(file)).unwrap()).unwrap();
        let steps = workflow["jobs"]["build-preview"]["steps"]
            .as_sequence()
            .unwrap();
        let compute = steps
            .iter()
            .find(|step| step["name"].as_str() == Some("Compute preview path"))
            .unwrap();
        for (base_path, base_url) in [
            ("/", "https://folioh.site"),
            ("/folioh", "https://pguijas.github.io/folioh"),
        ] {
            let output = Command::new("bash")
                .args(["-euo", "pipefail", "-c", compute["run"].as_str().unwrap()])
                .env("PATH", &path)
                .env("HEAD_REF", "feature/demo")
                .env("PR_NUMBER", "17")
                .env("PAGES_BASE_PATH", base_path)
                .env("PAGES_BASE_URL", base_url)
                .env_remove("GITHUB_OUTPUT")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{file}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let stdout = String::from_utf8(output.stdout).unwrap();
            assert!(
                stdout.contains(&format!(
                    "base_path={}/previews/pr-17-feature-demo",
                    base_path.trim_end_matches('/')
                )),
                "{file}: {stdout}"
            );
            assert!(
                stdout.contains(&format!("url={base_url}/previews/pr-17-feature-demo/")),
                "{file}: {stdout}"
            );
        }
    }
}
