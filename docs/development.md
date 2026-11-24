# Development

## Build

```sh
cargo build              # debug
cargo build --release    # what you run every day
```

Linux needs `libxkbcommon-dev` and `libwayland-dev` (Debian and Ubuntu names). Rune is a git
dependency pinned to one revision. Apps that build Rune patch four crates from the same
revision; `Cargo.toml` has them under `[patch.crates-io]`. To move to a newer Rune, change all
six `rev` values together.

## Code layout

```text
src/
  lib.rs           the core: no window code
  buffer.rs        rope, undo, marks, saves
  editor.rs        Editor, Doc, Mode, Action, KeyInput
  vim/             the Neovim keymap, Ex commands, search, regex
  classic/         the classic keymap and multiple cursors
  syntax/          tree-sitter objects and motions
  main.rs          the app
  app/             state, key routing, mouse, pickers, Settings, setup, LSP glue
  ui/              views
  services/        workers: io, index, git, grep, watch, lsp
  config.rs        config.toml
  theme.rs         palettes and the Omarchy reader
  highlight.rs     syntax colours
  scenes.rs        named states for headless snapshots
docs/              this documentation and its media
assets/            the icon and the Linux desktop entry
```

Keep files under 800 lines, and put tests in a sibling `tests.rs` or `*_tests.rs`.

## Tests

```sh
cargo test                                    # every test
cargo test --lib                              # the core only: no window, no GPU
cargo test --bin editor app::tests            # the app, driven by keys and clicks
cargo test --bin editor lsp_live -- --ignored # against a real clangd
```

The app tests run the real view and key routing in Rune's headless runtime. For example:

```rust
let mut rt = run(app("neovim"));
typ(&mut rt, "jdd");
assert!(!text(&rt).contains("let a"));
```

## Scenes and snapshots

A scene is a named starting state in `src/scenes.rs`. `--snap` renders one to a PNG with no
window:

```sh
target/debug/editor --list-scenes
target/debug/editor --snap finder.png --scene finder --size 1440x900 --scale 2 --at 0.5
```

`--at 0.5` waits half a second, so the layout settles and the font is measured.

## Videos

Rune records a scene and a key script straight to video, with the keys drawn as caps:

```sh
target/release/editor --scene editor --script demo.json \
  --video demo.mp4 --fps 60 --size 1600x900 --scale 1.2 --keys bottom-right
```

`docs/media/editor-promo.mp4` is five such clips with title cards, joined with ffmpeg.

To keep your own paths out of public media, record with a clean home folder: set `HOME` to an
empty folder that holds a copy of the repository, and run from that copy.

## Releases

Push a tag that starts with `v`:

```sh
git tag v0.1.0
git push origin v0.1.0
```

`.github/workflows/release.yml` builds Linux x86-64 and ARM64 and macOS Apple silicon and
Intel, packs each binary with the README, the license, the icon and the desktop entry, and
attaches the archives to a GitHub release. `.github/workflows/ci.yml` runs the tests on every
push and pull request.
