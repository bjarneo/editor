//! Command-line and Ex command tests.

use std::path::PathBuf;

use super::tests::{at, cur, ed, feed, text};
use super::{move_lines, run_ex, search_matches};
use crate::buffer::Buffer;
use crate::editor::{Action, Mode};

#[test]
fn substitute_with_g_and_ranges() {
    let mut e = ed("a a\na a\na a\n");
    feed(&mut e, ":s/a/b/<cr>");
    assert_eq!(text(&e), "b a\na a\na a\n");
    feed(&mut e, ":%s/a/c/g<cr>");
    assert_eq!(text(&e), "b c\nc c\nc c\n");
    assert_eq!(e.msg.as_ref().map(|m| m.text.as_str()), Some("5 substitutions on 3 lines"));
    feed(&mut e, ":2,3s/c/d/<cr>");
    assert_eq!(text(&e), "b c\nd c\nd c\n");
    feed(&mut e, "u");
    assert_eq!(text(&e), "b c\nc c\nc c\n", ":s undoes as one step");
}

#[test]
fn substitute_groups_case_and_newlines() {
    let mut e = ed("key = value");
    run_ex(&mut e, "s/\\(\\w\\+\\) = \\(\\w\\+\\)/\\2: \\u\\1/");
    assert_eq!(text(&e), "value: Key");
    let mut e = ed("a,b");
    run_ex(&mut e, "s/,/\\r/");
    assert_eq!(text(&e), "a\nb");
    let mut e = ed("x+y");
    run_ex(&mut e, "s#+#[&]#");
    assert_eq!(text(&e), "x[+]y");
}

#[test]
fn substitute_errors_when_nothing_matches() {
    let mut e = ed("abc");
    let acts = run_ex(&mut e, "s/zz/y/");
    assert_eq!(acts, vec![Action::Error("E486: Pattern not found: zz".into())]);
    assert!(e.msg.as_ref().is_some_and(|m| m.error));
    assert!(run_ex(&mut e, "s/zz/y/e").is_empty());
}

#[test]
fn global_delete_and_vglobal() {
    let mut e = ed("keep\ndrop 1\nkeep\ndrop 2\n");
    feed(&mut e, ":g/drop/d<cr>");
    assert_eq!(text(&e), "keep\nkeep\n");
    feed(&mut e, "u");
    assert_eq!(text(&e), "keep\ndrop 1\nkeep\ndrop 2\n", ":g undoes as one step");
    run_ex(&mut e, "v/drop/d");
    assert_eq!(text(&e), "drop 1\ndrop 2\n");
    let mut e = ed("1\n2\n3\n");
    run_ex(&mut e, "g/^/m0");
    assert_eq!(text(&e), "3\n2\n1\n", ":g/^/m0 reverses the lines");
}

#[test]
fn write_quit_and_open_return_actions() {
    let mut e = ed("x");
    assert_eq!(run_ex(&mut e, "w"), vec![Action::Save(None)]);
    assert_eq!(run_ex(&mut e, "wq"), vec![Action::Save(None), Action::Quit { force: false }]);
    assert_eq!(run_ex(&mut e, "q!"), vec![Action::Quit { force: true }]);
    assert_eq!(run_ex(&mut e, "qa"), vec![Action::QuitAll { force: false }]);
    assert_eq!(run_ex(&mut e, "wa"), vec![Action::SaveAll]);
    assert_eq!(run_ex(&mut e, "x"), vec![Action::Quit { force: false }], ":x writes only a changed buffer");
    assert_eq!(run_ex(&mut e, "e src/main.rs"), vec![Action::Open(PathBuf::from("/tmp/project/src/main.rs"))]);
    assert_eq!(run_ex(&mut e, "e!"), vec![Action::Reload]);
    assert_eq!(run_ex(&mut e, "w /abs/f.rs"), vec![Action::Save(Some(PathBuf::from("/abs/f.rs")))]);
    assert_eq!(run_ex(&mut e, "bn"), vec![Action::NextBuffer]);
    assert_eq!(run_ex(&mut e, "Settings"), vec![Action::Settings]);
}

#[test]
fn unknown_commands_are_errors() {
    let mut e = ed("x");
    let acts = run_ex(&mut e, "frobnicate");
    assert_eq!(acts, vec![Action::Error("E492: Not an editor command: frobnicate".into())]);
}

#[test]
fn line_numbers_jump_and_ranges_delete() {
    let mut e = ed("1\n2\n3\n4\n5\n");
    feed(&mut e, ":4<cr>");
    assert_eq!(cur(&e), (3, 0));
    feed(&mut e, ":.,+1d<cr>");
    assert_eq!(text(&e), "1\n2\n3\n");
    feed(&mut e, ":1d 2<cr>");
    assert_eq!(text(&e), "3\n");
}

