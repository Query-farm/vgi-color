//! Pure color-science engine: sRGB ⇄ linear, HSL, CIELAB (D65), CIEDE2000 color
//! difference, WCAG relative luminance / contrast, hex parsing, and a static
//! table of CSS named colors for nearest-name lookup.
//!
//! ## Formulas implemented directly (no third-party color crate)
//!
//! Every transform here is a textbook standard formula, implemented from first
//! principles so the worker carries no extra dependency and no MSRV risk:
//!
//! * **sRGB ⇄ linear**: the IEC 61966-2-1 piecewise companding curve
//!   (`0.04045`/`12.92` threshold, `2.4` gamma, `0.055` offset).
//! * **Relative luminance (WCAG 2.x)**: `0.2126 R + 0.7152 G + 0.0722 B` over the
//!   *linearized* channels.
//! * **Contrast ratio (WCAG 2.x)**: `(L_light + 0.05) / (L_dark + 0.05)`,
//!   `1.0 … 21.0`.
//! * **HSL ⇄ RGB**: the standard hue-sector conversion (HSL with `h` in degrees,
//!   `s`/`l` in `0..1`).
//! * **CIELAB (D65)**: linear-sRGB → CIE XYZ (sRGB/D65 matrix) → L\*a\*b\* with the
//!   CIE `f(t)` companding and the D65 reference white.
//! * **CIEDE2000 (ΔE₀₀)**: the full Sharma/Wu/Dalal reference implementation,
//!   including the `a'` chroma correction, the `T`/`ΔΘ`/`R_C`/`R_T` terms and the
//!   `S_L`/`S_C`/`S_H` weighting.
//!
//! All public entry points take/return plain values; the Arrow adapters in
//! `scalar/` and `table/` are thin wrappers over these.

/// An 8-bit sRGB color (channels `0..=255`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }
}

