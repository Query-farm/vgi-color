//! Small Arrow helpers shared across the scalar functions: reading VARCHAR and
//! INT input cells, and the STRUCT output field sets that `from_hex`, `rgb_to_hsl`,
//! `hsl_to_rgb` and `rgb_to_lab` publish at bind and build at process. The
//! in-process test harness below drives a `ScalarFunction` end-to-end without the
//! RPC/IPC plumbing.

use arrow_array::cast::AsArray;
use arrow_array::types::{
    Int16Type, Int32Type, Int64Type, Int8Type, UInt16Type, UInt32Type, UInt64Type, UInt8Type,
};
use arrow_array::{Array, ArrayRef};
use arrow_schema::{DataType, Field, Fields};
use vgi_rpc::{Result, RpcError};

/// Borrow the UTF-8 text of a VARCHAR cell at `row`, or `None` if null. Errors if
/// the column isn't a string type.
pub fn text_str(col: &ArrayRef, row: usize) -> Result<Option<&str>> {
    if col.is_null(row) {
        return Ok(None);
    }
    Ok(Some(match col.data_type() {
        DataType::Utf8 => col.as_string::<i32>().value(row),
        DataType::LargeUtf8 => col.as_string::<i64>().value(row),
        other => {
            return Err(RpcError::value_error(format!(
                "expected a VARCHAR (string) argument, got {other:?}"
            )))
        }
    }))
}

/// Read element `row` of an integer column as `i32`, or `None` if null. Accepts
/// any of DuckDB's integer input widths (a literal like `to_hex(255, …)` may
/// arrive as a wider/narrower int). Errors on a non-integer column.
pub fn int_val(col: &ArrayRef, row: usize) -> Result<Option<i32>> {
    if col.is_null(row) {
        return Ok(None);
    }
    Ok(Some(match col.data_type() {
        DataType::Int32 => col.as_primitive::<Int32Type>().value(row),
        DataType::Int64 => col.as_primitive::<Int64Type>().value(row) as i32,
        DataType::Int16 => col.as_primitive::<Int16Type>().value(row) as i32,
        DataType::Int8 => col.as_primitive::<Int8Type>().value(row) as i32,
        DataType::UInt64 => col.as_primitive::<UInt64Type>().value(row) as i32,
        DataType::UInt32 => col.as_primitive::<UInt32Type>().value(row) as i32,
        DataType::UInt16 => col.as_primitive::<UInt16Type>().value(row) as i32,
        DataType::UInt8 => col.as_primitive::<UInt8Type>().value(row) as i32,
        other => {
            return Err(RpcError::value_error(format!(
                "expected an integer argument, got {other:?}"
            )))
        }
    }))
}

/// Read element `row` of a numeric column as `f64`, or `None` if null. Used by the
/// HSL builder (`h`/`s`/`l` are DOUBLE but a literal may arrive as INT).
pub fn double_val(col: &ArrayRef, row: usize) -> Result<Option<f64>> {
    use arrow_array::types::{Float32Type, Float64Type};
    if col.is_null(row) {
        return Ok(None);
    }
    Ok(Some(match col.data_type() {
        DataType::Float64 => col.as_primitive::<Float64Type>().value(row),
        DataType::Float32 => col.as_primitive::<Float32Type>().value(row) as f64,
        DataType::Int64 => col.as_primitive::<Int64Type>().value(row) as f64,
        DataType::Int32 => col.as_primitive::<Int32Type>().value(row) as f64,
        DataType::Int16 => col.as_primitive::<Int16Type>().value(row) as f64,
        DataType::Int8 => col.as_primitive::<Int8Type>().value(row) as f64,
        DataType::UInt64 => col.as_primitive::<UInt64Type>().value(row) as f64,
        DataType::UInt32 => col.as_primitive::<UInt32Type>().value(row) as f64,
        DataType::UInt16 => col.as_primitive::<UInt16Type>().value(row) as f64,
        DataType::UInt8 => col.as_primitive::<UInt8Type>().value(row) as f64,
        // DuckDB hands decimal literals (e.g. `0.5`) as Decimal128 — a scaled
        // integer. Recover the value as `raw / 10^scale`.
        DataType::Decimal128(_, scale) => {
            use arrow_array::types::Decimal128Type;
            let raw = col.as_primitive::<Decimal128Type>().value(row);
            raw as f64 / 10f64.powi(*scale as i32)
        }
        DataType::Decimal256(_, scale) => {
            use arrow_array::types::Decimal256Type;
            let raw = col.as_primitive::<Decimal256Type>().value(row);
            // i256 → f64 via its Display/string is exact enough for our use.
            raw.to_string().parse::<f64>().unwrap_or(f64::NAN) / 10f64.powi(*scale as i32)
        }
        other => {
            return Err(RpcError::value_error(format!(
                "expected a numeric (DOUBLE) argument, got {other:?}"
            )))
        }
    }))
}

