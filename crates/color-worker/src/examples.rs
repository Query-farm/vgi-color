//! Composed example queries: realistic SQL that combines two or more of this
//! worker's own objects (functions and the `named_colors` table) in one query.
//!
//! Each example is defined exactly once here and referenced from every place
//! that publishes it: the schema-level `vgi.example_queries` list (`lib.rs`) and
//! the example list of the object that drives the query (a scalar's
//! `FunctionMetadata.examples`, or the `named_colors` table's
//! `vgi.example_queries`). The vgi-lint agent simulator only shows agents the
//! first five examples of each table/function, so every composed example is kept
//! within the first five of its driving object.
//!
//! `README.md` and `test/sql/compose.test` carry copies for readers and for the
//! pinned E2E outputs; a unit test below fails if either copy drifts from the
//! constant (whitespace-insensitive).

use vgi::FunctionExample;

/// One described example query.
#[derive(Clone, Copy, Debug)]
pub struct Example {
    pub description: &'static str,
    pub sql: &'static str,
}

impl Example {
    /// The SDK's per-function example carrier.
    pub fn to_function_example(self) -> FunctionExample {
        FunctionExample {
            sql: self.sql.into(),
            description: self.description.into(),
            expected_output: None,
        }
    }
}

/// Serialize described examples into the `vgi.example_queries` JSON shape.
pub fn example_queries_json(examples: &[Example]) -> String {
    let fx: Vec<FunctionExample> = examples.iter().map(|e| e.to_function_example()).collect();
    crate::meta::example_queries_json(&fx)
}

/// Driving object: `wcag_level`. Uses from_hex, rgb_to_hsl, hsl_to_rgb, to_hex,
/// contrast_ratio, wcag_level.
pub const ACCESSIBLE_SHADE_LADDER: Example = Example {
    description: "Accessible shade finder: keep tomato's (#ff6347) HSL hue and saturation, step \
                  its lightness from 0.60 down to 0.30, and grade white text on each shade with \
                  contrast_ratio and wcag_level. Lightness 0.45 (#e52300, 4.59:1) is the \
                  lightest shade that reaches AA; 0.30 (#991700, 8.48:1) reaches AAA.",
    sql: "WITH base AS (SELECT color.main.rgb_to_hsl(c.r, c.g, c.b) AS hsl FROM (SELECT \
          color.main.from_hex('#ff6347') AS c)), shades AS (SELECT t.k / 100 AS lightness, \
          color.main.hsl_to_rgb(base.hsl.h, base.hsl.s, t.k / 100) AS rgb FROM base, range(30, \
          65, 5) AS t(k)), ladder AS (SELECT lightness, color.main.to_hex(rgb.r, rgb.g, rgb.b) \
          AS hex FROM shades) SELECT lightness, hex, ROUND(color.main.contrast_ratio('#ffffff', \
          hex), 2) AS ratio_on_white, color.main.wcag_level('#ffffff', hex) AS level FROM \
          ladder ORDER BY lightness DESC",
};

/// Driving object: `is_dark`. Uses is_dark, contrast_ratio, wcag_level.
pub const TEXT_COLOR_FOR_BACKGROUNDS: Example = Example {
    description: "Pick a readable label color for each background in a palette: is_dark chooses \
                  white text on dark backgrounds and black text on light ones, then \
                  contrast_ratio and wcag_level grade the pair. is_dark always picks the \
                  higher-contrast option, so every row reaches at least AA; the closest call is \
                  #e50914 with white text at 4.79:1, and mid-grey #808080 gets black text \
                  (5.32:1).",
    sql: "SELECT p.bg, CASE WHEN color.main.is_dark(p.bg) THEN '#ffffff' ELSE '#000000' END AS \
          text_hex, ROUND(color.main.contrast_ratio(text_hex, p.bg), 2) AS ratio, \
          color.main.wcag_level(text_hex, p.bg) AS level FROM (VALUES ('#e50914'), \
          ('#e1306c'), ('#808080'), ('#4285f4'), ('#3b5998'), ('#ff9900')) AS p(bg) ORDER BY \
          ratio, p.bg",
};

/// Driving object: the `named_colors` table. Uses named_colors, contrast_ratio,
/// wcag_level.
pub const READABLE_TEXT_ON_WHITE: Example = Example {
    description: "Which CSS named colors are safe for body text on a white page? Grade every \
                  named color as text on #ffffff with contrast_ratio and wcag_level, keep AA or \
                  better, and list the six lightest that still pass, starting with teal \
                  (4.77:1). Only 32 of the 148 names pass; most fail.",
    sql: "SELECT n.name, n.hex, ROUND(color.main.contrast_ratio(n.hex, '#ffffff'), 2) AS ratio, \
          color.main.wcag_level(n.hex, '#ffffff') AS level FROM color.main.named_colors AS n \
          WHERE color.main.wcag_level(n.hex, '#ffffff') IN ('AA', 'AAA') ORDER BY ratio, n.name \
          LIMIT 6",
};

/// Driving object: the `named_colors` table (self-join). Uses named_colors,
/// delta_e.
pub const NEAR_DUPLICATE_NAMED_COLORS: Example = Example {
    description: "Palette de-duplication: join named_colors to itself, score every pair of \
                  names with different hex values by CIEDE2000 delta_e, and keep the pairs a \
                  viewer can barely tell apart (ΔE00 < 2). Five pairs qualify, from chartreuse / \
                  lawngreen (0.652) to moccasin / wheat (1.99).",
    sql: "SELECT name_a, name_b, hex_a, hex_b, ROUND(d, 3) AS delta_e FROM (SELECT a.name AS \
          name_a, b.name AS name_b, a.hex AS hex_a, b.hex AS hex_b, color.main.delta_e(a.hex, \
          b.hex) AS d FROM color.main.named_colors AS a JOIN color.main.named_colors AS b ON \
          a.name < b.name AND a.hex <> b.hex) AS pairs WHERE d < 2 ORDER BY delta_e, name_a, \
          name_b",
};

