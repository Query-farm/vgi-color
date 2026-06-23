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

use vgi::Worker;

/// Worker version string, surfaced by `color_version()`.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
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

    let mut worker = Worker::new();
    scalar::register(&mut worker);
    table::register(&mut worker);
    worker.run();
}
