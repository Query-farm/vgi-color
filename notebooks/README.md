# Keep the color, earn the contrast

| Notebook | Question | Run |
|---|---|---|
| [01 · Keep the color, earn the contrast](01_keep_the_color_earn_the_contrast.ipynb) | How much must a brand background change before its fixed label reaches a contrast target—and could changing the label be cheaper? | [Open in Colab](https://colab.research.google.com/github/Query-farm/vgi-color/blob/main/notebooks/01_keep_the_color_earn_the_contrast.ipynb) |

The visible `github-auto://Query-farm/vgi-color@v0.5.0` ATTACH downloads and checks
the native release for the current platform. No Rust build, worker checkout,
account or external color measurement is required. Inputs are a **synthetic design
brief**, the real worker's CSS named-color catalog and a generated HSL grid.
Colors are opaque six-digit sRGB, with no gradients, alpha or display calibration.

In Colab choose **Runtime → Run all**. The visible bootstrap installs the base
shared package and retrieves its immutable hash-verified companion. Compatible
preinstalled kernel and plotting libraries remain installed. The `[runner]` extra
belongs only to the local locked publication environment:

```sh
uv sync --locked
uv run --locked vgi-notebooks
uv run --locked vgi-notebooks 01_keep_the_color_earn_the_contrast.ipynb --parameters '{"MIN_CONTRAST":7.0}' --output-dir artifacts/contrast-7
```

For an interactive local Jupyter environment select `notebooks/.venv/bin/python`.
The runner creates its own temporary kernel registration. Overrides require a
separate destination; a failed execution leaves the saved default untouched.
If an older Colab install replaced kernel libraries, save your work, choose
**Runtime → Disconnect and delete runtime**, and open this current notebook again.

The [shared standard](https://github.com/Query-farm/vgi-example-notebooks/blob/v0.3.6/docs/standard.md),
runner, canonical branding, isolated SQL-secret directory and SVG presentation live in
[vgi-example-notebooks](https://github.com/Query-farm/vgi-example-notebooks).
`color_example.py` validates parameters and presents returned results. All worker
measurements, transformations, candidate selection and relational checks stay in
visible SQL. Commit the generated `charts/` SVG files with the notebook for GitHub
rendering; Colab and Jupyter reruns emit native SVG. PNGs under `artifacts/review/`
are optional review exports, not the default notebook chart format.

The story meaningfully exercises **10 of 12 public named SQL surfaces**; the current
threshold is six. `coverage.json` binds actual release/current-source catalog
exports, execution receipts, SQL mappings and editorial review. The uncovered
`rgb_to_lab` and `nearest_color_name` remain explicit gaps. The release and current
source trees and public signatures match at review time; the reader runs **v0.5.0**.
Current-interface compatibility does not claim notebook execution of current-main code.

The default starts at `#ff6347` with white text and a **4.5:1** ordinary-text target.
It first separates label/background CIEDE2000 distance from contrast. CSS aliases
are grouped by hex so duplicate spellings do not receive extra weight. The shade
search then measures distance from the **original background**, holds its HSL hue
and saturation fixed and varies lightness from 0 through 1 in 0.01 steps. The original
is also a candidate. Eight-bit rounding can merge colors. This finite search is
not a continuous or unrestricted color-space optimum.

Selection uses full unrounded ratios, minimizing brand ΔE₀₀ among eligible colors,
then breaking ties by higher contrast and hexadecimal order. `MIN_CONTRAST` accepts
finite ratios from 1 through 21. `BRAND_HEX` and `LABEL_HEX` accept exactly `#RRGGBB`.
An empty eligible set stays absent; no failing or missing result becomes a success.
Nine relationship checks cover bounds, symmetry, luminance/contrast agreement,
unique candidates, selected-set coverage and minimum eligible distance.

The focused experiment changes only **`MIN_CONTRAST` from 4.5 to 7.0**. Candidate
colors and measurements must stay identical, the eligible set can only shrink,
and the closest eligible background cannot have a smaller brand distance. Compare
that change with keeping the background and choosing black/white text.

The [W3C minimum-contrast explanation](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html)
and [enhanced-contrast explanation](https://www.w3.org/WAI/WCAG22/Understanding/contrast-enhanced.html)
explain the text thresholds. Passing one ratio does not certify an interface's
accessibility or full WCAG conformance. CIEDE2000 uses the worker's D65 Lab model;
its [implementation reference](https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/)
does not justify a universal just-noticeable-difference cutoff here. Typography,
focus states, color-only meaning, color-vision deficiencies and physical display
conditions are outside this example.

Default, a detached one-parameter experiment, the empty-selection case and an
already-imported Colab-compatible kernel are checked separately. A Colab link is
not a claim of manually operating a hosted Colab session. Source checks from
`CLAUDE.md` remain required; notebook dependencies and outputs do not change the
worker source or root Cargo lock.
