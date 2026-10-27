//! Build-time internal link checker over the generated `.mdx` pages. Known
//! quirks are kept and pinned: fenced code is scanned, link titles stay in the
//! href, `//host` and `ftp:` count as relative.

use std::collections::HashSet;
use std::path::Path;

use folioh_config::canonicalize_lenient;

use crate::fs::{files_under, posix};
use crate::re;

/// One internal link that resolves to nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenLink {
    /// Source filename when known, otherwise the generated page route.
    pub source_page: String,
    /// The href exactly as written.
    pub target: String,
    /// 1-based line in that source file or generated `.mdx`.
    pub line_number: usize,
}

/// What `check_links` validates against.
pub struct LinkCheckInput<'a> {
    /// `.build/content`.
    pub content_dir: &'a Path,
    /// Valid routes: docs, api modules, `api-reference/index`, plugin-emitted.
    pub pages: &'a [String],
    /// `/docs` by default; a trailing `/` is stripped, `""` means `/docs`.
    pub docs_route_base: &'a str,
    /// Site-absolute routes such as `/roadmap`; `/` is always valid.
    pub site_routes: &'a HashSet<String>,
    /// `.build/public`: a site-absolute link is valid when a file exists there.
    pub static_root: Option<&'a Path>,
}

/// Normalise an internal href to a route; `None` skips it (external, anchor-only).
/// Site-absolute results keep their leading `/`.
pub fn normalize_target(href: &str, source_route: &str, docs_route_base: &str) -> Option<String> {
    if ["http://", "https://", "mailto:", "tel:"]
        .iter()
        .any(|p| href.starts_with(p))
        || href.starts_with('#')
    {
        return None;
    }
    let mut href = href
        .split('#')
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("");
    if href.is_empty() {
        return None;
    }
    for ext in [".mdx", ".md"] {
        if let Some(stripped) = href.strip_suffix(ext) {
            href = stripped;
        }
    }
    let base = docs_base(docs_route_base);
    if href == base || href.starts_with(&format!("{base}/")) {
        let remainder = href[base.len()..].trim_matches('/');
        return Some(if remainder.is_empty() {
            "index".to_string()
        } else {
            remainder.to_string()
        });
    }
    if href.starts_with('/') {
        let stripped = href.trim_matches('/');
        return Some(if stripped.is_empty() {
            "/".to_string()
        } else {
            format!("/{stripped}")
        });
    }
    let href = href.strip_prefix("./").unwrap_or(href);
    Some(file_relative_route(href, source_route).unwrap_or_else(|| href.to_string()))
}

/// `docs_route_base` without its trailing `/`; `""` means `/docs`.
fn docs_base(docs_route_base: &str) -> &str {
    match docs_route_base.trim_end_matches('/') {
        "" => "/docs",
        other => other,
    }
}

/// The link patterns, compiled once per scan. The checker and
/// `resolve_relative_links` read links through the same patterns.
pub struct LinkPatterns {
    inline_code: regex::Regex,
    markdown: regex::Regex,
    href: regex::Regex,
    href_prop: regex::Regex,
}

