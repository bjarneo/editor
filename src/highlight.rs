//! Syntax colours for the lines on screen. The lexers (`rune_highlights`, 80 languages) run
//! over a window of lines around the view, starting some lines above it so multi-line comments
//! and strings open before the first visible line. Results are cached per buffer version.

use std::cell::RefCell;
use std::ops::Range;
use std::path::Path;

use editor::buffer::Buffer;
use rune::Color;
use rune_highlights::{Highlighter, Lang, Token};

use crate::theme::Palette;

/// Lines lexed above the first visible line, so state that spans lines settles first.
const CONTEXT: usize = 300;

/// The language for a file, by name and extension.
pub fn lang_for(path: Option<&Path>) -> Lang {
    let Some(path) = path else { return Lang::PLAIN };
    let name = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let by_name = match name.as_str() {
        "makefile" | "gnumakefile" => Some("makefile"),
        "dockerfile" | "containerfile" => Some("dockerfile"),
        "cmakelists.txt" => Some("cmake"),
        "cargo.lock" | "uv.lock" | "poetry.lock" => Some("toml"),
        ".bashrc" | ".zshrc" | ".profile" | ".bash_profile" => Some("bash"),
        _ => None,
    };
    if let Some(l) = by_name.and_then(Lang::from_name) {
        return l;
    }
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let alias = match ext.as_str() {
        "rs" => "rust",
        "h" => "c",
        "hpp" | "cc" | "cxx" => "cpp",
        "yml" => "yaml",
        "mjs" | "cjs" => "javascript",
        "markdown" => "markdown",
        "conf" | "ini" => "toml",
        other => other,
    };
    Lang::from_name(alias).unwrap_or(Lang::PLAIN)
}

/// The language's name for the status line: `rust`, `markdown`, `text`.
pub fn lang_label(lang: Lang) -> &'static str {
    match lang.name() {
        "plain" => "text",
        n => n,
    }
}

/// What a token is drawn as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Keyword,
    Function,
    Type,
    String,
    Number,
    Comment,
    Macro,
    Property,
    Punct,
    Text,
}

pub fn kind(token: Token) -> Kind {
    let name = token.name();
    let head = name.split('.').next().unwrap_or(name);
    match head {
        "keyword" if name == "keyword.operator" => Kind::Punct,
        "keyword" | "tag" | "selector" => Kind::Keyword,
        "function" | "title" => Kind::Function,
        "type" | "constructor" | "enum" | "namespace" | "variant" => Kind::Type,
        "string" | "text" | "link_uri" => Kind::String,
        "number" | "boolean" | "constant" | "label" => Kind::Number,
        "comment" => Kind::Comment,
        "preproc" | "attribute" => Kind::Macro,
        "property" => Kind::Property,
        "punctuation" | "operator" => Kind::Punct,
        "variable" if name == "variable.special" => Kind::Keyword,
        _ => Kind::Text,
    }
}

pub fn color(p: &Palette, k: Kind) -> Color {
    match k {
        Kind::Keyword => p.kw,
        Kind::Function => p.fun,
        Kind::Type => p.ty,
        Kind::String => p.str_,
        Kind::Number => p.num,
        Kind::Comment => p.com,
        Kind::Macro => p.mac,
        Kind::Property => p.prop,
        Kind::Punct => p.pun,
        Kind::Text => p.fg,
    }
}

/// Coloured runs of one line: char ranges and their kinds, in order.
pub type LineRuns = Vec<(Range<usize>, Kind)>;

struct Entry {
    key: (u64, u64, usize, usize, Lang),
    first: usize,
    lines: Vec<LineRuns>,
}

/// One per window: the lexers and the last few windows they coloured.
pub struct Cache {
    hl: Highlighter,
    entries: Vec<Entry>,
}

impl Default for Cache {
    fn default() -> Self {
        Cache { hl: Highlighter::new(), entries: Vec::new() }
    }
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::default());
}

/// The coloured runs of lines `first..first + count` of `buf`. `id` names the buffer (its
/// cache slot); the buffer's version keys the result.
pub fn runs(id: u64, buf: &Buffer, lang: Lang, first: usize, count: usize) -> Vec<LineRuns> {
    if lang == Lang::PLAIN {
        return vec![Vec::new(); count];
    }
    // Lex a window aligned to 64 lines, so small scrolls reuse it.
    let start = first.saturating_sub(CONTEXT) / 64 * 64;
    let end = (first + count + 64).div_ceil(64) * 64;
    let end = end.min(buf.line_count());
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let key = (id, buf.version(), start, end, lang);
        if let Some(e) = c.entries.iter().find(|e| e.key == key) {
            return slice(&e.lines, e.first, first, count);
        }
        let mut src = String::new();
        let mut line_starts = Vec::with_capacity(end - start + 1);
        for l in start..end {
            line_starts.push(src.len());
            src.push_str(&buf.line(l));
            src.push('\n');
        }
        line_starts.push(src.len());
        let tokens = c.hl.highlight(lang, &src);
        let mut lines: Vec<LineRuns> = vec![Vec::new(); end - start];
        let mut li = 0;
        for (range, token) in tokens {
            while li + 1 < line_starts.len() && range.start >= line_starts[li + 1] {
                li += 1;
            }
            if li >= lines.len() {
                break;
            }
            // A token never crosses a line end; clip it to its line to be safe.
            let ls = line_starts[li];
            let le = line_starts[li + 1].saturating_sub(1).max(ls);
            let (a, b) = (range.start.max(ls), range.end.min(le));
            if a >= b {
                continue;
            }
            let text = &src[ls..le];
            let ca = text[..a - ls].chars().count();
            let cb = ca + text[a - ls..b - ls].chars().count();
            lines[li].push((ca..cb, kind(token)));
        }
        let out = slice(&lines, start, first, count);
        c.entries.push(Entry { key, first: start, lines });
        if c.entries.len() > 6 {
            c.entries.remove(0);
        }
        out
    })
}

fn slice(lines: &[LineRuns], at: usize, first: usize, count: usize) -> Vec<LineRuns> {
    (first..first + count).map(|l| l.checked_sub(at).and_then(|i| lines.get(i)).cloned().unwrap_or_default()).collect()
}

/// Colours a short source in one go, for previews (the finder, Settings, onboarding).
pub fn runs_of(src: &str, lang: Lang) -> Vec<LineRuns> {
    let buf = Buffer::from_str(src);
    let id = 0xFFFF_0000 ^ (src.len() as u64) ^ ((lang.name().len() as u64) << 20);
    runs(id, &buf, lang, 0, buf.line_count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_keywords_and_strings_get_their_kinds() {
        let lines = runs_of("fn main() {\n    let s = \"hi\";\n}\n", Lang::from_name("rust").unwrap());
        assert!(lines[0].iter().any(|(r, k)| r.start == 0 && *k == Kind::Keyword));
        assert!(lines[1].iter().any(|(_, k)| *k == Kind::String));
    }

    #[test]
    fn paths_pick_languages() {
        assert_eq!(lang_label(lang_for(Some(Path::new("src/main.rs")))), "rust");
        assert_eq!(lang_label(lang_for(Some(Path::new("Cargo.toml")))), "toml");
        assert_eq!(lang_label(lang_for(Some(Path::new("README.md")))), "markdown");
        assert_eq!(lang_label(lang_for(Some(Path::new(".gitignore")))), "text");
    }
}
