//! The app driven as a user drives it: keys and clicks through Rune's runtime, the real view
//! and the real key routing. No workers run, so nothing touches the disk.

use editor::buffer::{Buffer, Pos};
use editor::editor::{Doc, Keymap, Mode};
use rune::{Env, Key, KeyEvent, Mods, Runtime};

use super::{App, DocMeta, Overlay, PickKind};
use crate::config::Config;

const SRC: &str = "fn main() {\n    let a = 1;\n    let b = 2;\n    println!(\"{}\", a + b);\n}\n";

fn app(map: &str) -> App {
    let mut cfg = Config::default();
    cfg.keys.map = map.into();
    let root = std::env::temp_dir();
    let mut app = App::new(cfg, root.clone(), false, None);
    let path = root.join("editor-test-main.rs");
    let mut buf = Buffer::from_str(SRC);
    buf.path = Some(path.clone());
    app.meta.insert(path, DocMeta { lang: rune_highlights::Lang::from_name("rust"), ..Default::default() });
    app.ed.docs.push(Doc::new(buf));
    app.ed.active = 0;
    app
}

fn run(app: App) -> Runtime<App> {
    let mut rt = Runtime::new(app, crate::ui::view, (1200.0, 800.0), Env { system_dark: true, scale: 1.0 });
    rt.on_key = Some(super::keys::on_key);
    rt.use_clipboard(&rune::clipboard::memory());
    rt.frame(0.0);
    rt.frame(1.0 / 60.0);
    rt
}

fn press(rt: &mut Runtime<App>, key: Key, mods: Mods) {
    rt.key(KeyEvent { key, mods });
    rt.frame(1.0 / 60.0);
}

fn typ(rt: &mut Runtime<App>, s: &str) {
    for c in s.chars() {
        let key = match c {
            '\n' => Key::Enter,
            '\x1b' => Key::Escape,
            c => Key::Text(c.to_string()),
        };
        press(rt, key, Mods::default());
    }
}

fn ctrl(rt: &mut Runtime<App>, c: char) {
    press(rt, Key::Command(c), Mods { ctrl: true, cmd: true, ..Default::default() });
}

fn text(rt: &Runtime<App>) -> String {
    rt.state.ed.buf().text()
}

#[test]
fn the_view_measures_rows_and_columns() {
    let rt = run(app("neovim"));
    assert!(rt.state.ed.rows > 20, "rows come from the text area's layout: {}", rt.state.ed.rows);
    assert!(rt.state.m.cw > 5.0 && rt.state.m.cw < 12.0, "the probe measures a char: {}", rt.state.m.cw);
}

#[test]
fn dd_then_u_deletes_and_restores_a_line() {
    let mut rt = run(app("neovim"));
    typ(&mut rt, "jdd");
    assert!(!text(&rt).contains("let a"));
    assert!(rt.state.ed.buf().is_dirty());
    typ(&mut rt, "u");
    assert_eq!(text(&rt), SRC);
}

#[test]
fn insert_mode_types_and_escape_returns_to_normal() {
    let mut rt = run(app("neovim"));
    typ(&mut rt, "Ohi\x1b");
    assert_eq!(rt.state.ed.mode, Mode::Normal);
    assert!(text(&rt).starts_with("hi\nfn main"));
}

#[test]
fn yank_and_paste_go_through_the_clipboard_path() {
    let mut rt = run(app("neovim"));
    typ(&mut rt, "jyyp");
    for _ in 0..3 {
        rt.frame(1.0 / 60.0);
    }
    assert_eq!(text(&rt).matches("let a = 1;").count(), 2, "{}", text(&rt));
}

#[test]
fn space_opens_the_leader_menu_and_b_lists_buffers() {
    let mut rt = run(app("neovim"));
    typ(&mut rt, " ");
    assert!(matches!(rt.state.overlay, Overlay::Leader(None)));
    typ(&mut rt, "b");
    match &rt.state.overlay {
        Overlay::Pick(p) => assert_eq!(p.kind, PickKind::Buffers),
        _ => panic!("SPC b opens the buffer list"),
    }
    press(&mut rt, Key::Escape, Mods::default());
    assert!(matches!(rt.state.overlay, Overlay::None));
}

#[test]
fn colon_settings_opens_the_tab_and_w_writes_the_config() {
    let mut rt = run(app("neovim"));
    typ(&mut rt, ":Settings\n");
    assert!(rt.state.on_settings, "{:?}", rt.state.ed.msg);
    // The first row is the theme; `l` moves it from dark to system.
    typ(&mut rt, "l");
    assert_eq!(rt.state.cfg.theme.mode, "system");
    assert_ne!(rt.state.cfg, rt.state.disk_cfg);
    typ(&mut rt, ":w\n");
    assert_eq!(rt.state.cfg, rt.state.disk_cfg, "`:w` in Settings writes config.toml");
    typ(&mut rt, ":q\n");
    assert!(!rt.state.on_settings);
}

#[test]
fn set_keymap_switches_to_classic_and_back() {
    let mut rt = run(app("neovim"));
    typ(&mut rt, ":set keymap=classic\n");
    assert_eq!(rt.state.ed.keymap, Keymap::Classic);
    assert_eq!(rt.state.ed.mode, Mode::Insert);
    typ(&mut rt, "x");
    assert!(text(&rt).starts_with("xfn"), "classic types at once: {}", text(&rt));
}

