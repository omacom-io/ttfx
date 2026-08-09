//! xterm-256 <-> RGB conversion, ported from utils/hexterm.py.

include!("hexterm_table.rs");

/// RGB ints for an xterm code, derived from XTERM_TO_HEX like upstream's xterm_to_rgb_map.
fn xterm_rgb(code: usize) -> (i64, i64, i64) {
    let h = XTERM_TO_HEX[code];
    (
        i64::from_str_radix(&h[0..2], 16).unwrap(),
        i64::from_str_radix(&h[2..4], 16).unwrap(),
        i64::from_str_radix(&h[4..6], 16).unwrap(),
    )
}

/// Closest xterm-256 code by mean absolute channel difference; linear scan over
/// codes 0..=255 in order, strict `<` so the first minimum wins (upstream
/// hexterm.py hex_to_xterm).
pub fn hex_to_xterm(hex_color: &str) -> u8 {
    let s = hex_color.trim_matches('#');
    let r = i64::from_str_radix(&s[0..2], 16).unwrap();
    let g = i64::from_str_radix(&s[2..4], 16).unwrap();
    let b = i64::from_str_radix(&s[4..6], 16).unwrap();
    let mut min_diff = f64::INFINITY;
    let mut closest = 0u8;
    for code in 0..256usize {
        let (xr, xg, xb) = xterm_rgb(code);
        let diff = ((r - xr).abs() + (g - xg).abs() + (b - xb).abs()) as f64 / 3.0;
        if diff < min_diff {
            min_diff = diff;
            closest = code as u8;
        }
    }
    closest
}

/// xterm code -> hex string without leading '#'.
pub fn xterm_to_hex(xterm_color: u8) -> &'static str {
    XTERM_TO_HEX[xterm_color as usize]
}

/// Upstream is_valid_color for strings: 6 (or, faithfully, 7) hex digits with
/// optional leading '#'s. Integer codes are validated by range at the type level (u8).
pub fn is_valid_hex_color(color: &str) -> bool {
    let stripped_len = color.trim_start_matches('#').len();
    if stripped_len != 6 && stripped_len != 7 {
        return false;
    }
    i64::from_str_radix(color.trim_matches('#'), 16).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_spot_checks() {
        // Golden values from the pinned reference
        assert_eq!(xterm_to_hex(0), "000000");
        assert_eq!(xterm_to_hex(15), "ffffff");
        assert_eq!(xterm_to_hex(196), "ff0000");
    }

    #[test]
    fn valid_hex() {
        assert!(is_valid_hex_color("#ff00aa"));
        assert!(is_valid_hex_color("ff00aa"));
        assert!(!is_valid_hex_color("ff00a"));
        assert!(!is_valid_hex_color("gg00aa"));
        // Upstream quirk: 7 hex digits pass validation
        assert!(is_valid_hex_color("1234567"));
    }
}
