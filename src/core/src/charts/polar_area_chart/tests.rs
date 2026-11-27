use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ---------- helpers ----------

fn sample_data() -> Vec<DataPoint> {
    vec![
        DataPoint::new("Mon", 12.0, "#4f46e5"),
        DataPoint::new("Tue", 19.0, "#e11d48"),
        DataPoint::new("Wed", 7.0, "#0891b2"),
        DataPoint::new("Thu", 15.0, "#16a34a"),
        DataPoint::new("Fri", 22.0, "#d97706"),
    ]
}

fn radius_test_data() -> Vec<DataPoint> {
    vec![
        DataPoint::new("A", 100.0, "#000"),
        DataPoint::new("B", 25.0, "#111"),
        DataPoint::new("C", 4.0, "#222"),
    ]
}

fn three_equal_slices() -> Vec<DataPoint> {
    vec![
        DataPoint::new("A", 10.0, "#000"),
        DataPoint::new("B", 10.0, "#111"),
        DataPoint::new("C", 10.0, "#222"),
    ]
}

fn point_at(cx: f64, cy: f64, angle: f64, dist: f64) -> (f64, f64) {
    (cx + dist * angle.cos(), cy + dist * angle.sin())
}

// ---------- layout: edge cases ----------

#[wasm_bindgen_test]
fn empty_data_produces_no_layout() {
    assert!(compute_polar_layout(500.0, 500.0, &[], &PolarAreaChartConfig::default()).is_none());
}

#[wasm_bindgen_test]
fn non_positive_max_produces_no_layout() {
    let zeros = vec![
        DataPoint::new("A", 0.0, "#000"),
        DataPoint::new("B", 0.0, "#111"),
    ];
    let negatives = vec![
        DataPoint::new("A", -5.0, "#000"),
        DataPoint::new("B", -3.0, "#111"),
    ];
    let config = PolarAreaChartConfig::default();
    assert!(compute_polar_layout(500.0, 500.0, &zeros, &config).is_none());
    assert!(compute_polar_layout(500.0, 500.0, &negatives, &config).is_none());
}

#[wasm_bindgen_test]
fn zero_dimensions_produce_no_layout() {
    let config = PolarAreaChartConfig::default();
    assert!(compute_polar_layout(0.0, 500.0, &sample_data(), &config).is_none());
    assert!(compute_polar_layout(500.0, 0.0, &sample_data(), &config).is_none());
}

// ---------- layout: geometry ----------

