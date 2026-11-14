# vgi-color notebooks

<a href="https://query.farm/"><img src="assets/query-farm-strata-sun.svg" alt="Query.Farm — Strata Sun" width="252"></a>

<a href="https://query.farm/vgi/"><img src="assets/vgi-logo.png" alt="VGI — Vector Gateway Interface" width="280"></a>

[Query.Farm](https://query.farm/) · [VGI](https://query.farm/vgi/) · [Haybarn](https://query.farm/products/haybarn/)

| Notebook | Investigation |
|---|---|
| [01 · Keep the color, earn the contrast](01_keep_the_color_earn_the_contrast.ipynb) | Separate text contrast from color difference, search a controlled shade family, and compare changing a background with changing its label. |

The saved notebook includes executed SQL tables, interpretations and three embedded
charts. Haybarn connects to the published Rust worker running locally through visible JupySQL
`INSTALL`, `LOAD` and `ATTACH` statements. SQL performs the color conversions,
measurements, filtering and candidate selection; folded Python handles setup,
charts, result-derived prose and validation assertions.

Inputs are a **synthetic design brief**, the worker's CSS named-color catalog,
and a generated HSL lightness grid. Everything is local and deterministic, with
no live API, replay mode, acquisition timestamp or observation window. Colors
are opaque six-digit sRGB. Contrast checks do not certify whole-interface
accessibility, and CIEDE2000 distance does not guarantee distinguishability for
every viewer.

## Install and run

Prerequisite: `uv` on a release-supported native platform. No worker checkout or
Rust toolchain is required. From this `notebooks/` directory:

```sh
uv sync --locked
uv run --locked python execute.py
```

The separate notebook environment locks Haybarn **2.0.0rc11**, matching the
verified `haybarn-unittest` version, and supports Python 3.13 or 3.14 (`uv` can
provision Python). The pinned v0.5.0 release uses VGI Rust SDK **0.51.0**.
The notebook keeps the Rust SDK, runtime dependencies and root `Cargo.lock`
unchanged. The initial environment, VGI extension and worker release installation require
network access. The literal `github-auto://Query-farm/vgi-color@v0.5.0` SQL location
selects a platform release and checks its published SHA-256 sidecar. Analysis uses
local inputs. This pin was verified through Haybarn 2.0.0rc11 on macOS arm64;
release assets also exist for Linux amd64/arm64 and Windows amd64. To upgrade,
change the literal release tag, verify its platform assets, and rerun both targets.

In VS Code with the Python and Jupyter extensions, open the notebook and choose
**Select Kernel → Python Environments → notebooks/.venv/bin/python**. Use the
interpreter path picker if necessary. Start the kernel in this notebook directory and select **Run All**. No globally registered kernel is needed.
Python setup, plot source, interpretation source and cleanup carry portable
Jupyter folding metadata and tags. If the editor retains its own folding state,
collapse those inputs while keeping parameters and SQL expanded.

## One-parameter experiment

Change only `MIN_CONTRAST` from `4.5` to `7.0`. Predict a smaller eligible set and
a nondecreasing minimum background difference; the candidate colors themselves
must not change. Save the experiment without replacing the default:

```sh
uv run --locked python execute.py \
  --parameters '{"MIN_CONTRAST":7.0}' \
  --output-dir artifacts/contrast-7
```

`BRAND_HEX` and `LABEL_HEX` accept exactly `#RRGGBB` opaque sRGB values.
`MIN_CONTRAST` is a finite unitless ratio between 1 and 21 inclusive. Parameters
are validated before connecting to the worker. An unattainable target leaves the
selected candidate absent and keeps the other comparisons available.

The runner starts a fresh temporary kernel using the current interpreter,
requires a distinct output directory for overrides, rejects unknown parameter
names, and atomically replaces an output only after successful execution.
Failed cells preserve the last successful notebook. Experiment directories
include local branding, this guide and environment/helper companions for review.
Copied experiment directories contain everything needed to execute: enter that
directory, run `uv sync --locked`, then `uv run --locked python execute.py`. Run the default command again
without overrides to regenerate the saved example.

## What the search establishes

The initial background is `#ff6347` with a `#ffffff` label. The first chart scores
each distinct CSS background against the fixed label, grouping synonymous hex
colors. Its distance reference is the **label**. The shade search then holds the
brand's HSL hue and saturation constant, samples lightness from 0 through 1 in
0.01 steps, and includes the original background. Its distance reference is the
**original background**. All conversions and CIEDE2000 measurements come from
the worker; eligibility comparisons use unrounded contrast ratios.

Selection minimizes background ΔE₀₀ among eligible sampled colors, breaking ties
by higher contrast and then hex order. This is a finite search, not a continuous
or unrestricted color-space optimum. Altering hue/saturation or grid spacing can
change the answer. The final cards compare the original, the selected background
if present, and black/white text on the original background.

The notebook links W3C's [minimum contrast explanation](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html)
and [enhanced contrast explanation](https://www.w3.org/WAI/WCAG22/Understanding/contrast-enhanced.html),
and the authors' [CIEDE2000 implementation notes](https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/).
It does not apply a universal just-noticeable-difference cutoff. Alpha compositing,
gradients, display calibration, typography and color-vision simulation are outside
this example. Worker formulas are in `crates/color-worker/src/color.rs` in the
original checkout.

Relationship assertions cover metric bounds, unique candidates, ΔE symmetry,
agreement of luminance and contrast, missing-result handling, selected-set
coverage and the minimum-distance condition. No assertion requires a particular
winning color or count. The empty-eligible-set case remains supported.

Repository checks from `CLAUDE.md` remain:

```sh
cargo test --locked --workspace --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
make test-sql
uvx --refresh --prerelease allow --from 'vgi-lint-check>=2.5.1' \
  vgi-lint lint --execute --execute-mode run --fail-on info
```

`make test-sql` requires `haybarn-unittest` 2.0; the verified local version is
2.0.0rc11. Notebook logs, extracted review charts, experiments, caches and the
environment are ignored. The main notebook retains its embedded outputs.

Brand assets are unchanged copies of `vgi-perps/notebooks/assets/`. The official
adaptive Strata Sun + Query.Farm wordmark originates from the website media kit's
`logo/wordmark-adaptive.svg`; VGI's logo comes from `products/vgi/`.

---

<a href="https://query.farm/"><img src="assets/query-farm-strata-sun.svg" alt="Query.Farm — Strata Sun" width="210"></a>
