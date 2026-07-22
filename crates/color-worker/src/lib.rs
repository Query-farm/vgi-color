//! The `color` VGI worker (library).
//!
//! Function registration and catalog metadata live here so both entrypoints
//! share them verbatim: `main.rs` (the native binary, stdio/HTTP transport) and
//! the `color-wasm` crate (the browser build, which serves the same `Worker`
//! over a SharedArrayBuffer byte channel instead).
//!
//! A standalone binary that DuckDB launches and talks to over Apache Arrow IPC
//! (`ATTACH 'color' (TYPE vgi, LOCATION '…')`). It brings color science —
//! color-space conversions, CIEDE2000 color difference and WCAG contrast — to SQL
//! under the catalog `color`, schema `main`:
//!
//! ```sql
//! ATTACH 'color' (TYPE vgi, LOCATION './target/release/color-worker');
//! SET search_path = 'color.main';
//!
//! SELECT to_hex(255, 0, 0);                       -- '#ff0000'
//! SELECT (from_hex('#ff0000')).*;                 -- (r := 255, g := 0, b := 0)
//! SELECT (rgb_to_hsl(255, 0, 0)).*;               -- (h := 0, s := 1, l := 0.5)
//! SELECT ROUND(contrast_ratio('#000000','#ffffff'), 1);  -- 21.0
//! SELECT wcag_level('#000000', '#ffffff');        -- 'AAA'
//! SELECT nearest_color_name('#ff0000');           -- 'red'
//! SELECT * FROM named_colors();                   -- CSS named-color table
//! ```
//!
//! The pure color-science engine lives in [`color`]; the `scalar/` and `table/`
//! modules are thin Arrow adapters over it.

mod arrow_io;
pub mod color;
mod meta;
mod scalar;
mod table;

use vgi::catalog::{CatSchema, CatalogModel};
use vgi::Worker;

