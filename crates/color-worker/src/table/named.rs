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

pub struct NamedColors;

fn output_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new("hex", DataType::Utf8, false),
    ]))
}

impl TableFunction for NamedColors {
    fn name(&self) -> &str {
        "named_colors"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "List every CSS named color with its '#rrggbb' hex value".into(),
            tags: vec![(
                "vgi.columns_md".into(),
                "| column | type | description |\n\
                 |---|---|---|\n\
                 | `name` | VARCHAR | The CSS color name, e.g. `tomato`, `rebeccapurple`. |\n\
                 | `hex` | VARCHAR | The color's `#rrggbb` sRGB hex value. |"
                    .into(),
            )],
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
}
