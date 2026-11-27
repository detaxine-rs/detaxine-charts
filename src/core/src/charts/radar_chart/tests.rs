use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ---------- helpers ----------

fn five_axes() -> Vec<String> {
    vec![
        "Speed".to_string(),
        "Power".to_string(),
        "Defense".to_string(),
        "Agility".to_string(),
        "Stamina".to_string(),
    ]
}

fn two_series() -> Vec<(Series, Vec<f64>)> {
    vec![
        (
            Series::new("Player A", "#4f46e5"),
            vec![80.0, 65.0, 70.0, 90.0, 60.0],
        ),
        (
            Series::new("Player B", "#e11d48"),
            vec![60.0, 85.0, 75.0, 55.0, 80.0],
        ),
    ]
}

// ---------- layout: edge cases ----------

#[wasm_bindgen_test]
fn empty_data_produces_no_layout() {
    assert!(
        compute_radar_layout(
            500.0,
            500.0,
            &[],
            &five_axes(),
            &RadarChartConfig::default()
        )
        .is_none()
    );
}

#[wasm_bindgen_test]
fn fewer_than_three_axes_produces_no_layout() {
    let data = vec![(Series::new("X", "#000"), vec![1.0, 2.0])];
    let axes = vec!["A".to_string(), "B".to_string()];
    assert!(
        compute_radar_layout(500.0, 500.0, &data, &axes, &RadarChartConfig::default()).is_none()
    );

    let one_axis = vec!["A".to_string()];
    let data = vec![(Series::new("X", "#000"), vec![1.0])];
    assert!(
        compute_radar_layout(500.0, 500.0, &data, &one_axis, &RadarChartConfig::default())
            .is_none()
    );
}

#[wasm_bindgen_test]
fn zero_dimensions_produce_no_layout() {
    let config = RadarChartConfig::default();
    assert!(compute_radar_layout(0.0, 500.0, &two_series(), &five_axes(), &config).is_none());
    assert!(compute_radar_layout(500.0, 0.0, &two_series(), &five_axes(), &config).is_none());
}

// ---------- layout: geometry ----------

