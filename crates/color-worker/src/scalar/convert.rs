//! Color-space conversion scalars:
//!
//! * `to_hex(r INT, g INT, b INT) -> VARCHAR` — `#rrggbb`
//! * `from_hex(hex VARCHAR) -> STRUCT(r INT, g INT, b INT)` — NULL if invalid
//! * `rgb_to_hsl(r, g, b) -> STRUCT(h DOUBLE, s DOUBLE, l DOUBLE)`
//! * `hsl_to_rgb(h, s, l) -> STRUCT(r INT, g INT, b INT)`
//! * `rgb_to_lab(r, g, b) -> STRUCT(l DOUBLE, a DOUBLE, b DOUBLE)` (CIELAB, D65)
//!
//! RGB inputs are clamped to `0..=255`; any NULL operand yields a NULL result.

use std::sync::Arc;

use arrow_array::builder::{Float64Builder, Int32Builder, StringBuilder};
use arrow_array::{ArrayRef, RecordBatch, StructArray};
use arrow_buffer::NullBuffer;
use arrow_schema::DataType;
use vgi::{
    ArgSpec, BindParams, BindResponse, FunctionExample, FunctionMetadata, ProcessParams,
    ScalarFunction,
};
use vgi_rpc::{Result, RpcError};

use crate::arrow_io::{
    double_val, hsl_struct_fields, int_val, lab_struct_fields, rgb_struct_fields, text_str,
};
use crate::color::{self, Rgb};

/// `to_hex(r, g, b) -> VARCHAR`.
pub struct ToHex;

