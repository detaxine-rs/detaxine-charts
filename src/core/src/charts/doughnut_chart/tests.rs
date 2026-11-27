use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ---------- layout tests ----------

#[wasm_bindgen_test]
fn empty_data_produces_no_layout() {
    assert!(compute_doughnut_layout(500.0, 500.0, &[]).is_none());
}

#[wasm_bindgen_test]
fn zero_total_produces_no_layout() {
    let data = vec![
        ("A".to_string(), 0, "#ff0000".to_string()),
        ("B".to_string(), 0, "#00ff00".to_string()),
    ];
    assert!(compute_doughnut_layout(500.0, 500.0, &data).is_none());
}

#[wasm_bindgen_test]
fn layout_has_one_segment_per_point_and_preserves_fields() {
    let data = vec![
        ("A".to_string(), 10, "#ff0000".to_string()),
        ("B".to_string(), 20, "#00ff00".to_string()),
        ("C".to_string(), 30, "#0000ff".to_string()),
    ];
    let layout = compute_doughnut_layout(500.0, 500.0, &data).unwrap();

    assert_eq!(layout.segments.len(), 3);
    assert_eq!(layout.segments[0].label, "A");
    assert_eq!(layout.segments[0].value, 10);
    assert_eq!(layout.segments[0].color, "#ff0000");
    assert_eq!(layout.segments[1].label, "B");
    assert_eq!(layout.segments[2].label, "C");
    assert!((layout.total - 60.0).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn segments_span_a_full_circle_and_are_contiguous() {
    let data = vec![
        ("A".to_string(), 10, "#ff0000".to_string()),
        ("B".to_string(), 20, "#00ff00".to_string()),
        ("C".to_string(), 30, "#0000ff".to_string()),
    ];
    let layout = compute_doughnut_layout(500.0, 500.0, &data).unwrap();

    assert!((layout.segments[0].start_angle + PI / 2.0).abs() < 1e-9);
    assert!((layout.segments.last().unwrap().end_angle - 3.0 * PI / 2.0).abs() < 1e-9);

    for i in 1..layout.segments.len() {
        assert!((layout.segments[i].start_angle - layout.segments[i - 1].end_angle).abs() < 1e-9);
    }
}

#[wasm_bindgen_test]
fn sweep_angles_are_proportional_to_values() {
    let data = vec![
        ("A".to_string(), 10, "#ff0000".to_string()),
        ("B".to_string(), 20, "#00ff00".to_string()),
        ("C".to_string(), 30, "#0000ff".to_string()),
    ];
    let layout = compute_doughnut_layout(500.0, 500.0, &data).unwrap();

    let sweep: Vec<f64> = layout
        .segments
        .iter()
        .map(|s| s.end_angle - s.start_angle)
        .collect();

    assert!((sweep[0] - 2.0 * PI * 10.0 / 60.0).abs() < 1e-9);
    assert!((sweep[1] - 2.0 * PI * 20.0 / 60.0).abs() < 1e-9);
    assert!((sweep[2] - 2.0 * PI * 30.0 / 60.0).abs() < 1e-9);
    assert!((sweep.iter().sum::<f64>() - 2.0 * PI).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn center_and_radius_match_canvas_dimensions() {
    let data = vec![
        ("A".to_string(), 10, "#ff0000".to_string()),
        ("B".to_string(), 20, "#00ff00".to_string()),
    ];
    let layout = compute_doughnut_layout(500.0, 400.0, &data).unwrap();

    assert!((layout.center_x - 250.0).abs() < 1e-9);
    assert!((layout.center_y - 200.0).abs() < 1e-9);
    // min(500, 400)/2 * 0.85 = 170, inner = 85
    assert!((layout.radius - 170.0).abs() < 1e-9);
    assert!((layout.inner_radius - 85.0).abs() < 1e-9);
}

// ---------- interaction tests ----------

#[wasm_bindgen_test]
fn hit_testing_finds_the_correct_segment() {
    // Two equal halves: A is the right half, B is the left half.
    let data = vec![
        ("A".to_string(), 10, "#ff0000".to_string()),
        ("B".to_string(), 10, "#00ff00".to_string()),
    ];
    let layout = compute_doughnut_layout(500.0, 500.0, &data).unwrap();
    let segments = &layout.segments;

    let cx = layout.center_x;
    let cy = layout.center_y;
    let r = (layout.radius + layout.inner_radius) / 2.0;

    // Right of center: atan2(0, +r) = 0 → segment A [-PI/2, PI/2]
    let found = find_segment_at(segments, cx + r, cy);
    assert_eq!(found.map(|s| s.label.as_str()), Some("A"));

    // Left of center: atan2(0, -r) = PI → segment B [PI/2, 3PI/2]
    let found = find_segment_at(segments, cx - r, cy);
    assert_eq!(found.map(|s| s.label.as_str()), Some("B"));
}

#[wasm_bindgen_test]
fn hit_testing_rejects_points_in_the_hole() {
    let data = vec![("A".to_string(), 10, "#ff0000".to_string())];
    let layout = compute_doughnut_layout(500.0, 500.0, &data).unwrap();

    // Center is well inside inner_radius
    let found = find_segment_at(&layout.segments, layout.center_x, layout.center_y);
    assert!(found.is_none());
}

#[wasm_bindgen_test]
fn hit_testing_rejects_points_outside_radius() {
    let data = vec![("A".to_string(), 10, "#ff0000".to_string())];
    let layout = compute_doughnut_layout(500.0, 500.0, &data).unwrap();

    let found = find_segment_at(
        &layout.segments,
        layout.center_x + layout.radius + 10.0,
        layout.center_y,
    );
    assert!(found.is_none());
}

#[wasm_bindgen_test]
fn tooltip_flips_when_it_would_overflow() {
    assert_eq!(tooltip_left(100.0, 50.0, 500.0, 10.0), 110.0);
    assert_eq!(tooltip_left(470.0, 50.0, 500.0, 10.0), 410.0);
    assert_eq!(tooltip_left(10.0, 100.0, 50.0, 10.0), 0.0);
}

// ---------- browser smoke test ----------

fn mock_context() -> Option<CanvasRenderingContext2d> {
    let document = web_sys::window()?.document()?;
    let canvas = document
        .create_element("canvas")
        .ok()?
        .dyn_into::<HtmlCanvasElement>()
        .ok()?;
    get_context(&canvas)
}

#[wasm_bindgen_test]
fn test_draw_doughnut_chart() {
    let Some(context) = mock_context() else {
        return;
    };

    let data = vec![
        ("A".to_string(), 10, "#ff0000".to_string()),
        ("B".to_string(), 20, "#00ff00".to_string()),
        ("C".to_string(), 30, "#0000ff".to_string()),
    ];

    let segments = draw_doughnut_chart(&context, 500.0, 500.0, &data);

    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0].label, "A");
    assert_eq!(segments[2].label, "C");
    assert_eq!(segments[2].value, 30);
}
