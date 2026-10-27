//! Post-processing of the static export so it also works from `file://`:
//! relative URL rewriting, `opengraph-image.png` copies, the Turbopack
//! chunk-prefix shim and a Pagefind-derived search fallback.

use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;

use folioh_config::canonicalize_lenient;
use serde_json::Value;

use crate::fs::{files_under, write_text_if_changed};
use crate::re;

/// Rewrites one exported site in place.
pub struct StaticAssetRewriter {
    /// The exported site root.
    pub output_dir: PathBuf,
}

const PORTABLE_PREFIX: &str = r#"(()=>{let e="object"==typeof document&&document.currentScript&&document.currentScript.getAttribute("src")||"";if(e&&/^https?:$/.test(location.protocol))e=new URL(e,document.baseURI).pathname;let t=e.match(/^((?:.*\/)?_next\/)static\/chunks\//);return t?t[1]:"/_next/"})()"#;

/// `(scheme, netloc, path, query, fragment)` as Python's `urlsplit` sees a URL.
fn urlsplit(url: &str) -> (String, String, String, String, String) {
    let (rest, fragment) = match url.split_once('#') {
        Some((r, f)) => (r.to_string(), f.to_string()),
        None => (url.to_string(), String::new()),
    };
    let (rest, query) = match rest.split_once('?') {
        Some((r, q)) => (r.to_string(), q.to_string()),
        None => (rest, String::new()),
    };
    let (scheme, rest) = match rest.find(':') {
        Some(idx)
            if idx > 0
                && rest[..idx]
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
                && rest[..idx]
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_alphabetic())
                    .unwrap_or(false) =>
        {
            (rest[..idx].to_lowercase(), rest[idx + 1..].to_string())
        }
        _ => (String::new(), rest),
    };
    let (netloc, path) = match rest.strip_prefix("//") {
        Some(after) => match after.find('/') {
            Some(idx) => (after[..idx].to_string(), after[idx..].to_string()),
            None => (after.to_string(), String::new()),
        },
        None => (String::new(), rest),
    };
    (scheme, netloc, path, query, fragment)
}