impl Default for LinkPatterns {
    fn default() -> Self {
        LinkPatterns {
            inline_code: re("`[^`]*`"),
            markdown: re(r"\[[^\]]*\]\(([^)]+)\)"),
            href: re(r#"\bhref\s*=\s*(?:"([^"]+)"|'([^']+)')"#),
            href_prop: re(r#"\bhref\s*:\s*(?:"([^"]+)"|'([^']+)')"#),
        }
    }
}

/// One href found on a line: where it starts in the line and its text.
#[derive(Clone, Copy)]
enum LinkKind {
    Markdown,
    Attribute,
    Property,
}

struct LinkSpan {
    kind: LinkKind,
    start: usize,
    /// A markdown destination keeps its title (`./a "Title"`).
    href: String,
}

/// Every href on a line outside inline code, in reading order per pattern:
/// markdown links (images excluded), `href="..."` attributes, then, when
/// `props`, `href:` object props such as `items={[{ href: "/docs" }]}`.
fn link_spans(line: &str, patterns: &LinkPatterns, props: bool) -> Vec<LinkSpan> {
    // Blank inline code to spaces so the spans still index the original line.
    let mut blanked = line.to_string();
    for m in patterns.inline_code.find_iter(line) {
        blanked.replace_range(m.range(), &" ".repeat(m.len()));
    }
    let mut out = Vec::new();
    for caps in patterns.markdown.captures_iter(&blanked) {
        let start = caps.get(0).unwrap().start();
        if start > 0 && blanked.as_bytes()[start - 1] == b'!' {
            continue;
        }
        let dest = caps.get(1).unwrap();
        out.push(LinkSpan {
            kind: LinkKind::Markdown,
            start: dest.start(),
            href: blanked[dest.range()].to_string(),
        });
    }
    let prop_pattern = props.then_some((&patterns.href_prop, LinkKind::Property));
    for (pattern, kind) in
        std::iter::once((&patterns.href, LinkKind::Attribute)).chain(prop_pattern)
    {
        for caps in pattern.captures_iter(&blanked) {
            if let Some(value) = caps.get(1).or_else(|| caps.get(2)) {
                out.push(LinkSpan {
                    kind,
                    start: value.start(),
                    href: blanked[value.range()].to_string(),
                });
            }
        }
    }
    out
}

/// Every href on a line outside inline code: markdown links first (images
/// excluded), then `href="..."`/`href='...'` attributes, then `href:` props.
pub fn links_in_line(line: &str) -> Vec<String> {
    links_in_line_with(line, &LinkPatterns::default())
}

/// `links_in_line` with patterns compiled by the caller.
pub fn links_in_line_with(line: &str, patterns: &LinkPatterns) -> Vec<String> {
    link_spans(line, patterns, true)
        .into_iter()
        .map(|span| span.href.trim_end().to_string())
        .collect()
}

/// Canonical file-relative docs path shared by checking and rewriting.
fn file_relative_route(path: &str, route: &str) -> Option<String> {
    let mut parts: Vec<&str> = route.split('/').collect();
    parts.pop();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            name => parts.push(name),
        }
    }
    if let Some(last) = parts.last_mut() {
        for ext in [".mdx", ".md"] {
            if let Some(stripped) = last.strip_suffix(ext) {
                *last = stripped;
            }
        }
    }
    if parts.last() == Some(&"index") {
        parts.pop();
    }
    Some(if parts.is_empty() {
        "index".to_string()
    } else {
        parts.join("/")
    })
}

/// The site-absolute docs route a relative href names, read against the page
/// file at `route` the way Markdown reads it (`./a` beside `guide/setup` is
/// `{base}/guide/a`); `.md`/`.mdx` and a trailing `index` drop, the query and
/// fragment stay. `None` leaves the href as written: external, site-absolute,
/// anchor-only, `<...>`, an expression, or a `..` that climbs out of the docs.
fn resolve_href(href: &str, route: &str, docs_route_base: &str) -> Option<String> {
    if href.is_empty() || href.starts_with(['/', '#', '?', '<', '{']) {
        return None;
    }
    let (path, suffix) = href.split_at(href.find(['?', '#']).unwrap_or(href.len()));
    if path
        .split('/')
        .next()
        .is_some_and(|first| first.contains(':'))
    {
        return None;
    }
    let target = file_relative_route(path, route)?;
    let base = docs_base(docs_route_base);
    Some(if target == "index" {
        format!("{base}{suffix}")
    } else {
        format!("{base}/{target}{suffix}")
    })
}

