use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ---------- layout: edge cases ----------

#[wasm_bindgen_test]
fn empty_data_produces_no_layout() {
    assert!(compute_pie_layout(800.0, 600.0, &[]).is_none());
}

#[wasm_bindgen_test]
fn zero_total_produces_no_layout() {
    let data = vec![
        DataPoint::new("A", 0, "#ff0000"),
        DataPoint::new("B", 0, "#00ff00"),
    ];
    assert!(compute_pie_layout(800.0, 600.0, &data).is_none());
}

#[wasm_bindgen_test]
fn tiny_canvas_produces_no_layout() {
    let data = vec![DataPoint::new("A", 10, "#ff0000")];
    // radius = min(8,600)/2 - 5 = -1 (negative)
    assert!(compute_pie_layout(8.0, 600.0, &data).is_none());
    // radius = min(10,10)/2 - 5 = 0 (zero)
    assert!(compute_pie_layout(10.0, 10.0, &data).is_none());
}

#[wasm_bindgen_test]
fn zero_dimensions_produce_no_layout() {
    let data = vec![DataPoint::new("A", 10, "#ff0000")];
    assert!(compute_pie_layout(0.0, 600.0, &data).is_none());
    assert!(compute_pie_layout(800.0, 0.0, &data).is_none());
}

// ---------- layout: geometry ----------

#[wasm_bindgen_test]
fn layout_has_one_slice_per_point_and_preserves_fields() {
    let data = sample_data();
    let layout = compute_pie_layout(800.0, 600.0, &data).unwrap();

    assert_eq!(layout.slices.len(), 4);
    assert_eq!(layout.slices[0].name, "A");
    assert_eq!(layout.slices[0].value, 10);
    assert_eq!(layout.slices[1].name, "B");
    assert_eq!(layout.slices[2].name, "C");
    assert_eq!(layout.slices[3].name, "D");
    assert!((layout.total - 100.0).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn slices_span_a_full_circle_and_are_contiguous() {
    let data = sample_data();
    let layout = compute_pie_layout(800.0, 600.0, &data).unwrap();

    assert!((layout.slices[0].start_angle - 0.0).abs() < 1e-9);
    assert!((layout.slices.last().unwrap().end_angle - 2.0 * PI).abs() < 1e-9);

    for i in 1..layout.slices.len() {
        assert!((layout.slices[i].start_angle - layout.slices[i - 1].end_angle).abs() < 1e-9);
    }
}

#[wasm_bindgen_test]
fn sweep_angles_are_proportional_to_values() {
    let data = sample_data();
    let layout = compute_pie_layout(800.0, 600.0, &data).unwrap();

    let sweep: Vec<f64> = layout
        .slices
        .iter()
        .map(|s| s.end_angle - s.start_angle)
        .collect();

    assert!((sweep[0] - 2.0 * PI * 10.0 / 100.0).abs() < 1e-9);
    assert!((sweep[1] - 2.0 * PI * 20.0 / 100.0).abs() < 1e-9);
    assert!((sweep[2] - 2.0 * PI * 30.0 / 100.0).abs() < 1e-9);
    assert!((sweep[3] - 2.0 * PI * 40.0 / 100.0).abs() < 1e-9);
    assert!((sweep.iter().sum::<f64>() - 2.0 * PI).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn center_and_radius_match_canvas_dimensions() {
    let data = sample_data();
    let layout = compute_pie_layout(800.0, 600.0, &data).unwrap();

    assert!((layout.center_x - 400.0).abs() < 1e-9);
    assert!((layout.center_y - 300.0).abs() < 1e-9);
    // min(800, 600)/2 - 5 = 295
    assert!((layout.radius - 295.0).abs() < 1e-9);
}

// ---------- hit testing ----------

fn sample_data() -> Vec<DataPoint> {
    vec![
        DataPoint::new("A", 10, "#ff0000"),
        DataPoint::new("B", 20, "#00ff00"),
        DataPoint::new("C", 30, "#0000ff"),
        DataPoint::new("D", 40, "#ffff00"),
    ]
}

#[wasm_bindgen_test]
fn hit_testing_finds_correct_slice_in_each_quadrant() {
    // Four equal slices → A: [0, π/2], B: [π/2, π], C: [π, 3π/2], D: [3π/2, 2π]
    let data = vec![
        DataPoint::new("A", 25, "#ff0000"),
        DataPoint::new("B", 25, "#00ff00"),
        DataPoint::new("C", 25, "#0000ff"),
        DataPoint::new("D", 25, "#ffff00"),
    ];
    let layout = compute_pie_layout(800.0, 600.0, &data).unwrap();
    let slices = &layout.slices;

    let cx = layout.center_x;
    let cy = layout.center_y;
    let r = layout.radius / 2.0;
    let d = r * std::f64::consts::FRAC_1_SQRT_2;

    // 45° (bottom-right in screen coords) → A
    let found = find_slice_at(slices, cx + d, cy + d);
    assert_eq!(found.map(|s| s.name.as_str()), Some("A"));

    // 135° (bottom-left) → B
    let found = find_slice_at(slices, cx - d, cy + d);
    assert_eq!(found.map(|s| s.name.as_str()), Some("B"));

    // 225° (top-left) → C
    let found = find_slice_at(slices, cx - d, cy - d);
    assert_eq!(found.map(|s| s.name.as_str()), Some("C"));

    // 315° (top-right) → D
    let found = find_slice_at(slices, cx + d, cy - d);
    assert_eq!(found.map(|s| s.name.as_str()), Some("D"));
}

#[wasm_bindgen_test]
fn hit_testing_rejects_points_outside_radius() {
    let data = sample_data();
    let layout = compute_pie_layout(800.0, 600.0, &data).unwrap();

    let found = find_slice_at(
        &layout.slices,
        layout.center_x + layout.radius + 10.0,
        layout.center_y,
    );
    assert!(found.is_none());
}

#[wasm_bindgen_test]
fn hit_testing_at_shared_boundary_matches_first_slice() {
    // Two equal halves → A: [0, π], B: [π, 2π]
    let data = vec![
        DataPoint::new("A", 50, "#ff0000"),
        DataPoint::new("B", 50, "#00ff00"),
    ];
    let layout = compute_pie_layout(800.0, 600.0, &data).unwrap();

    // Bottom of canvas (angle = π/2): inside A only.
    let found = find_slice_at(
        &layout.slices,
        layout.center_x,
        layout.center_y + layout.radius / 2.0,
    );
    assert_eq!(found.map(|s| s.name.as_str()), Some("A"));

    // Left of canvas (angle = π, shared boundary): first match wins → A.
    let found = find_slice_at(
        &layout.slices,
        layout.center_x - layout.radius / 2.0,
        layout.center_y,
    );
    assert_eq!(found.map(|s| s.name.as_str()), Some("A"));
}

// ---------- tooltip ----------

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
fn test_draw_pie_chart() {
    let Some(context) = mock_context() else {
        return;
    };

    let data = sample_data();
    let slices = draw_pie_chart(&context, 800.0, 600.0, &data);

    assert_eq!(slices.len(), 4);
    assert_eq!(slices[0].name, "A");
    assert_eq!(slices[3].name, "D");
    assert_eq!(slices[3].value, 40);
}
