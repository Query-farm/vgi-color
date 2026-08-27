//! `named_colors() -> (name VARCHAR, hex VARCHAR)` — the CSS named-color table,
//! each name paired with its `#rrggbb` hex value.

use std::sync::Arc;

use arrow_array::builder::StringBuilder;
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use vgi::table_function::{TableFunction, TableProducer};
use vgi::{ArgSpec, BindParams, BindResponse, FunctionMetadata, ProcessParams};
use vgi_rpc::{OutputCollector, Result, RpcError};

use crate::color;

/// Guaranteed-runnable, catalog-qualified examples (VGI509). Each `sql` is
/// self-contained and re-runnable against an attached `color` worker. We omit
/// `expected_result` deliberately — the linter only needs each query to execute
/// cleanly, and pinning exact floating-point output would be brittle.
const EXECUTABLE_EXAMPLES: &str = r#"[
  {
    "description": "Format an RGB triple as a '#rrggbb' hex string.",
    "sql": "SELECT color.main.to_hex(255, 99, 71) AS hex"
  },
  {
    "description": "Parse a hex color into its red, green and blue channels.",
    "sql": "SELECT (color.main.from_hex('#ff6347')).r AS red, (color.main.from_hex('#ff6347')).g AS green, (color.main.from_hex('#ff6347')).b AS blue"
  },
  {
    "description": "Convert pure red into hue, saturation and lightness.",
    "sql": "SELECT (color.main.rgb_to_hsl(255, 0, 0)).h AS hue, (color.main.rgb_to_hsl(255, 0, 0)).s AS saturation, (color.main.rgb_to_hsl(255, 0, 0)).l AS lightness"
  },
  {
    "description": "Measure the WCAG contrast ratio between black and white.",
    "sql": "SELECT ROUND(color.main.contrast_ratio('#000000', '#ffffff'), 1) AS ratio"
  },
  {
    "description": "Classify the WCAG conformance level of dark-grey text on white.",
    "sql": "SELECT color.main.wcag_level('#595959', '#ffffff') AS level"
  },
  {
    "description": "Find the closest CSS named color to a hex value.",
    "sql": "SELECT color.main.nearest_color_name('#ff6347') AS name"
  },
  {
    "description": "List the first few CSS named colors with their hex values.",
    "sql": "SELECT name, hex FROM color.main.named_colors() ORDER BY name LIMIT 5"
  }
]"#;

pub struct NamedColors;

/// A non-nullable VARCHAR column field carrying a `comment` (surfaced via
/// `duckdb_columns().comment`, VGI202) so every column is documented wherever the
/// schema appears.
fn commented(name: &str, comment: &str) -> Field {
    Field::new(name, DataType::Utf8, false).with_metadata(std::collections::HashMap::from([(
        "comment".to_string(),
        comment.to_string(),
    )]))
}

/// Output schema of `named_colors`: `(name VARCHAR, hex VARCHAR)`. Shared by the
/// table-function bind and the catalog `CatTable` (so the parameterless function
/// is also exposed as a plain table, VGI311). Each column carries a comment.
pub fn output_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        commented(
            "name",
            "The CSS Color Module Level 4 color name, e.g. 'tomato' or 'rebeccapurple'.",
        ),
        commented("hex", "The color's lowercase '#rrggbb' sRGB hex value."),
    ]))
}

impl TableFunction for NamedColors {
    fn name(&self) -> &str {
        "named_colors"
    }

    fn metadata(&self) -> FunctionMetadata {
        let mut tags = crate::meta::object_tags(
            "CSS Named Colors Catalog",
            "List every CSS named color the worker knows, each paired with its '#rrggbb' sRGB hex \
             value. Use it to discover valid color names, to map names to hex, or as the lookup \
             table behind nearest_color_name.",
            "List every CSS named color with its `#rrggbb` hex value. Columns: `name`, `hex`.",
            &[
                "named_colors",
                "CSS colors",
                "color names",
                "named color table",
                "color catalog",
                "discovery",
                "list colors",
                "hex lookup",
            ],
            "Reference",
        );
        // VGI307/VGI321: the result schema is static, so declare it structurally
        // (JSON array of {name, type, description}) rather than the retired
        // free-form `vgi.result_columns_md` (VGI414).
        tags.push((
            "vgi.result_columns_schema".into(),
            r#"[
  {"name": "name", "type": "VARCHAR", "description": "The CSS Color Module Level 4 color name, e.g. 'tomato' or 'rebeccapurple'."},
  {"name": "hex", "type": "VARCHAR", "description": "The color's lowercase '#rrggbb' sRGB hex value."}
]"#
            .into(),
        ));
        tags.push(("vgi.executable_examples".into(), EXECUTABLE_EXAMPLES.into()));
        FunctionMetadata {
            description: "List every CSS named color with its '#rrggbb' hex value".into(),
            tags,
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        Vec::new()
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse {
            output_schema: output_schema(),
            opaque_data: Vec::new(),
        })
    }

    fn producer(&self, params: &ProcessParams) -> Result<Box<dyn TableProducer>> {
        Ok(Box::new(NamedProducer {
            schema: params.output_schema.clone(),
            done: false,
        }))
    }
}

struct NamedProducer {
    schema: SchemaRef,
    done: bool,
}

impl TableProducer for NamedProducer {
    fn next_batch(&mut self, _out: &mut OutputCollector) -> Result<Option<RecordBatch>> {
        if self.done {
            return Ok(None);
        }
        self.done = true;

        let mut name = StringBuilder::new();
        let mut hex = StringBuilder::new();
        for &(n, r, g, b) in color::NAMED_COLORS {
            name.append_value(n);
            hex.append_value(color::to_hex(r, g, b));
        }
        let cols: Vec<ArrayRef> = vec![Arc::new(name.finish()), Arc::new(hex.finish())];
        Ok(Some(
            RecordBatch::try_new(self.schema.clone(), cols)
                .map_err(|e| RpcError::runtime_error(e.to_string()))?,
        ))
    }

    fn resume_supported(&self) -> bool {
        false
    }
}