/// `STRUCT(r INT, g INT, b INT)` — `from_hex` and `hsl_to_rgb`.
pub fn rgb_struct_fields() -> Fields {
    Fields::from(vec![
        Field::new("r", DataType::Int32, true),
        Field::new("g", DataType::Int32, true),
        Field::new("b", DataType::Int32, true),
    ])
}

/// `STRUCT(h DOUBLE, s DOUBLE, l DOUBLE)` — `rgb_to_hsl`.
pub fn hsl_struct_fields() -> Fields {
    Fields::from(vec![
        Field::new("h", DataType::Float64, true),
        Field::new("s", DataType::Float64, true),
        Field::new("l", DataType::Float64, true),
    ])
}

/// `STRUCT(l DOUBLE, a DOUBLE, b DOUBLE)` — `rgb_to_lab`.
pub fn lab_struct_fields() -> Fields {
    Fields::from(vec![
        Field::new("l", DataType::Float64, true),
        Field::new("a", DataType::Float64, true),
        Field::new("b", DataType::Float64, true),
    ])
}

/// Test-only helpers shared by the scalar Arrow-boundary unit tests. These let a
/// `#[cfg(test)]` block drive a `ScalarFunction` end to end in-process (build the
/// input `RecordBatch`, run `on_bind` + `process`, inspect the result) without the
/// RPC/IPC plumbing.
#[cfg(test)]
pub mod test_support {
    use std::sync::Arc;

    use arrow_array::builder::StringBuilder;
    use arrow_array::{ArrayRef, RecordBatch};
    use arrow_schema::{Field, Schema, SchemaRef};
    use vgi::arguments::Arguments;
    use vgi::{BindParams, ProcessParams, ScalarFunction};
    use vgi_rpc::Result;

    /// Build a `RecordBatch` from already-built columns and matching field names.
    pub fn batch_of(cols: Vec<(&str, ArrayRef)>) -> RecordBatch {
        let fields: Vec<Field> = cols
            .iter()
            .map(|(n, a)| Field::new(*n, a.data_type().clone(), true))
            .collect();
        let arrays: Vec<ArrayRef> = cols.into_iter().map(|(_, a)| a).collect();
        let schema = Arc::new(Schema::new(fields));
        RecordBatch::try_new(schema, arrays).unwrap()
    }

    /// A single-column `Utf8` (VARCHAR) input batch. `None` entries become NULLs.
    pub fn text_batch(rows: &[Option<&str>]) -> RecordBatch {
        let mut b = StringBuilder::new();
        for r in rows {
            match r {
                Some(s) => b.append_value(s),
                None => b.append_null(),
            }
        }
        let arr: ArrayRef = Arc::new(b.finish());
        let schema = Arc::new(Schema::new(vec![Field::new(
            "text",
            arr.data_type().clone(),
            true,
        )]));
        RecordBatch::try_new(schema, vec![arr]).unwrap()
    }

    /// Build a `ProcessParams` carrying the given output schema and arguments.
    pub fn process_params(output_schema: SchemaRef, arguments: Arguments) -> ProcessParams {
        ProcessParams {
            substream_id: None,
            if_none_match: None,
            if_modified_since: None,
            output_schema,
            input_schema: None,
            execution_id: Vec::new(),
            init_opaque_data: Vec::new(),
            arguments,
            settings: Default::default(),
            secrets: Default::default(),
            auth_principal: None,
            projection_ids: None,
            pushdown_filters: None,
            join_keys: Vec::new(),
            storage: None,
            order_by_column: None,
            order_by_direction: None,
            order_by_null_order: None,
            order_by_limit: None,
            tablesample_percentage: None,
            tablesample_seed: None,
            attach_opaque_data: None,
            at_unit: None,
            at_value: None,
            copy_from: None,
        }
    }

    /// Run a scalar function over a prebuilt input batch: call `on_bind` to obtain
    /// the declared output schema, then `process`, returning the single result
    /// column. The `arguments` apply to both bind and process.
    pub fn run_scalar_on<F: ScalarFunction>(
        f: &F,
        batch: RecordBatch,
        arguments: Arguments,
    ) -> Result<ArrayRef> {
        let bind = BindParams {
            input_schema: Some(batch.schema()),
            arguments: arguments.clone(),
            ..Default::default()
        };
        let bound = f.on_bind(&bind)?;
        let params = process_params(bound.output_schema.clone(), arguments);
        let out = f.process(&params, &batch)?;
        Ok(out.column(0).clone())
    }

    /// Run a scalar over a single-column VARCHAR input batch.
    pub fn run_scalar_text<F: ScalarFunction>(
        f: &F,
        rows: &[Option<&str>],
        arguments: Arguments,
    ) -> Result<ArrayRef> {
        run_scalar_on(f, text_batch(rows), arguments)
    }

    /// The declared output `DataType` from `on_bind` for a scalar with no
    /// bind-time argument requirements.
    pub fn bound_type<F: ScalarFunction>(f: &F) -> arrow_schema::DataType {
        let bind = BindParams::default();
        let bound = f.on_bind(&bind).unwrap();
        bound.output_schema.field(0).data_type().clone()
    }
}