/// Clamp an `i32` channel argument (DuckDB hands us INTs) into `0..=255`.
/// Out-of-range values are clamped rather than rejected, matching the brief's
/// "clamp/validate inputs (rgb 0-255)".
pub fn clamp_channel(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// `to_hex`: `#rrggbb` lowercase.
pub fn to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// Parse a hex color string. Accepts `#rgb`, `#rrggbb`, and `#rrggbbaa` (alpha is
/// accepted and dropped). The leading `#` is optional. Returns `None` for any
/// malformed input (so callers can map to SQL NULL) — never panics.
pub fn from_hex(s: &str) -> Option<Rgb> {
    let h = s.trim();
    let h = h.strip_prefix('#').unwrap_or(h);
    if !h.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let hx = |a: u8, b: u8| -> Option<u8> {
        let hi = (a as char).to_digit(16)?;
        let lo = (b as char).to_digit(16)?;
        Some((hi * 16 + lo) as u8)
    };
    let bytes = h.as_bytes();
    match h.len() {
        3 => {
            // #rgb → each nibble doubled (e.g. f0a → ff00aa).
            let exp = |n: u8| hx(n, n);
            Some(Rgb::new(exp(bytes[0])?, exp(bytes[1])?, exp(bytes[2])?))
        }
        6 => Some(Rgb::new(
            hx(bytes[0], bytes[1])?,
            hx(bytes[2], bytes[3])?,
            hx(bytes[4], bytes[5])?,
        )),
        8 => Some(Rgb::new(
            hx(bytes[0], bytes[1])?,
            hx(bytes[2], bytes[3])?,
            hx(bytes[4], bytes[5])?,
        )),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// sRGB companding
// ---------------------------------------------------------------------------

/// One sRGB channel (`0..=255`) → linear-light `0.0..=1.0` (IEC 61966-2-1).
fn srgb_to_linear(c: u8) -> f64 {
    let cs = c as f64 / 255.0;
    if cs <= 0.04045 {
        cs / 12.92
    } else {
        ((cs + 0.055) / 1.055).powf(2.4)
    }
}

// ---------------------------------------------------------------------------
// HSL
// ---------------------------------------------------------------------------

/// `rgb_to_hsl`: returns `(h in [0,360), s in [0,1], l in [0,1])`.
pub fn rgb_to_hsl(rgb: Rgb) -> (f64, f64, f64) {
    let r = rgb.r as f64 / 255.0;
    let g = rgb.g as f64 / 255.0;
    let b = rgb.b as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let delta = max - min;
    if delta == 0.0 {
        return (0.0, 0.0, l);
    }
    let s = delta / (1.0 - (2.0 * l - 1.0).abs());
    let mut h = if max == r {
        ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    h *= 60.0;
    if h < 0.0 {
        h += 360.0;
    }
    (h, s, l)
}

/// `hsl_to_rgb`: `h` in degrees (wrapped), `s`/`l` clamped to `0..1`.
pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Rgb {
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);
    let h = h.rem_euclid(360.0);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to8 = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb::new(to8(r1), to8(g1), to8(b1))
}

// ---------------------------------------------------------------------------
// CIE XYZ / CIELAB (D65)
// ---------------------------------------------------------------------------

/// D65 reference white in CIE XYZ (sRGB, 2° observer).
const XN: f64 = 0.950_47;
const YN: f64 = 1.0;
const ZN: f64 = 1.088_83;

/// sRGB (8-bit) → CIE XYZ (D65), each component roughly `0..1`.
fn rgb_to_xyz(rgb: Rgb) -> (f64, f64, f64) {
    let r = srgb_to_linear(rgb.r);
    let g = srgb_to_linear(rgb.g);
    let b = srgb_to_linear(rgb.b);
    // sRGB → XYZ (D65) matrix (IEC 61966-2-1).
    let x = r * 0.412_456_4 + g * 0.357_576_1 + b * 0.180_437_5;
    let y = r * 0.212_672_9 + g * 0.715_152_2 + b * 0.072_175_0;
    let z = r * 0.019_333_9 + g * 0.119_192_0 + b * 0.950_304_1;
    (x, y, z)
}

/// `rgb_to_lab`: CIELAB under the D65 white. Returns `(L*, a*, b*)`.
pub fn rgb_to_lab(rgb: Rgb) -> (f64, f64, f64) {
    let (x, y, z) = rgb_to_xyz(rgb);
    // CIE f(t) companding.
    fn f(t: f64) -> f64 {
        const DELTA: f64 = 6.0 / 29.0;
        if t > DELTA * DELTA * DELTA {
            t.cbrt()
        } else {
            t / (3.0 * DELTA * DELTA) + 4.0 / 29.0
        }
    }
    let fx = f(x / XN);
    let fy = f(y / YN);
    let fz = f(z / ZN);
    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let b = 200.0 * (fy - fz);
    (l, a, b)
}

// ---------------------------------------------------------------------------
// CIEDE2000 color difference
// ---------------------------------------------------------------------------

/// CIEDE2000 (ΔE₀₀) between two CIELAB colors. Reference implementation
/// following Sharma, Wu & Dalal (2005).
pub fn ciede2000(lab1: (f64, f64, f64), lab2: (f64, f64, f64)) -> f64 {
    let (l1, a1, b1) = lab1;
    let (l2, a2, b2) = lab2;

    let deg = std::f64::consts::PI / 180.0;
    let rad = 180.0 / std::f64::consts::PI;

    let c1 = (a1 * a1 + b1 * b1).sqrt();
    let c2 = (a2 * a2 + b2 * b2).sqrt();
    let c_bar = (c1 + c2) / 2.0;

    let c_bar7 = c_bar.powi(7);
    let g = 0.5 * (1.0 - (c_bar7 / (c_bar7 + 25f64.powi(7))).sqrt());

    let a1p = (1.0 + g) * a1;
    let a2p = (1.0 + g) * a2;
    let c1p = (a1p * a1p + b1 * b1).sqrt();
    let c2p = (a2p * a2p + b2 * b2).sqrt();

    let hp = |bp: f64, ap: f64| -> f64 {
        if bp == 0.0 && ap == 0.0 {
            0.0
        } else {
            let mut h = bp.atan2(ap) * rad;
            if h < 0.0 {
                h += 360.0;
            }
            h
        }
    };
    let h1p = hp(b1, a1p);
    let h2p = hp(b2, a2p);

    let dl = l2 - l1;
    let dc = c2p - c1p;

    let dhp = if c1p * c2p == 0.0 {
        0.0
    } else {
        let mut d = h2p - h1p;
        if d > 180.0 {
            d -= 360.0;
        } else if d < -180.0 {
            d += 360.0;
        }
        d
    };
    let dh = 2.0 * (c1p * c2p).sqrt() * (dhp / 2.0 * deg).sin();

    let l_bar = (l1 + l2) / 2.0;
    let c_bar_p = (c1p + c2p) / 2.0;

    let h_bar_p = if c1p * c2p == 0.0 {
        h1p + h2p
    } else if (h1p - h2p).abs() <= 180.0 {
        (h1p + h2p) / 2.0
    } else if h1p + h2p < 360.0 {
        (h1p + h2p + 360.0) / 2.0
    } else {
        (h1p + h2p - 360.0) / 2.0
    };

    let t = 1.0 - 0.17 * ((h_bar_p - 30.0) * deg).cos()
        + 0.24 * ((2.0 * h_bar_p) * deg).cos()
        + 0.32 * ((3.0 * h_bar_p + 6.0) * deg).cos()
        - 0.20 * ((4.0 * h_bar_p - 63.0) * deg).cos();

    let d_theta = 30.0 * (-(((h_bar_p - 275.0) / 25.0).powi(2))).exp();
    let c_bar_p7 = c_bar_p.powi(7);
    let rc = 2.0 * (c_bar_p7 / (c_bar_p7 + 25f64.powi(7))).sqrt();
    let rt = -(2.0 * d_theta * deg).sin() * rc;

    let l_bar_m50_sq = (l_bar - 50.0).powi(2);
    let sl = 1.0 + (0.015 * l_bar_m50_sq) / (20.0 + l_bar_m50_sq).sqrt();
    let sc = 1.0 + 0.045 * c_bar_p;
    let sh = 1.0 + 0.015 * c_bar_p * t;

    let kl = 1.0;
    let kc = 1.0;
    let kh = 1.0;

    let term_l = dl / (kl * sl);
    let term_c = dc / (kc * sc);
    let term_h = dh / (kh * sh);

    (term_l * term_l + term_c * term_c + term_h * term_h + rt * term_c * term_h).sqrt()
}

/// ΔE₀₀ between two RGB colors.
pub fn delta_e(a: Rgb, b: Rgb) -> f64 {
    ciede2000(rgb_to_lab(a), rgb_to_lab(b))
}

// ---------------------------------------------------------------------------
// WCAG luminance / contrast
// ---------------------------------------------------------------------------

/// WCAG 2.x relative luminance of an sRGB color, `0.0..=1.0`.
pub fn luminance(rgb: Rgb) -> f64 {
    let r = srgb_to_linear(rgb.r);
    let g = srgb_to_linear(rgb.g);
    let b = srgb_to_linear(rgb.b);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// WCAG 2.x contrast ratio between two colors, `1.0..=21.0`.
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let la = luminance(a);
    let lb = luminance(b);
    let (light, dark) = if la >= lb { (la, lb) } else { (lb, la) };
    (light + 0.05) / (dark + 0.05)
}

/// WCAG conformance level for *normal text* given a contrast ratio:
/// `AAA` (≥7), `AA` (≥4.5), `AA Large` (≥3, large-text only), else `fail`.
pub fn wcag_level(ratio: f64) -> &'static str {
    if ratio >= 7.0 {
        "AAA"
    } else if ratio >= 4.5 {
        "AA"
    } else if ratio >= 3.0 {
        "AA Large"
    } else {
        "fail"
    }
}

/// A color is "dark" when its WCAG relative luminance is below `0.5`.
pub fn is_dark(rgb: Rgb) -> bool {
    luminance(rgb) < 0.5
}

// ---------------------------------------------------------------------------
// CSS named colors
// ---------------------------------------------------------------------------

/// The CSS Color Module Level 4 named-color table: `(name, r, g, b)`.
pub const NAMED_COLORS: &[(&str, u8, u8, u8)] = &[
    ("aliceblue", 240, 248, 255),
    ("antiquewhite", 250, 235, 215),
    ("aqua", 0, 255, 255),
    ("aquamarine", 127, 255, 212),
    ("azure", 240, 255, 255),
    ("beige", 245, 245, 220),
    ("bisque", 255, 228, 196),
    ("black", 0, 0, 0),
    ("blanchedalmond", 255, 235, 205),
    ("blue", 0, 0, 255),
    ("blueviolet", 138, 43, 226),
    ("brown", 165, 42, 42),
    ("burlywood", 222, 184, 135),
    ("cadetblue", 95, 158, 160),
    ("chartreuse", 127, 255, 0),
    ("chocolate", 210, 105, 30),
    ("coral", 255, 127, 80),
    ("cornflowerblue", 100, 149, 237),
    ("cornsilk", 255, 248, 220),
    ("crimson", 220, 20, 60),
    ("cyan", 0, 255, 255),
    ("darkblue", 0, 0, 139),
    ("darkcyan", 0, 139, 139),
    ("darkgoldenrod", 184, 134, 11),
    ("darkgray", 169, 169, 169),
    ("darkgreen", 0, 100, 0),
    ("darkkhaki", 189, 183, 107),
    ("darkmagenta", 139, 0, 139),
    ("darkolivegreen", 85, 107, 47),
    ("darkorange", 255, 140, 0),
    ("darkorchid", 153, 50, 204),
    ("darkred", 139, 0, 0),
    ("darksalmon", 233, 150, 122),
    ("darkseagreen", 143, 188, 143),
    ("darkslateblue", 72, 61, 139),
    ("darkslategray", 47, 79, 79),
    ("darkturquoise", 0, 206, 209),
    ("darkviolet", 148, 0, 211),
    ("deeppink", 255, 20, 147),
    ("deepskyblue", 0, 191, 255),
    ("dimgray", 105, 105, 105),
    ("dodgerblue", 30, 144, 255),
    ("firebrick", 178, 34, 34),
    ("floralwhite", 255, 250, 240),
    ("forestgreen", 34, 139, 34),
    ("fuchsia", 255, 0, 255),
    ("gainsboro", 220, 220, 220),
    ("ghostwhite", 248, 248, 255),
    ("gold", 255, 215, 0),
    ("goldenrod", 218, 165, 32),
    ("gray", 128, 128, 128),
    ("green", 0, 128, 0),
    ("greenyellow", 173, 255, 47),
    ("honeydew", 240, 255, 240),
    ("hotpink", 255, 105, 180),
    ("indianred", 205, 92, 92),
    ("indigo", 75, 0, 130),
    ("ivory", 255, 255, 240),
    ("khaki", 240, 230, 140),
    ("lavender", 230, 230, 250),
    ("lavenderblush", 255, 240, 245),
    ("lawngreen", 124, 252, 0),
    ("lemonchiffon", 255, 250, 205),
    ("lightblue", 173, 216, 230),
    ("lightcoral", 240, 128, 128),
    ("lightcyan", 224, 255, 255),
    ("lightgoldenrodyellow", 250, 250, 210),
    ("lightgray", 211, 211, 211),
    ("lightgreen", 144, 238, 144),
    ("lightpink", 255, 182, 193),
    ("lightsalmon", 255, 160, 122),
    ("lightseagreen", 32, 178, 170),
    ("lightskyblue", 135, 206, 250),
    ("lightslategray", 119, 136, 153),
    ("lightsteelblue", 176, 196, 222),
    ("lightyellow", 255, 255, 224),
    ("lime", 0, 255, 0),
    ("limegreen", 50, 205, 50),
    ("linen", 250, 240, 230),
    ("magenta", 255, 0, 255),
    ("maroon", 128, 0, 0),
    ("mediumaquamarine", 102, 205, 170),
    ("mediumblue", 0, 0, 205),
    ("mediumorchid", 186, 85, 211),
    ("mediumpurple", 147, 112, 219),
    ("mediumseagreen", 60, 179, 113),
    ("mediumslateblue", 123, 104, 238),
    ("mediumspringgreen", 0, 250, 154),
    ("mediumturquoise", 72, 209, 204),
    ("mediumvioletred", 199, 21, 133),
    ("midnightblue", 25, 25, 112),
    ("mintcream", 245, 255, 250),
    ("mistyrose", 255, 228, 225),
    ("moccasin", 255, 228, 181),
    ("navajowhite", 255, 222, 173),
    ("navy", 0, 0, 128),
    ("oldlace", 253, 245, 230),
    ("olive", 128, 128, 0),
    ("olivedrab", 107, 142, 35),
    ("orange", 255, 165, 0),
    ("orangered", 255, 69, 0),
    ("orchid", 218, 112, 214),
    ("palegoldenrod", 238, 232, 170),
    ("palegreen", 152, 251, 152),
    ("paleturquoise", 175, 238, 238),
    ("palevioletred", 219, 112, 147),
    ("papayawhip", 255, 239, 213),
    ("peachpuff", 255, 218, 185),
    ("peru", 205, 133, 63),
    ("pink", 255, 192, 203),
    ("plum", 221, 160, 221),
    ("powderblue", 176, 224, 230),
    ("purple", 128, 0, 128),
    ("rebeccapurple", 102, 51, 153),
    ("red", 255, 0, 0),
    ("rosybrown", 188, 143, 143),
    ("royalblue", 65, 105, 225),
    ("saddlebrown", 139, 69, 19),
    ("salmon", 250, 128, 114),
    ("sandybrown", 244, 164, 96),
    ("seagreen", 46, 139, 87),
    ("seashell", 255, 245, 238),
    ("sienna", 160, 82, 45),
    ("silver", 192, 192, 192),
    ("skyblue", 135, 206, 235),
    ("slateblue", 106, 90, 205),
    ("slategray", 112, 128, 144),
    ("snow", 255, 250, 250),
    ("springgreen", 0, 255, 127),
    ("steelblue", 70, 130, 180),
    ("tan", 210, 180, 140),
    ("teal", 0, 128, 128),
    ("thistle", 216, 191, 216),
    ("tomato", 255, 99, 71),
    ("turquoise", 64, 224, 208),
    ("violet", 238, 130, 238),
    ("wheat", 245, 222, 179),
    ("white", 255, 255, 255),
    ("whitesmoke", 245, 245, 245),
    ("yellow", 255, 255, 0),
    ("yellowgreen", 154, 205, 50),
];

/// Closest CSS named color to `rgb` by CIEDE2000 ΔE. Ties resolve to the first in
/// table order. The table is never empty, so this always returns a name.
pub fn nearest_color_name(rgb: Rgb) -> &'static str {
    let lab = rgb_to_lab(rgb);
    let mut best = NAMED_COLORS[0].0;
    let mut best_d = f64::INFINITY;
    for &(name, r, g, b) in NAMED_COLORS {
        let d = ciede2000(lab, rgb_to_lab(Rgb::new(r, g, b)));
        if d < best_d {
            best_d = d;
            best = name;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} != {b} (tol {tol})");
    }

    #[test]
    fn hex_roundtrip() {
        assert_eq!(to_hex(255, 0, 0), "#ff0000");
        assert_eq!(from_hex("#ff0000"), Some(Rgb::new(255, 0, 0)));
        assert_eq!(from_hex("ff0000"), Some(Rgb::new(255, 0, 0)));
        assert_eq!(from_hex("#f00"), Some(Rgb::new(255, 0, 0)));
        assert_eq!(from_hex("#ff0000ff"), Some(Rgb::new(255, 0, 0)));
        assert_eq!(from_hex("#abc"), Some(Rgb::new(170, 187, 204)));
    }

    #[test]
    fn invalid_hex_is_none_not_panic() {
        assert_eq!(from_hex("nothex"), None);
        assert_eq!(from_hex("#12"), None);
        assert_eq!(from_hex("#1234"), None);
        assert_eq!(from_hex("#gggggg"), None);
        assert_eq!(from_hex(""), None);
        assert_eq!(from_hex("#"), None);
    }

    #[test]
    fn red_to_hsl() {
        let (h, s, l) = rgb_to_hsl(Rgb::new(255, 0, 0));
        close(h, 0.0, 1e-9);
        close(s, 1.0, 1e-9);
        close(l, 0.5, 1e-9);
    }

    #[test]
    fn hsl_roundtrips_primaries() {
        for c in [
            Rgb::new(255, 0, 0),
            Rgb::new(0, 255, 0),
            Rgb::new(0, 0, 255),
            Rgb::new(128, 64, 200),
            Rgb::new(255, 255, 255),
            Rgb::new(0, 0, 0),
        ] {
            let (h, s, l) = rgb_to_hsl(c);
            assert_eq!(hsl_to_rgb(h, s, l), c, "roundtrip {c:?}");
        }
    }

    #[test]
    fn lab_known_values() {
        // White → L*≈100, a*≈0, b*≈0. Red → L*≈53.24, a*≈80.09, b*≈67.20.
        let (l, a, b) = rgb_to_lab(Rgb::new(255, 255, 255));
        close(l, 100.0, 0.01);
        close(a, 0.0, 0.01);
        close(b, 0.0, 0.01);
        let (l, a, b) = rgb_to_lab(Rgb::new(255, 0, 0));
        close(l, 53.24, 0.05);
        close(a, 80.09, 0.05);
        close(b, 67.20, 0.05);
    }

    #[test]
    fn contrast_extremes() {
        close(
            contrast_ratio(from_hex("#000000").unwrap(), from_hex("#ffffff").unwrap()),
            21.0,
            1e-9,
        );
        close(
            contrast_ratio(from_hex("#ffffff").unwrap(), from_hex("#ffffff").unwrap()),
            1.0,
            1e-9,
        );
    }

    #[test]
    fn luminance_extremes() {
        close(luminance(Rgb::new(0, 0, 0)), 0.0, 1e-12);
        close(luminance(Rgb::new(255, 255, 255)), 1.0, 1e-12);
    }

    #[test]
    fn wcag_levels() {
        assert_eq!(
            wcag_level(contrast_ratio(
                from_hex("#000000").unwrap(),
                from_hex("#ffffff").unwrap()
            )),
            "AAA"
        );
        assert_eq!(wcag_level(21.0), "AAA");
        assert_eq!(wcag_level(5.0), "AA");
        assert_eq!(wcag_level(3.5), "AA Large");
        assert_eq!(wcag_level(1.5), "fail");
    }

    #[test]
    fn delta_e_zero_and_nonzero() {
        close(
            delta_e(Rgb::new(255, 0, 0), Rgb::new(255, 0, 0)),
            0.0,
            1e-12,
        );
        // Red vs green is a large, well-separated difference.
        let d = delta_e(Rgb::new(255, 0, 0), Rgb::new(0, 255, 0));
        assert!(d > 80.0, "red vs green ΔE should be large, got {d}");
    }

    #[test]
    fn ciede2000_sharma_reference() {
        // Sharma, Wu & Dalal (2005) reference test pairs.
        // Pair #1: ΔE00 = 2.0425.
        close(
            ciede2000((50.0, 2.6772, -79.7751), (50.0, 0.0, -82.7485)),
            2.0425,
            1e-4,
        );
        // Pair #2: ΔE00 = 2.8615.
        close(
            ciede2000((50.0, 3.1571, -77.2803), (50.0, 0.0, -82.7485)),
            2.8615,
            1e-4,
        );
        // Lightness-only difference reduces to ΔL/S_L; black↔white (L 0↔100,
        // a=b=0) gives S_L=1 and so ΔE00 = 100.0 exactly.
        close(ciede2000((0.0, 0.0, 0.0), (100.0, 0.0, 0.0)), 100.0, 1e-9);
    }

    #[test]
    fn is_dark_basic() {
        assert!(is_dark(Rgb::new(0, 0, 0)));
        assert!(!is_dark(Rgb::new(255, 255, 255)));
    }

    #[test]
    fn nearest_names() {
        assert_eq!(nearest_color_name(Rgb::new(255, 0, 0)), "red");
        assert_eq!(nearest_color_name(Rgb::new(0, 0, 0)), "black");
        assert_eq!(nearest_color_name(Rgb::new(255, 255, 255)), "white");
        // A near-red maps to red.
        assert_eq!(nearest_color_name(Rgb::new(250, 5, 5)), "red");
    }
}
