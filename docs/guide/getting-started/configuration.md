---
title: Configuration
description: Customize RTK behavior via config.toml, environment variables, and per-project filters
sidebar:
  order: 4
---

# Configuration

## Config file location

| Platform | Path |
|----------|------|
| Linux | `~/.config/rtk/config.toml` |
| macOS | `~/Library/Application Support/rtk/config.toml` |

```bash
rtk config            # show current configuration
rtk config --create   # create config file with defaults
```

## Full config structure

```toml
[tracking]
enabled = true              # enable/disable token tracking
history_days = 90           # retention in days (auto-cleanup)
database_path = "/custom/path/history.db"   # optional override

[display]
colors = true               # colored output
emoji = true                # use emojis in output
max_width = 120             # maximum output width

[filters]
# These apply to file-reading commands (ls, find, grep, cat/rtk read).
# Paths matching these patterns are excluded from output, keeping noise low.
ignore_dirs = [".git", "node_modules", "target", "__pycache__", ".venv", "vendor"]
ignore_files = ["*.lock", "*.min.js", "*.min.css"]

[tee]
enabled = true              # save raw output on failure
mode = "failures"           # "failures" (default), "always", "never"
max_files = 20              # rotation: keep last N files
# directory = "/custom/tee/path"  # optional override

[telemetry]
enabled = true              # anonymous daily ping — see Telemetry & Privacy for full details

[hooks]
exclude_commands = []       # commands to never auto-rewrite

[layers]
decorative = "reasonable"   # chrome removal: "none" | "light" | "reasonable" | "high"
dedup = "exact"             # collapse repeated lines (fallback): "none" | "exact" | "normalized"
truncate = "reasonable"     # item caps "show N, +M more": "none" | "light" | "reasonable" | "high"
exclude = []                # extra commands to leave unfiltered (raw passthrough)
```

Every layer accepts `"none"` to turn it off. The default keeps RTK's current behavior.

For full details on what is collected, opt-out options, and GDPR rights, see [Telemetry & Privacy](../resources/telemetry.md).

## Environment variables

| Variable | Description |
|----------|-------------|
| `RTK_DISABLED=1` | Disable RTK for a single command (`RTK_DISABLED=1 git status`) |
| `RTK_DECORATIVE_LEVEL` | Decorative level for this invocation (`none`/`light`/`reasonable`/`high`) |
| `RTK_DEDUP_LEVEL` | Dedup level for the fallback (`none`/`exact`/`normalized`) |
| `RTK_TRUNCATE_LEVEL` | Item-cap level for this invocation (`none`/`light`/`reasonable`/`high`) |
| `RTK_TEE_DIR` | Override the tee directory |
| `RTK_TELEMETRY_DISABLED=1` | Disable telemetry |
| `RTK_HOOK_AUDIT=1` | Enable hook audit logging |
| `SKIP_ENV_VALIDATION=1` | Skip env validation (useful with Next.js) |

## Filter layers

Before each command's own filter, RTK runs a generic pipeline of layers. Today
that is the **decorative** layer — lossless chrome removal applied to every
command routed through RTK (and to otherwise-unsupported commands via the global
fallback).

```toml
[layers]
decorative = "reasonable"
exclude = ["mytool"]
```

| `decorative` | What it removes |
|--------------|-----------------|
| `none` | nothing (layer off) |
| `light` | ANSI color codes only (fully lossless) |
| `reasonable` (default) | ANSI + trailing whitespace + collapses blank-line runs |
| `high` | + drops pure box-drawing / separator lines |

Override for a single invocation:

```bash
RTK_DECORATIVE_LEVEL=high rtk <command>
```

Every layer also accepts `none` to disable it (e.g. `RTK_DECORATIVE_LEVEL=none`
for raw output).

### Truncate (item caps)

Each command caps how many items it lists ("show N errors, +M more"). The
`truncate` level scales those caps — higher = more compression (fewer items):

| `truncate` | Effect on caps |
|------------|----------------|
| `none` | no cap (show everything) |
| `light` | looser caps (×2 — show more) |
| `reasonable` (default) | today's per-command caps, unchanged |
| `high` | tighter caps (÷2 — fewer items, more compression) |

```bash
RTK_TRUNCATE_LEVEL=high rtk cargo clippy   # fewer items shown
RTK_TRUNCATE_LEVEL=none rtk pip list       # no truncation
```

### Dedup (unsupported commands)

For commands RTK doesn't have a specific filter for, it also collapses consecutive
repeated lines into `[×N] line` — useful for noisy logs, retries, and repeated warnings.

| `dedup` | Behavior |
|---------|----------|
| `none` | no collapse (layer off) |
| `exact` (default) | collapse byte-identical consecutive lines |
| `normalized` | mask volatile tokens (numbers, hex, timestamps) first, then collapse near-identical lines |

```bash
RTK_DEDUP_LEVEL=normalized rtk <command>
```

### Excluding commands from the pipeline

Raw-output commands must stay byte-exact, so RTK never filters them: `cat`,
`head`, `tail`, `base64`, `xxd`, `hexdump`, `od`, `strings`, `dd`. Add your own
with `[layers].exclude` (the `exclude` key under `[layers]`):

```toml
[layers]
exclude = ["mytool", "dump-binary"]
```

Matching is by command name (basename), so `/usr/bin/cat` and `cat` both match.

## Tee system

When a command fails, RTK saves the full raw output to a local file and prints the path:

```
FAILED: 2/15 tests
[full output: ~/.local/share/rtk/tee/1707753600_cargo_test.log]
```

Your AI assistant can then read the file if it needs more detail, without re-running the command.

| Setting | Default | Description |
|---------|---------|-------------|
| `tee.enabled` | `true` | Enable/disable |
| `tee.mode` | `"failures"` | `"failures"`, `"always"`, `"never"` |
| `tee.max_files` | `20` | Rotation: keep last N files |
| Min size | 500 bytes | Outputs shorter than this are not saved |
| Max file size | 1 MB | Truncated above this |

## Excluding commands from auto-rewrite

Prevent specific commands from being rewritten by the hook:

```toml
[hooks]
exclude_commands = ["git rebase", "git cherry-pick", "docker exec"]
```

Patterns match against the full command after stripping env prefixes (`sudo`, `VAR=val`), so `"psql"` excludes both `psql -h localhost` and `PGPASSWORD=x psql -h localhost`.

Subcommand patterns work too: `"git push"` excludes `git push origin main` but not `git status`.

Patterns starting with `^` are treated as regex:

```toml
[hooks]
exclude_commands = ["^curl", "^wget", "git rebase"]
```

Invalid regex patterns fall back to prefix matching.

Or for a single invocation:

```bash
RTK_DISABLED=1 git rebase main
```

## Telemetry

RTK sends one anonymous ping per day (23h interval). No personal data, no file paths, no command content.

Data sent: device hash, version, OS, architecture, command count/24h, top commands, savings %.

To opt out:

```bash
# Via environment variable
export RTK_TELEMETRY_DISABLED=1

# Via config.toml
[telemetry]
enabled = false
```

## Per-project filters

Create `.rtk/filters.toml` in your project root to add custom filters or override built-ins. See [`src/filters/README.md`](https://github.com/rtk-ai/rtk/blob/master/src/filters/README.md) for the full TOML DSL reference.
