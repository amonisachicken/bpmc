# bpmc

**English** · [简体中文](README.zh-CN.md)

[![CI](https://github.com/amonisachicken/bpmc/actions/workflows/ci.yml/badge.svg)](https://github.com/amonisachicken/bpmc/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/amonisachicken/bpmc?display_name=tag&sort=semver)](https://github.com/amonisachicken/bpmc/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A terminal BPM (beats per minute) counter written in Rust: **tap the space bar
to the beat** and the UI shows the current BPM in real time. Press `q` to quit.

```
        ███ ███ ███
        █ █ █ █   █
        ███ ███ ███
          █ █ █   █
        ███ ███ ███
        BPM Counter · Ready.

        Taps      Last interval      Jitter
          12           500 ms         ±7 ms

     ▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▂▃▄▅▆▇█ ...

     Space  tap    q / Esc  quit    r  reset
```

## Features

- Real-time BPM from space-bar taps
- Rolling window over the last 12 intervals (stable, but reacts quickly)
- Tap count, last interval, and jitter (standard deviation of intervals)
- Bar graph of the interval history
- Auto-detects idle after 3 s without a tap
- `r` resets the measurement
- Chinese / English UI chosen automatically from the locale
- Cross-platform: Windows / Linux / macOS

## Install

**Prebuilt binaries** for Linux, macOS and Windows are attached to every
[release](https://github.com/amonisachicken/bpmc/releases). Download the archive
for your platform, unpack it, and put `bpmc` somewhere on your `PATH`.

From source, straight from git:

```bash
cargo install --git https://github.com/amonisachicken/bpmc.git
```

Or clone and run:

```bash
git clone git@github.com:amonisachicken/bpmc.git   # SSH
# or: git clone https://github.com/amonisachicken/bpmc.git
cd bpmc

cargo run --release
```

Building yourself and running the binary directly:

```bash
cargo build --release
./target/release/bpmc        # Linux / macOS
.\target\release\bpmc.exe    # Windows
```

Requires a Rust toolchain (`cargo`). Built and tested with stable Rust.

## Usage

| Key | Action |
| --- | --- |
| `Space` | Record a tap (counts toward the BPM) |
| `r` | Reset the current measurement |
| `q` / `Esc` / `Ctrl-C` | Quit |

> Tip: BPM = 60000 ÷ mean interval in ms. For 120 BPM, tap twice per second
> (500 ms apart).

## Language

The UI language comes from the localization environment:

- checks `LC_ALL` → `LC_MESSAGES` → `LANG`, in that order
- if any of them **contains `zh`** (case-insensitive) → Chinese UI
- otherwise → English UI

```bash
LANG=zh_CN.UTF-8 cargo run --release    # 中文界面
LANG=en_US.UTF-8 cargo run --release    # English UI
```

On Windows `LANG` is usually unset, so the default is English; switch it per
session with `$env:LANG="zh_CN.UTF-8"` in PowerShell.

## Platform notes

The code depends only on [`ratatui`](https://crates.io/crates/ratatui) and the
[`crossterm`](https://crates.io/crates/crossterm) it re-exports. Both support
Windows (Windows Terminal and ConHost), Linux and other Unix terminals, and
macOS. There are no platform-specific `cfg` branches and no `unsafe` code.

The terminal needs UTF-8 and 256-colour/true-colour support for the block
digits and the bar graph; a very small window degrades to a single line of text.

## Development

```bash
cargo test                                    # unit tests
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check

# Headless render check: draws the UI into an in-memory buffer and prints it.
cargo run --example render_demo              # English layouts
cargo run --example render_demo -- zh        # Chinese layouts
cargo run --example render_demo -- --check   # smoke test used by CI
```

Layout:

| Path | Responsibility |
| --- | --- |
| `src/main.rs` | Terminal setup/teardown, event loop, key handling |
| `src/app.rs` | Session state: clock, pulse animation, tap count, history |
| `src/bpm.rs` | Turning tap timestamps into a BPM estimate (unit-tested) |
| `src/i18n.rs` | Chinese/English string tables and locale detection |
| `src/ui.rs` | All drawing |
| `examples/render_demo.rs` | TTY-free render check |
| `scripts/check.sh` | Tests + clippy + release build + render check |
| `scripts/ci-local.sh` | Replays every CI step locally |
| `scripts/run-release-steps.sh` | Simulates the release job locally, reading its matrix from the workflow |
| `scripts/check-workflows.sh` | Structural check of the workflow files and their build matrix |
| `.github/workflows/ci.yml` | CI: test and build on Linux, Windows, macOS |
| `.github/workflows/release.yml` | On a `v*` tag: build binaries and publish a release |

<details>
<summary>Building on Windows with MSYS2</summary>

With MSYS2's clang64 Rust toolchain, the `windows-gnu` link step needs
`dlltool.exe`, which only ships in the mingw64 tree, so `PATH` must contain
both prefixes:

```bash
export PATH=/clang64/bin:/mingw64/bin:/usr/bin:/bin:$PATH
```

`scripts/build.sh` and `scripts/check.sh` already do this. With rustup's
default MSVC toolchain none of it is needed.

</details>

## License

MIT — see [LICENSE](LICENSE).