#[test]
fn classic_shortcuts_open_the_palette_and_undo() {
    let mut rt = run(app("classic"));
    typ(&mut rt, "ab");
    assert!(text(&rt).starts_with("abfn"));
    ctrl(&mut rt, 'z');
    assert_eq!(text(&rt), SRC);
    press(&mut rt, Key::Command('p'), Mods { ctrl: true, cmd: true, shift: true, ..Default::default() });
    match &rt.state.overlay {
        Overlay::Pick(p) => assert_eq!(p.kind, PickKind::Commands),
        _ => panic!("Ctrl+Shift+P opens the command palette"),
    }
}

#[test]
fn a_click_places_the_cursor_and_a_drag_selects_in_visual_mode() {
    let mut rt = run(app("neovim"));
    let [x, y, _, _] = rt.state.m.text;
    let (cw, rh, gutter) = (rt.state.m.cw, rt.state.m.row_h, rt.state.m.gutter);
    let at = |line: f32, col: f32| (x + gutter + col * cw + cw / 2.0, y + line * rh + rh / 2.0);
    let (px, py) = at(2.0, 8.0);
    rt.pointer_down(px, py);
    rt.pointer_up(px, py);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.ed.cursor(), Pos::new(2, 8));
    let (qx, qy) = at(3.0, 4.0);
    rt.pointer_down(px, py);
    rt.pointer_move(qx, qy);
    rt.frame(1.0 / 60.0);
    rt.pointer_up(qx, qy);
    rt.frame(1.0 / 60.0);
    assert!(matches!(rt.state.ed.mode, Mode::Visual(_)), "{:?}", rt.state.ed.mode);
    assert_eq!(rt.state.ed.doc().view.anchor, Some(Pos::new(2, 8)));
}

#[test]
fn quitting_a_modified_buffer_is_refused() {
    let mut rt = run(app("neovim"));
    typ(&mut rt, "x:q\n");
    assert!(rt.state.ed.msg.as_ref().is_some_and(|m| m.error && m.text.starts_with("E37")));
    assert_eq!(rt.state.ed.docs.len(), 1);
}

#[test]
fn space_e_focuses_the_tree_and_space_e_again_hides_it() {
    let mut app = app("neovim");
    let files: Vec<String> = ["Cargo.toml", "src/main.rs", "src/lib.rs"].iter().map(|s| s.to_string()).collect();
    app.tree.index = super::tree::TreeIndex::build(&files);
    app.files = files;
    let mut rt = run(app);
    typ(&mut rt, " e");
    assert!(rt.state.tree.show && rt.state.tree.focus);
    typ(&mut rt, "l");
    assert!(rt.state.tree.open.contains("src"), "l opens the folder");
    typ(&mut rt, "j");
    assert_eq!(rt.state.tree.sel.as_deref(), Some("src/lib.rs"));
    typ(&mut rt, " e");
    assert!(!rt.state.tree.show && !rt.state.tree.focus);
}

#[test]
fn alt_click_adds_a_cursor_in_the_classic_keymap() {
    let mut rt = run(app("classic"));
    let [x, y, _, _] = rt.state.m.text;
    let (cw, rh, gutter) = (rt.state.m.cw, rt.state.m.row_h, rt.state.m.gutter);
    let (px, py) = (x + gutter + 4.0 * cw + cw / 4.0, y + 2.0 * rh + rh / 2.0);
    rt.pointer_move(px, py);
    rt.physical_input(rune::PhysicalInput::Modifiers(Mods { alt: true, ..Default::default() }));
    rt.frame(1.0 / 60.0);
    rt.pointer_down(px, py);
    rt.pointer_up(px, py);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.ed.doc().extra.len(), 1, "{:?}", rt.state.ed.doc().extra);
    assert_eq!(rt.state.ed.doc().extra[0].pos, Pos::new(2, 4));
}

fn rust_app(src: &str) -> App {
    let mut app = app("neovim");
    let doc = app.ed.doc_mut();
    doc.buf = Buffer::from_str(src);
    doc.buf.path = Some(std::env::temp_dir().join("editor-test-main.rs"));
    app
}

#[test]
fn tree_sitter_objects_and_motions_work_through_the_app() {
    let src = "/// Adds.\nfn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n\nfn main() {\n    let x = add(1, 2);\n}\n";
    let mut rt = run(rust_app(src));
    // From the doc comment, `]f` lands on `fn add`, a second `]f` on `fn main`, and `daf`
    // deletes it.
    typ(&mut rt, "]f");
    assert_eq!(rt.state.ed.cursor().line, 1, "{:?}", rt.state.ed.msg);
    typ(&mut rt, "]f");
    assert_eq!(rt.state.ed.cursor().line, 5, "{:?}", rt.state.ed.msg);
    typ(&mut rt, "daf");
    assert!(!text(&rt).contains("fn main"), "{}", text(&rt));
    assert!(text(&rt).contains("fn add"));
    typ(&mut rt, "u");
    // `dif` on the first function empties its body.
    typ(&mut rt, "ggjjdif");
    assert!(text(&rt).contains("fn add(a: u32, b: u32) -> u32 {\n}"), "{}", text(&rt));
    // `cia` changes one parameter.
    typ(&mut rt, "u");
    typ(&mut rt, "gg0jf(lcianum: u64\x1b");
    assert!(text(&rt).contains("fn add(num: u64, b: u32)"), "{}", text(&rt));
}

