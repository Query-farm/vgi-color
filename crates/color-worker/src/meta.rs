//! Shared helpers for the per-object discovery/description metadata that the
//! `vgi-lint` strict profile expects on **every** function and table.
//!
//! Each function/table surfaces these in its `FunctionMetadata.tags`:
//! - `vgi.title` (VGI124)        — human-friendly display name
//! - `vgi.doc_llm` (VGI112)      — narrative prose aimed at LLMs/agents
//! - `vgi.doc_md` (VGI113)       — Markdown narrative for human docs
//! - `vgi.keywords` (VGI126)     — JSON array of search terms/synonyms
//!
//! Provenance (`vgi.source_url`, VGI139) lives only on the catalog object, never
//! per-function — so it is intentionally not emitted here.

/// Analyst task suite (VGI152) that `vgi-lint simulate` runs against the worker
/// to measure how well an agent can actually discover and use these functions.
/// Each task is a natural-language `prompt` plus a `reference_sql` answer keyed to
/// the `color.main` functions; the harness scores the agent's query against it.
pub const AGENT_TEST_TASKS: &str = r#"[
  {
    "name": "rgb_to_hex",
    "prompt": "I have an sRGB color given as three 0-255 channels: red 255, green 99, blue 71. Using this worker, what is its lowercase '#rrggbb' hexadecimal color code? Return a single row with one column named hex.",
    "reference_sql": "SELECT color.main.to_hex(255, 99, 71) AS hex"
  },
  {
    "name": "hex_to_rgb_channels",
    "prompt": "Using this worker, break the sRGB hex color '#ff6347' into its individual red, green, and blue channel values. Return one row with three integer columns named r, g, and b.",
    "reference_sql": "SELECT (color.main.from_hex('#ff6347')).r AS r, (color.main.from_hex('#ff6347')).g AS g, (color.main.from_hex('#ff6347')).b AS b"
  },
  {
    "name": "contrast_ratio",
    "prompt": "Using this worker, compute the WCAG contrast ratio between the foreground color '#595959' and the background color '#ffffff', rounded to two decimal places. Return a single row with one column named ratio.",
    "reference_sql": "SELECT ROUND(color.main.contrast_ratio('#595959', '#ffffff'), 2) AS ratio"
  },
  {
    "name": "wcag_conformance_level",
    "prompt": "Using this worker, classify the WCAG conformance level of text colored '#595959' on a '#ffffff' background. Return a single row with one column named level.",
    "reference_sql": "SELECT color.main.wcag_level('#595959', '#ffffff') AS level"
  },
  {
    "name": "perceptual_difference",
    "prompt": "Using this worker's CIEDE2000 metric, how perceptually different are the two sRGB colors '#ff0000' and '#ee0000'? Round the result to four decimal places and return a single row with one column named delta_e.",
    "reference_sql": "SELECT ROUND(color.main.delta_e('#ff0000', '#ee0000'), 4) AS delta_e"
  },
  {
    "name": "nearest_named_color",
    "prompt": "Using this worker, what is the closest CSS named color to the sRGB hex value '#ff6347'? Return a single row with one column named name.",
    "reference_sql": "SELECT color.main.nearest_color_name('#ff6347') AS name"
  },
  {
    "name": "list_named_colors",
    "prompt": "Using this worker, list five CSS named colors in alphabetical order together with their hex values. Return exactly five rows with two columns named name and hex.",
    "reference_sql": "SELECT name, hex FROM color.main.named_colors() ORDER BY name LIMIT 5"
  },
  {
    "name": "rgb_to_hsl_lightness",
    "prompt": "Using this worker, convert the sRGB color with red 0, green 128, blue 0 to the HSL color space and report its lightness component rounded to three decimal places. Return a single row with one column named l.",
    "reference_sql": "SELECT ROUND((color.main.rgb_to_hsl(0, 128, 0)).l, 3) AS l"
  },
  {
    "name": "hsl_to_rgb_green_channel",
    "prompt": "Using this worker, convert the HSL color with hue 120 degrees, saturation 1.0 and lightness 0.5 into an sRGB triple and report its green channel. Return a single row with one integer column named g.",
    "reference_sql": "SELECT (color.main.hsl_to_rgb(120, 1.0, 0.5)).g AS g"
  },
  {
    "name": "rgb_to_lab_lightness",
    "prompt": "Using this worker, convert white (red 255, green 255, blue 255) to the CIELAB color space and report its L* lightness rounded to the nearest whole number. Return a single row with one column named l.",
    "reference_sql": "SELECT ROUND((color.main.rgb_to_lab(255, 255, 255)).l) AS l"
  },
  {
    "name": "luminance_threshold",
    "prompt": "Using this worker, is the WCAG relative luminance of the color '#808080' greater than 0.2? Return a single row with one boolean column named is_bright.",
    "reference_sql": "SELECT color.main.luminance('#808080') > 0.2 AS is_bright"
  },
  {
    "name": "is_color_dark",
    "prompt": "Using this worker, determine whether the color '#001f3f' is dark, so that light-colored text should be placed on top of it. Return a single row with one boolean column named dark.",
    "reference_sql": "SELECT color.main.is_dark('#001f3f') AS dark"
  },
  {
    "name": "worker_version",
    "prompt": "Using this worker, report the version string of the running color worker. Return a single row with one column named version.",
    "reference_sql": "SELECT color.main.color_version() AS version"
  }
]"#;

/// Encode a list of keyword/synonym strings as a JSON array literal (VGI138),
/// e.g. `["color","rgb to hex"]`. Each entry is escaped for `"` and `\`.
pub fn keywords_json(keywords: &[&str]) -> String {
    let mut out = String::from("[");
    for (i, kw) in keywords.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        for ch in kw.chars() {
            match ch {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                _ => out.push(ch),
            }
        }
        out.push('"');
    }
    out.push(']');
    out
}

/// Build the standard per-object discovery/description tags.
///
/// `keywords` is a slice of search terms/synonyms, serialized to a JSON array
/// (VGI138). `category` names one of the schema's `vgi.categories` (VGI413) so
/// the object appears under the right navigation/listing section. Provenance is
/// not emitted per-object (VGI139) — only the catalog carries `source_url`.
pub fn object_tags(
    title: &str,
    description_llm: &str,
    description_md: &str,
    keywords: &[&str],
    category: &str,
) -> Vec<(String, String)> {
    vec![
        ("vgi.title".to_string(), title.to_string()),
        ("vgi.doc_llm".to_string(), description_llm.to_string()),
        ("vgi.doc_md".to_string(), description_md.to_string()),
        ("vgi.keywords".to_string(), keywords_json(keywords)),
        ("vgi.category".to_string(), category.to_string()),
    ]
}