/// `os.path.relpath(target, start)` with `/` separators.
fn relpath(target: &Path, start: &Path) -> String {
    let target: Vec<_> = target.components().collect();
    let start: Vec<_> = start.components().collect();
    let common = target
        .iter()
        .zip(&start)
        .take_while(|(a, b)| a == b)
        .count();
    let mut parts: Vec<String> =
        std::iter::repeat_n("..".to_string(), start.len() - common).collect();
    parts.extend(
        target[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

impl StaticAssetRewriter {
    /// A rewriter over `output_dir`.
    pub fn new(output_dir: &Path) -> Self {
        StaticAssetRewriter {
            output_dir: output_dir.to_path_buf(),
        }
    }

    /// Rewrite every `.html`, `.txt` and `.js` file; returns warnings.
    pub fn fix_asset_paths(&self) -> std::io::Result<Vec<String>> {
        self.fix_asset_paths_with_base_path("")
    }

    /// As `fix_asset_paths`, with the deployment root used by the fallback
    /// 404 page, which a host can serve at an arbitrary missing URL.
    pub fn fix_asset_paths_with_base_path(&self, base_path: &str) -> std::io::Result<Vec<String>> {
        let mut warnings = Vec::new();
        self.copy_opengraph_images_with_png_extension()?;
        let has_file_search = self.write_file_search_fallback()?;
        let has_navigation = files_under(&self.output_dir)
            .iter()
            .any(|path| path.extension().is_some_and(|ext| ext == "html"));
        if has_navigation {
            write_text_if_changed(
                &self.output_dir.join("_folioh-navigation.js"),
                include_str!("navigation.js"),
            )?;
        }
        for path in files_under(&self.output_dir) {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if !(name.ends_with(".html") || name.ends_with(".txt") || name.ends_with(".js")) {
                continue;
            }
            if self.is_next_asset(&path) {
                self.patch_next_runtime_asset_prefix(&path, &mut warnings)?;
                continue;
            }
            let Ok(original) = std::fs::read_to_string(&path) else {
                continue;
            };
            // A host serves the root 404 page in place of any missing URL, at
            // any depth, and never from `file://`: it keeps the export's
            // root-absolute URLs.
            let root_404 = path == self.output_dir.join("404.html");
            let mut content = original.clone();
            let asset_base = if root_404 { base_path } else { "" };
            if has_file_search && name.ends_with(".html") {
                content = Self::inject_script(&content, "_folioh-search.js", asset_base);
            }
            if has_navigation && name.ends_with(".html") {
                content = Self::inject_script(&content, "_folioh-navigation.js", asset_base);
            }
            content = Self::rewrite_opengraph_image_urls(&content);
            let patched = if root_404 {
                content
            } else {
                self.rewrite_file_urls(&content, &path)
            };
            if patched != original {
                std::fs::write(&path, patched)?;
            }
        }
        Ok(warnings)
    }

    fn copy_opengraph_images_with_png_extension(&self) -> std::io::Result<()> {
        for path in files_under(&self.output_dir) {
            if path
                .file_name()
                .map(|n| n != "opengraph-image")
                .unwrap_or(true)
            {
                continue;
            }
            let image = std::fs::read(&path)?;
            let png = path.with_file_name("opengraph-image.png");
            if std::fs::read(&png)
                .map(|existing| existing != image)
                .unwrap_or(true)
            {
                std::fs::write(&png, &image)?;
            }
        }
        Ok(())
    }

    /// `opengraph-image` followed by `?`, `"`, `'`, `<`, `\` or the end becomes `opengraph-image.png`.
    pub fn rewrite_opengraph_image_urls(content: &str) -> String {
        const NEEDLE: &str = "opengraph-image";
        let mut out = String::with_capacity(content.len());
        let mut rest = content;
        while let Some(idx) = rest.find(NEEDLE) {
            let after = &rest[idx + NEEDLE.len()..];
            out.push_str(&rest[..idx + NEEDLE.len()]);
            if after.is_empty() || after.starts_with(['?', '"', '\'', '<', '\\']) {
                out.push_str(".png");
            }
            rest = after;
        }
        out.push_str(rest);
        out
    }

    fn stash_script_bodies(content: &str) -> (String, Vec<String>) {
        let mut bodies = Vec::new();
        let stashed = re(r"(?is)(<script\b[^>]*>)(.*?)(</script>)")
            .replace_all(content, |caps: &regex::Captures| {
                bodies.push(caps[2].to_string());
                format!(
                    "{}@@FOLIOH_SCRIPT_BODY_{}@@{}",
                    &caps[1],
                    bodies.len() - 1,
                    &caps[3]
                )
            })
            .into_owned();
        (stashed, bodies)
    }

    /// Relativise `href`/`src`/`poster`/`data-href` attributes and serialised
    /// root-relative attribute values, leaving `<script>` bodies alone.
    pub fn rewrite_file_urls(&self, content: &str, fpath: &Path) -> String {
        let (mut content, bodies) = Self::stash_script_bodies(content);
        let attr = re(r#"(data-href|href|poster|src)=("[^"']+"|'[^"']+')"#);
        let bytes: Vec<u8> = content.bytes().collect();
        let mut out = String::with_capacity(content.len());
        let mut last = 0;
        for caps in attr.captures_iter(&content.clone()) {
            let whole = caps.get(0).unwrap();
            let start = whole.start();
            if start > 0 {
                let prev = bytes[start - 1];
                if prev.is_ascii_alphanumeric() || prev == b'_' || prev == b':' || prev == b'-' {
                    continue;
                }
            }
            let quoted = &caps[2];
            let quote = &quoted[..1];
            let url = &quoted[1..quoted.len() - 1];
            let rewritten = self.rewrite_file_url(url, fpath, &caps[1] == "data-href");
            out.push_str(&content[last..start]);
            out.push_str(&format!("{}={quote}{rewritten}{quote}", &caps[1]));
            last = whole.end();
        }
        out.push_str(&content[last..]);
        content = out;
        let serialized = re(r#"(\\?"(data-href|href|poster|src)\\?":\\?")(/[^"\\]+)(\\?")"#);
        content = serialized
            .replace_all(&content, |caps: &regex::Captures| {
                // Flight stylesheets load after client navigation, so their
                // base is the browser's current route, not this payload file.
                if caps[3].contains("/_next/") {
                    return caps[0].to_string();
                }
                format!(
                    "{}{}{}",
                    &caps[1],
                    // Flight props describe router destinations rather than
                    // file loads; keep directory URLs for client navigation.
                    self.rewrite_file_url(
                        &caps[3],
                        fpath,
                        matches!(&caps[2], "href" | "data-href"),
                    ),
                    &caps[4]
                )
            })
            .into_owned();
        for (index, body) in bodies.iter().enumerate() {
            content = content.replace(&format!("@@FOLIOH_SCRIPT_BODY_{index}@@"), body);
        }
        content
    }

    /// Relativise one URL against the file holding it; unchanged when it is
    /// external, empty, an anchor or unresolvable.
    pub fn rewrite_file_url(&self, url: &str, fpath: &Path, route_index: bool) -> String {
        let (scheme, netloc, path, query, fragment) = urlsplit(url);
        if !scheme.is_empty()
            || !netloc.is_empty()
            || path.is_empty()
            || path.starts_with('#')
            || ["mailto:", "tel:", "javascript:", "data:", "blob:", "//"]
                .iter()
                .any(|p| url.starts_with(p))
        {
            return url.to_string();
        }
        let root = canonicalize_lenient(&self.output_dir);
        let current_dir = canonicalize_lenient(fpath.parent().unwrap_or(Path::new(".")));
        let target = if let Some(rooted) = path.strip_prefix('/') {
            Self::file_url_target_under_root(&root.join(rooted), &root)
        } else {
            let mut target = Self::file_url_target_under_root(&current_dir.join(&path), &root);
            if target.is_none()
                && fpath
                    .file_name()
                    .map(|n| n == "index.html")
                    .unwrap_or(false)
                && current_dir != root
            {
                target = Self::file_url_target_under_root(
                    &current_dir.parent().unwrap_or(&root).join(&path),
                    &root,
                );
            }
            target
        };
        let Some(target) = target else {
            return url.to_string();
        };
        let is_index = target
            .file_name()
            .map(|n| n == "index.html")
            .unwrap_or(false);
        let relative_target = if route_index && is_index {
            target.parent().unwrap().to_path_buf()
        } else {
            target.clone()
        };
        let mut relative = relpath(&relative_target, &current_dir);
        if relative.starts_with("_next/") {
            relative = format!("./{relative}");
        }
        if route_index && is_index {
            relative = if relative == "." {
                "./".to_string()
            } else {
                format!("{}/", relative.trim_end_matches('/'))
            };
        }
        if !query.is_empty() {
            relative = format!("{relative}?{query}");
        }
        if !fragment.is_empty() {
            relative = format!("{relative}#{fragment}");
        }
        relative
    }

    fn is_next_asset(&self, fpath: &Path) -> bool {
        canonicalize_lenient(fpath)
            .strip_prefix(canonicalize_lenient(&self.output_dir))
            .ok()
            .and_then(|rel| rel.components().next().map(|c| c.as_os_str() == "_next"))
            .unwrap_or(false)
    }

    fn patch_next_runtime_asset_prefix(
        &self,
        fpath: &Path,
        warnings: &mut Vec<String>,
    ) -> std::io::Result<()> {
        let name = fpath
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !name.starts_with("turbopack-") || !name.ends_with(".js") {
            return Ok(());
        }
        let content = std::fs::read_to_string(fpath)?;
        if !content.contains("\"/_next/\"") {
            return Ok(());
        }
        let mut patched = content.replacen("\"/_next/\"", PORTABLE_PREFIX, 1);
        let ident = r"[A-Za-z_$][\w$]*";
        // Registration keys must use the same stable HTTP paths as the
        // loader; mixing relative and root paths leaves modules unhydrated.
        patched = re(&format!(r#"src:({ident})\.getAttribute\("src"\)"#))
            .replace_all(&patched, |caps: &regex::Captures| {
                format!(
                    r#"src:/^https?:$/.test(location.protocol)?(()=>{{let u=new URL({0}.src);return u.pathname+u.search+u.hash}})():{0}.getAttribute("src")"#,
                    &caps[1]
                )
            })
            .into_owned();
        let simple = re(&format!(
            r#"function ({ident})\(e\)\{{return`\$\{{({ident})\}}\$\{{(e|e\.split\("/"\)\.map\(e=>encodeURIComponent\(e\)\)\.join\("/"\))\}}\$\{{({ident})\}}`\}}"#
        ));
        let mut matches = 0;
        patched = simple
            .replace_all(&patched, |caps: &regex::Captures| {
                matches += 1;
                format!(r#"function {}(e){{let _fp=e.indexOf("/_next/");_fp>=0&&(e=e.slice(_fp+7));return`${{{}}}${{{}}}${{{}}}`}}"#, &caps[1], &caps[2], &caps[3], &caps[4])
            })
            .into_owned();
        let encoded = re(&format!(
            r#"function ({ident})\(({ident}),({ident})=({ident})\)\{{let ({ident})=({ident})\.test\(({ident})\)\?({ident})\.split\("/"\)\.map\(encodeURIComponent\)\.join\("/"\):({ident});return`\$\{{({ident})\}}\$\{{({ident})\}}\$\{{({ident})\}}`\}}"#
        ));
        patched = encoded
            .replace_all(&patched, |caps: &regex::Captures| {
                let path = &caps[2];
                let same_path = &caps[7] == path && &caps[8] == path && &caps[9] == path;
                let same_prefix = caps[10] == caps[3];
                let same_encoded = caps[11] == caps[5];
                if !(same_path && same_prefix && same_encoded) {
                    return caps[0].to_string();
                }
                matches += 1;
                let guard = format!(
                    r#"let _fp={path}.indexOf("/_next/");_fp>=0&&({path}={path}.slice(_fp+7));"#
                );
                caps[0].replacen('{', &format!("{{{guard}"), 1)
            })
            .into_owned();
        if matches == 0 {
            warnings.push(format!(
                "static_rewriter: Turbopack chunk-path function not found in {name}; the runtime shape may have changed and hydration of relative-URL exports may break. Update StaticAssetRewriter::patch_next_runtime_asset_prefix for the new runtime."
            ));
        }
        if patched != content {
            std::fs::write(fpath, patched)?;
        }
        Ok(())
    }

    fn write_file_search_fallback(&self) -> std::io::Result<bool> {
        let fragment_dir = self.output_dir.join("_pagefind").join("fragment");
        if !fragment_dir.exists() {
            return Ok(false);
        }
        let mut documents = Vec::new();
        for path in files_under(&fragment_dir)
            .into_iter()
            .filter(|p| p.extension().map(|e| e == "pf_fragment").unwrap_or(false))
        {
            let Some(fragment) = Self::read_pagefind_fragment(&path) else {
                continue;
            };
            let url =
                self.pagefind_file_url(fragment.get("url").and_then(Value::as_str).unwrap_or(""));
            let content = fragment
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let title = fragment
                .get("meta")
                .and_then(|m| m.get("title"))
                .and_then(Value::as_str)
                .filter(|t| !t.is_empty())
                .map(str::to_string)
                .or_else(|| {
                    content
                        .split_once('.')
                        .map(|(head, _)| head.to_string())
                        .or_else(|| Some(content.clone()))
                        .filter(|t| !t.is_empty())
                })
                .unwrap_or_else(|| url.clone());
            documents.push(serde_json::json!({"url": url, "title": title, "content": content}));
        }
        if documents.is_empty() {
            return Ok(false);
        }
        let index_json = serde_json::to_string(&documents).expect("json");
        std::fs::write(
            self.output_dir.join("_folioh-search.js"),
            Self::file_search_script(&index_json),
        )?;
        Ok(true)
    }

    fn read_pagefind_fragment(path: &Path) -> Option<serde_json::Map<String, Value>> {
        let mut payload = Vec::new();
        GzDecoder::new(std::fs::File::open(path).ok()?)
            .read_to_end(&mut payload)
            .ok()?;
        let start = payload.iter().position(|b| *b == b'{')?;
        match serde_json::from_slice::<Value>(&payload[start..]).ok()? {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    fn pagefind_file_url(&self, url: &str) -> String {
        let (scheme, netloc, path, query, fragment) = urlsplit(url);
        if !scheme.is_empty() || !netloc.is_empty() {
            return url.to_string();
        }
        let root = canonicalize_lenient(&self.output_dir);
        let stripped = path.trim_start_matches('/');
        let mut result = match Self::file_url_target(&canonicalize_lenient(&root.join(stripped))) {
            Some(target) => relpath(&target, &root),
            None if stripped.is_empty() => "index.html".to_string(),
            None if stripped.ends_with('/') => format!("{stripped}index.html"),
            None if Path::new(stripped).extension().is_none() => format!("{stripped}/index.html"),
            None => stripped.to_string(),
        };
        if !query.is_empty() {
            result = format!("{result}?{query}");
        }
        if !fragment.is_empty() {
            result = format!("{result}#{fragment}");
        }
        result
    }

    fn inject_script(content: &str, name: &str, base_path: &str) -> String {
        let existing = re(&format!(
            r#"(?i)<script\b[^>]*\bsrc=["'][^"']*{}(?:\?[^"']*)?["']"#,
            regex::escape(name)
        ));
        if existing.is_match(content) {
            return content.to_string();
        }
        let src = format!("{}/{name}", base_path.trim_end_matches('/'))
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;");
        let script = format!("<script defer src=\"{src}\"></script>");
        if content.contains("<head>") {
            content.replacen("<head>", &format!("<head>{script}"), 1)
        } else if content.contains("</head>") {
            content.replacen("</head>", &format!("{script}</head>"), 1)
        } else {
            content.to_string()
        }
    }

    fn file_search_script(index_json: &str) -> String {
        format!(
            "(function(){{\nconst documents={index_json};\nlet options={{}};\nconst script=document.currentScript;\nconst rootHref=new URL('./',script&&script.src||document.baseURI).href;\n{}",
            r#"function normalize(value){return String(value||'').normalize('NFD').replace(/[̀-ͯ]/g,'').toLowerCase();}
function escapeHtml(value){return String(value).replace(/[&<>"']/g,function(ch){return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[ch];});}
function escapeRegex(value){return String(value).replace(/[.*+?^${}()|[\]\\]/g,'\\$&');}
function splitPath(path){return path.split('/').filter(Boolean);}
function currentDir(){const current=new URL(location.href);const root=new URL(rootHref);let rel=decodeURIComponent(current.pathname).slice(decodeURIComponent(root.pathname).length);if(!rel||rel.endsWith('/'))return rel;return rel.split('/').slice(0,-1).join('/')+'/';}
function relativePath(target){if(/^(https?:)?\/\//.test(target))return target;const hashIndex=target.indexOf('#');const hash=hashIndex>=0?target.slice(hashIndex):'';const noHash=hashIndex>=0?target.slice(0,hashIndex):target;const queryIndex=noHash.indexOf('?');const query=queryIndex>=0?noHash.slice(queryIndex):'';const path=queryIndex>=0?noHash.slice(0,queryIndex):noHash;const from=splitPath(currentDir());const to=splitPath(path);while(from.length&&to.length&&from[0]===to[0]){from.shift();to.shift();}let rel=[...from.map(function(){return '..';}),...to].join('/')||'index.html';if(location.protocol==='file:'&&/\.html$/.test(rel))rel+='?folioh-search=1';return rel+query+hash;}
function excerpt(content,terms){const normalized=normalize(content);let index=-1;for(const term of terms){index=normalized.indexOf(term);if(index!==-1)break;}const start=Math.max(0,index-80);const raw=String(content||'').slice(start,start+220).trim();let html=escapeHtml((start>0?'... ':'')+raw+(start+220<String(content||'').length?' ...':''));for(const term of terms){if(!term)continue;html=html.replace(new RegExp('('+escapeRegex(escapeHtml(term))+')','ig'),'<mark>$1</mark>');}return html;}
function search(term){const terms=normalize(term).split(/\s+/).filter(Boolean);if(!terms.length)return [];return documents.map(function(doc){const title=normalize(doc.title);const content=normalize(doc.content);const haystack=title+' '+content;if(!terms.every(function(token){return haystack.includes(token);})){return null;}const titleHits=terms.filter(function(token){return title.includes(token);}).length;const score=titleHits*10+terms.reduce(function(total,token){return total+(content.includes(token)?1:0);},0);const href=relativePath(doc.url);return {id:doc.url,score:score,words:[],data:async function(){return {url:href,meta:{title:doc.title},sub_results:[{title:doc.title,url:href,excerpt:excerpt(doc.content,terms)}]};}};}).filter(Boolean).sort(function(a,b){return b.score-a.score;});}
const api={options:async function(nextOptions){options=nextOptions||options;},preload:async function(){return null;},search:async function(term){const results=search(term);return {results:results,unfilteredResultCount:results.length,filters:{},totalFilters:{},timings:{preload:0,search:0,total:0}};},debouncedSearch:async function(term){return api.search(term,options);}};
window.__foliohStaticSearch=api;
if(location.protocol==='file:'&&!window.pagefind){window.pagefind=api;}
})();
"#
        )
    }

    fn file_url_target_under_root(target: &Path, root: &Path) -> Option<PathBuf> {
        let resolved = canonicalize_lenient(target);
        if !resolved.starts_with(root) {
            return None;
        }
        Self::file_url_target(&resolved)
    }

    fn file_url_target(target: &Path) -> Option<PathBuf> {
        if target.is_dir() {
            let index = target.join("index.html");
            return index.exists().then_some(index);
        }
        if target.is_file() {
            return Some(target.to_path_buf());
        }
        let index = target.join("index.html");
        if index.exists() {
            return Some(index);
        }
        if target.extension().is_none() {
            let html = target.with_extension("html");
            if html.exists() {
                return Some(html);
            }
        }
        None
    }
}

#[cfg(test)]
#[path = "rewriter_tests.rs"]
mod tests;
