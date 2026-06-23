//! Integration tests for the pure color-science engine against known values.
//! These exercise `color-worker`'s public API the same way the Arrow adapters do,
//! but without any Arrow/RPC plumbing.

use color_worker::color::{self, Rgb};

fn close(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() <= tol, "{a} != {b} (tol {tol})");
}

#[test]
fn hex_to_from_roundtrip() {
    assert_eq!(color::to_hex(255, 0, 0), "#ff0000");
    assert_eq!(color::from_hex("#ff0000"), Some(Rgb::new(255, 0, 0)));
    assert_eq!(color::from_hex("#FF0000"), Some(Rgb::new(255, 0, 0)));
    assert_eq!(color::from_hex("#f00"), Some(Rgb::new(255, 0, 0)));
    assert_eq!(color::from_hex("#ff000080"), Some(Rgb::new(255, 0, 0)));
    // round trip via the worker's own functions
    let c = color::from_hex("#3a7bd5").unwrap();
    assert_eq!(color::to_hex(c.r, c.g, c.b), "#3a7bd5");
}

#[test]
fn invalid_hex_yields_none() {
    for s in ["", "#", "zzz", "#12", "#1234", "#gggggg", "purple"] {
        assert_eq!(color::from_hex(s), None, "{s} should be None");
    }
}

#[test]
fn red_rgb_to_hsl() {
    let (h, s, l) = color::rgb_to_hsl(Rgb::new(255, 0, 0));
    close(h, 0.0, 1e-9);
    close(s, 1.0, 1e-9);
    close(l, 0.5, 1e-9);
}

#[test]
fn contrast_black_white_is_21() {
    let black = color::from_hex("#000000").unwrap();
    let white = color::from_hex("#ffffff").unwrap();
    close(color::contrast_ratio(black, white), 21.0, 1e-9);
    close(color::contrast_ratio(white, white), 1.0, 1e-9);
}

#[test]
fn wcag_level_black_on_white_is_aaa() {
    let black = color::from_hex("#000000").unwrap();
    let white = color::from_hex("#ffffff").unwrap();
    assert_eq!(
        color::wcag_level(color::contrast_ratio(black, white)),
        "AAA"
    );
}

#[test]
fn delta_e_identity_zero_and_known_pair() {
    let red = color::from_hex("#ff0000").unwrap();
    close(color::delta_e(red, red), 0.0, 1e-12);
    // Known nonzero pair: pure red vs pure blue is a large difference.
    let blue = color::from_hex("#0000ff").unwrap();
    let d = color::delta_e(red, blue);
    assert!(d > 40.0, "red vs blue ΔE should be large, got {d}");
}

#[test]
fn ciede2000_sharma_reference_pair() {
    // Sharma et al. (2005) reference test pair #1 → ΔE00 = 2.0425.
    let d = color::ciede2000((50.0, 2.6772, -79.7751), (50.0, 0.0, -82.7485));
    close(d, 2.0425, 1e-4);
}

#[test]
fn luminance_extremes() {
    close(color::luminance(Rgb::new(0, 0, 0)), 0.0, 1e-12);
    close(color::luminance(Rgb::new(255, 255, 255)), 1.0, 1e-12);
}

#[test]
fn nearest_color_name_red() {
    assert_eq!(color::nearest_color_name(Rgb::new(255, 0, 0)), "red");
    assert_eq!(color::nearest_color_name(Rgb::new(0, 0, 0)), "black");
}

#[test]
fn named_table_is_populated() {
    assert!(color::NAMED_COLORS.len() > 100);
    // 'red' is present and exactly #ff0000.
    let red = color::NAMED_COLORS
        .iter()
        .find(|(n, ..)| *n == "red")
        .unwrap();
    assert_eq!(color::to_hex(red.1, red.2, red.3), "#ff0000");
}
