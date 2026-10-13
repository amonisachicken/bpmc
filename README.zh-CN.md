<!-- LANGUAGE: zh -->

# bpmc · BPM 计数器

[English](README.md) · **简体中文**

[![CI](https://github.com/amonisachicken/bpmc/actions/workflows/ci.yml/badge.svg)](https://github.com/amonisachicken/bpmc/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/amonisachicken/bpmc?display_name=tag&sort=semver)](https://github.com/amonisachicken/bpmc/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

一个用 Rust 写的终端 BPM（每分钟节拍数）计数器：**跟着节拍按空格键**，
界面实时显示当前 BPM。按 `q` 退出。

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

## 功能

- 按空格键实时计算并显示 BPM
- 滚动窗口取最近 12 次敲击的间隔平均值（稳定又能快速跟上变速）
- 显示按键次数、最近间隔、抖动（间隔标准差，衡量你敲得稳不稳）
- 间隔历史柱状图：只保留最近 20 次敲击，新数据到来时自动丢弃最旧的
- 3 秒不按自动标记为「空闲」
- 按 `r` 重置当前测量
- 中英文界面根据本地化环境变量自动切换
- 跨平台：Windows / Linux / macOS

## 安装

每个 [Release](https://github.com/amonisachicken/bpmc/releases) 都附带
Linux、macOS、Windows 的**预编译二进制**。下载对应平台的压缩包，解压后把
`bpmc` 放进 `PATH` 即可。

直接从 git 安装：

```bash
cargo install --git https://github.com/amonisachicken/bpmc.git
```

或者克隆后运行：

```bash
git clone git@github.com:amonisachicken/bpmc.git   # SSH
# 或： git clone https://github.com/amonisachicken/bpmc.git
cd bpmc

cargo run --release
```

自己构建后直接跑二进制：

```bash
cargo build --release
./target/release/bpmc        # Linux / macOS
.\target\release\bpmc.exe    # Windows
```

需要 Rust 工具链（`cargo`）。使用 stable 工具链构建与测试。

## 用法

| 键 | 动作 |
| --- | --- |
| `Space` | 记录一次敲击（计入 BPM） |
| `r` | 重置当前测量 |
| `q` / `Esc` / `Ctrl-C` | 退出 |

> 提示：BPM = 60000 ÷ 平均间隔（毫秒）。想要 120 BPM，就每秒敲两次
> （间隔 500 ms）。

## 语言

界面语言由本地化环境变量决定：

- 依次检查 `LC_ALL` → `LC_MESSAGES` → `LANG`
- 任一变量（不区分大小写）**包含 `zh`** → 中文界面
- 否则 → 英文界面

```bash
LANG=zh_CN.UTF-8 cargo run --release    # 中文界面
LANG=en_US.UTF-8 cargo run --release    # English UI
```

Windows 上通常没有 `LANG`，所以默认英文；可以在 PowerShell 里用
`$env:LANG="zh_CN.UTF-8"` 临时切换。

## 跨平台说明

代码只依赖 [`ratatui`](https://crates.io/crates/ratatui) 以及它重导出的
[`crossterm`](https://crates.io/crates/crossterm)，两者都支持 Windows
（Windows Terminal / ConHost）、Linux 和各种 Unix 终端、macOS。
没有平台相关的 `cfg` 分支，也没有 `unsafe` 代码。

终端需要支持 UTF-8 与 256 色/真彩色才能完整显示方块数字和柱状图；
窗口过小时会退化为一行提示或纯数字显示。

## 开发

```bash
cargo test                                    # 单元测试
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check

# 无需真实终端的渲染自检：把界面画进内存缓冲区并打印出来
cargo run --example render_demo              # 英文排版
cargo run --example render_demo -- zh        # 中文排版
cargo run --example render_demo -- --check   # CI 用的烟雾检查
```

代码结构：

| 路径 | 职责 |
| --- | --- |
| `src/main.rs` | 终端初始化/清理、事件循环、按键处理 |
| `src/app.rs` | 应用状态：时钟、脉冲动画、计数、历史 |
| `src/bpm.rs` | 敲击时间戳 → BPM 的估算逻辑（含单元测试） |
| `src/i18n.rs` | 中英文字符串表与语言探测 |
| `src/ui.rs` | 所有绘制逻辑 |
| `examples/render_demo.rs` | 无 TTY 的渲染自检 |
| `scripts/check.sh` | 测试 + clippy + 发布构建 + 渲染自检 |
| `scripts/ci-local.sh` | 本地逐条复现 CI 步骤 |
| `scripts/run-release-steps.sh` | 本地模拟发布 job，矩阵直接从工作流文件读取 |
| `scripts/check-workflows.sh` | 工作流文件结构与其构建矩阵的静态检查 |
| `.github/workflows/ci.yml` | CI：Linux / Windows / macOS 测试与构建 |
| `.github/workflows/release.yml` | 打 `v*` tag 时构建二进制并发布 Release |

<details>
<summary>在 Windows 上用 MSYS2 构建</summary>

若使用 MSYS2 的 clang64 Rust 工具链，`windows-gnu` 的链接步骤需要
`dlltool.exe`，它只在 mingw64 树里，因此 `PATH` 需要同时包含两者：

```bash
export PATH=/clang64/bin:/mingw64/bin:/usr/bin:/bin:$PATH
```

`scripts/build.sh` 与 `scripts/check.sh` 已封装该设置。使用 rustup 默认的
MSVC 工具链时不需要这些。

</details>

## License

MIT — 见 [LICENSE](LICENSE)。
