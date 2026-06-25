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

/// Build the four standard per-object discovery/description tags.
///
/// `keywords` is a slice of search terms/synonyms, serialized to a JSON array
/// (VGI138). Provenance is not emitted per-object (VGI139) — only the catalog
/// carries `source_url`.
pub fn object_tags(
    title: &str,
    description_llm: &str,
    description_md: &str,
    keywords: &[&str],
) -> Vec<(String, String)> {
    vec![
        ("vgi.title".to_string(), title.to_string()),
        ("vgi.doc_llm".to_string(), description_llm.to_string()),
        ("vgi.doc_md".to_string(), description_md.to_string()),
        ("vgi.keywords".to_string(), keywords_json(keywords)),
    ]
}
