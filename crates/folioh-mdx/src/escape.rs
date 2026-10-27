//! MDX prose escaping shared with the IR renderer in `folioh-docs`. The rules
//! need lookbehind and backreferences, so these are hand-written scanners
//! rather than regexes.

/// `{`→`\{`, `}`→`\}`, `<`→`&lt;`, `>`→`&gt;`, in that order. Prose only.
pub fn escape_mdx_text(text: &str) -> String {
    escape_dunder_identifiers(text)
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Dunder identifiers are literal prose, rather than Markdown strong emphasis.
/// Leave escaped text and URL destinations alone; callers spare code spans.
fn escape_dunder_identifiers(text: &str) -> String {
    static NAMES: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let names = NAMES.get_or_init(|| crate::re(r"\b__[A-Za-z][A-Za-z0-9_]*__\b"));
    names
        .replace_all(text, |captures: &regex::Captures<'_>| {
            let name = captures.get(0).expect("matched identifier");
            let prefix = text[..name.start()]
                .rsplit(char::is_whitespace)
                .next()
                .unwrap_or("");
            let escaped = prefix
                .bytes()
                .rev()
                .take_while(|&byte| byte == b'\\')
                .count()
                % 2
                == 1;
            if escaped || prefix.contains("://") || prefix.contains("](") {
                name.as_str().to_string()
            } else {
                name.as_str().replace('_', "\\_")
            }
        })
        .into_owned()
}

/// A value placed inside a double-quoted JSX attribute.
pub fn escape_jsx_attr(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('{', "&#123;")
        .replace('}', "&#125;")
}

/// End of the code span opening at `start`, emulating `(`+)(.+?)\1` (DOTALL):
/// the opener is greedy and backtracks, the body is at least one char, and the
/// first later run of the opener's length closes it.
fn inline_code_span(text: &str, start: usize) -> Option<usize> {
    let run = text.as_bytes()[start..]
        .iter()
        .take_while(|&&b| b == b'`')
        .count();
    for n in (1..=run).rev() {
        let body = start + n;
        let first = text[body..].chars().next()?;
        let from = body + first.len_utf8();
        if let Some(pos) = text[from..].find(&text[start..body]) {
            return Some(from + pos + n);
        }
    }
    None
}

/// Escapes prose between inline code spans; the spans are copied verbatim and
/// an unterminated backtick is literal text.
fn escape_outside_inline_code(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let (mut pos, mut i) = (0, 0);
    while i < text.len() {
        if text.as_bytes()[i] == b'`' {
            if let Some(end) = inline_code_span(text, i) {
                out.push_str(&escape_mdx_text(&text[pos..i]));
                out.push_str(&text[i..end]);
                pos = end;
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out.push_str(&escape_mdx_text(&text[pos..]));
    out
}

/// `^\s*(`{3,}|~{3,})(.*)$` → (fence char, run length, info string).
fn fence_marker(line: &str) -> Option<(u8, usize, &str)> {
    let rest = line.trim_start();
    let first = *rest.as_bytes().first()?;
    if first != b'`' && first != b'~' {
        return None;
    }
    let run = rest.bytes().take_while(|&b| b == first).count();
    (run >= 3).then(|| (first, run, &rest[run..]))
}

/// Marks the lines inside a fenced block, fence lines included. Only a run of
/// the opener's own character, at least as long and with no info string,
/// closes it, so a 3-tick fence nested in a 4-tick one does not.
pub fn code_fence_flags(lines: &[&str]) -> Vec<bool> {
    let mut flags = Vec::with_capacity(lines.len());
    let mut fence: Option<(u8, usize)> = None;
    for line in lines {
        let marker = fence_marker(line);
        match fence {
            None => {
                fence = marker.map(|(c, n, _)| (c, n));
                flags.push(marker.is_some());
            }
            Some((c, n)) => {
                flags.push(true);
                if let Some((mc, mn, info)) = marker {
                    if mc == c && mn >= n && info.trim().is_empty() {
                        fence = None;
                    }
                }
            }
        }
    }
    flags
}

/// Escapes docstring prose, leaving fenced blocks and inline code alone.
pub fn escape_mdx(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let in_code = code_fence_flags(&lines);
    let mut out = Vec::new();
    let mut start = 0;
    while start < lines.len() {
        let mut end = start;
        while end < lines.len() && in_code[end] == in_code[start] {
            end += 1;
        }
        let chunk = lines[start..end].join("\n");
        out.push(if in_code[start] {
            chunk
        } else {
            escape_outside_inline_code(&chunk)
        });
        start = end;
    }
    out.join("\n")
}

/// End of the inline math span opening at `start`, emulating
/// `\$\$.+?\$\$|(?<!\$)\$(?!\$)(.+?)(?<!\$)\$(?!\$)`.
fn inline_math_span(line: &str, start: usize) -> Option<usize> {
    let b = line.as_bytes();
    if b.get(start + 1) == Some(&b'$') {
        let first = line[start + 2..].chars().next()?;
        let from = start + 2 + first.len_utf8();
        return line[from..].find("$$").map(|p| from + p + 2);
    }
    if start > 0 && b[start - 1] == b'$' {
        return None;
    }
    let first = line[start + 1..].chars().next()?;
    let mut e = start + 1 + first.len_utf8();
    while e < b.len() {
        if b[e] == b'$' && b[e - 1] != b'$' && b.get(e + 1) != Some(&b'$') {
            return Some(e + 1);
        }
        e += 1;
    }
    None
}

/// `{` not after `\` and not before `/*` → `\{`; `}` not after `*/` → `\}`.
/// The two rules spare an MDX comment `{/* … */}` on both ends.
fn escape_bare_curly(segment: &str, out: &mut String) {
    let b = segment.as_bytes();
    for (i, ch) in segment.char_indices() {
        match ch {
            '{' if !(i > 0 && b[i - 1] == b'\\') && !b[i + 1..].starts_with(b"/*") => {
                out.push_str("\\{")
            }
            '}' if !(i >= 2 && &b[i - 2..i] == b"*/") => out.push_str("\\}"),
            _ => out.push(ch),
        }
    }
}

/// Escapes bare curly braces on one line, sparing inline code (tried first)
/// and inline math, which MDX passes through untouched.
pub fn escape_curly_outside_math(line: &str) -> String {
    let mut out = String::with_capacity(line.len() + 8);
    let (mut pos, mut i) = (0, 0);
    let bytes = line.as_bytes();
    while i < bytes.len() {
        let span = match bytes[i] {
            b'`' => inline_code_span(line, i),
            b'$' => inline_math_span(line, i),
            _ => None,
        };
        match span {
            Some(end) => {
                escape_bare_curly(&line[pos..i], &mut out);
                out.push_str(&line[i..end]);
                pos = end;
                i = end;
            }
            None => i += 1,
        }
    }
    escape_bare_curly(&line[pos..], &mut out);
    out
}

#[cfg(test)]
#[path = "escape_tests.rs"]
mod tests;
