<p align="center">
  <img src="https://query.farm/media-kit/products/vgi/vgi-logo.png" alt="Vector Gateway Interface (VGI)" width="320">
</p>

<p align="center"><em>A <a href="https://query.farm">Query.Farm</a> VGI worker for DuckDB.</em></p>

# Color Science — RGB/HSL/LAB, CIEDE2000 ΔE & WCAG Contrast in DuckDB

> **vgi-color** · a [Query.Farm](https://query.farm) VGI worker

A [VGI](https://query.farm) worker that brings **color science** — color-space
conversions, **CIEDE2000** color difference (ΔE), and **WCAG** contrast — to
DuckDB over Apache Arrow.

```sql
LOAD vgi;
ATTACH 'color' (TYPE vgi, LOCATION 'github-auto://Query-farm/vgi-color@v0.4.0');
SET search_path = 'color.main';

SELECT to_hex(255, 0, 0);                       -- '#ff0000'
SELECT (from_hex('#ff0000')).*;                 -- (r := 255, g := 0, b := 0)
SELECT (rgb_to_hsl(255, 0, 0)).*;               -- (h := 0, s := 1, l := 0.5)
SELECT (rgb_to_lab(255, 0, 0)).*;               -- (l := 53.24, a := 80.09, b := 67.20)
SELECT ROUND(delta_e('#ff0000', '#ff4500'), 2); -- CIEDE2000 ΔE
SELECT ROUND(contrast_ratio('#000000','#ffffff'), 1);  -- 21.0
SELECT wcag_level('#000000', '#ffffff');        -- 'AAA'
SELECT is_dark('#222222');                      -- true
SELECT nearest_color_name('#ff0000');           -- 'red'
SELECT * FROM named_colors();                   -- CSS named-color table
```

## Quick start

Every [GitHub release](https://github.com/Query-farm/vgi-color/releases) ships a prebuilt worker binary. The `vgi` extension downloads the one for your platform, checks its SHA-256 and caches it, so there is nothing to install besides DuckDB:

```sql
INSTALL vgi FROM community;
LOAD vgi;

ATTACH 'color' AS color (TYPE vgi, LOCATION 'github-auto://Query-farm/vgi-color@v0.4.0');

-- Format an RGB triple as a '#rrggbb' hex string.
SELECT color.main.to_hex(255, 99, 71);

-- Parse a hex color into its red channel.
SELECT (color.main.from_hex('#ff6347')).r AS red;
```

Prebuilt binaries cover macOS (Apple silicon), Linux (x86-64 and arm64). `github-auto://` needs an explicit release tag; change `@v0.4.0` to move to a newer release.

### Other ways to run it

- **Docker** (the image bundles the worker; needs a container runtime):

  ```sql
  ATTACH 'color' AS color (TYPE vgi, LOCATION 'docker://ghcr.io/query-farm/vgi-color:0.4.0');
  ```

- **Manual download.** Download the archive for your platform from the [releases page](https://github.com/Query-farm/vgi-color/releases), verify it against its `.sha256` file (a cosign `.cosign.bundle` is published alongside), unpack it, and set `LOCATION` to the path of the extracted worker executable.

### Build from source

To build the worker yourself, see [Development](#development).

## Color library / formulas

The worker implements every transform **directly from the standard formulas** —
no third-party color crate — so it carries no extra dependency and no MSRV risk
(it builds on the workspace `rust-version = 1.86`). The pure engine lives in
`crates/color-worker/src/color.rs`; the `scalar/` and `table/` modules are thin
Arrow adapters over it.

| Transform | Standard |
| --- | --- |
| sRGB ⇄ linear | IEC 61966-2-1 piecewise companding (`0.04045`/`12.92`, `2.4` gamma, `0.055` offset) |
| Relative luminance | WCAG 2.x: `0.2126 R + 0.7152 G + 0.0722 B` over linearized channels |
| Contrast ratio | WCAG 2.x: `(L_light + 0.05) / (L_dark + 0.05)`, `1.0 … 21.0` |
| HSL ⇄ RGB | Standard hue-sector conversion (`h` degrees, `s`/`l` in `0..1`) |
| CIELAB | linear-sRGB → CIE XYZ (sRGB/D65 matrix) → L\*a\*b\* with the D65 white |
| CIEDE2000 (ΔE₀₀) | Sharma, Wu & Dalal (2005) reference implementation |

The CSS Color Module Level 4 named-color table (147 names) backs
`nearest_color_name` (closest name by ΔE₀₀) and the `named_colors()` table.

> A `palette`-crate-based implementation was considered; implementing the formulas
> directly was chosen for zero added dependencies and full MSRV control. `palette`
> is BSD/MIT/Apache; this worker uses none of it.

## Function surface

Scalars (positional-only):

| Function | Signature | Notes |
| --- | --- | --- |
| `to_hex` | `to_hex(r INT, g INT, b INT) -> VARCHAR` | `'#rrggbb'`; channels clamped to 0-255 |
| `from_hex` | `from_hex(hex VARCHAR) -> STRUCT(r INT, g INT, b INT)` | Accepts `#rgb`/`#rrggbb`/`#rrggbbaa`; NULL if invalid |
| `rgb_to_hsl` | `rgb_to_hsl(r, g, b) -> STRUCT(h DOUBLE, s DOUBLE, l DOUBLE)` | `h` in `[0,360)`, `s`/`l` in `[0,1]` |
| `hsl_to_rgb` | `hsl_to_rgb(h, s, l) -> STRUCT(r INT, g INT, b INT)` | `h` degrees, `s`/`l` clamped to `0..1` |
| `rgb_to_lab` | `rgb_to_lab(r, g, b) -> STRUCT(l DOUBLE, a DOUBLE, b DOUBLE)` | CIELAB, D65 |
| `delta_e` | `delta_e(hex_a, hex_b) -> DOUBLE` | CIEDE2000 ΔE₀₀ |
| `luminance` | `luminance(hex) -> DOUBLE` | WCAG relative luminance, `0..1` |
| `contrast_ratio` | `contrast_ratio(hex_a, hex_b) -> DOUBLE` | WCAG, `1..21` |
| `wcag_level` | `wcag_level(hex_fg, hex_bg) -> VARCHAR` | `'AAA'`/`'AA'`/`'AA Large'`/`'fail'` (normal text) |
| `is_dark` | `is_dark(hex) -> BOOLEAN` | luminance `< 0.5` |
| `nearest_color_name` | `nearest_color_name(hex) -> VARCHAR` | Closest CSS named color by ΔE |

The worker's build version is published as the catalog's `implementation_version`
(readable via `vgi_catalogs()`), so there is no separate `version()` scalar.

Table function:

| Function | Columns |
| --- | --- |
| `named_colors()` | `name VARCHAR, hex VARCHAR` |

### NULL / error policy

NULL input → NULL output. Invalid hex strings and out-of-range RGB channels are
treated as data (invalid hex → NULL; channels clamped to `0..=255`). The worker
never panics.

## Development

```sh
make test       # cargo unit/integration tests + SQL E2E
make test-unit  # cargo test --workspace
make test-sql   # build release worker + DuckDB sqllogictest suite (haybarn-unittest)
make lint       # clippy (deny warnings) + rustfmt --check
make fmt        # rustfmt the workspace
```

The SQL E2E suite uses [`haybarn-unittest`](https://query.farm)
2.0 (`uv tool install --prerelease allow "haybarn-unittest>=2.0.0rc11,<2.1"`).
The worker speaks VGI protocol vgi.v3 (3.3), so it requires haybarn 2.0 and its
community `vgi` extension; haybarn 1.5 cannot attach it.

## License

MIT — see [LICENSE](LICENSE).

---

## Authorship & License

Written by [Query.Farm](https://query.farm).

Copyright 2026 Query Farm LLC - https://query.farm

