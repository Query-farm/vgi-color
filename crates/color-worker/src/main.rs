//! The `color` VGI worker.
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
//! The pure color-science engine lives in `color.rs`; the `scalar/` and `table/`
//! modules are thin Arrow adapters over it.

mod arrow_io;
mod color;
mod meta;
mod scalar;
mod table;

use vgi::catalog::{CatSchema, CatalogModel};
use vgi::Worker;

/// Worker version string, surfaced by `color_version()`.
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
                 ## Functions and use cases\n\n\
                 Conversion scalars move colors between representations: `to_hex`, `from_hex`, \
                 `rgb_to_hsl`, `hsl_to_rgb`, and `rgb_to_lab`. Color-difference functions \
                 `delta_e` (CIEDE2000 ΔE00) and `nearest_color_name` power palette de-duplication \
                 and mapping arbitrary colors to the closest CSS name. Accessibility functions \
                 `luminance`, `contrast_ratio`, `wcag_level` (AAA / AA / AA Large / fail) and \
                 `is_dark` let you audit foreground/background pairs for WCAG conformance. The \
                 `named_colors` table function lists every CSS named color with its hex value, and \
                 `color_version` reports the worker version. Typical queries: \
                 `SELECT to_hex(255, 99, 71)`, \
                 `SELECT ROUND(contrast_ratio('#000000', '#ffffff'), 1)`, \
                 `SELECT wcag_level('#595959', '#ffffff')`, and \
                 `SELECT * FROM named_colors()`."
                    .to_string(),
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
                    "# color.main\n\nColor-science functions over Apache Arrow. **Conversions** \
                     move colors between sRGB hex, RGB, HSL and CIELAB (`to_hex`, `from_hex`, \
                     `rgb_to_hsl`, `hsl_to_rgb`, `rgb_to_lab`). **Color difference** measures \
                     perceptual distance with CIEDE2000 (`delta_e`) and maps a color to its \
                     nearest CSS name (`nearest_color_name`). **Accessibility** computes WCAG \
                     relative luminance and contrast ratio and classifies conformance \
                     (`luminance`, `contrast_ratio`, `wcag_level`, `is_dark`). The \
                     `named_colors` table lists every CSS named color with its hex value. Use \
                     this schema for color conversion, palette analysis, and WCAG \
                     contrast/accessibility checks directly in SQL."
                        .to_string(),
                ),
                // VGI506 representative example queries for the schema.
                (
                    "vgi.example_queries".to_string(),
                    "SELECT color.main.to_hex(255, 99, 71);\n\
                     SELECT (color.main.from_hex('#ff6347')).r AS red;\n\
                     SELECT (color.main.rgb_to_hsl(255, 0, 0)).h AS hue;\n\
                     SELECT ROUND(color.main.contrast_ratio('#000000', '#ffffff'), 1);\n\
                     SELECT color.main.wcag_level('#595959', '#ffffff');\n\
                     SELECT color.main.nearest_color_name('#ff6347');\n\
                     SELECT * FROM color.main.named_colors() ORDER BY name LIMIT 5;"
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

fn main() {
    // Logs MUST go to stderr — stdout is the Arrow-IPC channel.
    let _ = env_logger::Builder::from_env(env_logger::Env::default().filter_or("VGI_LOG", "info"))
        .format_timestamp_millis()
        .try_init();

    // The catalog name DuckDB sees in `ATTACH 'color' (TYPE vgi, …)`. Default to
    // `color`, but honor an explicit override so a test harness can rename it.
    if std::env::var_os("VGI_WORKER_CATALOG_NAME").is_none() {
        std::env::set_var("VGI_WORKER_CATALOG_NAME", "color");
    }
    let catalog_name =
        std::env::var("VGI_WORKER_CATALOG_NAME").unwrap_or_else(|_| "color".to_string());

    let mut worker = Worker::new();
    scalar::register(&mut worker);
    // `named_colors` is registered as a function-backed catalog *table* inside
    // `catalog_metadata` (via `CatTable::with_function`); `set_catalog` then
    // auto-registers its scan, so no separate `table::register` is needed.
    worker.set_catalog(catalog_metadata(&catalog_name));
    worker.run();
}
