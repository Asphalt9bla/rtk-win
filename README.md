# rtk-win — Rust Token Killer for Windows

**Native Windows fork of [rtk](https://github.com/rtk-ai/rtk)** — replaces Unix-only commands (`ls`, `tree`, `wc`, `find`) with Rust-native implementations and adds PowerShell cmdlet TOML filters for 60-90% LLM token savings on Windows.

## What's Different

| Area | Upstream rtk | rtk-win |
|------|-------------|---------|
| `ls`, `tree`, `wc`, `find` | Call Unix binaries via shell | Rust-native using `std::fs` / `walkdir` |
| PowerShell cmdlets | Not supported | 15 TOML filters (`Get-ChildItem`, `Select-String`, etc.) |
| Windows package managers | Not supported | `winget` TOML filter (strips progress, truncates tables) |
| Install | `install.sh` (Unix) | `install.ps1` (PowerShell) |
| Hooks | Bash scripts (`.sh`) | PowerShell scripts (`.ps1`) |
| `rtk init --opencode` | Not available | Installs OpenCode plugin for transparent rewrite |
| System dependencies | `libc`, Unix signal handlers, process groups | All removed — pure Win32 API via Rust stdlib |

## Token Savings (Windows Benchmark)

```
Command                    Raw (avg)    RTK (avg)     Saved
ls (project src)             10,810         460       95.7%
ls (drivers large)           40,303       1,725       95.7%
wc (src .rs files)              878         160       81.8%
find (src .rs)                3,848         758       80.3%
find (src .toml)              2,612         768       70.6%
git status                       72          22       69.4%
tree (src dirs)               5,317       2,671       49.8%
winget list                   6,897       3,520       49.0%
systeminfo                    3,637       2,520       30.7%

OVERALL (15 tests):        97,029      35,456       63.5%
```

> Run `scripts/benchmark.ps1` on your machine to get your own numbers.

## Installation

```powershell
# From source (recommended)
git clone https://github.com/YOUR_USER/rtk-win.git
cd rtk-win
.\install.ps1

# Or build manually
cargo build --release
Copy-Item target\release\rtk.exe $env:USERPROFILE\.cargo\bin\
```

### Prerequisites

- [Rust](https://rustup.rs/) (MSVC toolchain, `rustup default stable-x86_64-pc-windows-msvc`)
- Windows 10+ (ARM64 or x64)

## Quick Start

```powershell
# Install for OpenCode
rtk init -g --opencode

# Or for Claude Code / Cline / Gemini CLI
rtk init -g --agent claude
rtk init --agent cline
rtk init -g --gemini

# Test it
rtk ls .                        # Compact directory listing
rtk tree src                    # Truncated directory tree
rtk wc src\*.rs                 # Line/word/byte counts
rtk find -name "*.rs"           # File search (grouped by directory)
rtk git status                  # Compact git status
rtk winget list                 # Truncated package list
```

## Rust-Native Commands

These commands run entirely inside RTK — no external process needed:

- **`ls`** — Compact directory listing with human-readable sizes, grouped by type, extension summary, `max_lines` truncation
- **`tree`** — Unicode tree visualization, filters noise dirs (`node_modules`, `.git`, `target`), `max_lines` truncation
- **`wc`** — Line/word/byte/char counting with common prefix stripping for multi-file mode
- **`find`** — Glob-based file search supporting `-name`, `-iname`, `-type`, `-maxdepth`, results grouped by directory with extension summary (max 50 results by default)

## PowerShell Cmdlet TOML Filters

15 TOML filter files enable RTK to strip headers, progress bars, and noise from PowerShell command output when routed through `rtk proxy` or the OpenCode plugin hook:

`Get-ChildItem`, `Select-String`, `Get-Content`, `Measure-Object`, `Get-Process`, `Get-Service`, `Get-Help`, `Where-Object`, `Compare-Object`, `Get-Item`, `Get-Date`, `Get-Command`, `Get-Member`, `Get-Alias`, `Invoke-Command`

All filters use `(?i)` case-insensitive matching and apply `strip_ansi`, `max_lines`, and `truncate_lines_at` for compact output.

## Windows-Specific TOML Filters

- **`winget`** — Strips leading progress noise, ANSI spinner garbage, truncates long ARP paths, limits to 40 rows
- **`tasklist`** — Limits to 40 processes, truncates long lines at 80 chars
- **`systeminfo`** — Strips blank lines, limits to 30 key-value pairs

## Build & Test

```powershell
cargo build --release
cargo fmt --all
cargo clippy --all-targets
cargo test                        # 2222+ pass, 0 fail
.\scripts\benchmark.ps1           # Run Windows benchmark
```

> **Note**: Full `cargo test` requires `[profile.test] debug = 0` in `Cargo.toml` to avoid OOM during debug test compilation on Windows.

## Architecture

```
CLI request  →  main.rs (Clap dispatch)
                    ├── Rust-native handler (ls, tree, wc, find, git, cargo, ...)
                    └── run_fallback()
                          └── TOML filter match
                                └── execute real command + apply filter pipeline
                                      (strip_ansi → replace → strip_lines → 
                                       truncate → head/tail → max_lines → on_empty)
```

For full details, see [ARCHITECTURE.md](docs/contributing/ARCHITECTURE.md) and [CONTRIBUTING.md](CONTRIBUTING.md) (upstream docs, mostly applicable).

## Differences from Upstream v0.43.0

- Removed all `#[cfg(unix)]` blocks across 6 files: `main.rs`, `core/utils.rs`, `core/stream.rs`, `core/telemetry.rs`, `hooks/integrity.rs`, `hooks/init.rs`
- Removed `libc` dependency from `Cargo.toml`
- Added `/STACK:8388608` linker flag in `build.rs` for Windows stack size
- Replaced `ls.rs`, `tree.rs`, `wc_cmd.rs` with Rust-native implementations
- Added `tool_exists` check in `search.rs` with Windows-friendly error message
- Gemini hook changed from `.sh` to `.ps1` extension
- Added 81 built-in TOML filters (41 upstream + 22 upstream additions + 15 PowerShell + winget + systeminfo + tasklist)
- Added `install.ps1` for Windows-native installation
- Added `scripts/benchmark.ps1` for Windows benchmarking
- Added `[profile.test] debug = 0` to prevent OOM during test compilation

## License

Apache License 2.0 — see [LICENSE](LICENSE).

Forked from [rtk-ai/rtk](https://github.com/rtk-ai/rtk) v0.43.0.