impl ScalarFunction for ToHex {
    fn name(&self) -> &str {
        "to_hex"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "Format an RGB triple (0-255, clamped) as a '#rrggbb' hex string".into(),
            return_type: Some(DataType::Utf8),
            examples: vec![FunctionExample {
                sql: "SELECT color.main.to_hex(255, 99, 71);".into(),
                description: "Format the RGB triple for 'tomato' as a '#rrggbb' hex string.".into(),
                expected_output: None,
            }],
            tags: crate::meta::object_tags(
                "Format RGB as Hex",
                "Format an RGB triple (each channel 0-255, out-of-range values clamped) as a \
                 lowercase '#rrggbb' sRGB hex color string. Returns NULL if any channel is NULL.",
                "Format an RGB triple as a `#rrggbb` hex string, e.g. `to_hex(255, 99, 71)` → \
                 `#ff6347`.",
                &[
                    "to_hex",
                    "rgb to hex",
                    "hex color",
                    "hex string",
                    "rrggbb",
                    "format color",
                    "encode color",
                    "sRGB",
                ],
                "Conversion",
            ),
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![
            ArgSpec::column(
                "r",
                0,
                "int32",
                "Red channel, 0-255 (out-of-range values clamped)",
            ),
            ArgSpec::column(
                "g",
                1,
                "int32",
                "Green channel, 0-255 (out-of-range values clamped)",
            ),
            ArgSpec::column(
                "b",
                2,
                "int32",
                "Blue channel, 0-255 (out-of-range values clamped)",
            ),
        ]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(DataType::Utf8))
    }

    fn process(&self, params: &ProcessParams, batch: &RecordBatch) -> Result<RecordBatch> {
        let (r, g, b) = (batch.column(0), batch.column(1), batch.column(2));
        let rows = batch.num_rows();
        let mut out = StringBuilder::new();
        for i in 0..rows {
            match (int_val(r, i)?, int_val(g, i)?, int_val(b, i)?) {
                (Some(r), Some(g), Some(b)) => out.append_value(color::to_hex(
                    color::clamp_channel(r),
                    color::clamp_channel(g),
                    color::clamp_channel(b),
                )),
                _ => out.append_null(),
            }
        }
        let arr: ArrayRef = Arc::new(out.finish());
        RecordBatch::try_new(params.output_schema.clone(), vec![arr])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

/// `from_hex(hex) -> STRUCT(r INT, g INT, b INT)`. Invalid hex → NULL row.
pub struct FromHex;

impl ScalarFunction for FromHex {
    fn name(&self) -> &str {
        "from_hex"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "Parse '#rgb' / '#rrggbb' / '#rrggbbaa' into STRUCT(r, g, b); NULL if \
                          the string is not a valid hex color"
                .into(),
            examples: vec![FunctionExample {
                sql: "SELECT (color.main.from_hex('#ff6347')).r AS red;".into(),
                description: "Parse a '#rrggbb' hex string into its red, green and blue channels."
                    .into(),
                expected_output: None,
            }],
            tags: crate::meta::object_tags(
                "Parse Hex to RGB",
                "Parse an sRGB hex color string ('#rgb', '#rrggbb' or '#rrggbbaa') into a \
                 STRUCT(r, g, b) of 0-255 integer channels. Returns a NULL struct when the input \
                 is NULL or not a valid hex color (alpha is accepted but dropped).",
                "Parse a hex color into a `STRUCT(r, g, b)`, e.g. `from_hex('#ff6347')` → \
                 `(255, 99, 71)`.",
                &[
                    "from_hex",
                    "hex to rgb",
                    "parse hex",
                    "decode color",
                    "hex color",
                    "rrggbb",
                    "color parsing",
                    "sRGB",
                ],
                "Conversion",
            ),
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![ArgSpec::column(
            "hex",
            0,
            "varchar",
            "sRGB hex color to parse: '#rgb', '#rrggbb' or '#rrggbbaa' (alpha is dropped)",
        )]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(DataType::Struct(rgb_struct_fields())))
    }

    fn process(&self, params: &ProcessParams, batch: &RecordBatch) -> Result<RecordBatch> {
        let col = batch.column(0);
        let rows = batch.num_rows();
        let mut rb = Int32Builder::new();
        let mut gb = Int32Builder::new();
        let mut bb = Int32Builder::new();
        let mut valid: Vec<bool> = Vec::with_capacity(rows);
        for i in 0..rows {
            let parsed = text_str(col, i)?.and_then(color::from_hex);
            match parsed {
                Some(Rgb { r, g, b }) => {
                    rb.append_value(r as i32);
                    gb.append_value(g as i32);
                    bb.append_value(b as i32);
                    valid.push(true);
                }
                None => {
                    rb.append_null();
                    gb.append_null();
                    bb.append_null();
                    valid.push(false);
                }
            }
        }
        let arrays: Vec<ArrayRef> = vec![
            Arc::new(rb.finish()),
            Arc::new(gb.finish()),
            Arc::new(bb.finish()),
        ];
        let out: ArrayRef = Arc::new(StructArray::new(
            rgb_struct_fields(),
            arrays,
            Some(NullBuffer::from(valid)),
        ));
        RecordBatch::try_new(params.output_schema.clone(), vec![out])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

/// `rgb_to_hsl(r, g, b) -> STRUCT(h DOUBLE, s DOUBLE, l DOUBLE)`.
pub struct RgbToHsl;

impl ScalarFunction for RgbToHsl {
    fn name(&self) -> &str {
        "rgb_to_hsl"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "Convert an RGB triple (0-255) to HSL: STRUCT(h in [0,360), s in [0,1], \
                          l in [0,1])"
                .into(),
            examples: vec![FunctionExample {
                sql: "SELECT (color.main.rgb_to_hsl(255, 0, 0)).h AS hue;".into(),
                description: "Convert pure red to its hue, saturation and lightness.".into(),
                expected_output: None,
            }],
            tags: crate::meta::object_tags(
                "Convert RGB to HSL",
                "Convert an RGB triple (each channel 0-255, clamped) to the HSL color space, \
                 returning STRUCT(h in [0,360) degrees, s in [0,1], l in [0,1]). Returns a NULL \
                 struct if any channel is NULL.",
                "Convert RGB to HSL, e.g. `rgb_to_hsl(255, 0, 0)` → `(h := 0, s := 1, l := 0.5)`.",
                &[
                    "rgb_to_hsl",
                    "rgb to hsl",
                    "hue saturation lightness",
                    "color space conversion",
                    "HSL",
                    "hue",
                    "saturation",
                    "lightness",
                ],
                "Conversion",
            ),
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![
            ArgSpec::column(
                "r",
                0,
                "int32",
                "Red channel, 0-255 (out-of-range values clamped)",
            ),
            ArgSpec::column(
                "g",
                1,
                "int32",
                "Green channel, 0-255 (out-of-range values clamped)",
            ),
            ArgSpec::column(
                "b",
                2,
                "int32",
                "Blue channel, 0-255 (out-of-range values clamped)",
            ),
        ]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(DataType::Struct(hsl_struct_fields())))
    }

    fn process(&self, params: &ProcessParams, batch: &RecordBatch) -> Result<RecordBatch> {
        let (r, g, b) = (batch.column(0), batch.column(1), batch.column(2));
        let rows = batch.num_rows();
        let mut hb = Float64Builder::new();
        let mut sb = Float64Builder::new();
        let mut lb = Float64Builder::new();
        let mut valid: Vec<bool> = Vec::with_capacity(rows);
        for i in 0..rows {
            match (int_val(r, i)?, int_val(g, i)?, int_val(b, i)?) {
                (Some(r), Some(g), Some(b)) => {
                    let (h, s, l) = color::rgb_to_hsl(Rgb::new(
                        color::clamp_channel(r),
                        color::clamp_channel(g),
                        color::clamp_channel(b),
                    ));
                    hb.append_value(h);
                    sb.append_value(s);
                    lb.append_value(l);
                    valid.push(true);
                }
                _ => {
                    hb.append_null();
                    sb.append_null();
                    lb.append_null();
                    valid.push(false);
                }
            }
        }
        let arrays: Vec<ArrayRef> = vec![
            Arc::new(hb.finish()),
            Arc::new(sb.finish()),
            Arc::new(lb.finish()),
        ];
        let out: ArrayRef = Arc::new(StructArray::new(
            hsl_struct_fields(),
            arrays,
            Some(NullBuffer::from(valid)),
        ));
        RecordBatch::try_new(params.output_schema.clone(), vec![out])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

/// `hsl_to_rgb(h, s, l) -> STRUCT(r INT, g INT, b INT)`.
pub struct HslToRgb;

impl ScalarFunction for HslToRgb {
    fn name(&self) -> &str {
        "hsl_to_rgb"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "Convert HSL (h degrees, s/l in [0,1]) to an RGB triple: \
                          STRUCT(r, g, b) in 0-255"
                .into(),
            examples: vec![FunctionExample {
                sql: "SELECT (color.main.hsl_to_rgb(120, 1.0, 0.5)).g AS green;".into(),
                description: "Convert fully-saturated green (hue 120°) back to an RGB triple."
                    .into(),
                expected_output: None,
            }],
            tags: crate::meta::object_tags(
                "Convert HSL to RGB",
                "Convert an HSL color (h in degrees, s and l in [0,1]) back to an RGB triple, \
                 returning STRUCT(r, g, b) of 0-255 integer channels. Returns a NULL struct if \
                 any component is NULL.",
                "Convert HSL to RGB, e.g. `hsl_to_rgb(120, 1.0, 0.5)` → `(r := 0, g := 255, \
                 b := 0)`.",
                &[
                    "hsl_to_rgb",
                    "hsl to rgb",
                    "hue saturation lightness",
                    "color space conversion",
                    "RGB",
                    "from hsl",
                ],
                "Conversion",
            ),
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![
            ArgSpec::column("h", 0, "double", "Hue angle in degrees (wraps modulo 360)"),
            ArgSpec::column(
                "s",
                1,
                "double",
                "Saturation from 0 (grey) to 1 (full color)",
            ),
            ArgSpec::column("l", 2, "double", "Lightness from 0 (black) to 1 (white)"),
        ]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(DataType::Struct(rgb_struct_fields())))
    }

    fn process(&self, params: &ProcessParams, batch: &RecordBatch) -> Result<RecordBatch> {
        let (h, s, l) = (batch.column(0), batch.column(1), batch.column(2));
        let rows = batch.num_rows();
        let mut rb = Int32Builder::new();
        let mut gb = Int32Builder::new();
        let mut bb = Int32Builder::new();
        let mut valid: Vec<bool> = Vec::with_capacity(rows);
        for i in 0..rows {
            match (double_val(h, i)?, double_val(s, i)?, double_val(l, i)?) {
                (Some(h), Some(s), Some(l)) => {
                    let rgb = color::hsl_to_rgb(h, s, l);
                    rb.append_value(rgb.r as i32);
                    gb.append_value(rgb.g as i32);
                    bb.append_value(rgb.b as i32);
                    valid.push(true);
                }
                _ => {
                    rb.append_null();
                    gb.append_null();
                    bb.append_null();
                    valid.push(false);
                }
            }
        }
        let arrays: Vec<ArrayRef> = vec![
            Arc::new(rb.finish()),
            Arc::new(gb.finish()),
            Arc::new(bb.finish()),
        ];
        let out: ArrayRef = Arc::new(StructArray::new(
            rgb_struct_fields(),
            arrays,
            Some(NullBuffer::from(valid)),
        ));
        RecordBatch::try_new(params.output_schema.clone(), vec![out])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

/// `rgb_to_lab(r, g, b) -> STRUCT(l DOUBLE, a DOUBLE, b DOUBLE)` (CIELAB, D65).
pub struct RgbToLab;

impl ScalarFunction for RgbToLab {
    fn name(&self) -> &str {
        "rgb_to_lab"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "Convert an RGB triple (0-255) to CIELAB (D65): STRUCT(l, a, b)".into(),
            examples: vec![FunctionExample {
                sql: "SELECT (color.main.rgb_to_lab(255, 255, 255)).l AS lightness;".into(),
                description: "Convert white to CIELAB (D65) lightness L* and a*/b* chroma.".into(),
                expected_output: None,
            }],
            tags: crate::meta::object_tags(
                "Convert RGB to CIELAB",
                "Convert an RGB triple (0-255, clamped) to the perceptually-uniform CIELAB color \
                 space under the D65 reference white, returning STRUCT(l, a, b) where l is \
                 lightness (0-100) and a/b are the chroma axes. Returns a NULL struct if any \
                 channel is NULL.",
                "Convert RGB to CIELAB (D65), e.g. `rgb_to_lab(255, 255, 255)` → `(l ≈ 100, \
                 a ≈ 0, b ≈ 0)`.",
                &[
                    "rgb_to_lab",
                    "rgb to lab",
                    "CIELAB",
                    "Lab color",
                    "L*a*b*",
                    "perceptual color space",
                    "D65",
                    "color space conversion",
                ],
                "Conversion",
            ),
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![
            ArgSpec::column(
                "r",
                0,
                "int32",
                "Red channel, 0-255 (out-of-range values clamped)",
            ),
            ArgSpec::column(
                "g",
                1,
                "int32",
                "Green channel, 0-255 (out-of-range values clamped)",
            ),
            ArgSpec::column(
                "b",
                2,
                "int32",
                "Blue channel, 0-255 (out-of-range values clamped)",
            ),
        ]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(DataType::Struct(lab_struct_fields())))
    }

    fn process(&self, params: &ProcessParams, batch: &RecordBatch) -> Result<RecordBatch> {
        let (r, g, b) = (batch.column(0), batch.column(1), batch.column(2));
        let rows = batch.num_rows();
        let mut lb = Float64Builder::new();
        let mut ab = Float64Builder::new();
        let mut bb = Float64Builder::new();
        let mut valid: Vec<bool> = Vec::with_capacity(rows);
        for i in 0..rows {
            match (int_val(r, i)?, int_val(g, i)?, int_val(b, i)?) {
                (Some(r), Some(g), Some(b)) => {
                    let (l, a, bv) = color::rgb_to_lab(Rgb::new(
                        color::clamp_channel(r),
                        color::clamp_channel(g),
                        color::clamp_channel(b),
                    ));
                    lb.append_value(l);
                    ab.append_value(a);
                    bb.append_value(bv);
                    valid.push(true);
                }
                _ => {
                    lb.append_null();
                    ab.append_null();
                    bb.append_null();
                    valid.push(false);
                }
            }
        }
        let arrays: Vec<ArrayRef> = vec![
            Arc::new(lb.finish()),
            Arc::new(ab.finish()),
            Arc::new(bb.finish()),
        ];
        let out: ArrayRef = Arc::new(StructArray::new(
            lab_struct_fields(),
            arrays,
            Some(NullBuffer::from(valid)),
        ));
        RecordBatch::try_new(params.output_schema.clone(), vec![out])
            .map_err(|e| RpcError::runtime_error(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrow_io::test_support::{batch_of, bound_type, run_scalar_on, run_scalar_text};
    use arrow_array::cast::AsArray;
    use arrow_array::types::{Float64Type, Int32Type};
    use arrow_array::{Array, Float64Array, Int32Array};
    use vgi::arguments::Arguments;

    fn rgb_batch(
        r: &[Option<i32>],
        g: &[Option<i32>],
        b: &[Option<i32>],
    ) -> arrow_array::RecordBatch {
        batch_of(vec![
            ("r", Arc::new(Int32Array::from(r.to_vec())) as ArrayRef),
            ("g", Arc::new(Int32Array::from(g.to_vec())) as ArrayRef),
            ("b", Arc::new(Int32Array::from(b.to_vec())) as ArrayRef),
        ])
    }

    #[test]
    fn to_hex_red() {
        let batch = rgb_batch(&[Some(255)], &[Some(0)], &[Some(0)]);
        let out = run_scalar_on(&ToHex, batch, Arguments::default()).unwrap();
        assert_eq!(out.as_string::<i32>().value(0), "#ff0000");
    }

    #[test]
    fn to_hex_clamps_and_nulls() {
        let batch = rgb_batch(
            &[Some(300), None],
            &[Some(-5), Some(0)],
            &[Some(0), Some(0)],
        );
        let out = run_scalar_on(&ToHex, batch, Arguments::default()).unwrap();
        assert_eq!(out.as_string::<i32>().value(0), "#ff0000");
        assert!(out.is_null(1), "NULL operand → NULL");
    }

    #[test]
    fn from_hex_struct() {
        assert_eq!(bound_type(&FromHex), DataType::Struct(rgb_struct_fields()));
        let out = run_scalar_text(
            &FromHex,
            &[Some("#ff0000"), Some("nothex"), None],
            Arguments::default(),
        )
        .unwrap();
        let s = out.as_struct();
        assert_eq!(s.column(0).as_primitive::<Int32Type>().value(0), 255);
        assert_eq!(s.column(1).as_primitive::<Int32Type>().value(0), 0);
        assert_eq!(s.column(2).as_primitive::<Int32Type>().value(0), 0);
        assert!(out.is_null(1), "invalid hex → NULL struct row");
        assert!(out.is_null(2), "NULL input → NULL struct row");
    }

    #[test]
    fn rgb_to_hsl_red() {
        assert_eq!(bound_type(&RgbToHsl), DataType::Struct(hsl_struct_fields()));
        let batch = rgb_batch(&[Some(255)], &[Some(0)], &[Some(0)]);
        let out = run_scalar_on(&RgbToHsl, batch, Arguments::default()).unwrap();
        let s = out.as_struct();
        assert!((s.column(0).as_primitive::<Float64Type>().value(0) - 0.0).abs() < 1e-9);
        assert!((s.column(1).as_primitive::<Float64Type>().value(0) - 1.0).abs() < 1e-9);
        assert!((s.column(2).as_primitive::<Float64Type>().value(0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn hsl_to_rgb_red() {
        assert_eq!(bound_type(&HslToRgb), DataType::Struct(rgb_struct_fields()));
        let batch = batch_of(vec![
            (
                "h",
                Arc::new(Float64Array::from(vec![Some(0.0)])) as ArrayRef,
            ),
            (
                "s",
                Arc::new(Float64Array::from(vec![Some(1.0)])) as ArrayRef,
            ),
            (
                "l",
                Arc::new(Float64Array::from(vec![Some(0.5)])) as ArrayRef,
            ),
        ]);
        let out = run_scalar_on(&HslToRgb, batch, Arguments::default()).unwrap();
        let s = out.as_struct();
        assert_eq!(s.column(0).as_primitive::<Int32Type>().value(0), 255);
        assert_eq!(s.column(1).as_primitive::<Int32Type>().value(0), 0);
        assert_eq!(s.column(2).as_primitive::<Int32Type>().value(0), 0);
    }

    #[test]
    fn rgb_to_lab_white() {
        assert_eq!(bound_type(&RgbToLab), DataType::Struct(lab_struct_fields()));
        let batch = rgb_batch(&[Some(255)], &[Some(255)], &[Some(255)]);
        let out = run_scalar_on(&RgbToLab, batch, Arguments::default()).unwrap();
        let s = out.as_struct();
        assert!((s.column(0).as_primitive::<Float64Type>().value(0) - 100.0).abs() < 0.01);
    }
}
