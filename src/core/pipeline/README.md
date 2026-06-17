# Filter Pipeline

Generic, layered output filtering applied **before** each command's own filter.
A command's bespoke filter always runs last; the pipeline handles the
cross-cutting layers (currently: decorative chrome removal).

## Concepts

- **Node layer** — a generic transformation pass run before the custom filter.
  Each lives in its own file with a whole-string (captured) and per-line
  (streaming) form:
  - `decorative` — chrome removal (ANSI, blank runs, box-drawing). Safe pre-custom.
  - `dedup` — collapse consecutive repeats into `[×N] line`. Default **off**: it
    must run after parsing, so today it's enabled only in the global fallback
    (no parser to corrupt).
- **Dial layer** — not a pass; a global level the command's own renderer reads.
  - `truncate` — scales the item caps (`core::truncate::caps()`): each command
    keeps deciding *which* items to cap, but reads a level-scaled value instead of
    the `CAP_*` const. `reasonable` = today; `high` ÷2; `light` ×2; `none` = no cap.
- **`Routing`** (`mod.rs`) — a per-command, **code-level** policy of which node
  layers run. Not user-configurable. A command opts out with `Routing { decorative: false }`.
- **`Levels`** (`levels.rs`) — the **user-configurable** aggressivity per layer
  (`DecorativeLevel`/`DedupLevel`/`TruncateLevel`), resolved once and cached. Every
  level has a `None` variant = layer off.
- **custom filter** — the command's own `cmds/` filter. Always the terminal step.

## Two execution modes

`Pipeline::with_routing(routing)` then either:

- `run(raw, custom)` — **captured**: apply enabled layers to the whole output,
  then call `custom`. Used by `runner` for `run_filtered` / `run_filtered_with_exit`.
- `stream(inner)` — **streamed**: wrap the command's `StreamFilter` so enabled
  layers run per-line before it. Used by `runner` for `run_streamed`. Only
  line-oriented layers have a streaming form; whole-output layers cannot stream.

In both, the raw output kept for tee/tracking is the untouched original — layers
only affect what the custom filter (and the user) sees.

## Where it is wired

The pipeline is applied centrally in `core::runner`, so every command routed
through `runner` inherits it:

- captured paths (`run_filtered`, `run_filtered_with_exit`) → `Pipeline::run`
- streamed path (`run_streamed`) → `Pipeline::stream`

Commands that bypass `runner` (direct `stream::exec_capture` /
`stream::run_streaming`) do not go through the pipeline.

## Global fallback

Unsupported commands (no `cmds/` handler, no TOML filter) reach `run_fallback`
in `main.rs`. Routing order: **cmds → TOML → global fallback**. The fallback:

- **terminal stdout** → passthrough (inherit stdio) so interactive apps and
  color work.
- **excluded command** (`is_excluded`) → passthrough untouched, so raw-output
  commands (`cat`, `head`, …) stay byte-exact.
- **otherwise (piped)** → stream through the pipeline with an `Identity` custom
  filter (decorative only, no command-specific filtering).

The exclude list is a built-in `const` set in `levels.rs`, extended by the user
via `[layers].exclude`.

## Level resolution (`levels.rs`)

Resolved once per process (cached in a `OnceLock`) to keep config off the hot
path. Precedence, highest first:

1. env (`RTK_DECORATIVE_LEVEL`, `RTK_DEDUP_LEVEL`)
2. config `[layers]` (`~/.config/rtk/config.toml`)
3. built-in default (`reasonable`)

## Adding a layer

1. New file `pipeline/<layer>.rs` with its level enum + whole-string and (if
   line-oriented) per-line forms.
2. Add a field to `Routing`.
3. Apply it in `Pipeline::run` (and `stream` if it has a per-line form), in
   canonical order, before the custom step.
4. If user-tunable, add a field to `Levels` + `LayersConfig` and resolve it in
   `levels.rs`.
