//! Fragment checks use the exported IDs, including renderer-generated headings
//! and explicit component/API anchors. No second heading slugger is needed.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use folioh_mdx::MarkdownPage;

use super::{
    docs_base, link_spans, normalize_target, percent_decode, BrokenLink, LinkKind, LinkPatterns,
};
use crate::fs::{files_under, posix};
use crate::{re, Result, SiteError};

/// Validate fragments after a static export. Source documents retain their
/// original filename and line; generated pages use their MDX file. The index
/// is rebuilt each time, so a warm export cannot retain a removed heading.
/// Missing pages are left to the route checker; non-HTML assets are skipped.
pub fn check_fragments(
    content_dir: &Path,
    output_dir: &Path,
    docs_route_base: &str,
    docs: &[MarkdownPage],
) -> Result<Vec<BrokenLink>> {
    let mut ids = HashMap::new();
    let hidden = re(r"(?is)<!--.*?-->|<script\b[^>]*>.*?</script\s*>|<style\b[^>]*>.*?</style\s*>");
    let tag = re(r#"(?s)<[A-Za-z](?:"[^"]*"|'[^']*'|[^'">])*>"#);
    let attr = re(r#"\s+([^\s=/>]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?"#);
    for path in files_under(output_dir) {
        if path.extension().is_none_or(|ext| ext != "html") {
            continue;
        }
        let html = std::fs::read_to_string(&path).map_err(|e| SiteError::io(&path, e))?;
        let html = hidden.replace_all(&html, "");
        let anchors: HashSet<String> = tag
            .find_iter(&html)
            .flat_map(|tag| attr.captures_iter(tag.as_str()))
            .filter(|cap| cap[1].eq_ignore_ascii_case("id"))
            .filter_map(|cap| {
                cap.get(2)
                    .or_else(|| cap.get(3))
                    .or_else(|| cap.get(4))
                    .map(|value| html_escape::decode_html_entities(value.as_str()).into_owned())
            })
            .collect();
        ids.insert(
            path.strip_prefix(output_dir).unwrap().to_path_buf(),
            anchors,
        );
    }
    let patterns = LinkPatterns::default();
    let omitted = re(
        r"(?is)<!--.*?-->|\{/\*.*?\*/\}|<(script|style|iframe)\b[^>]*>.*?</(?:script|style|iframe)\s*>",
    );
    let mut broken = Vec::new();
    for generated in files_under(content_dir) {
        if generated.extension().is_none_or(|ext| ext != "mdx") {
            continue;
        }
        let route = posix(
            &generated
                .strip_prefix(content_dir)
                .unwrap()
                .with_extension(""),
        );
        let source_public = format!(
            "{}/{}",
            docs_base(docs_route_base),
            route
                .strip_suffix("/index")
                .unwrap_or(if route == "index" { "" } else { &route })
        );
        if anchors_at(&ids, &source_public).is_none() {
            continue;
        }
        let source = docs
            .iter()
            .find(|doc| doc.route == route)
            .map(|doc| Path::new(&doc.source_file))
            .filter(|path| path.is_file())
            .unwrap_or(&generated);
        let text = std::fs::read_to_string(source).map_err(|e| SiteError::io(source, e))?;
        // Preserve newline/byte positions while dropping content that is not
        // rendered. Fences and frontmatter are handled separately below.
        let text = omitted.replace_all(&text, |caps: &regex::Captures<'_>| {
            caps[0]
                .chars()
                .map(|c| if c == '\n' || c == '\r' { c } else { ' ' })
                .collect::<String>()
        });
        let lines = super::splitlines(&text);
        let fences = folioh_mdx::code_fence_flags(&lines);
        let mut frontmatter = lines.first().is_some_and(|line| line.trim() == "---");
        for (i, line) in lines.iter().enumerate() {
            if frontmatter {
                if i > 0 && line.trim() == "---" {
                    frontmatter = false;
                }
                continue;
            }
            if fences[i] {
                continue;
            }
            for span in link_spans(line, &patterns, true) {
                let href = match span.kind {
                    LinkKind::Property => span.href.clone(),
                    LinkKind::Attribute => {
                        html_escape::decode_html_entities(&span.href).into_owned()
                    }
                    LinkKind::Markdown => {
                        let dest = span.href.trim();
                        let dest = if let Some(angle) = dest.strip_prefix('<') {
                            angle.split('>').next().unwrap_or(angle)
                        } else {
                            dest.split_whitespace().next().unwrap_or("")
                        };
                        html_escape::decode_html_entities(dest).into_owned()
                    }
                };
                let Some((path, fragment)) = href.split_once('#') else {
                    continue;
                };
                if path.starts_with("//")
                    || path
                        .split(['/', '?'])
                        .next()
                        .is_some_and(|first| first.contains(':'))
                {
                    continue;
                }
                let fragment = fragment.split(":~:").next().unwrap_or("");
                if fragment.is_empty() {
                    continue;
                }
                let target = if path.is_empty() || path.starts_with('?') {
                    route.clone()
                } else if let Some(target) = normalize_target(&href, &route, docs_route_base) {
                    target
                } else {
                    continue;
                };
                let public = if target.starts_with('/') {
                    target
                } else {
                    let target = target.trim_end_matches('/');
                    let target = target.strip_suffix("/index").unwrap_or(target);
                    let target = if target == "index" { "" } else { target };
                    format!("{}/{target}", docs_base(docs_route_base))
                };
                let Some(anchors) = anchors_at(&ids, &public) else {
                    continue;
                };
                if !anchors.contains(&percent_decode(fragment)) {
                    broken.push(BrokenLink {
                        source_page: source.to_string_lossy().into_owned(),
                        target: span.href,
                        line_number: i + 1,
                    });
                }
            }
        }
    }
    Ok(broken)
}

fn anchors_at<'a>(
    ids: &'a HashMap<PathBuf, HashSet<String>>,
    public: &str,
) -> Option<&'a HashSet<String>> {
    let public = PathBuf::from(percent_decode(public.trim_matches('/')));
    if public
        .components()
        .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return None;
    }
    if public.extension().is_some_and(|ext| ext == "html") {
        return ids.get(&public);
    }
    ids.get(&public.join("index.html")).or_else(|| {
        public
            .extension()
            .is_none()
            .then(|| ids.get(&public.with_extension("html")))
            .flatten()
    })
}

#[cfg(test)]
#[path = "links_fragments_tests.rs"]
mod tests;
