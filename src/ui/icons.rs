//! The UI's icons: drawn SVG on a 16 point grid with one stroke weight, tinted from the
//! palette. File icons take the colour of a syntax token, so they follow the theme.

use rune::{Color, El, Icon};

use crate::theme::Palette;

const PAGE: &str = r#"<path d="M4.75 1.75h4.5l3 3v8.5a1 1 0 0 1-1 1h-6.5a1 1 0 0 1-1-1V2.75a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" fill="currentColor" fill-opacity="0.14"/><path d="M9.25 1.75v3h3" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>"#;

pub static FILE: Icon = Icon { name: "ed-file", view: 16.0, body: PAGE };

pub static FILE_CODE: Icon = Icon {
    name: "ed-file-code",
    view: 16.0,
    body: concat!(
        r#"<path d="M4.75 1.75h4.5l3 3v8.5a1 1 0 0 1-1 1h-6.5a1 1 0 0 1-1-1V2.75a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" fill="currentColor" fill-opacity="0.14"/><path d="M9.25 1.75v3h3" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>"#,
        r#"<path d="M7 8 5.75 9.5 7 11M9 8l1.25 1.5L9 11" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round"/>"#
    ),
};

pub static FILE_TEXT: Icon = Icon {
    name: "ed-file-text",
    view: 16.0,
    body: concat!(
        r#"<path d="M4.75 1.75h4.5l3 3v8.5a1 1 0 0 1-1 1h-6.5a1 1 0 0 1-1-1V2.75a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" fill="currentColor" fill-opacity="0.14"/><path d="M9.25 1.75v3h3" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>"#,
        r#"<path d="M6 8h4M6 10.5h3" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>"#
    ),
};

pub static FILE_CONFIG: Icon = Icon {
    name: "ed-file-config",
    view: 16.0,
    body: concat!(
        r#"<path d="M4.75 1.75h4.5l3 3v8.5a1 1 0 0 1-1 1h-6.5a1 1 0 0 1-1-1V2.75a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" fill="currentColor" fill-opacity="0.14"/><path d="M9.25 1.75v3h3" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>"#,
        r#"<circle cx="6.5" cy="8.25" r="0.9" fill="currentColor"/><circle cx="6.5" cy="11" r="0.9" fill="currentColor"/><path d="M8.25 8.25H10M8.25 11H10" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>"#
    ),
};

pub static FILE_IMAGE: Icon = Icon {
    name: "ed-file-image",
    view: 16.0,
    body: concat!(
        r#"<path d="M4.75 1.75h4.5l3 3v8.5a1 1 0 0 1-1 1h-6.5a1 1 0 0 1-1-1V2.75a1 1 0 0 1 1-1Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" fill="currentColor" fill-opacity="0.14"/><path d="M9.25 1.75v3h3" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>"#,
        r#"<path d="m5.5 12 2-2.5 1.25 1.5.75-1 1 2" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round"/>"#
    ),
};

pub static FOLDER: Icon = Icon {
    name: "ed-folder",
    view: 16.0,
    body: r#"<path d="M1.75 4a1 1 0 0 1 1-1h3.2l1.5 1.6h5.8a1 1 0 0 1 1 1V12a1 1 0 0 1-1 1H2.75a1 1 0 0 1-1-1Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" fill="currentColor" fill-opacity="0.22"/>"#,
};

pub static FOLDER_OPEN: Icon = Icon {
    name: "ed-folder-open",
    view: 16.0,
    body: r#"<path d="M1.75 12.5V4a1 1 0 0 1 1-1h3.2l1.5 1.6h4.8a1 1 0 0 1 1 1v1.4" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/><path d="M1.75 12.5 3.4 7.8a1 1 0 0 1 .95-.67h9.2a.7.7 0 0 1 .66.94l-1.5 4.1a1 1 0 0 1-.94.63Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" fill="currentColor" fill-opacity="0.22"/>"#,
};

pub static CHEVRON_RIGHT: Icon = Icon {
    name: "ed-chevron-right",
    view: 16.0,
    body: r#"<path d="m6.25 4 4 4-4 4" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static CHEVRON_DOWN: Icon = Icon {
    name: "ed-chevron-down",
    view: 16.0,
    body: r#"<path d="m4 6.25 4 4 4-4" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub static CLOSE: Icon = Icon {
    name: "ed-close",
    view: 16.0,
    body: r#"<path d="m4.75 4.75 6.5 6.5m0-6.5-6.5 6.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>"#,
};

