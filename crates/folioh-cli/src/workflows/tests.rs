use super::*;
use serde_yaml_ng::{Mapping, Value};

const INSTALL_LINE: &str = "curl -LsSf https://folioh.site/install.sh | sh";

fn load(text: &str) -> Mapping {
    serde_yaml_ng::from_str::<Value>(text)
        .unwrap()
        .as_mapping()
        .unwrap()
        .clone()
}

fn keys(mapping: &Mapping) -> Vec<String> {
    mapping
        .keys()
        .map(|k| k.as_str().unwrap().to_string())
        .collect()
}

fn job<'a>(workflow: &'a Mapping, name: &str) -> &'a Mapping {
    workflow["jobs"][name].as_mapping().unwrap()
}

fn steps(job: &Mapping) -> Vec<&Mapping> {
    job["steps"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|s| s.as_mapping().unwrap())
        .collect()
}

fn step_names(job: &Mapping) -> Vec<String> {
    steps(job)
        .iter()
        .map(|s| s["name"].as_str().unwrap().to_string())
        .collect()
}

fn step<'a>(job: &'a Mapping, name: &str) -> &'a Mapping {
    steps(job)
        .into_iter()
        .find(|s| s["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("step {name}"))
}

fn assert_installs_the_binary(job: &Mapping) {
    let install = step(job, "Install Folioh")["run"].as_str().unwrap();
    assert!(install.contains(INSTALL_LINE), "{install}");
    assert!(
        install.contains("echo \"$HOME/.local/bin\" >> \"$GITHUB_PATH\""),
        "{install}"
    );
    assert_eq!(
        step(job, "Install pnpm")["with"]["version"].as_u64(),
        Some(10)
    );
    assert_eq!(
        step(job, "Set up Node")["with"]["node-version"].as_u64(),
        Some(20)
    );
}

#[test]
fn no_python_toolchain_survives_in_either_workflow() {
    for (path, text) in github_pages_workflows() {
        for stale in [
            "uv ",
            "setup-python",
            "setup-uv",
            "python",
            "Python",
            "pip ",
        ] {
            assert!(!text.contains(stale), "{path} still mentions `{stale}`");
        }
        assert!(text.contains(INSTALL_LINE), "{path} installs the binary");
        assert!(
            text.contains("folioh build --clean"),
            "{path} runs the binary bare"
        );
        assert!(!text.contains("render-previews-index"));
        assert!(!text.contains("python - <<'PY'"));
        assert!(!text.contains("\\\\"), "{path} carries a doubled backslash");
    }
}

#[test]
fn pages_workflow_keeps_the_two_job_deploy_shape() {
    let workflow = load(FOLIOH_PAGES_WORKFLOW);
    assert_eq!(workflow["name"], Value::from("Deploy Docs"));
    let on = workflow["on"].as_mapping().unwrap();
    assert_eq!(keys(on), ["push", "workflow_dispatch"]);
    assert_eq!(
        on["push"]["branches"],
        serde_yaml_ng::to_value(vec!["main"]).unwrap()
    );
    assert!(workflow.get("env").is_none());
    assert_eq!(workflow["permissions"]["contents"], Value::from("read"));
    assert_eq!(workflow["concurrency"]["group"], Value::from("pages"));
    assert_eq!(
        workflow["concurrency"]["cancel-in-progress"],
        Value::from(false)
    );
    assert_eq!(
        keys(workflow["jobs"].as_mapping().unwrap()),
        ["build", "deploy"]
    );

    let build = job(&workflow, "build");
    assert_eq!(build["permissions"]["contents"], Value::from("write"));
    assert_eq!(build["permissions"]["pages"], Value::from("write"));
    assert_eq!(build["permissions"]["pull-requests"], Value::from("read"));
    assert_eq!(
        step_names(build),
        [
            "Check out repository",
            "Configure GitHub Pages",
            "Install pnpm",
            "Set up Node",
            "Install Folioh",
            "Build docs",
            "Preserve branch previews",
            "Prune stale previews",
            "Write previews data",
            "Save Pages state",
            "Upload Pages artifact",
        ]
    );
    assert_eq!(
        step(build, "Check out repository")["with"]["fetch-depth"].as_u64(),
        Some(0)
    );
    assert_eq!(
        step(build, "Configure GitHub Pages")["id"],
        Value::from("pages")
    );
    assert_installs_the_binary(build);
    let build_docs = step(build, "Build docs");
    assert_eq!(build_docs["run"], Value::from("folioh build --clean"));
    assert_eq!(
        build_docs["env"]["FOLIOH_BASE_PATH"],
        Value::from("${{ steps.pages.outputs.base_path || '/' }}")
    );
    assert_eq!(
            step(build, "Preserve branch previews")["run"],
            Value::from("folioh github-pages preserve-previews --site-dir _site --git-repo . --state-dir _pages-state --state-branch folioh-pages-state")
        );
    let prune = step(build, "Prune stale previews");
    assert_eq!(prune["env"]["GH_TOKEN"], Value::from("${{ github.token }}"));
    let prune_run = prune["run"].as_str().unwrap();
    assert!(prune_run.contains("open_prs=\"$(gh pr list --state open --json number,headRefName)\""));
    assert!(prune_run.contains("folioh github-pages prune-previews \\\n"));
    assert!(prune_run.contains("--open-prs-json \"$open_prs\""));
    assert_eq!(
            step(build, "Save Pages state")["run"],
            Value::from("folioh github-pages save-state --artifact-dir _site --git-repo . --state-dir _pages-state --state-branch folioh-pages-state --commit-message \"Update Pages state\"")
        );
    assert_eq!(
        step(build, "Upload Pages artifact")["with"]["path"],
        Value::from("_site")
    );

    let deploy = job(&workflow, "deploy");
    assert_eq!(deploy["needs"], Value::from("build"));
    assert_eq!(deploy["environment"]["name"], Value::from("github-pages"));
    assert_eq!(
        deploy["environment"]["url"],
        Value::from("${{ steps.deployment.outputs.page_url }}")
    );
    assert_eq!(deploy["permissions"]["id-token"], Value::from("write"));
    assert_eq!(
        step_names(deploy),
        [
            "Install Folioh",
            "Deploy to GitHub Pages",
            "Verify and print deployment URL"
        ]
    );
    let verify = step(deploy, "Verify and print deployment URL")["run"]
        .as_str()
        .unwrap();
    assert!(verify.contains("index_url=\"${url%/}/previews/\""));
    assert!(verify.contains("folioh github-pages verify-url \\\n"));
    assert!(verify.contains("--summary-heading \"Production docs\""));
    assert!(verify
        .contains("--error-message \"Deployment URL or previews index did not return HTTP 200\""));
}

#[test]
fn branch_preview_workflow_keeps_the_three_job_shape() {
    let text = FOLIOH_BRANCH_PREVIEW_WORKFLOW;
    let workflow = load(text);
    assert_eq!(workflow["name"], Value::from("Deploy Branch Preview Docs"));
    let on = workflow["on"].as_mapping().unwrap();
    assert_eq!(keys(on), ["pull_request_target"]);
    assert_eq!(
        on["pull_request_target"]["types"],
        serde_yaml_ng::to_value(vec![
            "opened",
            "synchronize",
            "reopened",
            "ready_for_review"
        ])
        .unwrap()
    );
    assert_eq!(
        workflow["permissions"]["pull-requests"],
        Value::from("read")
    );
    assert!(workflow.get("concurrency").is_none());
    assert_eq!(
        keys(workflow["jobs"].as_mapping().unwrap()),
        ["validate", "build-preview", "deploy"]
    );

    let validate = job(&workflow, "validate");
    assert_eq!(
        keys(validate["outputs"].as_mapping().unwrap()),
        ["enabled", "pr_number", "head_sha", "head_ref", "pr_title"]
    );
    assert_eq!(step_names(validate), ["Validate preview PR"]);
    let gate = step(validate, "Validate preview PR");
    assert_eq!(gate["id"], Value::from("preview-gate"));
    let gate_run = gate["run"].as_str().unwrap();
    for message in [
            "Preview deploy skipped because the workflow could not resolve an open pull request.",
            "Preview deploy skipped because PR #${PR_NUMBER} is not open.",
            "Preview deploy skipped because PR #${PR_NUMBER} is still a draft.",
            "Preview deploy skipped because PR #${PR_NUMBER} comes from a fork.",
            "Preview deploy skipped because PR #${PR_NUMBER} head has not caught up to ${EVENT_HEAD_SHA}.",
            "pr_title<<__FOLIOH_EOF__",
        ] {
            assert!(gate_run.contains(message), "{message}");
        }

    let build = job(&workflow, "build-preview");
    assert_eq!(build["needs"], Value::from("validate"));
    assert_eq!(
        build["if"],
        Value::from("needs.validate.outputs.enabled == 'true'")
    );
    assert_eq!(
        step_names(build),
        [
            "Check out preview branch",
            "Install pnpm",
            "Set up Node",
            "Install Folioh",
            "Read GitHub Pages configuration",
            "Compute preview path",
            "Build preview docs",
            "Upload preview artifact"
        ]
    );
    let checkout = step(build, "Check out preview branch")["with"]
        .as_mapping()
        .unwrap();
    assert_eq!(
        checkout["ref"],
        Value::from("${{ needs.validate.outputs.head_sha }}")
    );
    assert_eq!(checkout["path"], Value::from("preview-source"));
    assert_eq!(checkout["persist-credentials"], Value::from(false));
    assert_installs_the_binary(build);
    let compute = step(build, "Compute preview path")["run"].as_str().unwrap();
    assert_eq!(build["permissions"]["pages"], Value::from("read"));
    assert_eq!(
        step(build, "Read GitHub Pages configuration")["uses"],
        Value::from("actions/configure-pages@v5")
    );
    assert!(compute.contains("--pages-base-path \"$PAGES_BASE_PATH\""));
    assert!(compute.contains("--pages-base-url \"$PAGES_BASE_URL\""));
    assert!(compute.contains("folioh github-pages compute-preview-path \\\n"));
    let build_docs = step(build, "Build preview docs");
    assert_eq!(
        build_docs["working-directory"],
        Value::from("preview-source")
    );
    assert_eq!(
        build_docs["env"]["FOLIOH_BASE_PATH"],
        Value::from("${{ steps.preview.outputs.base_path }}")
    );
    assert_eq!(
        build_docs["env"]["FOLIOH_PREVIEW_URL"],
        Value::from("${{ steps.preview.outputs.url }}")
    );
    assert_eq!(build_docs["run"], Value::from("folioh build --clean"));
    assert_eq!(
        step(build, "Upload preview artifact")["with"]["if-no-files-found"],
        Value::from("error")
    );
    assert_eq!(build["permissions"]["contents"], Value::from("read"));
    assert!(build["permissions"].get("id-token").is_none());

    let deploy = job(&workflow, "deploy");
    assert_eq!(
        deploy["needs"],
        serde_yaml_ng::to_value(vec!["validate", "build-preview"]).unwrap()
    );
    assert_eq!(deploy["concurrency"]["group"], Value::from("pages"));
    assert!(deploy.get("environment").is_none());
    for permission in ["contents", "issues", "pages", "id-token", "pull-requests"] {
        assert_eq!(
            deploy["permissions"][permission],
            Value::from("write"),
            "{permission}"
        );
    }
    assert_eq!(
        step_names(deploy),
        [
            "Revalidate preview PR",
            "Check out production branch",
            "Configure GitHub Pages",
            "Install pnpm",
            "Set up Node",
            "Install Folioh",
            "Build production docs",
            "Compute preview path",
            "Download preview artifact",
            "Prepare Pages artifact",
            "Copy branch preview",
            "Write preview metadata",
            "Prune stale previews",
            "Write previews data",
            "Save Pages state",
            "Upload Pages artifact",
            "Deploy to GitHub Pages",
            "Verify and print preview URL",
            "Comment preview URL",
        ]
    );
    for s in steps(deploy).iter().skip(1) {
        assert_eq!(
            s["if"],
            Value::from("steps.deploy-gate.outputs.enabled == 'true'"),
            "{:?}",
            s["name"]
        );
    }
    assert_installs_the_binary(deploy);
    assert!(step(deploy, "Revalidate preview PR")["run"]
        .as_str()
        .unwrap()
        .contains("head has not caught up to ${HEAD_SHA}."));
    assert_eq!(
        step(deploy, "Check out production branch")["with"]["ref"],
        Value::from("main")
    );
    assert_eq!(
        step(deploy, "Build production docs")["run"],
        Value::from("folioh build --clean")
    );
    assert_eq!(
        step(deploy, "Download preview artifact")["with"]["name"],
        Value::from("branch-preview-site")
    );
    for (name, fragment) in [
            ("Prepare Pages artifact", "folioh github-pages prepare-artifact \\\n  --production-site _site \\\n  --artifact-dir ../_pages-artifact"),
            ("Copy branch preview", "--preview-id \"${{ steps.preview.outputs.safe_branch }}\""),
            ("Write preview metadata", "--updated-at \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\""),
            ("Prune stale previews", "--previews-dir ../_pages-artifact/previews"),
            ("Write previews data", "folioh github-pages write-previews-data --previews-dir ../_pages-artifact/previews"),
            ("Save Pages state", "--commit-message \"Update preview for PR #${{ needs.validate.outputs.pr_number }}\""),
            ("Verify and print preview URL", "--summary-heading \"Branch preview\""),
            ("Comment preview URL", "folioh github-pages comment-preview \\\n  --repo \"${{ github.repository }}\""),
        ] {
            assert!(step(deploy, name)["run"].as_str().unwrap().contains(fragment), "{name}: {fragment}");
        }
    assert_eq!(
        step(deploy, "Deploy to GitHub Pages")["env"]["GITHUB_SHA"],
        Value::from("${{ needs.validate.outputs.head_sha }}")
    );
    assert_eq!(
        step(deploy, "Upload Pages artifact")["with"]["path"],
        Value::from("_pages-artifact")
    );
    assert!(text.contains("gh pr list --state open"));
    for forbidden in [
        "CLOUDFLARE",
        "author_association",
        "/reviews",
        "trusted approval",
        "github.event.pull_request.head.sha || github.sha",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
}