/// Rewrite every relative link in a generated page to the site-absolute docs
/// route it names (see `resolve_href`), through the patterns the checker
/// reads. A relative href in rendered MDX resolves against the page URL,
/// which ends in `/`, so `./installation` on `/docs/quickstart/` would open
/// `/docs/quickstart/installation`; the absolute route lands on the same page
/// from the client router, a cold load and the Markdown mirror, and Next adds
/// the deploy base path itself. Frontmatter, fenced and inline code, images
/// and link titles stay as written.
pub fn resolve_relative_links(mdx: &str, route: &str, docs_route_base: &str) -> String {
    if !mdx.contains("](") && !mdx.contains("href") {
        return mdx.to_string();
    }
    let patterns = LinkPatterns::default();
    let lines: Vec<&str> = mdx.split_inclusive('\n').collect();
    let bare: Vec<&str> = lines
        .iter()
        .map(|line| line.trim_end_matches(['\n', '\r']))
        .collect();
    let fenced = folioh_mdx::code_fence_flags(&bare);
    let body_start = match bare.first() {
        Some(&"---") => bare
            .iter()
            .skip(1)
            .position(|line| *line == "---")
            .map_or(0, |close| close + 2),
        _ => 0,
    };
    let mut out = String::with_capacity(mdx.len());
    for (index, line) in lines.iter().enumerate() {
        if index < body_start || fenced[index] {
            out.push_str(line);
            continue;
        }
        let mut spans = link_spans(line, &patterns, true);
        spans.sort_by_key(|span| span.start);
        let mut last = 0;
        for span in spans {
            if span.start < last {
                continue;
            }
            let url_len = span
                .href
                .find(char::is_whitespace)
                .unwrap_or(span.href.len());
            let Some(resolved) = resolve_href(&span.href[..url_len], route, docs_route_base) else {
                continue;
            };
            out.push_str(&line[last..span.start]);
            out.push_str(&resolved);
            last = span.start + url_len;
        }
        out.push_str(&line[last..]);
    }
    out
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && bytes[i + 1].is_ascii_hexdigit()
            && bytes[i + 2].is_ascii_hexdigit()
        {
            out.push(u8::from_str_radix(&text[i + 1..i + 3], 16).expect("two hex digits"));
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn static_target_exists(target: &str, static_root: Option<&Path>) -> bool {
    let Some(root) = static_root else {
        return false;
    };
    let Ok(root) = root.canonicalize() else {
        return false;
    };
    let candidate =
        canonicalize_lenient(&root.join(percent_decode(target).trim_start_matches('/')));
    if !candidate.starts_with(&root) {
        return false;
    }
    candidate.is_file() || candidate.join("index.html").is_file()
}

/// Python `str.splitlines()`.
fn splitlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let is_break = matches!(
            c,
            '\n' | '\r'
                | '\x0b'
                | '\x0c'
                | '\x1c'
                | '\x1d'
                | '\x1e'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        );
        if !is_break {
            continue;
        }
        lines.push(&text[start..i]);
        let mut end = i + c.len_utf8();
        if c == '\r' {
            if let Some((j, '\n')) = chars.peek().copied() {
                chars.next();
                end = j + 1;
            }
        }
        start = end;
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// Report every internal link under `content_dir` that matches no known page,
/// its `/index` twin, a site route or a static file.
pub fn check_links(input: &LinkCheckInput) -> Vec<BrokenLink> {
    let known: HashSet<&str> = input.pages.iter().map(String::as_str).collect();
    let patterns = LinkPatterns::default();
    let mut broken = Vec::new();
    for path in files_under(input.content_dir) {
        if path.extension().map(|e| e != "mdx").unwrap_or(true) {
            continue;
        }
        let rel = path.strip_prefix(input.content_dir).unwrap_or(&path);
        let source_route = posix(&rel.with_extension(""));
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let lines = splitlines(&text);
        let fenced = folioh_mdx::code_fence_flags(&lines);
        for (index, line) in lines.iter().enumerate() {
            // Fenced code is scanned except for `href:` props,
            // which there are config keys and code, not page links.
            let spans = link_spans(line, &patterns, !fenced[index]);
            for href in spans
                .into_iter()
                .map(|span| span.href.trim_end().to_string())
            {
                let Some(normalized) =
                    normalize_target(&href, &source_route, input.docs_route_base)
                else {
                    continue;
                };
                let report = || BrokenLink {
                    source_page: source_route.clone(),
                    target: href.clone(),
                    line_number: index + 1,
                };
                if normalized.starts_with('/') {
                    let site_target = match normalized.trim_end_matches('/') {
                        "" => "/",
                        other => other,
                    };
                    if site_target == "/" || input.site_routes.contains(site_target) {
                        continue;
                    }
                    let raw = href
                        .split('#')
                        .next()
                        .unwrap_or("")
                        .split('?')
                        .next()
                        .unwrap_or("");
                    if static_target_exists(site_target, input.static_root)
                        || static_target_exists(raw, input.static_root)
                    {
                        continue;
                    }
                    broken.push(report());
                    continue;
                }
                let normalized = normalized.trim_end_matches('/');
                if !known.contains(normalized)
                    && !known.contains(format!("{normalized}/index").as_str())
                {
                    broken.push(report());
                }
            }
        }
    }
    broken
}

#[cfg(test)]
#[path = "links_tests.rs"]
mod tests;

#[path = "links_fragments.rs"]
mod fragments;
pub use fragments::check_fragments;
