//! Named states for headless snapshots: `editor --snap out.png --scene finder`. Each scene
//! opens this repository's own `src/editor.rs` synchronously, so a snapshot needs no workers.

use editor::buffer::{Buffer, Pos};
use editor::editor::{Doc, Keymap, Mode, VisualKind};

use crate::app::{App, DocMeta, Overlay, PickKind, Screen, Settings, setup::Setup};

/// A scene's name and what it does to the first state.
type Scene = (&'static str, fn(&mut App));

pub const SCENES: &[Scene] = &[
    ("editor", editor),
    ("insert", insert),
    ("visual", visual),
    ("leader", leader),
    ("finder", finder),
    ("keys", keys),
    ("keys-search", keys_search),
    ("tree-scrolled", tree_scrolled),
    ("classic", classic),
    ("light", light),
    ("omarchy", omarchy),
    ("settings", settings),
    ("settings-font", settings_font),
    ("setup", setup),
    ("setup-look", setup_look),
    ("setup-shell", setup_shell),
    ("setup-ready", setup_ready),
    ("empty", empty),
    ("menu", menu),
    ("multi", multi),
    ("rename", rename),
    ("actions", actions),
    ("promo-multi", promo_multi),
    ("proposal", proposal),
    ("agent", agent),
    ("promo-mouse", promo_mouse),
];

fn sample(app: &mut App) {
    app.screen = Screen::Editor;
    // This repository: the current folder when it is a copy of it, else where it was built.
    let cwd = std::env::current_dir().unwrap_or_default();
    let root = if cwd.join("src/editor.rs").is_file() { cwd } else { std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")) };
    app.ed.root = root.clone();
    // `EDITOR_SCENE_THEME=gruvbox` renders the scene in that built-in theme, for screenshots.
    if let Ok(id) = std::env::var("EDITOR_SCENE_THEME") {
        let light = crate::theme::builtin(&id).light;
        app.cfg.theme.mode = if light { "light" } else { "dark" }.into();
        if light {
            app.cfg.theme.light = id;
        } else {
            app.cfg.theme.dark = id;
        }
    }
    app.files = crate::services::index::walk(&root);
    app.tree.index = crate::app::tree::TreeIndex::build(&app.files);
    app.tree.open.insert("src".into());
    app.tree.open.insert("src/app".into());
    app.ed.docs.clear();
    for f in ["src/main.rs", "src/app/keys.rs", "src/editor.rs"] {
        let path = root.join(f);
        if let Ok(buf) = Buffer::open(&path) {
            app.meta.insert(path.clone(), DocMeta { lang: Some(crate::highlight::lang_for(Some(&path))), ..Default::default() });
            app.ed.docs.push(Doc::new(buf));
        }
    }
    app.ed.active = app.ed.docs.len().saturating_sub(1);
    app.ed.rows = 34;
    app.ed.cols = 120;
    // The cursor sits in `pub struct Registers`, wherever editor.rs has it now.
    let at = line_of(app, "    pub yank: Reg,");
    app.ed.set_cursor(Pos::new(at, 8));
    let doc = app.ed.doc_mut();
    doc.view.top = at.saturating_sub(16);
    app.git.branch = Some("main".into());
    app.reveal_in_tree(&root.join("src/editor.rs"));
}

/// The first line of the active document that is exactly `text`.
fn line_of(app: &App, text: &str) -> usize {
    let buf = app.ed.buf();
    (0..buf.line_count()).find(|&l| buf.line(l) == text).unwrap_or(0)
}

fn editor(app: &mut App) {
    sample(app);
}

fn insert(app: &mut App) {
    sample(app);
    app.ed.mode = Mode::Insert;
}

fn visual(app: &mut App) {
    sample(app);
    let at = app.ed.cursor().line;
    app.ed.mode = Mode::Visual(VisualKind::Line);
    app.ed.doc_mut().view.anchor = Some(Pos::new(at, 0));
    app.ed.set_cursor(Pos::new(at + 3, 8));
}

fn leader(app: &mut App) {
    sample(app);
    app.overlay = Overlay::Leader(None);
}

fn finder(app: &mut App) {
    sample(app);
    app.open_picker(PickKind::Files);
    if let Overlay::Pick(p) = &mut app.overlay {
        p.query = "key".into();
        p.refilter(&app.files);
        if let Some(m) = p.matches.first() {
            let path = app.ed.root.join(&app.files[m.index]);
            p.preview = std::fs::read_to_string(&path).ok().map(|t| (path, t));
        }
    }
}

fn keys(app: &mut App) {
    sample(app);
    app.open_key_list(String::new());
}

fn keys_search(app: &mut App) {
    sample(app);
    app.open_key_list("tree".into());
}

/// The tree scrolled 30 rows down by the wheel. The keyboard row stays where it was.
fn tree_scrolled(app: &mut App) {
    sample(app);
    app.tree.top = 30;
    app.tree.follow = crate::app::tree::Follow::No;
}

fn classic(app: &mut App) {
    sample(app);
    app.cfg.keys.map = "classic".into();
    app.ed.set_keymap(Keymap::Classic);
}

fn light(app: &mut App) {
    sample(app);
    app.cfg.theme.mode = "light".into();
}

fn omarchy(app: &mut App) {
    sample(app);
    app.cfg.theme.mode = "omarchy".into();
}

fn settings(app: &mut App) {
    sample(app);
    app.settings = Some(Settings::default());
    app.on_settings = true;
}

fn settings_font(app: &mut App) {
    settings(app);
    if let Some(s) = app.settings.as_mut() {
        s.section = crate::app::settings::Section::Font;
    }
}

fn setup(app: &mut App) {
    app.screen = Screen::Setup(Box::new(Setup::new(app.omarchy.as_ref())));
}

fn setup_look(app: &mut App) {
    let mut s = Setup::new(app.omarchy.as_ref());
    s.step = 3;
    app.screen = Screen::Setup(Box::new(s));
}

fn setup_shell(app: &mut App) {
    let mut s = Setup::new(app.omarchy.as_ref());
    s.step = 4;
    s.check_name();
    app.screen = Screen::Setup(Box::new(s));
}

fn setup_ready(app: &mut App) {
    let mut s = Setup::new(app.omarchy.as_ref());
    s.step = 5;
    app.screen = Screen::Setup(Box::new(s));
}

fn empty(app: &mut App) {
    app.screen = Screen::Editor;
    app.ed.docs.clear();
}

fn menu(app: &mut App) {
    sample(app);
    app.cfg.mouse.right_click = true;
    app.overlay = Overlay::Menu(crate::app::menu::Menu { x: 620.0, y: 420.0, sel: 1 });
}

fn multi(app: &mut App) {
    classic(app);
    // Every `Reg` in `pub struct Registers` selected, as Ctrl+D leaves it.
    let at = app.ed.cursor().line;
    let lines: Vec<(usize, usize)> = (at.saturating_sub(4)..=at)
        .filter_map(|l| app.ed.buf().line(l).find(": Reg").map(|b| (l, app.ed.buf().line(l)[..b].chars().count() + 2)))
        .collect();
    let doc = app.ed.doc_mut();
    for (i, &(l, c)) in lines.iter().enumerate() {
        let (a, b) = (Pos::new(l, c), Pos::new(l, c + 3));
        if i + 1 == lines.len() {
            doc.view.anchor = Some(a);
            doc.view.cursor = b;
        } else {
            doc.extra.push(editor::editor::Cursor { pos: b, anchor: Some(a) });
        }
    }
}

fn rename(app: &mut App) {
    sample(app);
    editor::vim::open_cmdline(&mut app.ed, ':', "Rename yank_reg");
}

fn actions(app: &mut App) {
    sample(app);
    app.open_actions(vec![
        "Add missing field `linewise`".into(),
        "Convert to tuple struct".into(),
        "Generate `Default` impl".into(),
        "Inline type alias".into(),
    ]);
}

/// A short Rust snippet in the classic keymap, for showing multiple cursors.
fn promo_multi(app: &mut App) {
    sample(app);
    app.cfg.keys.map = "classic".into();
    app.ed.set_keymap(Keymap::Classic);
    let src = "/// The area a frame covers, in pixels.\nfn area(frame: &Frame) -> u32 {\n    let width = frame.width;\n    let height = frame.height;\n    let area = width * height;\n    println!(\"{width} x {height} = {area}\");\n    area\n}\n";
    let path = app.ed.root.join("src/frame.rs");
    let mut buf = Buffer::from_str(src);
    buf.path = Some(path.clone());
    app.meta.insert(path, DocMeta { lang: Some(crate::highlight::lang_for(Some(std::path::Path::new("x.rs")))), ..Default::default() });
    app.ed.docs.push(Doc::new(buf));
    app.ed.active = app.ed.docs.len() - 1;
    app.ed.set_cursor(Pos::new(2, 8));
    app.ed.doc_mut().view.top = 0;
}

/// An agent's proposed change, as `editor ctl propose` shows it.
fn proposal(app: &mut App) {
    sample(app);
    let old = app.ed.buf().text();
    let new = old
        .replace("    pub yank: Reg,\n", "    pub yank: Reg,\n    /// The last small delete, `-`.\n    pub small: Reg,\n")
        .replace("pub fn get(&self, name: Option<char>)", "pub fn get(&self, name: Option<char>) -> Option<&Reg>")
        .replace(
            "pub fn get(&self, name: Option<char>) -> Option<&Reg> -> Option<&Reg>",
            "pub fn read(&self, name: Option<char>) -> Option<&Reg>",
        );
    let path = app.ed.buf().path.clone().unwrap_or_default();
    let version = app.ed.buf().version();
    let p = crate::app::review::Proposal::new(path, "add a small-delete register".into(), &old, new, version);
    app.proposals.push(p);
}

/// An agent at work: requests arrive at set times, as `editor ctl` sends them. The file that
/// `EDITOR_SCENE_AGENT` names holds them as `[[seconds, {"cmd": …, "args": {…}}], …]`. Without
/// it, the agent points at a struct and then proposes a change to it.
fn agent(app: &mut App) {
    sample(app);
    app.tree.show = false;
    let script = std::env::var("EDITOR_SCENE_AGENT").ok().and_then(|p| std::fs::read_to_string(p).ok());
    let steps: Vec<(f64, serde_json::Value)> = match script.and_then(|s| serde_json::from_str(&s).ok()) {
        Some(steps) => steps,
        None => vec![
            (
                1.0,
                serde_json::json!({"cmd": "reveal", "args": {"path": "src/editor.rs", "line": 222, "end_line": 228, "note": "this struct holds every register"}}),
            ),
            (3.0, serde_json::json!({"cmd": "message", "args": {"text": "agent: done"}})),
        ],
    };
    app.scripted = steps
        .into_iter()
        .enumerate()
        .filter_map(|(i, (t, v))| {
            let req = crate::ipc::Request { id: i as u64 + 1, cmd: v["cmd"].as_str()?.to_string(), args: v["args"].clone() };
            Some((t, req))
        })
        .collect();
}

/// The editor with every mouse feature on, right-click menus included, for showing the mouse.
fn promo_mouse(app: &mut App) {
    sample(app);
    app.cfg.mouse.right_click = true;
}
