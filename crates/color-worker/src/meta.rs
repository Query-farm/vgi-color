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
/// Each task is a natural-language `prompt` keyed to the `color.main` functions.
/// The private grader (`reference_sql`) lives in `vgi-agent-tests.yaml` at the repo
/// root, keyed by task name, so the agent being graded never sees it (VGI416).
pub const AGENT_TEST_TASKS: &str = r#"[
  {
    "name": "rgb_to_hex",
    "prompt": "I have an sRGB color given as three 0-255 channels: red 255, green 99, blue 71. Using this worker, what is its lowercase '#rrggbb' hexadecimal color code? Return a single row with one column named hex."
  },
  {
    "name": "hex_to_rgb_channels",
    "prompt": "Using this worker, break the sRGB hex color '#ff6347' into its individual red, green, and blue channel values. Return one row with three integer columns named r, g, and b."
  },
  {
    "name": "contrast_ratio",
    "prompt": "Using this worker, compute the WCAG contrast ratio between the foreground color '#595959' and the background color '#ffffff', rounded to two decimal places. Return a single row with one column named ratio."
  },
  {
    "name": "wcag_conformance_level",
    "prompt": "Using this worker, classify the WCAG conformance level of text colored '#595959' on a '#ffffff' background. Return a single row with one column named level."
  },
  {
    "name": "perceptual_difference",
    "prompt": "Using this worker's CIEDE2000 metric, how perceptually different are the two sRGB colors '#ff0000' and '#ee0000'? Round the result to four decimal places and return a single row with one column named delta_e."
  },
  {
    "name": "nearest_named_color",
    "prompt": "Using this worker, what is the closest CSS named color to the sRGB hex value '#ff6347'? Return a single row with one column named name."
  },
  {
    "name": "list_named_colors",
    "prompt": "Using this worker, list five CSS named colors in alphabetical order together with their hex values. Return exactly five rows with two columns named name and hex."
  },
  {
    "name": "rgb_to_hsl_lightness",
    "prompt": "Using this worker, convert the sRGB color with red 0, green 128, blue 0 to the HSL color space and report its lightness component rounded to three decimal places. Return a single row with one column named l."
  },
  {
    "name": "hsl_to_rgb_green_channel",
    "prompt": "Using this worker, convert the HSL color with hue 120 degrees, saturation 1.0 and lightness 0.5 into an sRGB triple and report its green channel. Return a single row with one integer column named g."
  },
  {
    "name": "rgb_to_lab_lightness",
    "prompt": "Using this worker, convert white (red 255, green 255, blue 255) to the CIELAB color space and report its L* lightness rounded to the nearest whole number. Return a single row with one column named l."
  },
  {
    "name": "luminance_threshold",
    "prompt": "Using this worker, is the WCAG relative luminance of the color '#808080' greater than 0.2? Return a single row with one boolean column named is_bright."
  },
  {
    "name": "is_color_dark",
    "prompt": "Using this worker, determine whether the color '#001f3f' is dark, so that light-colored text should be placed on top of it. Return a single row with one boolean column named dark."
  },
  {
    "name": "accessible_shade_on_white",
    "prompt": "Using this worker, find a darker version of the brand color '#1e90ff' that stays legible under white text. Keep its HSL hue and saturation unchanged and try the HSL lightness values k/100 for every whole number k from 0 to 100, converting each back to an sRGB '#rrggbb' color. Which of those colors is the lightest one on which white '#ffffff' text still reaches at least WCAG AA for normal text (a contrast ratio of 4.5 or more)? Return a single row with one VARCHAR column named hex holding the lowercase '#rrggbb' value."
  },
  {
    "name": "named_colors_readable_on_dark_mode",
    "prompt": "Using this worker's CSS named-color table, how many named colors are legible as normal-size text on the dark-mode background '#121212', meaning they reach WCAG level AA or AAA? Count every name in the table, including alternate spellings such as gray and grey. Return a single row with one integer column named n."
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
        json_string(&mut out, kw);
    }
    out.push(']');
    out
}

/// Append `s` to `out` as a JSON-escaped double-quoted string literal.
fn json_string(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Serialize a function's usage examples into the `vgi.example_queries` tag shape
/// (VGI515): a JSON list of `{"description": ..., "sql": ...}` objects.
///
/// Every function ALSO carries the same examples in [`FunctionMetadata::examples`]
/// (the native `duckdb_functions().examples` column), but that column is a bare
/// `VARCHAR[]` of SQL strings and drops the per-example description. The linter
/// merges the two carriers by SQL, so emitting the identical `sql` here lets the
/// described tag entry win — giving every example a human-readable description
/// without duplicating the query text.
pub fn example_queries_json(examples: &[vgi::FunctionExample]) -> String {
    let mut out = String::from("[");
    for (i, ex) in examples.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"description\":");
        json_string(&mut out, &ex.description);
        out.push_str(",\"sql\":");
        json_string(&mut out, &ex.sql);
        out.push('}');
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
