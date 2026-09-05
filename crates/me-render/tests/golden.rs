//! Golden (snapshot) test for the deterministic typesetting plans.
//! Regenerate via: cargo run -p me-render --example write_golden

use me_render::{fit_font_size, LayoutInput, TextStyle};

fn plan_json(name: &str, input: &LayoutInput) -> String {
    let plan = fit_font_size(input, 8, 48);
    format!("// {}\n{}", name, serde_json::to_string_pretty(&plan).unwrap())
}

#[test]
fn golden_layout_plans() {
    let cases: Vec<(&str, LayoutInput)> = vec![
        (
            "horizontal_cjk",
            LayoutInput {
                text: "こんにちは、世界！".into(),
                box_w: 220,
                box_h: 90,
                style: TextStyle::default(),
            },
        ),
        (
            "vertical_japanese",
            LayoutInput {
                text: "またね、またね".into(),
                box_w: 90,
                box_h: 200,
                style: TextStyle {
                    vertical: true,
                    ..Default::default()
                },
            },
        ),
        (
            "long_latin_wrap",
            LayoutInput {
                text: "The quick brown fox jumps over the lazy dog again and again".into(),
                box_w: 180,
                box_h: 120,
                style: TextStyle::default(),
            },
        ),
    ];
    let mut all = String::new();
    for (name, input) in &cases {
        all.push_str(&plan_json(name, input));
        all.push('\n');
    }
    let snap_path = format!("{}/tests/snapshots/plans.txt", env!("CARGO_MANIFEST_DIR"));
    // git autocrlf may check the snapshot out with CRLF on Windows
    let expected = std::fs::read_to_string(&snap_path)
        .map(|e| e.replace("\r\n", "\n"))
        .unwrap_or_else(|_| {
            panic!(
                "missing snapshot {snap_path}; regenerate via: cargo run -p me-render --example write_golden"
            )
        });
    assert_eq!(
        all, expected,
        "layout plan changed; review and regenerate via: cargo run -p me-render --example write_golden"
    );
}