#[wasm_bindgen_test]
fn layout_has_one_slice_per_point() {
    let layout = compute_polar_layout(
        500.0,
        500.0,
        &sample_data(),
        &PolarAreaChartConfig::default(),
    )
    .unwrap();
    assert_eq!(layout.slices.len(), 5);
    assert_eq!(layout.slices[0].name, "Mon");
    assert_eq!(layout.slices[4].name, "Fri");
    assert!((layout.max_value - 22.0).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn center_and_max_radius_match_canvas() {
    let layout = compute_polar_layout(
        500.0,
        500.0,
        &sample_data(),
        &PolarAreaChartConfig::default(),
    )
    .unwrap();
    assert!((layout.center_x - 250.0).abs() < 1e-9);
    assert!((layout.center_y - 250.0).abs() < 1e-9);
    // label_margin = 500*0.12 = 60 → clamp 40; max_radius = 250 - 40 = 210
    assert!((layout.max_radius - 210.0).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn tiny_canvas_clamps_max_radius_to_floor() {
    let layout =
        compute_polar_layout(20.0, 20.0, &sample_data(), &PolarAreaChartConfig::default()).unwrap();
    // label_margin clamps to 20, radius = 10 - 20 = -10, clamped to 10
    assert!((layout.max_radius - 10.0).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn slices_are_evenly_spaced_in_angle() {
    let layout = compute_polar_layout(
        500.0,
        500.0,
        &sample_data(),
        &PolarAreaChartConfig::default(),
    )
    .unwrap();
    let slice_angle = 2.0 * PI / 5.0;
    let gap = 0.04;

    // Slice width (end - start in unwrapped form) = slice_angle - gap.
    // Check slice 2, 3, 4 (non-wrap) directly.
    for i in 2..5 {
        let start_raw = -PI / 2.0 + i as f64 * slice_angle + gap / 2.0;
        let end_raw = -PI / 2.0 + (i + 1) as f64 * slice_angle - gap / 2.0;
        let sweep = layout.slices[i].end_angle - layout.slices[i].start_angle;
        assert!((sweep - (end_raw - start_raw)).abs() < 1e-9);
    }
}

#[wasm_bindgen_test]
fn first_slice_starts_just_past_twelve_oclock() {
    let layout = compute_polar_layout(
        500.0,
        500.0,
        &sample_data(),
        &PolarAreaChartConfig::default(),
    )
    .unwrap();
    let gap = 0.04;
    let expected_start = normalize_angle(-PI / 2.0 + gap / 2.0, 0.0);
    assert!((layout.slices[0].start_angle - expected_start).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn last_slice_ends_just_before_twelve_oclock() {
    let layout = compute_polar_layout(
        500.0,
        500.0,
        &sample_data(),
        &PolarAreaChartConfig::default(),
    )
    .unwrap();
    let gap = 0.04;
    let expected_end = 3.0 * PI / 2.0 - gap / 2.0;
    assert!((layout.slices[4].end_angle - expected_end).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn slice_radius_scales_as_sqrt_of_value_ratio() {
    let layout = compute_polar_layout(
        500.0,
        500.0,
        &radius_test_data(),
        &PolarAreaChartConfig::default(),
    )
    .unwrap();
    let max_radius = layout.max_radius;

    // 100/100 → radius = max_radius
    assert!((layout.slices[0].slice_radius - max_radius).abs() < 1e-9);
    // 25/100 → sqrt(1/4) = 0.5
    assert!((layout.slices[1].slice_radius - max_radius * 0.5).abs() < 1e-9);
    // 4/100 → sqrt(0.04) = 0.2
    assert!((layout.slices[2].slice_radius - max_radius * 0.2).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn negative_value_becomes_zero_radius_slice() {
    let data = vec![
        DataPoint::new("A", 10.0, "#000"),
        DataPoint::new("B", -5.0, "#111"),
    ];
    let layout =
        compute_polar_layout(500.0, 500.0, &data, &PolarAreaChartConfig::default()).unwrap();
    assert!((layout.slices[1].slice_radius - 0.0).abs() < 1e-9);
}

// ---------- layout: grid rings ----------

#[wasm_bindgen_test]
fn grid_rings_scale_linearly_to_max_value() {
    let layout = compute_polar_layout(
        500.0,
        500.0,
        &sample_data(),
        &PolarAreaChartConfig::default(),
    )
    .unwrap();
    assert_eq!(layout.grid_rings.len(), 4);

    for (i, ring) in layout.grid_rings.iter().enumerate() {
        let frac = (i + 1) as f64 / 4.0;
        assert!((ring.radius - layout.max_radius * frac).abs() < 1e-9);
        assert!((ring.value - layout.max_value * frac).abs() < 1e-9);
    }
}

#[wasm_bindgen_test]
fn grid_ring_count_uses_config_with_floor_of_one() {
    let custom = PolarAreaChartConfig {
        num_grid_rings: 7,
        ..Default::default()
    };
    let layout = compute_polar_layout(500.0, 500.0, &sample_data(), &custom).unwrap();
    assert_eq!(layout.grid_rings.len(), 7);

    let zero = PolarAreaChartConfig {
        num_grid_rings: 0,
        ..Default::default()
    };
    let layout = compute_polar_layout(500.0, 500.0, &sample_data(), &zero).unwrap();
    assert_eq!(layout.grid_rings.len(), 1);
}

// ---------- hit testing ----------

#[wasm_bindgen_test]
fn hit_testing_finds_non_wrap_slice() {
    // Slice 2 (Wed, value 7) does not wrap; angle range [~0.96, ~2.18].
    let layout = compute_polar_layout(
        500.0,
        500.0,
        &sample_data(),
        &PolarAreaChartConfig::default(),
    )
    .unwrap();
    let (x, y) = point_at(layout.center_x, layout.center_y, 1.5, 50.0);
    let found = find_slice_at(&layout.slices, x, y);
    assert_eq!(found.map(|s| s.name.as_str()), Some("Wed"));
}

#[wasm_bindgen_test]
fn hit_testing_handles_wrap_slice() {
    // 3 equal slices: slice 0 (A) wraps from ~4.73 through 2π → ~0.50.
    let layout = compute_polar_layout(
        500.0,
        500.0,
        &three_equal_slices(),
        &PolarAreaChartConfig::default(),
    )
    .unwrap();

    // angle 5.0 (before wrap) → A
    let (x, y) = point_at(layout.center_x, layout.center_y, 5.0, 50.0);
    assert_eq!(
        find_slice_at(&layout.slices, x, y).map(|s| s.name.as_str()),
        Some("A")
    );

    // angle 0.3 (after wrap) → A
    let (x, y) = point_at(layout.center_x, layout.center_y, 0.3, 50.0);
    assert_eq!(
        find_slice_at(&layout.slices, x, y).map(|s| s.name.as_str()),
        Some("A")
    );
}

#[wasm_bindgen_test]
fn hit_testing_rejects_points_outside_radius() {
    let layout = compute_polar_layout(
        500.0,
        500.0,
        &sample_data(),
        &PolarAreaChartConfig::default(),
    )
    .unwrap();
    let (x, y) = point_at(
        layout.center_x,
        layout.center_y,
        0.0,
        layout.max_radius + 100.0,
    );
    assert!(find_slice_at(&layout.slices, x, y).is_none());
}

#[wasm_bindgen_test]
fn hit_testing_no_match_inside_gap() {
    // 3 equal slices: gap between slice 1 end (~2.60) and slice 2 start (~2.64).
    let layout = compute_polar_layout(
        500.0,
        500.0,
        &three_equal_slices(),
        &PolarAreaChartConfig::default(),
    )
    .unwrap();
    let (x, y) = point_at(layout.center_x, layout.center_y, 2.62, 50.0);
    assert!(find_slice_at(&layout.slices, x, y).is_none());
}

// ---------- with_alpha ----------

#[wasm_bindgen_test]
fn with_alpha_maps_opacity_to_hex_suffix() {
    assert_eq!(with_alpha("#4f46e5", 1.0), "#4f46e5ff");
    assert_eq!(with_alpha("#4f46e5", 0.0), "#4f46e500");
    // 0.5 * 255 = 127.5 → rounds to 128 = 0x80
    assert_eq!(with_alpha("#4f46e5", 0.5), "#4f46e580");
    // 0.85 * 255 = 216.75 → rounds to 217 = 0xd9
    assert_eq!(with_alpha("#4f46e5", 0.85), "#4f46e5d9");
}

#[wasm_bindgen_test]
fn with_alpha_clamps_out_of_range_opacity() {
    assert_eq!(with_alpha("#000000", -0.5), "#00000000");
    assert_eq!(with_alpha("#000000", 1.5), "#000000ff");
}

// ---------- tooltip ----------

#[wasm_bindgen_test]
fn tooltip_flips_when_it_would_overflow() {
    assert_eq!(tooltip_left(100.0, 50.0, 500.0, 10.0), 110.0);
    assert_eq!(tooltip_left(470.0, 50.0, 500.0, 10.0), 410.0);
    assert_eq!(tooltip_left(10.0, 100.0, 50.0, 10.0), 0.0);
}

// ---------- smoke tests ----------

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
fn test_draw_polar_area_chart() {
    let Some(context) = mock_context() else {
        return;
    };

    let slices = draw_polar_area_chart(
        &context,
        500.0,
        500.0,
        &sample_data(),
        &PolarAreaChartConfig::default(),
    );

    assert_eq!(slices.len(), 5);
    assert_eq!(slices[0].name, "Mon");
    assert_eq!(slices[4].name, "Fri");
}

#[wasm_bindgen_test]
fn test_draw_polar_area_chart_empty() {
    let Some(context) = mock_context() else {
        return;
    };

    let slices = draw_polar_area_chart(
        &context,
        500.0,
        500.0,
        &[],
        &PolarAreaChartConfig::default(),
    );

    assert!(slices.is_empty());
}
