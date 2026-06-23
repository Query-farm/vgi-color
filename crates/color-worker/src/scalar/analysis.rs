//! Color-analysis scalars operating on hex strings:
//!
//! * `delta_e(hex_a, hex_b) -> DOUBLE` — CIEDE2000 color difference
//! * `luminance(hex) -> DOUBLE` — WCAG relative luminance (0..1)
//! * `contrast_ratio(hex_a, hex_b) -> DOUBLE` — WCAG contrast (1..21)
//! * `wcag_level(hex_fg, hex_bg) -> VARCHAR` — 'AAA'|'AA'|'AA Large'|'fail'
//! * `is_dark(hex) -> BOOLEAN` — luminance < 0.5
//! * `nearest_color_name(hex) -> VARCHAR` — closest CSS named color by ΔE
//!
//! Invalid hex is treated as missing data → NULL (never an error, never a panic).

use std::sync::Arc;

use arrow_array::builder::{BooleanBuilder, Float64Builder, StringBuilder};
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::DataType;
use vgi::{ArgSpec, BindParams, BindResponse, FunctionMetadata, ProcessParams, ScalarFunction};
use vgi_rpc::{Result, RpcError};

use crate::arrow_io::text_str;
use crate::color;

/// `delta_e(hex_a, hex_b) -> DOUBLE` (CIEDE2000). NULL if either hex is invalid.
pub struct DeltaE;