#[test]
fn r_overtypes_and_the_status_line_says_replace() {
    let mut rt = run(app("neovim"));
    typ(&mut rt, "R");
    assert!(editor::vim::replacing(&rt.state.ed.vim));
    typ(&mut rt, "xy\x1b");
    assert!(text(&rt).starts_with("xy main"), "{}", text(&rt));
    assert!(!editor::vim::replacing(&rt.state.ed.vim));
}

#[test]
fn ctrl_d_adds_cursors_and_typing_edits_at_each() {
    let src = "let a = 1;\nlet b = a;\nlet c = a;\n";
    let mut app = app("classic");
    app.ed.doc_mut().buf = Buffer::from_str(src);
    let mut rt = run(app);
    // Put the cursor on the `a` of line 2's `= a`, then select it and the next match.
    press(&mut rt, Key::Down, Mods::default());
    press(&mut rt, Key::End, Mods::default());
    press(&mut rt, Key::Left, Mods::default());
    press(&mut rt, Key::Left, Mods::default());
    ctrl(&mut rt, 'd');
    ctrl(&mut rt, 'd');
    assert_eq!(rt.state.ed.doc().extra.len(), 1, "{:?}", rt.state.ed.doc().extra);
    typ(&mut rt, "z");
    let t = text(&rt);
    assert!(t.contains("let b = z;") && t.contains("let c = z;"), "{t}");
    ctrl(&mut rt, 'z');
    assert_eq!(text(&rt), src);
}

#[test]
fn right_click_opens_the_menu_and_copy_takes_the_selection() {
    let mut app = app("neovim");
    app.cfg.mouse.right_click = true;
    let mut rt = run(app);
    // Select line 2 in Visual line mode, then right-click on it.
    typ(&mut rt, "jV");
    let [x, y, _, _] = rt.state.m.text;
    let (rh, gutter) = (rt.state.m.row_h, rt.state.m.gutter);
    let (px, py) = (x + gutter + 20.0, y + rh * 1.5);
    rt.secondary_down(px, py);
    rt.secondary_up(px, py);
    rt.frame(1.0 / 60.0);
    assert!(matches!(rt.state.overlay, Overlay::Menu(_)), "Linux draws its own menu");
    // The first item is Cut; Down moves to Copy.
    press(&mut rt, Key::Down, Mods::default());
    press(&mut rt, Key::Enter, Mods::default());
    assert!(matches!(rt.state.overlay, Overlay::None));
    assert_eq!(rt.state.ed.regs.unnamed.text, "    let a = 1;\n");
    assert_eq!(rt.state.ed.mode, Mode::Normal);
}

#[test]
fn a_middle_click_closes_a_clean_tab() {
    let mut app = app("neovim");
    let mut second = Buffer::from_str("two\n");
    second.path = Some(std::env::temp_dir().join("editor-test-two.rs"));
    app.ed.docs.push(Doc::new(second));
    let mut rt = run(app);
    // The second tab sits after the first.
    let tabs: Vec<_> = rt.elements().into_iter().filter(|e| e.role == "tab").collect();
    assert_eq!(tabs.len(), 2);
    let r = tabs[1].rect;
    let (cx, cy) = (r[0] + 20.0, r[1] + r[3] / 2.0);
    rt.button_down(cx, cy, rune::PointerButton::Middle);
    rt.button_up(cx, cy, rune::PointerButton::Middle);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.ed.docs.len(), 1);
}

fn ask(rt: &mut Runtime<App>, cmd: &str, args: serde_json::Value) -> crate::ipc::Response {
    let (tx, rx) = std::sync::mpsc::channel();
    let req = crate::ipc::Request { id: 7, cmd: cmd.into(), args };
    rt.update(move |s: &mut App, t| s.ipc(req, tx, t));
    rt.frame(1.0 / 60.0);
    rx.try_recv().expect("an answer")
}

#[test]
fn agents_read_state_and_unsaved_text_through_the_socket_api() {
    let mut rt = run(app("neovim"));
    typ(&mut rt, "jx");
    let st = ask(&mut rt, "state", serde_json::json!({}));
    assert!(st.ok);
    assert_eq!(st.result["active"]["line"], 2);
    assert_eq!(st.result["active"]["dirty"], true);
    let read = ask(&mut rt, "read", serde_json::json!({ "start_line": 2, "end_line": 2 }));
    assert_eq!(read.result["text"], "   let a = 1;\n", "the unsaved edit, not the disk copy");
    let bad = ask(&mut rt, "nope", serde_json::json!({}));
    assert!(!bad.ok && bad.error.unwrap().contains("unknown command"));
}

