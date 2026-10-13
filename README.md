# bpmc · BPM 计数器

一个用 Rust 写的终端 BPM（每分钟节拍数）计数器：**跟着节拍按空格键**，
界面实时显示当前 BPM。按 `q` 退出。

A terminal BPM counter written in Rust: **tap the space bar to the beat**
and the UI shows the current BPM in real time. Press `q` to quit.

```
        ███ ███ ███
        █ █ █ █   █
        ███ ███ ███
          █ █ █   █
        ███ ███ ███
         BPM 计数器 · 就绪

        按键次数     最近间隔      抖动
           12        500 ms      ±7 ms

     ▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▂▃▄▅▆▇█ ...

    空格  计数      q / Esc  退出      r  重置
```

## 功能 / Features

| 功能 | Feature |
| --- | --- |
| 按空格键实时显示 BPM | Real-time BPM from space-bar taps |
| 滚动窗口取最近 12 次敲击的平均间隔 | Rolling window over the last 12 intervals |
| 显示按键次数、最近间隔、抖动（间隔标准差） | Tap count, last interval, jitter (std. dev.) |
| 间隔柱状图历史 | Bar graph of recent intervals |
| 长时间不按自动标记为空闲 | Auto-detects idle after 3 s |
| 按 `r` 重置 | `r` resets the measurement |
| 中英文界面自动切换 | Automatic Chinese / English UI |
| 跨平台：Windows / Linux / macOS | Cross-platform: Windows / Linux / macOS |

## 安装与运行 / Build and run

需要 Rust 工具链（`cargo`）。

```bash
git clone git@github.com:amonisachicken/bpmc.git   # SSH
# 或： git clone https://github.com/amonisachicken/bpmc.git
cd bpmc

cargo run --release
```

或者构建后直接运行二进制：

```bash
cargo build --release
./target/release/bpmc        # Linux / macOS
.\target\release\bpmc.exe    # Windows
```

也可以安装到 `~/.cargo/bin`：

```bash
cargo install --path .
bpmc
```

## 按键 / Keys

| 键 | 动作 |
| --- | --- |
| `Space` | 记录一次敲击（计数） |
| `r` | 重置当前测量 |
| `q` / `Esc` / `Ctrl-C` | 退出 |

> 提示：BPM = 60000 ÷ 平均间隔（毫秒）。如果你想要 120 BPM，
> 就每秒敲两次（间隔 500 ms）。

## 语言 / Language

界面语言由本地化环境变量决定：

- 依次检查 `LC_ALL` → `LC_MESSAGES` → `LANG`
- 任一变量（不区分大小写）**包含 `zh`** → 中文界面
- 否则 → 英文界面

```bash
LANG=zh_CN.UTF-8 cargo run --release    # 中文
LANG=en_US.UTF-8 cargo run --release    # English
```

Windows 上通常没有 `LANG`，所以默认是英文；可以用
`$env:LANG="zh_CN.UTF-8"`（PowerShell）临时切换。

## 跨平台说明 / Platform notes

代码只依赖 [`ratatui`](https://crates.io/crates/ratatui) 以及它重导出的
[`crossterm`](https://crates.io/crates/crossterm)，两者都支持 Windows
（含 Windows Terminal / ConHost）、Linux 和各种 Unix 终端、macOS。
没有平台相关的 `cfg` 分支，也没有 unsafe 代码。

终端需要支持 UTF-8 和 256 色/真彩色才能完整显示方块数字与柱状图；
若窗口太小会退化为一行提示或纯数字显示。

## 开发 / Development

```bash
cargo test      # 单元测试：BPM 计算、空闲判定、语言探测、状态机
cargo clippy --all-targets

# 无需真实终端的渲染自检：把界面画进内存缓冲区并打印出来
cargo run --example render_demo          # 英文
cargo run --example render_demo -- zh    # 中文
cargo run --example render_demo -- --check   # 只做烟雾检查（CI 用）
```

代码结构：

| 文件 | 职责 |
| --- | --- |
| `src/main.rs` | 终端初始化/清理、事件循环、按键处理 |
| `src/app.rs` | 应用状态（时钟、脉冲动画、计数、历史） |
| `src/bpm.rs` | 敲击时间戳 → BPM 的估算逻辑（含单元测试） |
| `src/i18n.rs` | 中英文字符串表与语言探测 |
| `src/ui.rs` | 所有绘制逻辑 |
| `examples/render_demo.rs` | 无 TTY 的渲染自检 |
| `scripts/check.sh` | 一键跑测试 + clippy + 发布构建 + 渲染自检 |
| `scripts/ci-local.sh` | 本地复现 CI 的全部步骤 |
| `.github/workflows/ci.yml` | GitHub Actions：Linux / Windows / macOS 三平台测试与构建 |

### 关于 Windows 上的 MSYS2 构建

若使用 MSYS2 的 clang64 Rust 工具链，链接步骤（`windows-gnu`）需要
`dlltool.exe`，它只在 mingw64 树里，因此 `PATH` 需要同时包含两者：

```bash
export PATH=/clang64/bin:/mingw64/bin:/usr/bin:/bin:$PATH
```

`scripts/build.sh` 与 `scripts/check.sh` 已封装该设置。使用 rustup 默认的
MSVC 工具链时不需要这些。

## License

MIT
