# Performance

Speed decides every feature. These are the targets from the design, and what a release build
measures on Linux, x86-64.

| Target | Result | How |
| --- | --- | --- |
| Open a 100 MB file in under 1 s | 175 ms | `big_file_opens_and_edits_fast` |
| An edit, undo and jump in a 100 MB file | 24 µs | the same test |
| Keystroke to frame in under 8 ms | 0.3 ms median, 1.4 ms at the 95th percentile | 60 keys, headless, 2× |
| An idle window draws nothing | One frame per key, then none | headless frame counts |
| No LSP or file I/O on the UI thread | Workers do all of it | [architecture](architecture.md) |

## What keeps it fast

- **A rope.** [ropey](https://docs.rs/ropey) makes line lookups, inserts and removes cheap at
  any file size.
- **Only what shows.** The view builds the visible lines, tree rows and picker rows.
- **Syntax colours by window.** The lexers run over the lines on screen plus 300 above them,
  aligned to 64-line blocks and cached per buffer version.
- **Instant changes.** Text and the cursor change in the next frame with no animation, so a key
  costs one frame.
- **Workers.** Opening, saving, indexing, git, ripgrep, file watching and LSP run on their own
  threads.
- **Limits.** Buffers over 4 MiB skip git signs and LSP. Swap files stop at 16 MiB. Tree-sitter
  skips files over 4 MiB.

## Measure it yourself

Open and edit a large file:

```sh
python3 -c "
import sys
with open('/tmp/huge.rs', 'w') as f:
    for i in range(1_050_000):
        f.write(f'    let value_{i} = compute(\"item {i}\", {i}) + offset; // eighty chars\n')
"
EDITOR_BIG=/tmp/huge.rs cargo test --release --lib big_file -- --ignored --nocapture
```

Time keystrokes headless. A script types keys, and Rune prints the CPU time per frame:

```sh
cargo build --release
target/release/editor --scene editor --script keys.json --snap out.png --at 5 --scale 2
```

```json
{"events": [
  {"t": 0.3, "type": "type", "text": "j"},
  {"t": 0.35, "type": "type", "text": "j"},
  {"t": 0.4, "type": "key", "name": "d", "ctrl": true}
]}
```

The output has lines such as:

```text
rune: 301 frames at 60 fps, frame cpu mean 0.578 ms p50 0.289 p95 1.440
rune: animating frames 27, at-rest frames 273 (the window draws none)
```

Headless numbers leave out GPU upload and presentation. The 27 animating frames come from
startup, when layout settles.