/// Worker version string, published as the catalog's `implementation_version`.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Catalog + schema metadata (description, provenance) surfaced to DuckDB and
/// the `vgi-lint` metadata-quality linter. The function objects themselves are
/// served from the registered scalars/table; this only adds catalog/schema-level
/// comments and tags.
fn catalog_metadata(name: &str) -> CatalogModel {
    CatalogModel {
        name: name.to_string(),
        comment: Some(
            "Color science for SQL: color-space conversions, CIEDE2000 (ΔE00) color difference, \
             and WCAG contrast/accessibility."
                .to_string(),
        ),
        tags: vec![
            (
                "vgi.title".to_string(),
                "Color Science & Accessibility".to_string(),
            ),
            (
                "vgi.keywords".to_string(),
                meta::keywords_json(&[
                    "color",
                    "colour",
                    "color science",
                    "color space",
                    "RGB",
                    "HSL",
                    "hex",
                    "CIELAB",
                    "Lab",
                    "CIEDE2000",
                    "delta E",
                    "color difference",
                    "WCAG",
                    "contrast",
                    "contrast ratio",
                    "luminance",
                    "accessibility",
                    "named colors",
                    "palette",
                ]),
            ),
            (
                "vgi.doc_llm".to_string(),
                "Color-science functions over Apache Arrow. Convert colors between sRGB hex, RGB, \
                 HSL and CIELAB; measure perceptual color difference with CIEDE2000 (ΔE00); \
                 compute WCAG relative luminance and contrast ratios and classify WCAG \
                 conformance ('AAA'/'AA'/'AA Large'/'fail'); test whether a color is dark; find \
                 the nearest CSS named color; and list the CSS named-color table. Use for color \
                 conversion, accessibility/contrast checks, and palette analysis in SQL."
                    .to_string(),
            ),
            (
                "vgi.doc_md".to_string(),
                "# Color Science for SQL\n\n\
                 Bring professional color science to DuckDB — convert between color spaces, \
                 measure perceptual color difference, and check accessibility contrast directly \
                 in SQL, with no application code required.\n\n\
                 The `color` extension is a [VGI](https://query.farm) worker that adds sRGB hex, \
                 RGB, HSL and CIELAB color tools to DuckDB over Apache Arrow. It is built for \
                 designers, data engineers, and accessibility teams who need to normalize colors, \
                 build and de-duplicate palettes, compute \
                 [CIEDE2000](https://en.wikipedia.org/wiki/Color_difference#CIEDE2000) (ΔE00) \
                 color differences, and audit \
                 [WCAG](https://www.w3.org/TR/WCAG21/#contrast-minimum) contrast at warehouse \
                 scale.\n\n\
                 ## How it works\n\n\
                 Every transform is implemented directly from the canonical specifications — \
                 [sRGB / IEC 61966-2-1](https://en.wikipedia.org/wiki/SRGB) companding, the \
                 [CIELAB (D65)](https://en.wikipedia.org/wiki/CIELAB_color_space) color space, \
                 the Sharma–Wu–Dalal \
                 [CIEDE2000](https://en.wikipedia.org/wiki/Color_difference#CIEDE2000) ΔE formula, \
                 and the \
                 [WCAG 2.x relative-luminance and contrast](https://www.w3.org/WAI/WCAG21/Understanding/contrast-minimum.html) \
                 definitions — so results match the standards exactly with zero external color \
                 dependencies. The same algorithms are available in open-source libraries such as \
                 the Rust [palette](https://github.com/Ogeon/palette) crate \
                 ([docs](https://docs.rs/palette)); here they run inside a fast, dependency-free \
                 worker. The 147 CSS named colors come from the \
                 [CSS Color Module Level 4](https://www.w3.org/TR/css-color-4/#named-colors) \
                 specification.\n\n\
                 ## Key concepts\n\n\
                 Colors are represented as sRGB hex strings, integer RGB triples, HSL, or \
                 perceptually-uniform CIELAB. Perceptual similarity is measured with CIEDE2000 \
                 (ΔE00), where a value near 1 is roughly a just-noticeable difference. \
                 Accessibility is expressed as WCAG relative luminance (0–1) and contrast ratio \
                 (1–21), which classify into the WCAG conformance levels AAA, AA, AA Large and \
                 fail.\n\n\
                 ## When to use it\n\n\
                 Reach for this worker whenever colors live in your data: normalizing \
                 user-entered or imported hex values, building and de-duplicating palettes, \
                 labeling arbitrary colors with their nearest name, or auditing \
                 foreground/background pairs for accessibility. Its functions are organized into \
                 four categories — conversion, color difference, accessibility, and reference."
                    .to_string(),
            ),
            (
                "vgi.agent_test_tasks".to_string(),
                meta::AGENT_TEST_TASKS.to_string(),
            ),
            ("vgi.author".to_string(), "Query.Farm".to_string()),
            (
                "vgi.copyright".to_string(),
                "Copyright 2026 Query Farm LLC - https://query.farm".to_string(),
            ),
            ("vgi.license".to_string(), "MIT".to_string()),
            (
                "vgi.support_contact".to_string(),
                "https://github.com/Query-farm/vgi-color/issues".to_string(),
            ),
            (
                "vgi.support_policy_url".to_string(),
                "https://github.com/Query-farm/vgi-color/blob/main/README.md".to_string(),
            ),
        ],
        source_url: Some("https://github.com/Query-farm/vgi-color".to_string()),
        // The running binary's semantic version. An agent reads this straight from
        // `vgi_catalogs()` (VGI328), so no parameterless `version()` scalar is
        // needed — and it can never drift from the build that emitted it.
        implementation_version: Some(version().to_string()),
        schemas: vec![CatSchema {
            name: "main".to_string(),
            comment: Some(
                "Color-space conversion, color-difference (CIEDE2000) and WCAG contrast functions."
                    .to_string(),
            ),
            tags: vec![
                ("vgi.title".to_string(), "Color — main".to_string()),
                (
                    "vgi.keywords".to_string(),
                    meta::keywords_json(&[
                        "color",
                        "colour",
                        "to_hex",
                        "from_hex",
                        "rgb_to_hsl",
                        "hsl_to_rgb",
                        "rgb_to_lab",
                        "delta_e",
                        "luminance",
                        "contrast_ratio",
                        "wcag_level",
                        "is_dark",
                        "nearest_color_name",
                        "named_colors",
                        "color space",
                        "CIELAB",
                        "CIEDE2000",
                        "WCAG",
                        "contrast",
                        "accessibility",
                    ]),
                ),
                // VGI123 classifying tags (bare keys: domain/category/topic) for faceting.
                ("domain".to_string(), "color-science".to_string()),
                ("category".to_string(), "conversion".to_string()),
                (
                    "topic".to_string(),
                    "color-spaces-and-accessibility".to_string(),
                ),
                (
                    "vgi.doc_llm".to_string(),
                    "Color-science functions: convert between sRGB hex, RGB, HSL and CIELAB; \
                     measure CIEDE2000 (ΔE00) color difference; compute WCAG relative luminance \
                     and contrast ratio and classify WCAG conformance; test whether a color is \
                     dark; and look up the nearest CSS named color."
                        .to_string(),
                ),
                (
                    "vgi.doc_md".to_string(),
                    "# color.main\n\n\
                     Color science over Apache Arrow. Everything the `color` worker exposes \
                     lives in this single schema.\n\n\
                     Three kinds of operation are available. **Conversions** move a color \
                     between representations — sRGB hex, RGB, HSL and CIELAB. \
                     **Color-difference** operations measure how far apart two colors look, \
                     using the perceptual CIEDE2000 (ΔE00) metric, and can map an arbitrary \
                     color to its nearest CSS name. **Accessibility** operations compute WCAG \
                     relative luminance and contrast ratios and classify conformance for text \
                     on a background.\n\n\
                     Use this schema for color normalization, palette analysis and \
                     de-duplication, and WCAG contrast/accessibility auditing directly in SQL. \
                     The functions are grouped into four categories: conversion, color \
                     difference, accessibility, and reference."
                        .to_string(),
                ),
                // VGI413 category registry: an ordered list of navigation sections. Each
                // function/table carries a matching `vgi.category` tag naming one of these.
                (
                    "vgi.categories".to_string(),
                    r#"[
  {"name": "Conversion", "description": "Convert colors between representations: sRGB hex, RGB, HSL and CIELAB."},
  {"name": "Color Difference", "description": "Measure perceptual color difference with CIEDE2000 (ΔE00) and map colors to their nearest CSS name."},
  {"name": "Accessibility", "description": "Compute WCAG relative luminance and contrast ratios and classify WCAG conformance levels."},
  {"name": "Reference", "description": "Reference data: the CSS named-color catalog."}
]"#
                        .to_string(),
                ),
                // VGI506/VGI515 representative example queries for the schema, as a
                // JSON list of {description, sql} so every example is described.
                (
                    "vgi.example_queries".to_string(),
                    r#"[
  {"description": "Format an RGB triple as a '#rrggbb' hex string.", "sql": "SELECT color.main.to_hex(255, 99, 71)"},
  {"description": "Parse a hex color into its red channel.", "sql": "SELECT (color.main.from_hex('#ff6347')).r AS red"},
  {"description": "Convert pure red to its HSL hue.", "sql": "SELECT (color.main.rgb_to_hsl(255, 0, 0)).h AS hue"},
  {"description": "Compute the WCAG contrast ratio between black and white.", "sql": "SELECT ROUND(color.main.contrast_ratio('#000000', '#ffffff'), 1) AS ratio"},
  {"description": "Classify the WCAG conformance level of dark-grey text on white.", "sql": "SELECT color.main.wcag_level('#595959', '#ffffff') AS level"},
  {"description": "Find the closest CSS named color to a hex value.", "sql": "SELECT color.main.nearest_color_name('#ff6347') AS name"},
  {"description": "List a few CSS named colors with their hex values.", "sql": "SELECT name, hex FROM color.main.named_colors() ORDER BY name LIMIT 5"}
]"#
                        .to_string(),
                ),
            ],
            views: Vec::new(),
            macros: Vec::new(),
            tables: vec![table::named_colors_table()],
        }],
        ..Default::default()
    }
}

/// The catalog name DuckDB sees in `ATTACH 'color' (TYPE vgi, …)`. Defaults to
/// `color`, but honors an explicit override so a test harness can rename it.
/// Also exports the variable so downstream SDK code observes the same default.
pub fn catalog_name() -> String {
    if std::env::var_os("VGI_WORKER_CATALOG_NAME").is_none() {
        std::env::set_var("VGI_WORKER_CATALOG_NAME", "color");
    }
    std::env::var("VGI_WORKER_CATALOG_NAME").unwrap_or_else(|_| "color".to_string())
}

/// Build a fully-registered worker: every scalar function plus the catalog
/// metadata (which carries the function-backed `named_colors` table). Callers
/// choose the transport — `run()` natively, `serve_reader_writer()` in the
/// browser.
pub fn build_worker() -> Worker {
    let name = catalog_name();
    let mut worker = Worker::new();
    scalar::register(&mut worker);
    // `named_colors` is registered as a function-backed catalog *table* inside
    // `catalog_metadata` (via `CatTable::with_function`); `set_catalog` then
    // auto-registers its scan, so no separate `table::register` is needed.
    worker.set_catalog(catalog_metadata(&name));
    worker
}
