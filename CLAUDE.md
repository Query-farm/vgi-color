# CLAUDE.md — vgi-color

Contributor/agent notes. User-facing docs live in `README.md`; this is the
"how it's built and where the sharp edges are" companion.

## What this is

A [VGI](https://query.farm) worker (Rust, compiled binary) exposing **color
science** — color-space conversions, **CIEDE2000** color difference (ΔE), and
**WCAG** contrast — to DuckDB/SQL over Arrow IPC. Built on the `vgi` crate
(crates.io), modeled on `vgi-image` / `vgi-units` / `vgi-useragent`. Catalog name
`color` (single `main` schema).

The color engine implements every transform **directly from the standard
formulas** — no third-party color crate. A `palette`-based engine was considered;
implementing the formulas directly was chosen for zero added dependencies and
full MSRV control (workspace `rust-version = 1.97`).

## Layout

```
Cargo.toml                          workspace; pins vgi = "0.51.0", arrow 59.2 (vgi.v3 / haybarn 2.0)
crates/color-worker/
  src/main.rs                       native binary: logging + `build_worker().run()`
  src/lib.rs                        build_worker(): registers scalars + catalog metadata (shared with color-wasm)
  src/meta.rs                       per-object tag helpers + vgi.agent_test_tasks prompts (graders: vgi-agent-tests.yaml)
  src/examples.rs                   composed example queries, each defined once (schema list + driving object)
  src/color.rs                      PURE engine (no Arrow): conversions, ΔE, WCAG, named table + unit tests
  src/arrow_io.rs                   VARCHAR/INT/DOUBLE cell reads + STRUCT field sets + in-process scalar test harness
  src/scalar/{convert,analysis,mod}.rs   thin Arrow scalar adapters
  src/table/{named,mod}.rs          thin Arrow table-producer adapter (named_colors)
  tests/color_science.rs            integration tests against known values
crates/color-wasm/                  browser build (same build_worker over a SAB channel)
test/sql/*.test                     haybarn-unittest sqllogictest — authoritative E2E
vgi-lint.toml                       vgi-lint settings (location + fail_on = "info")
Makefile                            test / test-unit / test-sql / lint / fmt / build / clean
```

Pattern: keep computation in `color.rs` (pure, unit-tested), keep Arrow
marshalling in `arrow_io.rs` + `scalar/*.rs` + `table/*.rs` (thin, harness-tested).

## The formulas (all standard, implemented directly in `color.rs`)

- **sRGB ⇄ linear**: IEC 61966-2-1 piecewise companding (`0.04045`/`12.92`
  threshold, `2.4` gamma, `0.055` offset).
- **Relative luminance (WCAG 2.x)**: `0.2126 R + 0.7152 G + 0.0722 B` over the
  *linearized* channels.
- **Contrast ratio (WCAG 2.x)**: `(L_light + 0.05) / (L_dark + 0.05)`, `1.0…21.0`.
- **HSL ⇄ RGB**: standard hue-sector conversion (`h` degrees, `s`/`l` in `0..1`).
- **CIELAB (D65)**: linear-sRGB → CIE XYZ (sRGB/D65 matrix) → L\*a\*b\* with CIE
  `f(t)` companding and the D65 reference white (`Xn=0.95047, Yn=1, Zn=1.08883`).
- **CIEDE2000 (ΔE₀₀)**: full Sharma/Wu/Dalal (2005) reference implementation,
  verified against their published test pair #1 (ΔE00 = 2.0425).

## Sharp edges

1. **`haybarn-unittest` skips `require vgi`** — `.test` files use explicit
   `statement ok` + `LOAD vgi;`, and ATTACH `LOCATION '{VGI_COLOR_WORKER}'` (the
   2.0 runner's env syntax; `${VAR}` is deprecated). analysis/convert set
   `search_path = 'color.main'`; compose.test uses catalog-qualified names exactly
   as published. Each file ends `USE memory` before `DETACH color`. Determinism:
   float assertions use `ROUND(...)`. Use haybarn-unittest **2.0** (it has no
   `--version`; check `uv tool list`) — 1.x rejects `--keep-home` and cannot attach
   a vgi.v3 worker.

2. **NULL / clamp policy (deliberate, no errors).** NULL input → NULL output.
   Invalid hex → NULL (treated as data, not an error). Out-of-range RGB channels
   are clamped to `0..=255`; NaN/infinite HSL components → NULL
   (`color::hsl_to_rgb_checked`). The worker never panics — `from_hex` returns
   `Option<Rgb>` and every scalar maps `None` to a NULL cell.

3. **STRUCT return types must match bind↔process.** `from_hex`/`hsl_to_rgb` return
   `STRUCT(r,g,b INT)`, `rgb_to_hsl` returns `STRUCT(h,s,l DOUBLE)`, `rgb_to_lab`
   returns `STRUCT(l,a,b DOUBLE)`. The exact `Fields` come from
   `arrow_io::{rgb,hsl,lab}_struct_fields()` and are used in both `on_bind`
   (`BindResponse::result(Struct(...))`) and `process` (`StructArray::new` with a
   `NullBuffer`, so invalid/NULL rows are NULL structs).

4. **Scalars are positional-only.** `to_hex`/`rgb_to_hsl`/`rgb_to_lab` read INT
   columns 0/1/2 via `arrow_io::int_val` (accepts any DuckDB integer width, since
   a literal `to_hex(255,…)` may not arrive as INT32). `hsl_to_rgb` reads its
   `h/s/l` via `double_val`. Hex-string scalars read column(s) via `text_str`.

5. **Everything lives in the lib.** `main.rs` only calls
   `color_worker::build_worker().run()`; `color-wasm` calls the same
   `build_worker`, and `tests/` uses the re-exported `pub mod color`.

6. **Named colors.** `color::NAMED_COLORS` is the CSS Color Module Level 4 table
   (`(name, r, g, b)`). It backs both `nearest_color_name` (closest by ΔE₀₀, ties
   resolve to first-in-table order) and the `named_colors()` table function. It has
   all 148 CSS keywords, including the seven `grey` aliases (placed after their
   `gray` twin so `nearest_color_name` still answers `gray`).

7. **`is_dark` is the WCAG crossover, not luminance < 0.5.** It is true when white
   text has a higher contrast ratio than black (luminance < ≈0.1791), so choosing
   text by `is_dark` always gets the better of black/white (≥ 4.58:1). v0.4.0
   and earlier used 0.5, which picked the worse text color for 50 of the 141
   named colors then in the table.

8. **Composed examples.** `src/examples.rs` defines each once; `lib.rs` publishes
   all of them in the schema `vgi.example_queries` (≤ 8,000 chars, VGI417) and each
   is also on its driving object's list within the first five (the vgi-lint agent
   sim only shows per-object examples). `compose.test` and README copies are
   checked against the constants by unit tests.

## Testing

```sh
cargo test --workspace --all-features    # pure unit + arrow-boundary harness + integration
cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --all -- --check
make test-sql                            # builds release, sets VGI_COLOR_WORKER, haybarn 2.0 over test/sql/*
uvx --refresh --prerelease allow --from 'vgi-lint-check>=2.5.1' vgi-lint lint --execute --execute-mode run --fail-on info
make test                                # cargo test + sql
```

CI (`.github/workflows/ci.yml`) runs fmt/clippy/build/test/rustdoc plus an
`integration` matrix (subprocess/http/unix × ubuntu/macos) that downloads the
newest haybarn-v2.0 `haybarn-unittest` and runs `ci/run-integration.sh`.

## Function surface

Scalars: `to_hex` (VARCHAR), `from_hex` (STRUCT r/g/b), `rgb_to_hsl`,
`hsl_to_rgb`, `rgb_to_lab` (STRUCTs), `delta_e`/`luminance`/`contrast_ratio`
(DOUBLE), `wcag_level`/`nearest_color_name` (VARCHAR), `is_dark` (BOOLEAN).
Table: `named_colors` (name/hex). 148 CSS named colors. The worker's build
version is exposed as the catalog `implementation_version` (via `vgi_catalogs()`),
not a parameterless `version()` scalar (VGI328).