#[test]
fn move_copy_join_sort_and_shift() {
    let mut e = ed("a\nb\nc\n");
    run_ex(&mut e, "m+1");
    assert_eq!(text(&e), "b\na\nc\n");
    assert_eq!(cur(&e), (1, 0));
    run_ex(&mut e, "m-2");
    assert_eq!(text(&e), "a\nb\nc\n");
    run_ex(&mut e, "1t$");
    assert_eq!(text(&e), "a\nb\nc\na\n");
    run_ex(&mut e, "1,2j");
    assert_eq!(text(&e), "a b\nc\na\n");
    run_ex(&mut e, "sort");
    assert_eq!(text(&e), "a\na b\nc\n");
    run_ex(&mut e, "sort!");
    assert_eq!(text(&e), "c\na b\na\n");
    run_ex(&mut e, "%>");
    assert_eq!(text(&e), "    c\n    a b\n    a\n");
}

#[test]
fn move_lines_moves_the_cursor_too() {
    let mut e = ed("a\nb\nc");
    move_lines(&mut e, 0, 0, true);
    assert_eq!(text(&e), "b\na\nc");
    assert_eq!(cur(&e), (1, 0));
    move_lines(&mut e, 1, 2, false);
    assert_eq!(text(&e), "a\nc\nb");
    assert_eq!(cur(&e), (0, 0));
    move_lines(&mut e, 0, 0, false);
    assert_eq!(text(&e), "a\nc\nb", "the first line cannot move up");
}

#[test]
fn set_applies_core_options_and_reports_all() {
    let mut e = ed("x");
    let acts = run_ex(&mut e, "set ts=2 noet wrap keymap=classic");
    assert_eq!(e.opts.tab_width, 2);
    assert!(!e.opts.spaces);
    assert!(e.opts.wrap);
    assert!(acts.contains(&Action::SetOption("keymap".into(), "classic".into())));
    assert!(acts.contains(&Action::SetOption("expandtab".into(), "false".into())));
}

#[test]
fn normal_runs_keys_on_each_line() {
    let mut e = ed("a\nb\nc");
    run_ex(&mut e, "%norm A;");
    assert_eq!(text(&e), "a;\nb;\nc;");
    assert_eq!(e.mode, Mode::Normal);
}

#[test]
fn noh_turns_highlight_off_until_the_next_search() {
    let mut e = ed("foo");
    feed(&mut e, "/o<cr>");
    assert!(e.vim.hlsearch);
    run_ex(&mut e, "noh");
    assert!(!e.vim.hlsearch);
    feed(&mut e, "n");
    assert!(e.vim.hlsearch);
}

#[test]
fn cmdline_editing_and_history() {
    let mut e = ed("x\ny\nz");
    feed(&mut e, ":2<cr>");
    feed(&mut e, ":abc<bs><bs><bs><bs>");
    assert_eq!(e.mode, Mode::Normal, "backspace on an empty line cancels");
    feed(&mut e, ":<up>");
    assert_eq!(e.cmd.as_ref().map(|c| c.text.as_str()), Some("2"));
    feed(&mut e, "<esc>");
    assert_eq!(e.mode, Mode::Normal);
    assert_eq!(e.cmd, None);
}

#[test]
fn search_matches_for_highlighting() {
    let b = Buffer::from_str("Foo foo\nbar foo");
    assert_eq!(search_matches(&b, "foo", 0, 1), vec![(0, 3), (4, 7), (12, 15)]);
    assert_eq!(search_matches(&b, "Foo", 0, 1), vec![(0, 3)], "an uppercase letter makes case count");
    assert!(search_matches(&b, "", 0, 1).is_empty());
}

#[test]
fn at_colon_repeats_the_last_ex_command() {
    let mut e = ed("a\nb\nc\n");
    at(&mut e, 0, 0);
    feed(&mut e, ":d<cr>");
    feed(&mut e, "@:");
    assert_eq!(text(&e), "c\n");
}

#[test]
fn bang_filters_a_range_or_runs_a_command() {
    let mut e = ed("a\nb\nc\n");
    assert_eq!(run_ex(&mut e, "1,2!sort -r"), vec![Action::Filter { first: 0, last: 1, cmd: "sort -r".into() }]);
    assert_eq!(run_ex(&mut e, "!cargo fmt"), vec![Action::Shell("cargo fmt".into())]);
    assert!(matches!(run_ex(&mut e, "!").as_slice(), [Action::Error(_)]));
}