#[test]
fn an_agent_edit_is_one_undo_step_and_can_be_turned_off() {
    let mut rt = run(app("neovim"));
    let new = SRC.replace("let b = 2;", "let b = 20;");
    let r = ask(&mut rt, "edit", serde_json::json!({ "text": new, "label": "bump b" }));
    assert!(r.ok, "{:?}", r.error);
    assert!(text(&rt).contains("let b = 20;"));
    typ(&mut rt, "u");
    assert_eq!(text(&rt), SRC);
    rt.update(|s: &mut App, _| s.cfg.ipc.allow_edits = false);
    let r = ask(&mut rt, "edit", serde_json::json!({ "text": "x" }));
    assert!(!r.ok);
    assert_eq!(text(&rt), SRC);
}

#[test]
fn a_proposal_shows_until_y_accepts_or_n_rejects() {
    let mut rt = run(app("neovim"));
    let new = SRC.replace("println!", "eprintln!");
    let r = ask(&mut rt, "propose", serde_json::json!({ "text": new, "label": "log to stderr" }));
    assert!(r.ok);
    assert_eq!(r.result["added"], 1);
    assert!(rt.state.proposal_here().is_some());
    typ(&mut rt, "j");
    assert_eq!(text(&rt), SRC, "j scrolls the diff, it does not edit");
    typ(&mut rt, "y");
    assert!(text(&rt).contains("eprintln!"));
    assert!(rt.state.proposals.is_empty());
    typ(&mut rt, "u");
    let r = ask(&mut rt, "propose", serde_json::json!({ "text": new }));
    assert!(r.ok);
    typ(&mut rt, "n");
    assert_eq!(text(&rt), SRC);
}

#[test]
fn reveal_marks_lines_and_esc_clears_them() {
    let mut rt = run(app("neovim"));
    let r = ask(&mut rt, "reveal", serde_json::json!({ "line": 3, "end_line": 4, "note": "these add up" }));
    assert!(r.ok);
    let n = rt.state.note.clone().expect("a note");
    assert_eq!((n.first, n.last), (2, 3));
    assert!(rt.state.ed.msg.as_ref().is_some_and(|m| m.text.contains("these add up")));
    press(&mut rt, Key::Escape, Mods::default());
    assert!(rt.state.note.is_none());
}

#[test]
fn open_wait_answers_when_the_buffer_closes() {
    let mut rt = run(app("neovim"));
    let path = rt.state.ed.buf().path.clone().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let req = crate::ipc::Request { id: 9, cmd: "open".into(), args: serde_json::json!({ "path": path, "wait": true }) };
    rt.update(move |s: &mut App, t| s.ipc(req, tx, t));
    rt.frame(1.0 / 60.0);
    rt.update(|s: &mut App, t| {
        s.poll(0.1, t);
    });
    assert!(rx.try_recv().is_err(), "held while the buffer is open");
    typ(&mut rt, ":q\n");
    rt.update(|s: &mut App, t| {
        s.poll(0.2, t);
    });
    let r = rx.try_recv().expect("answered after :q");
    assert_eq!(r.result["closed"], true);
}

/// A C file in git: committed, then changed on lines 4 and 5 and on line 15. `main` calls `twice`, and
/// `broken` holds an error for the language server to report.
const LEADER_C: &str = "#include <stdio.h>

static int twice(int n) {
    int base = 1;
    return n * 2 * base;
}

