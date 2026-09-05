//! Regenerates tests/snapshots/plans.txt: cargo run -p me-render --example write_golden
use me_render::{fit_font_size, LayoutInput, TextStyle};

fn main() {
    let cases: Vec<(&str, LayoutInput)> = vec![
        ("horizontal_cjk", LayoutInput { text: "こんにちは、世界！".into(), box_w: 220, box_h: 90, style: TextStyle::default() }),
        ("vertical_japanese", LayoutInput { text: "またね、またね".into(), box_w: 90, box_h: 200, style: TextStyle { vertical: true, ..Default::default() } }),
        ("long_latin_wrap", LayoutInput { text: "The quick brown fox jumps over the lazy dog again and again".into(), box_w: 180, box_h: 120, style: TextStyle::default() }),
    ];
    let mut all = String::new();
    for (name, input) in &cases {
        let plan = fit_font_size(input, 8, 48);
        all.push_str(&format!("// {}\n{}\n", name, serde_json::to_string_pretty(&plan).unwrap()));
    }
    let path = format!("{}/tests/snapshots/plans.txt", env!("CARGO_MANIFEST_DIR"));
    std::fs::create_dir_all(std::path::Path::new(&path).parent().unwrap()).unwrap();
    std::fs::write(&path, &all).unwrap();
    println!("written {}", path);
}