pub static DOT: Icon = Icon { name: "ed-dot", view: 16.0, body: r#"<circle cx="8" cy="8" r="3.25" fill="currentColor"/>"# };

pub static BRANCH: Icon = Icon {
    name: "ed-branch",
    view: 16.0,
    body: r#"<circle cx="5" cy="3.75" r="1.6" stroke="currentColor" stroke-width="1.2"/><circle cx="5" cy="12.25" r="1.6" stroke="currentColor" stroke-width="1.2"/><circle cx="11" cy="5.25" r="1.6" stroke="currentColor" stroke-width="1.2"/><path d="M5 5.35v5.3M11 6.85c0 2.4-6 1.6-6 3.4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>"#,
};

pub static ERROR: Icon = Icon {
    name: "ed-error",
    view: 16.0,
    body: r#"<circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.3"/><path d="m5.9 5.9 4.2 4.2m0-4.2-4.2 4.2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>"#,
};

pub static WARNING: Icon = Icon {
    name: "ed-warning",
    view: 16.0,
    body: r#"<path d="M7.13 2.5a1 1 0 0 1 1.74 0l5.4 9.5a1 1 0 0 1-.87 1.5H2.6a1 1 0 0 1-.87-1.5Z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/><path d="M8 6.25v3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/><circle cx="8" cy="11.1" r="0.8" fill="currentColor"/>"#,
};

pub static INFO: Icon = Icon {
    name: "ed-info",
    view: 16.0,
    body: r#"<circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.3"/><path d="M8 7.4v3.6" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/><circle cx="8" cy="5.1" r="0.8" fill="currentColor"/>"#,
};

pub static SETTINGS: Icon = Icon {
    name: "ed-settings",
    view: 16.0,
    body: r#"<path d="M2 4.75h3.25M8.75 4.75H14M2 11.25h7M12.5 11.25H14" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/><circle cx="7" cy="4.75" r="1.75" stroke="currentColor" stroke-width="1.2"/><circle cx="10.75" cy="11.25" r="1.75" stroke="currentColor" stroke-width="1.2"/>"#,
};

pub static DIFF: Icon = Icon {
    name: "ed-diff",
    view: 16.0,
    body: r#"<path d="M5.5 2.5v6M2.5 5.5h6M8 12.5h5.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>"#,
};

pub static PENCIL: Icon = Icon {
    name: "ed-pencil",
    view: 16.0,
    body: r#"<path d="m10.4 2.6 3 3-7.9 7.9H2.5v-3Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/><path d="m8.75 4.25 3 3" stroke="currentColor" stroke-width="1.2"/>"#,
};

/// An icon element, `size` points square, in `color`.
pub fn draw(i: &'static Icon, size: f32, color: Color) -> El {
    rune::icon(i, size).color(color)
}

/// The icon and its colour for a file, by its extension or its whole name.
pub fn file(name: &str, p: &Palette) -> (&'static Icon, Color) {
    let lower = name.to_ascii_lowercase();
    let ext = lower.rsplit_once('.').map_or("", |(_, e)| e);
    match (lower.as_str(), ext) {
        ("makefile" | "dockerfile" | "justfile", _) => (&FILE_CONFIG, p.ty),
        ("license" | "licence" | "copying", _) => (&FILE_TEXT, p.fg2),
        (_, "rs") => (&FILE_CODE, p.mac),
        (_, "ts" | "tsx" | "c" | "h" | "cc" | "cpp" | "hpp" | "lua" | "swift" | "zig") => (&FILE_CODE, p.fun),
        (_, "js" | "jsx" | "mjs" | "cjs") => (&FILE_CODE, p.prop),
        (_, "py" | "go" | "dart") => (&FILE_CODE, p.ty),
        (_, "java" | "kt" | "html" | "htm" | "erb" | "php") => (&FILE_CODE, p.num),
        (_, "rb") => (&FILE_CODE, p.mac),
        (_, "css" | "scss" | "sass" | "less" | "vue" | "svelte" | "astro" | "ex" | "exs" | "hs" | "ml" | "scala") => (&FILE_CODE, p.kw),
        (_, "sh" | "bash" | "zsh" | "fish" | "nu" | "ps1") => (&FILE_CODE, p.str_),
        (_, "wgsl" | "glsl" | "metal" | "sql" | "cs") => (&FILE_CODE, p.ty),
        (_, "json" | "jsonc" | "toml" | "yaml" | "yml" | "ini" | "cfg" | "conf" | "env" | "xml") => (&FILE_CONFIG, p.warn),
        (_, "lock") => (&FILE_CONFIG, p.fg3),
        (_, "md" | "mdx" | "rst" | "adoc") => (&FILE_TEXT, p.fun),
        (_, "txt" | "csv" | "log") => (&FILE_TEXT, p.fg2),
        (_, "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico" | "bmp" | "avif" | "mp4" | "webm") => (&FILE_IMAGE, p.kw),
        _ => (&FILE, p.fg3),
    }
}