impl ScalarFunction for DeltaE {
    fn name(&self) -> &str {
        "delta_e"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "CIEDE2000 (ΔE00) color difference between two hex colors; NULL if \
                          either is not a valid hex color"
                .into(),
            return_type: Some(DataType::Float64),
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![
            ArgSpec::any_column("hex_a", 0, "First hex color (VARCHAR)"),
            ArgSpec::any_column("hex_b", 1, "Second hex color (VARCHAR)"),
        ]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(DataType::Float64))
    }

    fn process(&self, params: &ProcessParams, batch: &RecordBatch) -> Result<RecordBatch> {
        let (a, b) = (batch.column(0), batch.column(1));
        let rows = batch.num_rows();
        let mut out = Float64Builder::new();
        for i in 0..rows {
            let ca = text_str(a, i)?.and_then(color::from_hex);
            let cb = text_str(b, i)?.and_then(color::from_hex);
            match (ca, cb) {
                (Some(ca), Some(cb)) => out.append_value(color::delta_e(ca, cb)),
                _ => out.append_null(),
            }
        }
        let arr: ArrayRef = Arc::new(out.finish());
        RecordBatch::try_new(params.output_schema.clone(), vec![arr])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

/// `luminance(hex) -> DOUBLE` (WCAG relative luminance). NULL if invalid.
pub struct Luminance;

impl ScalarFunction for Luminance {
    fn name(&self) -> &str {
        "luminance"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "WCAG relative luminance (0..1) of a hex color; NULL if invalid".into(),
            return_type: Some(DataType::Float64),
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![ArgSpec::any_column("hex", 0, "Hex color (VARCHAR)")]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(DataType::Float64))
    }

    fn process(&self, params: &ProcessParams, batch: &RecordBatch) -> Result<RecordBatch> {
        let col = batch.column(0);
        let rows = batch.num_rows();
        let mut out = Float64Builder::new();
        for i in 0..rows {
            match text_str(col, i)?.and_then(color::from_hex) {
                Some(c) => out.append_value(color::luminance(c)),
                None => out.append_null(),
            }
        }
        let arr: ArrayRef = Arc::new(out.finish());
        RecordBatch::try_new(params.output_schema.clone(), vec![arr])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

/// `contrast_ratio(hex_a, hex_b) -> DOUBLE` (WCAG, 1..21). NULL if invalid.
pub struct ContrastRatio;

impl ScalarFunction for ContrastRatio {
    fn name(&self) -> &str {
        "contrast_ratio"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "WCAG contrast ratio (1..21) between two hex colors; NULL if either is \
                          invalid"
                .into(),
            return_type: Some(DataType::Float64),
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![
            ArgSpec::any_column("hex_a", 0, "First hex color (VARCHAR)"),
            ArgSpec::any_column("hex_b", 1, "Second hex color (VARCHAR)"),
        ]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(DataType::Float64))
    }

    fn process(&self, params: &ProcessParams, batch: &RecordBatch) -> Result<RecordBatch> {
        let (a, b) = (batch.column(0), batch.column(1));
        let rows = batch.num_rows();
        let mut out = Float64Builder::new();
        for i in 0..rows {
            let ca = text_str(a, i)?.and_then(color::from_hex);
            let cb = text_str(b, i)?.and_then(color::from_hex);
            match (ca, cb) {
                (Some(ca), Some(cb)) => out.append_value(color::contrast_ratio(ca, cb)),
                _ => out.append_null(),
            }
        }
        let arr: ArrayRef = Arc::new(out.finish());
        RecordBatch::try_new(params.output_schema.clone(), vec![arr])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

/// `wcag_level(hex_fg, hex_bg) -> VARCHAR`. NULL if either hex is invalid.
pub struct WcagLevel;

impl ScalarFunction for WcagLevel {
    fn name(&self) -> &str {
        "wcag_level"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "WCAG conformance level for normal text ('AAA'|'AA'|'AA Large'|'fail') \
                          given foreground/background hex colors; NULL if either is invalid"
                .into(),
            return_type: Some(DataType::Utf8),
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![
            ArgSpec::any_column("hex_fg", 0, "Foreground (text) hex color (VARCHAR)"),
            ArgSpec::any_column("hex_bg", 1, "Background hex color (VARCHAR)"),
        ]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(DataType::Utf8))
    }

    fn process(&self, params: &ProcessParams, batch: &RecordBatch) -> Result<RecordBatch> {
        let (fg, bg) = (batch.column(0), batch.column(1));
        let rows = batch.num_rows();
        let mut out = StringBuilder::new();
        for i in 0..rows {
            let cf = text_str(fg, i)?.and_then(color::from_hex);
            let cb = text_str(bg, i)?.and_then(color::from_hex);
            match (cf, cb) {
                (Some(cf), Some(cb)) => {
                    out.append_value(color::wcag_level(color::contrast_ratio(cf, cb)))
                }
                _ => out.append_null(),
            }
        }
        let arr: ArrayRef = Arc::new(out.finish());
        RecordBatch::try_new(params.output_schema.clone(), vec![arr])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

/// `is_dark(hex) -> BOOLEAN` (luminance < 0.5). NULL if invalid.
pub struct IsDark;

impl ScalarFunction for IsDark {
    fn name(&self) -> &str {
        "is_dark"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "True if a hex color's WCAG relative luminance is below 0.5; NULL if \
                          invalid"
                .into(),
            return_type: Some(DataType::Boolean),
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![ArgSpec::any_column("hex", 0, "Hex color (VARCHAR)")]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(DataType::Boolean))
    }

    fn process(&self, params: &ProcessParams, batch: &RecordBatch) -> Result<RecordBatch> {
        let col = batch.column(0);
        let rows = batch.num_rows();
        let mut out = BooleanBuilder::new();
        for i in 0..rows {
            match text_str(col, i)?.and_then(color::from_hex) {
                Some(c) => out.append_value(color::is_dark(c)),
                None => out.append_null(),
            }
        }
        let arr: ArrayRef = Arc::new(out.finish());
        RecordBatch::try_new(params.output_schema.clone(), vec![arr])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

/// `nearest_color_name(hex) -> VARCHAR` (closest CSS named color by ΔE).
pub struct NearestColorName;

impl ScalarFunction for NearestColorName {
    fn name(&self) -> &str {
        "nearest_color_name"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "Closest CSS named color to a hex color, by CIEDE2000 ΔE; NULL if invalid"
                .into(),
            return_type: Some(DataType::Utf8),
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![ArgSpec::any_column("hex", 0, "Hex color (VARCHAR)")]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(DataType::Utf8))
    }

    fn process(&self, params: &ProcessParams, batch: &RecordBatch) -> Result<RecordBatch> {
        let col = batch.column(0);
        let rows = batch.num_rows();
        let mut out = StringBuilder::new();
        for i in 0..rows {
            match text_str(col, i)?.and_then(color::from_hex) {
                Some(c) => out.append_value(color::nearest_color_name(c)),
                None => out.append_null(),
            }
        }
        let arr: ArrayRef = Arc::new(out.finish());
        RecordBatch::try_new(params.output_schema.clone(), vec![arr])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrow_io::test_support::{batch_of, bound_type, run_scalar_on, run_scalar_text};
    use arrow_array::cast::AsArray;
    use arrow_array::types::Float64Type;
    use arrow_array::{Array, StringArray};
    use vgi::arguments::Arguments;

    fn pair(a: &[Option<&str>], b: &[Option<&str>]) -> arrow_array::RecordBatch {
        batch_of(vec![
            ("a", Arc::new(StringArray::from(a.to_vec())) as ArrayRef),
            ("b", Arc::new(StringArray::from(b.to_vec())) as ArrayRef),
        ])
    }

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} != {b}");
    }

    #[test]
    fn delta_e_zero_and_invalid() {
        assert_eq!(bound_type(&DeltaE), DataType::Float64);
        let out = run_scalar_on(
            &DeltaE,
            pair(
                &[Some("#ff0000"), Some("nothex")],
                &[Some("#ff0000"), Some("#000000")],
            ),
            Arguments::default(),
        )
        .unwrap();
        let d = out.as_primitive::<Float64Type>();
        close(d.value(0), 0.0, 1e-9);
        assert!(out.is_null(1), "invalid hex → NULL");
    }

    #[test]
    fn contrast_extremes() {
        let out = run_scalar_on(
            &ContrastRatio,
            pair(
                &[Some("#000000"), Some("#ffffff")],
                &[Some("#ffffff"), Some("#ffffff")],
            ),
            Arguments::default(),
        )
        .unwrap();
        let d = out.as_primitive::<Float64Type>();
        close(d.value(0), 21.0, 1e-9);
        close(d.value(1), 1.0, 1e-9);
    }

    #[test]
    fn luminance_extremes() {
        let out = run_scalar_text(
            &Luminance,
            &[Some("#000000"), Some("#ffffff"), Some("not-a-color")],
            Arguments::default(),
        )
        .unwrap();
        let d = out.as_primitive::<Float64Type>();
        close(d.value(0), 0.0, 1e-12);
        close(d.value(1), 1.0, 1e-12);
        assert!(out.is_null(2));
    }

    #[test]
    fn wcag_level_aaa() {
        assert_eq!(bound_type(&WcagLevel), DataType::Utf8);
        let out = run_scalar_on(
            &WcagLevel,
            pair(&[Some("#000000")], &[Some("#ffffff")]),
            Arguments::default(),
        )
        .unwrap();
        assert_eq!(out.as_string::<i32>().value(0), "AAA");
    }

    #[test]
    fn is_dark_basic() {
        let out = run_scalar_text(
            &IsDark,
            &[Some("#000000"), Some("#ffffff"), None],
            Arguments::default(),
        )
        .unwrap();
        let bb = out.as_boolean();
        assert!(bb.value(0));
        assert!(!bb.value(1));
        assert!(out.is_null(2));
    }

    #[test]
    fn nearest_name_red() {
        let out = run_scalar_text(
            &NearestColorName,
            &[Some("#ff0000"), Some("not-a-color")],
            Arguments::default(),
        )
        .unwrap();
        assert_eq!(out.as_string::<i32>().value(0), "red");
        assert!(out.is_null(1));
    }
}
