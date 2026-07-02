//! Table functions exposed by the color worker, registered under `color.main`.

mod named;

use std::sync::Arc;

use vgi::catalog::CatTable;

/// Build the catalog `CatTable` that exposes `named_colors` as a regular table
/// (`SELECT * FROM color.main.named_colors`, no parentheses — VGI311), backed by
/// the [`named::NamedColors`] scan function.
///
/// `CatTable::with_function` stores the function instance, so
/// [`vgi::Worker::set_catalog`] auto-registers it into the dispatch table; no
/// separate `register_table` call is needed. The table carries the same
/// discovery tags, example queries, column comments, and a primary key as any
/// well-documented catalog object so it lints clean on its own.
pub fn named_colors_table() -> CatTable {
    let mut t = CatTable::with_function(
        "named_colors",
        named::output_schema(),
        Arc::new(named::NamedColors),
        Some("Every CSS named color paired with its '#rrggbb' sRGB hex value.".to_string()),
        Some(crate::color::NAMED_COLORS.len() as i64),
    );
    // `name` (column 0) is the unique row identity — declare it the primary key
    // (VGI807) and the table's NOT NULL/unique constraints (VGI806).
    t.primary_key = vec![vec![0]];
    t.not_null = vec![0, 1];
    t.unique = vec![vec![0]];
    t.tags =
        vec![
        ("vgi.title".to_string(), "CSS Named Colors Catalog".to_string()),
        (
            "vgi.doc_llm".to_string(),
            "Every CSS named color the worker knows, each paired with its lowercase '#rrggbb' \
             sRGB hex value. Query it to discover valid CSS color names, to map a name to its \
             hex value, or as the reference table behind nearest_color_name. One row per color."
                .to_string(),
        ),
        (
            "vgi.doc_md".to_string(),
            "# named_colors\n\nThe discovery table of every CSS Color Module Level 4 named color. \
             One row per color, with columns `name` (the color name, e.g. `tomato`) and `hex` \
             (its lowercase `#rrggbb` sRGB value). Use it to list valid color names, look up a \
             name's hex value, or browse the palette that backs `nearest_color_name`."
                .to_string(),
        ),
        (
            "vgi.keywords".to_string(),
            crate::meta::keywords_json(&[
                "named_colors",
                "CSS colors",
                "color names",
                "named color table",
                "color catalog",
                "discovery",
                "list colors",
                "hex lookup",
            ]),
        ),
        ("domain".to_string(), "color-science".to_string()),
        ("category".to_string(), "discovery".to_string()),
        ("topic".to_string(), "named-colors".to_string()),
        // VGI411/VGI413: assign this table to one of the schema's `vgi.categories`.
        ("vgi.category".to_string(), "Reference".to_string()),
        (
            "vgi.example_queries".to_string(),
            r#"[
  {
    "description": "List the first few CSS named colors with their hex values.",
    "sql": "SELECT name, hex FROM color.main.named_colors ORDER BY name LIMIT 5"
  },
  {
    "description": "Look up the hex value of a specific CSS named color.",
    "sql": "SELECT hex FROM color.main.named_colors WHERE name = 'tomato'"
  }
]"#
            .to_string(),
        ),
    ];
    t
}