#[wasm_bindgen_test]
fn center_and_max_radius_match_canvas() {
    let layout = compute_radar_layout(
        500.0,
        500.0,
        &two_series(),
        &five_axes(),
        &RadarChartConfig::default(),
    )
    .unwrap();
    assert!((layout.center_x - 250.0).abs() < 1e-9);
    assert!((layout.center_y - 250.0).abs() < 1e-9);
    // label_margin = min(500,500)*0.16 = 80 → clamp 60; max_radius = 250-60 = 190
    assert!((layout.max_radius - 190.0).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn tiny_canvas_clamps_max_radius_to_floor() {
    let layout = compute_radar_layout(
        20.0,
        20.0,
        &two_series(),
        &five_axes(),
        &RadarChartConfig::default(),
    )
    .unwrap();
    // radius = 10 - 30 = -20, clamped to 10
    assert!((layout.max_radius - 10.0).abs() < 1e-9);
}

// ---------- layout: max value ----------

#[wasm_bindgen_test]
fn auto_max_value_from_largest_data_point() {
    let layout = compute_radar_layout(
        500.0,
        500.0,
        &two_series(),
        &five_axes(),
        &RadarChartConfig::default(),
    )
    .unwrap();
    // max value across both series is 90.0 (Player A agility)
    assert!((layout.max_value - 90.0).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn config_max_value_overrides_auto() {
    let config = RadarChartConfig {
        max_value: Some(100.0),
        ..Default::default()
    };
    let layout = compute_radar_layout(500.0, 500.0, &two_series(), &five_axes(), &config).unwrap();
    assert!((layout.max_value - 100.0).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn all_zero_values_fall_back_to_epsilon() {
    let data = vec![(Series::new("Z", "#000"), vec![0.0, 0.0, 0.0, 0.0, 0.0])];
    let layout = compute_radar_layout(
        500.0,
        500.0,
        &data,
        &five_axes(),
        &RadarChartConfig::default(),
    )
    .unwrap();
    assert!((layout.max_value - f64::EPSILON).abs() < 1e-30);
    // every point should be at center since ratio = 0
    for p in &layout.points {
        assert!((p.x - layout.center_x).abs() < 1e-9);
        assert!((p.y - layout.center_y).abs() < 1e-9);
    }
}

// ---------- layout: grid rings ----------

#[wasm_bindgen_test]
fn grid_rings_scale_uniformly() {
    let layout = compute_radar_layout(
        500.0,
        500.0,
        &two_series(),
        &five_axes(),
        &RadarChartConfig::default(),
    )
    .unwrap();
    assert_eq!(layout.grid_radii.len(), 4);
    for (i, r) in layout.grid_radii.iter().enumerate() {
        let frac = (i + 1) as f64 / 4.0;
        assert!((r - layout.max_radius * frac).abs() < 1e-9);
    }
}

#[wasm_bindgen_test]
fn grid_ring_count_uses_config_with_floor_of_one() {
    let custom = RadarChartConfig {
        num_grid_rings: 6,
        ..Default::default()
    };
    let layout = compute_radar_layout(500.0, 500.0, &two_series(), &five_axes(), &custom).unwrap();
    assert_eq!(layout.grid_radii.len(), 6);

    let zero = RadarChartConfig {
        num_grid_rings: 0,
        ..Default::default()
    };
    let layout = compute_radar_layout(500.0, 500.0, &two_series(), &five_axes(), &zero).unwrap();
    assert_eq!(layout.grid_radii.len(), 1);
}

// ---------- layout: axis angles and labels ----------

#[wasm_bindgen_test]
fn first_axis_points_up_and_axes_are_evenly_spaced() {
    let num = 5;
    let a0 = axis_angle(0, num);
    assert!((a0 - (-PI / 2.0)).abs() < 1e-9);

    let step = 2.0 * PI / num as f64;
    for i in 0..num {
        assert!((axis_angle(i, num) - (a0 + i as f64 * step)).abs() < 1e-9);
    }
}

#[wasm_bindgen_test]
fn label_text_align_matches_cos_bin() {
    assert_eq!(label_text_align(0.0), "center");
    assert_eq!(label_text_align(0.1), "center");
    assert_eq!(label_text_align(-0.1), "center");
    assert_eq!(label_text_align(0.9), "left");
    assert_eq!(label_text_align(-0.9), "right");
}

#[wasm_bindgen_test]
fn axis_labels_placed_outside_max_radius() {
    let layout = compute_radar_layout(
        500.0,
        500.0,
        &two_series(),
        &five_axes(),
        &RadarChartConfig::default(),
    )
    .unwrap();

    assert_eq!(layout.axis_labels.len(), 5);
    for (i, label) in layout.axis_labels.iter().enumerate() {
        let angle = axis_angle(i, 5);
        let dx = label.x - layout.center_x;
        let dy = label.y - layout.center_y;
        let dist = (dx * dx + dy * dy).sqrt();
        assert!(dist > layout.max_radius);
        // And roughly along the spoke angle
        assert!((dy.atan2(dx) - angle.sin().atan2(angle.cos())).abs() < 0.2);
    }
}

// ---------- layout: points ----------

#[wasm_bindgen_test]
fn layout_has_one_point_per_series_axis_pair() {
    let layout = compute_radar_layout(
        500.0,
        500.0,
        &two_series(),
        &five_axes(),
        &RadarChartConfig::default(),
    )
    .unwrap();
    // 2 series × 5 axes = 10 points
    assert_eq!(layout.points.len(), 10);
}

#[wasm_bindgen_test]
fn point_distance_from_center_is_proportional_to_value() {
    let layout = compute_radar_layout(
        500.0,
        500.0,
        &two_series(),
        &five_axes(),
        &RadarChartConfig::default(),
    )
    .unwrap();

    // Player A speed = 80, max_value = 90 → ratio 80/90
    let p = layout
        .points
        .iter()
        .find(|p| p.series_index == 0 && p.axis_index == 0)
        .unwrap();
    let dx = p.x - layout.center_x;
    let dy = p.y - layout.center_y;
    let dist = (dx * dx + dy * dy).sqrt();
    let expected = layout.max_radius * (80.0 / 90.0);
    assert!((dist - expected).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn negative_values_clamp_to_center() {
    let data = vec![(Series::new("N", "#000"), vec![-10.0, 5.0, 5.0, 5.0, 5.0])];
    let layout = compute_radar_layout(
        500.0,
        500.0,
        &data,
        &five_axes(),
        &RadarChartConfig::default(),
    )
    .unwrap();

    let p = layout
        .points
        .iter()
        .find(|p| p.series_index == 0 && p.axis_index == 0)
        .unwrap();
    assert!((p.x - layout.center_x).abs() < 1e-9);
    assert!((p.y - layout.center_y).abs() < 1e-9);
    // Stored value is still the original (tooltip shows the truth)
    assert!((p.value - (-10.0)).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn values_above_config_max_clamp_to_outer_ring() {
    let config = RadarChartConfig {
        max_value: Some(50.0),
        ..Default::default()
    };
    let data = vec![(
        Series::new("X", "#000"),
        vec![100.0, 25.0, 25.0, 25.0, 25.0],
    )];
    let layout = compute_radar_layout(500.0, 500.0, &data, &five_axes(), &config).unwrap();

    let p = layout
        .points
        .iter()
        .find(|p| p.series_index == 0 && p.axis_index == 0)
        .unwrap();
    let dx = p.x - layout.center_x;
    let dy = p.y - layout.center_y;
    let dist = (dx * dx + dy * dy).sqrt();
    // 100 / 50 = 2.0 → clamped to 1.0 → at max_radius
    assert!((dist - layout.max_radius).abs() < 1e-9);
    // But stored value is original
    assert!((p.value - 100.0).abs() < 1e-9);
}

// ---------- series validation ----------

#[wasm_bindgen_test]
fn series_with_wrong_point_count_is_skipped() {
    let axes = five_axes();
    let data = vec![
        (
            Series::new("Good", "#111"),
            vec![10.0, 10.0, 10.0, 10.0, 10.0],
        ),
        (Series::new("Bad", "#222"), vec![5.0, 5.0]), // wrong count
    ];
    let layout =
        compute_radar_layout(500.0, 500.0, &data, &axes, &RadarChartConfig::default()).unwrap();

    assert_eq!(layout.points.len(), 5);
    for p in &layout.points {
        assert_eq!(p.series_name, "Good");
    }
}

// ---------- hit testing ----------

#[wasm_bindgen_test]
fn hit_testing_finds_point_within_radius() {
    let layout = compute_radar_layout(
        500.0,
        500.0,
        &two_series(),
        &five_axes(),
        &RadarChartConfig::default(),
    )
    .unwrap();

    let p = layout
        .points
        .iter()
        .find(|p| p.series_index == 0 && p.axis_index == 0)
        .unwrap();
    let found = find_point_at(&layout.points, p.x, p.y, 8.0);
    assert!(found.is_some());
    assert_eq!(found.unwrap().series_name, "Player A");
}

#[wasm_bindgen_test]
fn hit_testing_rejects_point_outside_radius() {
    let layout = compute_radar_layout(
        500.0,
        500.0,
        &two_series(),
        &five_axes(),
        &RadarChartConfig::default(),
    )
    .unwrap();

    let p = layout.points[0].clone();
    let found = find_point_at(&layout.points, p.x + 100.0, p.y + 100.0, 8.0);
    assert!(found.is_none());
}

#[wasm_bindgen_test]
fn hit_testing_at_boundary_is_inclusive() {
    let layout = compute_radar_layout(
        500.0,
        500.0,
        &two_series(),
        &five_axes(),
        &RadarChartConfig::default(),
    )
    .unwrap();

    let p = &layout.points[0];
    // exactly 8.0 away → hit (uses <=)
    assert!(find_point_at(&layout.points, p.x + 8.0, p.y, 8.0).is_some());
    // just past 8.0 → miss
    assert!(find_point_at(&layout.points, p.x + 8.5, p.y, 8.0).is_none());
}

#[wasm_bindgen_test]
fn hit_testing_empty_points_returns_none() {
    assert!(find_point_at(&[], 100.0, 100.0, 8.0).is_none());
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
fn test_draw_radar_chart() {
    let Some(context) = mock_context() else {
        return;
    };

    let points = draw_radar_chart(
        &context,
        500.0,
        500.0,
        &two_series(),
        &five_axes(),
        &RadarChartConfig::default(),
    );

    assert_eq!(points.len(), 10);
    assert_eq!(points[0].series_name, "Player A");
}

#[wasm_bindgen_test]
fn test_draw_radar_chart_too_few_axes() {
    let Some(context) = mock_context() else {
        return;
    };

    let axes = vec!["A".to_string(), "B".to_string()];
    let data = vec![(Series::new("X", "#4f46e5"), vec![1.0, 2.0])];

    let points = draw_radar_chart(
        &context,
        500.0,
        500.0,
        &data,
        &axes,
        &RadarChartConfig::default(),
    );

    assert!(points.is_empty());
}