/* One. */
/* Two. */
/* Three. */
/* Four. */
int main(void) {
    int total = twice(21);
    printf(\"%d\\n\", total);
    return total;
}

int broken(void) { return missing_name; }
";

/// Polls the app with its real workers until `done` holds. Panics with `what` after 20 s.
fn settle(rt: &mut Runtime<App>, what: &str, done: impl Fn(&App) -> bool) {
    let start = std::time::Instant::now();
    loop {
        let clock = 100.0 + start.elapsed().as_secs_f64();
        rt.update(move |s: &mut App, t| {
            s.poll(clock, t);
        });
        rt.frame(1.0 / 60.0);
        if rt.update_if(|s: &mut App, _| done(s)) {
            return;
        }
        assert!(start.elapsed().as_secs() < 20, "{what}: nothing happened in 20 s");
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
}

fn leader(rt: &mut Runtime<App>, keys: &str) {
    typ(rt, " ");
    typ(rt, keys);
}

fn said(s: &App, part: &str) -> bool {
    s.ed.msg.as_ref().is_some_and(|m| m.text.contains(part))
}

fn picking(s: &App, kind: PickKind) -> bool {
    matches!(&s.overlay, Overlay::Pick(p) if p.kind == kind)
}

/// Every key the leader menu shows, pressed in the real app with its real workers: git, the
/// file watcher, ripgrep and clangd. Each one must do what its label says. The LSP keys run
/// when clangd is installed.
#[test]
fn every_leader_menu_key_does_what_it_says() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git").args(args).current_dir(&root).output().expect("git");
        assert!(out.status.success(), "git {args:?}");
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "test@example.com"]);
    git(&["config", "user.name", "test"]);
    std::fs::write(root.join("main.c"), LEADER_C).unwrap();
    std::fs::write(root.join("notes.txt"), "a needle in the notes\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-qm", "base"]);
    let changed = LEADER_C
        .replace("int base = 1;", "int base = 2;")
        .replace("return n * 2 * base;", "return n * 2 * base * 1;")
        .replace("return total;", "return total + 0;");
    std::fs::write(root.join("main.c"), &changed).unwrap();
    let main_c = root.join("main.c");

    let mut app = App::new(Config::default(), root.clone(), false, None);
    app.svc = Some(crate::services::Services::start(root.clone(), std::sync::Arc::new(|| {}), false));
    let mut rt = run(app);
    let open = main_c.clone();
    rt.update(move |s: &mut App, _| s.open_path(open, None));
    settle(&mut rt, "open main.c", |s| s.ed.has_doc() && s.ed.buf().path.as_deref() == Some(main_c.as_path()));
    let lsp = std::process::Command::new("clangd").arg("--version").output().is_ok();

    // Top level.
    leader(&mut rt, "f");
    settle(&mut rt, "SPC f opens the file finder", |s| picking(s, PickKind::Files));
    typ(&mut rt, "\x1b");
    leader(&mut rt, "b");
    settle(&mut rt, "SPC b lists buffers", |s| picking(s, PickKind::Buffers));
    typ(&mut rt, "\x1b");
    leader(&mut rt, "/");
    settle(&mut rt, "SPC / opens project search", |s| picking(s, PickKind::Grep));
    typ(&mut rt, "needle");
    settle(&mut rt, "SPC / finds the needle", |s| !s.grep.hits.is_empty());
    typ(&mut rt, "\x1b");
    leader(&mut rt, "e");
    settle(&mut rt, "SPC e focuses the tree", |s| s.tree.show && s.tree.focus);
    typ(&mut rt, "\x1b");
    settle(&mut rt, "Esc leaves the tree", |s| !s.tree.focus);
    leader(&mut rt, ",");
    settle(&mut rt, "SPC , opens settings", |s| s.on_settings);
    rt.update(|s: &mut App, _| s.on_settings = false);

    // +git: the two changed blocks, in order and around.
    settle(&mut rt, "git signs", |s| s.meta.get(&s.ed.buf().path.clone().unwrap()).is_some_and(|m| !m.signs.is_empty()));
    // From the line just above a two-line change: to its first line, then past its second.
    rt.update(|s: &mut App, _| s.ed.set_cursor(Pos::new(2, 0)));
    leader(&mut rt, "gn");
    settle(&mut rt, "SPC g n goes to the first change", |s| s.ed.cursor().line == 3);
    leader(&mut rt, "gn");
    settle(&mut rt, "SPC g n goes to the next change, not the next line", |s| s.ed.cursor().line == 14);
    leader(&mut rt, "gn");
    settle(&mut rt, "SPC g n wraps to the first change", |s| s.ed.cursor().line == 3);
    leader(&mut rt, "gp");
    settle(&mut rt, "SPC g p wraps to the last change", |s| s.ed.cursor().line == 14);
    leader(&mut rt, "gs");
    settle(&mut rt, "SPC g s focuses the tree", |s| s.tree.focus);
    typ(&mut rt, "\x1b");

    // +lsp, on `twice` where main calls it.
    if lsp {
        rt.update(|s: &mut App, _| s.ed.set_cursor(Pos::new(12, 17)));
        settle(&mut rt, "clangd reports the error", |s| s.meta.get(&s.ed.buf().path.clone().unwrap()).is_some_and(|m| !m.diags.is_empty()));
        leader(&mut rt, "d");
        settle(&mut rt, "SPC d lists the error clangd reports", |s| {
            picking(s, PickKind::Locations) && s.locations.iter().any(|l| l.text.contains("missing_name"))
        });
        typ(&mut rt, "\x1b");
        leader(&mut rt, "lh");
        settle(&mut rt, "SPC l h shows hover", |s| s.hover.as_deref().is_some_and(|h| h.contains("twice")));
        typ(&mut rt, "\x1b");
        leader(&mut rt, "lr");
        settle(&mut rt, "SPC l r lists references", |s| picking(s, PickKind::Locations) && s.locations.len() >= 2);
        typ(&mut rt, "\x1b");
        rt.update(|s: &mut App, _| s.ed.set_cursor(Pos::new(12, 17)));
        leader(&mut rt, "la");
        settle(&mut rt, "SPC l a answers with code actions", |s| picking(s, PickKind::Actions) || said(s, "no code actions"));
        if picking_now(&mut rt) {
            typ(&mut rt, "\x1b");
        }
        rt.update(|s: &mut App, _| s.ed.set_cursor(Pos::new(12, 17)));
        leader(&mut rt, "ld");
        settle(&mut rt, "SPC l d goes to the definition", |s| s.ed.cursor().line == 2);
    } else {
        eprintln!("clangd is not installed: the SPC l and SPC d checks did not run");
    }
    rt.update(|s: &mut App, _| s.ed.set_cursor(Pos::new(2, 11)));
    leader(&mut rt, "ln");
    settle(&mut rt, "SPC l n opens :Rename", |s| s.ed.cmd.as_ref().is_some_and(|c| c.text == "Rename twice"));
    typ(&mut rt, "\x1b");

    // SPC w writes; then an outside change is marked, listed and cleared.
    typ(&mut rt, "ggx");
    leader(&mut rt, "w");
    settle(&mut rt, "SPC w writes the file", |s| !s.ed.buf().is_dirty());
    std::thread::sleep(std::time::Duration::from_millis(400));
    std::fs::write(root.join("main.c"), format!("{changed}/* From outside. */\n")).unwrap();
    settle(&mut rt, "the outside change is marked", |s| !s.outside.is_empty());
    leader(&mut rt, "al");
    settle(&mut rt, "SPC a l lists outside changes", |s| picking(s, PickKind::Locations));
    typ(&mut rt, "\x1b");
    leader(&mut rt, "ac");
    settle(&mut rt, "SPC a c clears the marks", |s| s.outside.is_empty() && said(s, "cleared"));

    // +agent: notes and proposals.
    leader(&mut rt, "ap");
    settle(&mut rt, "SPC a p with none says so", |s| said(s, "no proposals"));
    let r = ask(&mut rt, "reveal", serde_json::json!({"path": "main.c", "line": 3, "end_line": 5, "note": "look"}));
    assert!(r.ok, "{:?}", r.error);
    settle(&mut rt, "the note shows", |s| s.note.is_some());
    leader(&mut rt, "an");
    settle(&mut rt, "SPC a n clears the note", |s| s.note.is_none());
    let notes = root.join("notes.txt");
    rt.update(move |s: &mut App, _| s.open_path(notes, None));
    settle(&mut rt, "open notes.txt", |s| s.ed.buf().path.as_deref().is_some_and(|p| p.ends_with("notes.txt")));
    let r = ask(&mut rt, "propose", serde_json::json!({"path": "main.c", "text": "int main(void) { return 0; }\n", "label": "smaller"}));
    assert!(r.ok, "{:?}", r.error);
    leader(&mut rt, "ap");
    settle(&mut rt, "SPC a p shows the proposal", |s| {
        s.ed.buf().path.as_deref().is_some_and(|p| p.ends_with("main.c")) && s.proposal_here().is_some()
    });
    typ(&mut rt, "n");

    // SPC q closes the buffer.
    let before = rt.update_if(|s: &mut App, _| s.ed.docs.len() == 2);
    assert!(before, "two buffers open");
    leader(&mut rt, "q");
    settle(&mut rt, "SPC q closes the buffer", |s| s.ed.docs.len() == 1);

    // A key added to the menu must be added here too.
    let tested = ["f", "b", "/", "e", ",", "d", "gn", "gp", "gs", "ld", "lr", "lh", "ln", "la", "w", "al", "ac", "ap", "an", "q"];
    let shown = super::keys::LEADER.iter().filter(|(_, _, group)| !group).map(|(c, _, _)| c.to_string());
    let groups = [('g', super::keys::LEADER_GIT), ('l', super::keys::LEADER_LSP), ('a', super::keys::LEADER_AGENT)];
    let shown = shown.chain(groups.iter().flat_map(|(g, keys)| keys.iter().map(move |(c, _, _)| format!("{g}{c}"))));
    for key in shown {
        assert!(tested.contains(&key.as_str()), "SPC {key} shows in the menu but this test does not press it");
    }
}

