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

#[test]
fn preview_cache_prefixes_cannot_restore_production_or_ci_caches() {
    use serde_yaml_ng::Value;
    use std::{fs, path::Path};

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut preview = Vec::new();
    let mut trusted = Vec::new();
    for file in [
        "branch-previews.yml",
        "pages.yml",
        "rust.yml",
        "release.yml",
    ] {
        let workflow: Value = serde_yaml_ng::from_str(
            &fs::read_to_string(root.join(".github/workflows").join(file)).unwrap(),
        )
        .unwrap();
        for job in workflow["jobs"].as_mapping().unwrap().values() {
            let Some(steps) = job["steps"].as_sequence() else {
                continue;
            };
            for step in steps {
                if step["uses"].as_str() != Some("Swatinem/rust-cache@v2") {
                    continue;
                }
                // rust-cache retains prefix-key in both its exact and fallback keys.
                let prefix = step["with"]["prefix-key"].as_str().unwrap_or("v0-rust");
                if file == "branch-previews.yml" {
                    assert_ne!(
                        prefix, "v0-rust",
                        "preview caches require an explicit boundary"
                    );
                    preview.push(prefix.to_owned());
                } else {
                    trusted.push(prefix.to_owned());
                }
            }
        }
    }
    assert_eq!(
        preview.len(),
        2,
        "cover PR builds and trusted preview deploys"
    );
    assert!(!trusted.is_empty());
    for prefix in preview {
        assert!(!prefix.is_empty() && !prefix.contains("${{"));
        for other in &trusted {
            assert!(
                !prefix.starts_with(other) && !other.starts_with(&prefix),
                "preview cache prefix {prefix} overlaps trusted prefix {other}"
            );
        }
    }
}

#[test]
fn temporary_artifacts_expire_and_cleanup_stays_inside_successful_run_consumers() {
    use serde_json::json;
    use serde_yaml_ng::Value;
    use std::{fs, path::Path, process::Command};

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut cases = Vec::new();
    let mut uploads = 0;
    for (file, consumer, verified) in [
        ("branch-previews.yml", "deploy", "Comment preview URL"),
        ("pages.yml", "deploy", "Verify and print deployment URL"),
        ("release.yml", "publish", "GitHub Release"),
    ] {
        let workflow: Value = serde_yaml_ng::from_str(
            &fs::read_to_string(root.join(".github/workflows").join(file)).unwrap(),
        )
        .unwrap();
        assert!(workflow["permissions"]["actions"].is_null());
        for (name, job) in workflow["jobs"].as_mapping().unwrap() {
            if name.as_str() != Some(consumer) {
                assert_ne!(job["permissions"]["actions"].as_str(), Some("write"));
            }
            for step in job["steps"].as_sequence().into_iter().flatten() {
                if step["uses"].as_str().is_some_and(|action| {
                    action.starts_with("actions/upload-artifact@")
                        || action.starts_with("actions/upload-pages-artifact@")
                }) {
                    assert_eq!(step["with"]["retention-days"].as_u64(), Some(1), "{file}");
                    uploads += 1;
                }
            }
        }
        let job = &workflow["jobs"][consumer];
        assert_eq!(job["permissions"]["actions"].as_str(), Some("write"));
        let steps = job["steps"].as_sequence().unwrap();
        let verification = steps.iter().position(|s| s["name"] == verified).unwrap();
        let cleanup = steps
            .iter()
            .position(|s| s["name"] == "Remove consumed temporary artifacts")
            .unwrap();
        assert!(
            cleanup > verification,
            "{file}: consume and verify before cleanup"
        );
        let step = &steps[cleanup];
        assert_eq!(step["uses"].as_str(), Some("actions/github-script@v7"));
        assert_eq!(step["continue-on-error"].as_bool(), Some(true));
        let names = if file == "release.yml" {
            workflow["jobs"]["build"]["strategy"]["matrix"]["include"]
                .as_sequence()
                .unwrap()
                .iter()
                .map(|target| format!("folioh-{}", target["target"].as_str().unwrap()))
                .collect::<Vec<_>>()
        } else if file == "branch-previews.yml" {
            vec!["branch-preview-site".into(), "github-pages".into()]
        } else {
            vec!["github-pages".into()]
        };
        cases.push(json!({
            "file": file,
            "condition": step["if"].as_str().unwrap(),
            "script": step["with"]["script"].as_str().unwrap(),
            "names": names,
        }));
    }
    assert_eq!(
        uploads, 4,
        "include every current repository artifact producer"
    );
    let output = Command::new("node")
        .args([
            "-e",
            r#"
const assert = require('node:assert/strict')
const { runInNewContext } = require('node:vm')
const AsyncFunction = Object.getPrototypeOf(async function() {}).constructor
;(async () => {
  for (const { file, condition, script, names } of JSON.parse(process.argv[1])) {
    const preview = file === 'branch-previews.yml'
    for (const [success, outcome, enabled, expected] of [
      [true, 'success', 'true', true],
      [false, 'success', 'true', false], // verification failed after deployment
      [false, 'failure', 'true', false],
      [true, 'skipped', 'true', false],
      [true, 'success', 'false', !preview]
    ]) {
      const steps = { deployment: { outcome }, release: { outcome }, 'deploy-gate': { outputs: { enabled } } }
      const actual = runInNewContext(condition.replaceAll('steps.deploy-gate', "steps['deploy-gate']"), { success: () => success, steps })
      assert.equal(actual, expected, `${file}: ${success}/${outcome}/${enabled}`)
    }
    const context = { repo: { owner: 'example', repo: 'docs' }, runId: 42 }
    const artifacts = Array.from({length: 105}, (_, id) => ({ id, name: 'unrelated-diagnostics' }))
    artifacts.unshift({ id: 404, name: names[0] }) // already expired is harmless
    artifacts.push(...names.map((name, index) => ({ id: 1000 + index, name })))
    const execute = new AsyncFunction('github', 'context', 'core', script)
    for (const fail of [false, true]) {
      let listed = 0
      const deleted = []
      const listWorkflowRunArtifacts = () => { throw Error('pagination required') }
      const github = {
        paginate: async (endpoint, options) => {
          assert.equal(endpoint, listWorkflowRunArtifacts)
          assert.deepEqual(options, { owner: 'example', repo: 'docs', run_id: 42, per_page: 100 })
          listed++
          return artifacts
        },
        rest: { actions: {
          listWorkflowRunArtifacts,
          deleteArtifact: async options => {
            assert.deepEqual(Object.keys(options).sort(), ['artifact_id', 'owner', 'repo'])
            assert.equal(options.owner, 'example'); assert.equal(options.repo, 'docs')
            if (options.artifact_id === 404) throw Object.assign(Error('gone'), { status: 404 })
            if (fail) throw Object.assign(Error('denied'), { status: 403 })
            deleted.push(options.artifact_id)
          }
        } }
      }
      if (fail) await assert.rejects(execute(github, context, { info() {} }), { status: 403 })
      else {
        await execute(github, context, { info() {} })
        assert.deepEqual(deleted, names.map((_, index) => 1000 + index), file)
      }
      assert.equal(listed, 1, `${file}: paginate the current run only`)
    }
  }
})().catch(error => { console.error(error); process.exitCode = 1 })
"#,
        ])
        .arg(serde_json::to_string(&cases).unwrap())
        .output()
        .expect("Node.js is required to verify workflow cleanup boundaries");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
