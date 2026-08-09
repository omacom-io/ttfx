//! Gradient/color goldens vs CPython (tools/goldens/gen_graphics.py).

use ttfx::utils::graphics::{shift_color_towards, Color, Gradient, GradientDirection};

fn generate_lines() -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    let grad_cases: Vec<(Vec<&str>, Vec<i64>, bool)> = vec![
        (vec!["8A008A", "00D1FF", "FFFFFF"], vec![12], false),
        (vec!["8A008A", "00D1FF", "FFFFFF"], vec![6, 3], false),
        (vec!["ffffff", "000000"], vec![10], false),
        (vec!["000000", "ffffff"], vec![7], false),
        (vec!["ff0000", "00ff00", "0000ff"], vec![5], true),
        (vec!["123456"], vec![4], false),
        (vec!["ff5733", "33ff57", "5733ff", "f0f0f0"], vec![3, 9], false),
        (vec!["0a0b0c", "f1e2d3"], vec![1], false),
    ];
    for (stops, steps, do_loop) in &grad_cases {
        let colors: Vec<Color> = stops.iter().map(|s| Color::from_hex(s).unwrap()).collect();
        // tuple-shaped steps in the Python generator (never scalar), so skip
        // the int-only validation like upstream does for tuples
        let g = Gradient::new(&colors, steps, false, *do_loop).unwrap();
        let steps_repr = if steps.len() == 1 {
            format!("({},)", steps[0])
        } else {
            format!("({})", steps.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", "))
        };
        let py_loop = if *do_loop { "True" } else { "False" };
        lines.push(format!(
            "grad {} s={steps_repr} loop={py_loop}: {}",
            stops.join("+"),
            g.spectrum.iter().map(|c| c.rgb_color.clone()).collect::<Vec<_>>().join(";")
        ));
    }

    let colors: Vec<Color> = ["8A008A", "00D1FF", "FFFFFF"].iter().map(|s| Color::from_hex(s).unwrap()).collect();
    let g = Gradient::with_steps(&colors, 12, false).unwrap();
    for i in 0..=20 {
        let f = i as f64 / 20.0;
        let f_label = if f == f.trunc() { format!("{f:.1}") } else { format!("{f}") };
        lines.push(format!("frac {f_label}: {}", g.get_color_at_fraction(f).unwrap().rgb_color));
    }

    for (name, direction) in [
        ("VERTICAL", GradientDirection::Vertical),
        ("HORIZONTAL", GradientDirection::Horizontal),
        ("RADIAL", GradientDirection::Radial),
        ("DIAGONAL", GradientDirection::Diagonal),
    ] {
        for (label, (min_row, max_row, min_column, max_column)) in
            [("mapping", (1, 5, 1, 8)), ("mapping_offset", (2, 6, 3, 9))]
        {
            let mapping = g
                .build_coordinate_color_mapping(min_row, max_row, min_column, max_column, direction)
                .unwrap();
            let entries = mapping
                .iter()
                .map(|(c, col)| format!("{},{}={}", c.column, c.row, col.rgb_color))
                .collect::<Vec<_>>()
                .join(";");
            lines.push(format!("{label} {name}: {entries}"));
        }
    }

    for factor in [0.0, 0.1, 0.25, 0.5, 0.75, 0.99, 1.0] {
        let c = shift_color_towards(
            &Color::from_hex("ff8040").unwrap(),
            &Color::from_hex("103050").unwrap(),
            factor,
        )
        .unwrap();
        let f_label = if factor == factor.trunc() { format!("{factor:.1}") } else { format!("{factor}") };
        lines.push(format!("shift {f_label}: {}", c.rgb_color));
    }

    lines
}

#[test]
fn graphics_matches_python() {
    let fixture = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/graphics_goldens.txt"
    ))
    .expect("run tools/goldens/gen_graphics.py first");
    let expected: Vec<&str> = fixture.lines().collect();
    let mut actual_iter = generate_lines().into_iter();
    let mut mismatches = 0;
    for e in &expected {
        let a = actual_iter.next().unwrap_or_default();
        // mapping lines are ordered — compare in order
        if *e != a {
            if mismatches < 5 {
                eprintln!("expected: {e}\n  actual: {a}\n");
            }
            mismatches += 1;
        }
    }
    assert_eq!(mismatches, 0, "{mismatches} graphics golden lines diverge");
}