/// Driving object: `hsl_to_rgb`. Uses from_hex, rgb_to_hsl, hsl_to_rgb, to_hex,
/// nearest_color_name.
pub const COLOR_HARMONY: Example = Example {
    description: "Color-harmony generator: convert #ff6347 to HSL, rotate its hue by 120, 180 \
                  and 240 degrees, convert each back with hsl_to_rgb and to_hex, and label it \
                  with nearest_color_name. Result: base #ff6347 tomato, triad 1 #47ff63 \
                  springgreen, complement #47e3ff skyblue, triad 2 #6347ff slateblue.",
    sql: "WITH base AS (SELECT color.main.rgb_to_hsl(c.r, c.g, c.b) AS hsl FROM (SELECT \
          color.main.from_hex('#ff6347') AS c)), harmony AS (SELECT t.role, t.shift, \
          color.main.hsl_to_rgb((base.hsl.h + t.shift) % 360, base.hsl.s, base.hsl.l) AS rgb \
          FROM base, (VALUES ('base', 0), ('triad 1', 120), ('complement', 180), ('triad 2', \
          240)) AS t(role, shift)), palette AS (SELECT role, shift, color.main.to_hex(rgb.r, \
          rgb.g, rgb.b) AS hex FROM harmony) SELECT role, hex, \
          color.main.nearest_color_name(hex) AS nearest_name FROM palette ORDER BY shift",
};

/// Driving object: `nearest_color_name`. Uses nearest_color_name, named_colors,
/// delta_e.
pub const SNAP_TO_CSS_NAME: Example = Example {
    description: "Snap brand hex values to CSS names and flag which swaps are safe: \
                  nearest_color_name picks the name, joining named_colors recovers its hex, and \
                  delta_e measures the error; close_match is true when ΔE00 < 2. #ffd800 → gold \
                  (0.32) and #fafafa → white (1.0) are close; #f40009 → red (2.55), #4285f4 → \
                  dodgerblue (5.05) and #1da1f2 → dodgerblue (7.03) are visibly off.",
    sql: "SELECT p.hex, n.name AS css_name, n.hex AS css_hex, \
          ROUND(color.main.delta_e(p.hex, n.hex), 2) AS delta_e, color.main.delta_e(p.hex, \
          n.hex) < 2 AS close_match FROM (VALUES ('#fafafa'), ('#ffd800'), ('#f40009'), \
          ('#4285f4'), ('#1da1f2')) AS p(hex) JOIN color.main.named_colors AS n ON n.name = \
          color.main.nearest_color_name(p.hex) ORDER BY delta_e, p.hex",
};

/// Driving object: `rgb_to_lab`. Uses named_colors, from_hex, rgb_to_hsl,
/// rgb_to_lab.
pub const HSL_VS_LAB_LIGHTNESS: Example = Example {
    description: "HSL lightness is not perceived lightness: take the 18 named colors whose HSL \
                  lightness (rgb_to_hsl) is exactly 0.5 and sort them by CIELAB L* \
                  (rgb_to_lab). L* runs from 97.1 for yellow down to 32.3 for blue, so sort or \
                  match palettes by L*, not HSL l.",
    sql: "WITH rgb AS (SELECT n.name, n.hex, color.main.from_hex(n.hex) AS c FROM \
          color.main.named_colors AS n), spaces AS (SELECT name, hex, \
          color.main.rgb_to_hsl(c.r, c.g, c.b) AS hsl, color.main.rgb_to_lab(c.r, c.g, c.b) AS \
          lab FROM rgb) SELECT name, hex, ROUND(lab.l, 1) AS lab_lightness FROM spaces WHERE \
          ROUND(hsl.l, 3) = 0.5 ORDER BY lab_lightness DESC, name",
};

/// Every composed example, in the order the schema-level list publishes them.
pub const COMPOSED: &[Example] = &[
    ACCESSIBLE_SHADE_LADDER,
    TEXT_COLOR_FOR_BACKGROUNDS,
    READABLE_TEXT_ON_WHITE,
    NEAR_DUPLICATE_NAMED_COLORS,
    COLOR_HARMONY,
    SNAP_TO_CSS_NAME,
    HSL_VS_LAB_LIGHTNESS,
];

#[cfg(test)]
mod tests {
    use super::*;

    fn squash(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn compose_test_pins_every_composed_example() {
        let test = squash(include_str!("../../../test/sql/compose.test"));
        for ex in COMPOSED {
            assert!(
                test.contains(&squash(ex.sql)),
                "test/sql/compose.test is missing or has drifted from:\n{}",
                ex.sql
            );
        }
    }

    #[test]
    fn readme_copies_match_the_constants() {
        let readme = squash(include_str!("../../../README.md"));
        for ex in [
            ACCESSIBLE_SHADE_LADDER,
            TEXT_COLOR_FOR_BACKGROUNDS,
            COLOR_HARMONY,
        ] {
            assert!(
                readme.contains(&squash(ex.sql)),
                "README.md is missing or has drifted from:\n{}",
                ex.sql
            );
        }
    }

    #[test]
    fn composed_examples_are_qualified_ordered_and_described() {
        for ex in COMPOSED {
            assert!(ex.sql.contains("color.main."), "{}", ex.sql);
            assert!(ex.sql.contains("ORDER BY"), "{}", ex.sql);
            assert!(!ex.sql.contains("SELECT *"), "{}", ex.sql);
            assert!(!ex.description.is_empty());
        }
    }
}