fn picking_now(rt: &mut Runtime<App>) -> bool {
    rt.update_if(|s: &mut App, _| matches!(s.overlay, Overlay::Pick(_)))
}

/// Every entry of the command palette, run in the real app with its real workers: each one must
/// do what its label says, or say why it cannot.
#[test]
fn every_palette_command_does_what_it_says() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    std::fs::write(root.join("a.txt"), "alpha\n").unwrap();
    std::fs::write(root.join("b.txt"), "beta\n").unwrap();
    let mut app = App::new(Config::default(), root.clone(), false, None);
    app.svc = Some(crate::services::Services::start(root.clone(), std::sync::Arc::new(|| {}), false));
    let mut rt = run(app);
    for f in ["a.txt", "b.txt"] {
        let path = root.join(f);
        rt.update(move |s: &mut App, _| s.open_path(path, None));
        settle(&mut rt, "open", |s| s.ed.has_doc() && s.ed.buf().path.as_deref().is_some_and(|p| p.ends_with(f)));
    }
    let cmd = |rt: &mut Runtime<App>, id: &'static str| rt.update(move |s: &mut App, t| s.run_command(id, t));
    let mut tested = Vec::new();
    let mut check = |rt: &mut Runtime<App>, id: &'static str, what: &str, done: &dyn Fn(&App) -> bool| {
        cmd(rt, id);
        settle(rt, what, done);
        tested.push(id);
        typ(rt, "\x1b");
    };
    check(&mut rt, "find", "find opens the finder", &|s| picking(s, PickKind::Files));
    check(&mut rt, "grep", "grep opens project search", &|s| picking(s, PickKind::Grep));
    typ(&mut rt, "x");
    check(&mut rt, "save", "save writes the file", &|s| !s.ed.buf().is_dirty());
    typ(&mut rt, "x");
    check(&mut rt, "save-all", "save all writes every file", &|s| s.ed.docs.iter().all(|d| !d.buf.is_dirty()));
    let shown = rt.update_if(|s: &mut App, _| s.tree.show);
    check(&mut rt, "tree", "tree toggles the file tree", &move |s| s.tree.show != shown);
    check(&mut rt, "settings", "settings opens settings", &|s| s.on_settings);
    // Close the Settings tab, so next and prev cycle the two files only.
    rt.update(|s: &mut App, _| {
        s.on_settings = false;
        s.settings = None;
    });
    check(&mut rt, "next", "next goes to the next buffer", &|s| s.ed.buf().path.as_deref().is_some_and(|p| p.ends_with("a.txt")));
    check(&mut rt, "prev", "prev goes to the previous buffer", &|s| s.ed.buf().path.as_deref().is_some_and(|p| p.ends_with("b.txt")));
    check(&mut rt, "keymap=classic", "the classic keymap", &|s| s.ed.keymap == Keymap::Classic);
    check(&mut rt, "keymap=neovim", "the Neovim keymap", &|s| s.ed.keymap == Keymap::Vim);
    for (id, mode) in [("theme=light", "light"), ("theme=system", "system"), ("theme=omarchy", "omarchy"), ("theme=dark", "dark")] {
        check(&mut rt, id, id, &move |s| s.cfg.theme.mode == mode);
    }
    let wrap = rt.update_if(|s: &mut App, _| s.cfg.editor.wrap);
    check(&mut rt, "wrap", "wrap toggles soft wrap", &move |s| s.cfg.editor.wrap != wrap);
    check(&mut rt, "outside", "outside says when there is nothing", &|s| said(s, "no changes from outside"));
    check(&mut rt, "clear-outside", "clear-outside confirms", &|s| said(s, "cleared"));
    check(&mut rt, "proposal", "proposal says when there is none", &|s| said(s, "no proposals"));
    typ(&mut rt, "x");
    check(&mut rt, "quit", "quit refuses to drop unsaved edits", &|s| s.ed.msg.as_ref().is_some_and(|m| m.error && m.text.contains("E37")));
    typ(&mut rt, "u");
    check(&mut rt, "close", "close closes the buffer", &|s| s.ed.docs.len() == 1);

    // `:set theme=` takes a theme id too.
    typ(&mut rt, ":set theme=gruvbox\n");
    settle(&mut rt, ":set theme=gruvbox", |s| s.cfg.theme.mode == "dark" && s.cfg.theme.dark == "gruvbox");
    typ(&mut rt, ":set theme=flexoki-light\n");
    settle(&mut rt, ":set theme=flexoki-light", |s| s.cfg.theme.mode == "light" && s.cfg.theme.light == "flexoki-light");
    typ(&mut rt, ":set theme=nope\n");
    settle(&mut rt, ":set theme=nope is an error", |s| s.ed.msg.as_ref().is_some_and(|m| m.error));

    // An entry added to the palette must be added here too.
    for (label, id) in super::picker::COMMANDS {
        assert!(tested.contains(id), "\"{label}\" ({id}) shows in the palette but this test does not run it");
    }
}

