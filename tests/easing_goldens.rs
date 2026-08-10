//! Bit-exact comparison of every easing function against CPython-generated
//! goldens (tools/goldens/gen_easing.py). A mismatch prints the first diverging
//! sample with both bit patterns.

use ttfx::utils::easing::Easing;

const EASING_GOLDEN_ORDER: &[Easing] = &[
    Easing::Linear,
    Easing::InSine,
    Easing::OutSine,
    Easing::InOutSine,
    Easing::InQuad,
    Easing::OutQuad,
    Easing::InOutQuad,
    Easing::InCubic,
    Easing::OutCubic,
    Easing::InOutCubic,
    Easing::InQuart,
    Easing::OutQuart,
    Easing::InOutQuart,
    Easing::InQuint,
    Easing::OutQuint,
    Easing::InOutQuint,
    Easing::InExpo,
    Easing::OutExpo,
    Easing::InOutExpo,
    Easing::InCirc,
    Easing::OutCirc,
    Easing::InOutCirc,
    Easing::InBack,
    Easing::OutBack,
    Easing::InOutBack,
    Easing::InElastic,
    Easing::OutElastic,
    Easing::InOutElastic,
    Easing::InBounce,
    Easing::OutBounce,
    Easing::InOutBounce,
    Easing::CubicBezier(0.25, 0.1, 0.25, 1.0),
    Easing::CubicBezier(0.42, 0.0, 0.58, 1.0),
    Easing::CubicBezier(0.68, -0.55, 0.265, 1.55),
];

#[test]
fn easing_matches_python_bit_exactly() {
    let data = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/easing_goldens.bin"))
        .expect("run tools/goldens/gen_easing.py first");
    assert_eq!(data.len(), EASING_GOLDEN_ORDER.len() * 1001 * 8);
    let mut offset = 0;
    let mut mismatches = 0;
    for easing in EASING_GOLDEN_ORDER {
        for i in 0..=1000 {
            let expected = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            offset += 8;
            let p = i as f64 / 1000.0;
            let actual = easing.ease(p);
            // Bit-exactness is only promised on the pinned parity platform
            // (Linux/glibc — plan.md §9 "pin the parity platform"). Elsewhere
            // the system libm rounds sin/cos/pow a last-ulp differently
            // (measured max 2.3e-16 absolute on macOS), so other platforms get
            // the boundary-tolerant assertion instead. Quantized effect output
            // absorbs this either way.
            let within_tolerance = if cfg!(target_os = "linux") {
                // CubicBezier tolerates 1 ulp even here: in optimized builds
                // LLVM const-folds some powf calls, which can differ from
                // runtime libm by an ulp.
                let tolerance = if matches!(easing, Easing::CubicBezier(..)) { 1 } else { 0 };
                actual.to_bits().abs_diff(expected.to_bits()) <= tolerance
            } else {
                (actual - expected).abs() <= 1e-15
            };
            if !within_tolerance {
                if mismatches < 5 {
                    eprintln!(
                        "{easing:?} at p={p}: expected {expected:?} ({:016x}), got {actual:?} ({:016x})",
                        expected.to_bits(),
                        actual.to_bits()
                    );
                }
                mismatches += 1;
            }
        }
    }
    assert_eq!(mismatches, 0, "{mismatches} easing samples diverge from Python");
}
