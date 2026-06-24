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
                "vgi.description_llm".to_string(),
                "Color-science functions over Apache Arrow. Convert colors between sRGB hex, RGB, \
                 HSL and CIELAB; measure perceptual color difference with CIEDE2000 (ΔE00); \
                 compute WCAG relative luminance and contrast ratios and classify WCAG \
                 conformance ('AAA'/'AA'/'AA Large'/'fail'); test whether a color is dark; find \
                 the nearest CSS named color; and list the CSS named-color table. Use for color \
                 conversion, accessibility/contrast checks, and palette analysis in SQL."
                    .to_string(),
            ),
            (
                "vgi.description_md".to_string(),
                "# color\n\nColor science over Apache Arrow: color-space conversions, CIEDE2000 \
                 (ΔE00) color difference, and WCAG contrast/accessibility.\n\nScalars: `to_hex`, \
                 `from_hex`, `rgb_to_hsl`, `hsl_to_rgb`, `rgb_to_lab`, `delta_e`, `luminance`, \
                 `contrast_ratio`, `wcag_level`, `is_dark`, `nearest_color_name`, \
                 `color_version`. Table: `named_colors`."
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
                (
                    "vgi.description_llm".to_string(),
                    "Color-science functions: convert between sRGB hex, RGB, HSL and CIELAB; \
                     measure CIEDE2000 (ΔE00) color difference; compute WCAG relative luminance \
                     and contrast ratio and classify WCAG conformance; test whether a color is \
                     dark; and look up the nearest CSS named color."
                        .to_string(),
                ),
                (
                    "vgi.description_md".to_string(),
                    "Color-space conversion, CIEDE2000 color-difference and WCAG \
                     contrast/accessibility functions over Apache Arrow."
                        .to_string(),
                ),
            ],
            views: Vec::new(),
            macros: Vec::new(),
            tables: Vec::new(),
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
    table::register(&mut worker);
    worker.set_catalog(catalog_metadata(&catalog_name));
    worker.run();
}