/// A drag near the bottom keeps the text under the pointer still. Scrolloff used to scroll at
/// every move there, so the selection ran away to the end of the file.
#[test]
fn a_drag_near_the_bottom_selects_what_is_under_the_pointer() {
    let src: String = (1..=300).map(|i| format!("line {i}\n")).collect();
    let mut rt = run(rust_app(&src));
    let [x, y, _, h] = rt.state.m.text;
    let (cw, rh, gutter) = (rt.state.m.cw, rt.state.m.row_h, rt.state.m.gutter);
    let row = (h / rh).floor() - 3.0;
    let py = y + row * rh + rh / 2.0;
    let px = x + gutter + cw / 2.0;
    rt.pointer_move(px, py);
    rt.pointer_down(px, py);
    rt.frame(1.0 / 60.0);
    let top = rt.state.ed.doc().view.top;
    for i in 1..=60 {
        rt.pointer_move(px + i as f32 * cw / 10.0, py);
        rt.frame(1.0 / 60.0);
    }
    rt.pointer_up(px + 6.0 * cw, py);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.ed.doc().view.top, top, "the text held still under the pointer");
    assert_eq!(rt.state.ed.cursor().line, top + row as usize, "the cursor is on the row under the pointer");
}

/// Keys of the pickers, the tree, Settings, a proposal, completion and setup, as docs/keymaps.md lists them.
#[test]
fn panel_and_popup_keys_from_the_key_reference() {
    let mut a = app("neovim");
    for n in ["two", "three"] {
        let mut b = Buffer::from_str("x\n");
        b.path = Some(std::env::temp_dir().join(format!("editor-test-{n}.rs")));
        a.ed.docs.push(Doc::new(b));
    }
    let files: Vec<String> = ["Cargo.toml", "src/lib.rs", "src/main.rs"].iter().map(|s| s.to_string()).collect();
    a.tree.index = super::tree::TreeIndex::build(&files);
    let mut rt = run(a);
    let (plain, shift) = (Mods::default(), Mods { shift: true, ..Default::default() });
    let pick = |rt: &Runtime<App>| if let Overlay::Pick(p) = &rt.state.overlay { (p.query.clone(), p.sel) } else { ("closed".into(), 0) };
    leader(&mut rt, "b");
    for (k, sel) in [('n', 1), ('p', 0), ('j', 1), ('k', 0)] {
        ctrl(&mut rt, k);
        assert_eq!(pick(&rt).1, sel, "Ctrl {k}");
    }
    for m in [plain, plain, shift] {
        press(&mut rt, Key::Tab, m);
    }
    assert_eq!(pick(&rt).1, 1, "Tab and Shift+Tab move");
    typ(&mut rt, "test-t");
    ctrl(&mut rt, 'w');
    assert_eq!(pick(&rt).0, "test-");
    ctrl(&mut rt, 'u');
    typ(&mut rt, "three\n");
    assert_eq!((pick(&rt).0.as_str(), rt.state.ed.active), ("closed", 2));
    // The tree: o opens a folder, G and g go to the ends, h goes to the parent and closes it, q leaves.
    typ(&mut rt, " e");
    for (keys, want) in [("o", "src"), ("G", "Cargo.toml"), ("g", "src"), ("jh", "src")] {
        typ(&mut rt, keys);
        assert_eq!(rt.state.tree.sel.as_deref(), Some(want), "tree {keys}");
    }
    typ(&mut rt, "hq");
    assert!(!rt.state.tree.open.contains("src") && !rt.state.tree.focus);
    // Settings: J, K, Tab, Shift+Tab and 1 to 5 pick a section. j and k move. h, l, Enter and Space change.
    typ(&mut rt, ":Settings\nJ");
    let section = |rt: &Runtime<App>| rt.state.settings.as_ref().map(|s| s.section);
    assert_eq!(section(&rt), Some(super::settings::Section::Font));
    typ(&mut rt, "K");
    press(&mut rt, Key::Tab, plain);
    press(&mut rt, Key::Tab, shift);
    assert_eq!(section(&rt), Some(super::settings::Section::Appearance));
    typ(&mut rt, "3l");
    assert_eq!(rt.state.cfg.editor.numbers, "relative");
    typ(&mut rt, "hj\n");
    assert_eq!((rt.state.cfg.editor.numbers.as_str(), rt.state.cfg.editor.cursor.as_str()), ("hybrid", "bar"));
    typ(&mut rt, " ");
    assert_eq!(rt.state.cfg.editor.cursor, "underline");
    // A proposal: Tab and ] go to the next change, [ to the one before, g to the top. Esc rejects, Enter accepts.
    let src: String = (0..60).map(|i| format!("line {i}\n")).collect();
    let mut rt = run(rust_app(&src));
    let new = src.replace("line 2\n", "two\n").replace("line 50\n", "fifty\n");
    assert!(ask(&mut rt, "propose", serde_json::json!({ "text": new })).ok);
    let top = |rt: &Runtime<App>| rt.state.proposals[0].top;
    let first = top(&rt);
    press(&mut rt, Key::Tab, plain);
    assert!(top(&rt) > first);
    typ(&mut rt, "[");
    assert_eq!(top(&rt), first);
    typ(&mut rt, "]cg");
    assert_eq!(top(&rt), 0);
    press(&mut rt, Key::Escape, plain);
    assert!(rt.state.proposals.is_empty() && text(&rt) == src);
    assert!(ask(&mut rt, "propose", serde_json::json!({ "text": new })).ok);
    press(&mut rt, Key::Enter, plain);
    assert!(text(&rt).contains("fifty"));
    // The completion list: Ctrl+N, Ctrl+P and Shift+Tab move, Tab takes an item, Ctrl+E closes it.
    let list = |s: &mut App, _: &mut rune::Tx| {
        let items = ["foo", "fob"].map(|l| crate::services::lsp::Item { label: l.into(), detail: "".into(), kind: "", insert: l.into() });
        s.completion = Some(super::lsp::Completion { start: Pos::new(0, 0), items: items.to_vec(), shown: vec![0, 1], sel: 0 });
    };
    typ(&mut rt, "ggOfo");
    rt.update(list);
    ctrl(&mut rt, 'n');
    ctrl(&mut rt, 'p');
    press(&mut rt, Key::Tab, shift);
    press(&mut rt, Key::Tab, plain);
    assert!(text(&rt).starts_with("fob\n"), "{}", text(&rt));
    rt.update(list);
    ctrl(&mut rt, 'e');
    assert!(rt.state.completion.is_none() && rt.state.ed.mode == Mode::Insert);
    // Setup: j, k, h, l and arrows move, Enter and Space pick, Tab and Shift+Tab change the step, Esc goes back.
    let mut rt = run(App::new(Config::default(), std::env::temp_dir(), true, None));
    let st = |rt: &Runtime<App>| if let super::Screen::Setup(s) = &rt.state.screen { (s.step, s.focus, s.vim()) } else { (0, 0, true) };
    typ(&mut rt, "jkljh");
    press(&mut rt, Key::Down, plain);
    typ(&mut rt, "k\n");
    assert_eq!(st(&rt), (1, 1, false), "Enter picks the classic keymap");
    typ(&mut rt, "k ");
    assert_eq!(st(&rt), (1, 0, true), "Space picks the Neovim keymap");
    press(&mut rt, Key::Tab, plain);
    assert_eq!(st(&rt).0, 2);
    press(&mut rt, Key::Tab, shift);
    press(&mut rt, Key::Tab, plain);
    typ(&mut rt, "\x1b");
    assert_eq!(st(&rt).0, 1);
}
