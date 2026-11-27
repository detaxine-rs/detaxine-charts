use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ---------- layout: edge cases ----------

#[wasm_bindgen_test]
fn empty_data_produces_no_layout() {
    assert!(compute_multiline_layout(800.0, 480.0, &[], &[]).is_none());
}

#[wasm_bindgen_test]
fn single_point_per_series_produces_no_layout() {
    let data = vec![(Series::new("A", "#000"), vec![DataPoint::new(10)])];
    let x = vec!["Jan".to_string()];
    assert!(compute_multiline_layout(800.0, 480.0, &data, &x).is_none());
}

#[wasm_bindgen_test]
fn zero_dimensions_produce_no_layout() {
    let data = sample_data();
    let x = sample_labels();
    assert!(compute_multiline_layout(0.0, 480.0, &data, &x).is_none());
    assert!(compute_multiline_layout(800.0, 0.0, &data, &x).is_none());
    assert!(compute_multiline_layout(-1.0, -1.0, &data, &x).is_none());
}

// ---------- layout: point positions ----------

fn sample_data() -> Vec<(Series, Vec<DataPoint>)> {
    vec![
        (
            Series::new("Revenue", "#4f46e5"),
            vec![
                DataPoint::new(120),
                DataPoint::new(85),
                DataPoint::new(200),
                DataPoint::new(150),
            ],
        ),
        (
            Series::new("Expenses", "#e11d48"),
            vec![
                DataPoint::new(80),
                DataPoint::new(90),
                DataPoint::new(110),
                DataPoint::new(95),
            ],
        ),
    ]
}

fn sample_labels() -> Vec<String> {
    vec![
        "Jan".to_string(),
        "Feb".to_string(),
        "Mar".to_string(),
        "Apr".to_string(),
    ]
}

#[wasm_bindgen_test]
fn layout_has_one_point_per_series_point() {
    let data = sample_data();
    let x = sample_labels();
    let layout = compute_multiline_layout(800.0, 480.0, &data, &x).unwrap();

    assert_eq!(layout.point_positions.len(), 8);
    assert_eq!(layout.num_points, 4);
}

#[wasm_bindgen_test]
fn points_share_x_across_series_at_same_index() {
    let data = sample_data();
    let x = sample_labels();
    let layout = compute_multiline_layout(800.0, 480.0, &data, &x).unwrap();

    for i in 0..4 {
        let s0 = layout
            .point_positions
            .iter()
            .find(|p| p.series_index == 0 && p.point_index == i)
            .unwrap();
        let s1 = layout
            .point_positions
            .iter()
            .find(|p| p.series_index == 1 && p.point_index == i)
            .unwrap();
        assert!((s0.x - s1.x).abs() < 1e-9);
    }
}

#[wasm_bindgen_test]
fn point_x_coordinates_use_point_spacing() {
    let data = sample_data();
    let x = sample_labels();
    let layout = compute_multiline_layout(800.0, 480.0, &data, &x).unwrap();

    let first = layout
        .point_positions
        .iter()
        .find(|p| p.series_index == 0 && p.point_index == 0)
        .unwrap();
    let second = layout
        .point_positions
        .iter()
        .find(|p| p.series_index == 0 && p.point_index == 1)
        .unwrap();

    assert!((first.x - layout.left_padding).abs() < 1e-9);
    assert!((second.x - first.x - layout.point_spacing).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn point_y_is_proportional_to_value() {
    let data = sample_data();
    let x = sample_labels();
    let layout = compute_multiline_layout(800.0, 480.0, &data, &x).unwrap();

    // Within series 0: 85 (min) should be lower than 200 (max)
    let p_85 = layout
        .point_positions
        .iter()
        .find(|p| p.series_index == 0 && p.value == 85)
        .unwrap();
    let p_200 = layout
        .point_positions
        .iter()
        .find(|p| p.series_index == 0 && p.value == 200)
        .unwrap();
    assert!(p_85.y > p_200.y); // larger y = lower on screen

    // Ratio: (y_0 - y_val) / available_height should equal value / max_value
    let available = 480.0 - layout.axis_padding * 2.0;
    let y_0 = 480.0 - layout.axis_padding;
    let expected_200 = y_0 - (200.0 / layout.max_value) * available;
    assert!((p_200.y - expected_200).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn points_inside_plot_area() {
    let data = sample_data();
    let x = sample_labels();
    let layout = compute_multiline_layout(800.0, 480.0, &data, &x).unwrap();

    for p in &layout.point_positions {
        assert!(p.x >= layout.left_padding - 1e-9);
        assert!(p.y >= layout.axis_padding - 1e-9);
        assert!(p.y <= 480.0 - layout.axis_padding + 1e-9);
    }
}

#[wasm_bindgen_test]
fn labels_and_series_metadata_propagate() {
    let data = sample_data();
    let x = sample_labels();
    let layout = compute_multiline_layout(800.0, 480.0, &data, &x).unwrap();

    let p = layout
        .point_positions
        .iter()
        .find(|p| p.series_index == 0 && p.point_index == 2)
        .unwrap();
    assert_eq!(p.label, "Mar");
    assert_eq!(p.value, 200);
    assert_eq!(p.series_name, "Revenue");
    assert_eq!(p.series_color, "#4f46e5");
}

#[wasm_bindgen_test]
fn missing_labels_are_empty_strings() {
    let data = sample_data();
    let layout = compute_multiline_layout(800.0, 480.0, &data, &[]).unwrap();
    for p in &layout.point_positions {
        assert!(p.label.is_empty());
    }
}

// ---------- layout: grid ----------

#[wasm_bindgen_test]
fn grid_lines_span_the_plot_area() {
    let data = sample_data();
    let x = sample_labels();
    let layout = compute_multiline_layout(800.0, 480.0, &data, &x).unwrap();

    assert_eq!(layout.grid_lines.len(), 6);
    assert!((layout.grid_lines[0].value - 0.0).abs() < 1e-9);
    assert!((layout.grid_lines.last().unwrap().value - layout.max_value).abs() < 1e-9);
    assert!(layout.grid_lines[0].y > layout.grid_lines.last().unwrap().y);
}

// ---------- hit testing ----------

#[wasm_bindgen_test]
fn hover_within_threshold_returns_closest() {
    let data = sample_data();
    let x = sample_labels();
    let layout = compute_multiline_layout(800.0, 480.0, &data, &x).unwrap();

    let p1 = layout
        .point_positions
        .iter()
        .find(|p| p.series_index == 0 && p.point_index == 1)
        .unwrap();
    // cursor slightly to the right of p1
    let found = find_hovered_points(&layout.point_positions, p1.x + 5.0, 40.0, 0.5);
    let (closest_x, _) = found.expect("expected hit");
    assert!((closest_x - p1.x).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn hover_outside_threshold_returns_none() {
    let data = sample_data();
    let x = sample_labels();
    let layout = compute_multiline_layout(800.0, 480.0, &data, &x).unwrap();

    let found = find_hovered_points(&layout.point_positions, -1000.0, 40.0, 0.5);
    assert!(found.is_none());
}

#[wasm_bindgen_test]
fn hover_returns_all_series_at_same_x() {
    let data = sample_data();
    let x = sample_labels();
    let layout = compute_multiline_layout(800.0, 480.0, &data, &x).unwrap();

    let p1 = layout
        .point_positions
        .iter()
        .find(|p| p.series_index == 0 && p.point_index == 1)
        .unwrap();
    let (_, matched) = find_hovered_points(&layout.point_positions, p1.x, 40.0, 0.5).unwrap();
    assert_eq!(matched.len(), 2);
    let names: Vec<_> = matched.iter().map(|p| p.series_name.as_str()).collect();
    assert!(names.contains(&"Revenue"));
    assert!(names.contains(&"Expenses"));
}

#[wasm_bindgen_test]
fn hover_on_empty_positions_returns_none() {
    assert!(find_hovered_points(&[], 0.0, 40.0, 0.5).is_none());
}

#[wasm_bindgen_test]
fn hover_threshold_is_inclusive_at_boundary() {
    let data = sample_data();
    let x = sample_labels();
    let layout = compute_multiline_layout(800.0, 480.0, &data, &x).unwrap();

    let p0 = layout
        .point_positions
        .iter()
        .find(|p| p.series_index == 0 && p.point_index == 0)
        .unwrap();
    // exactly at threshold → Some
    assert!(find_hovered_points(&layout.point_positions, p0.x + 40.0, 40.0, 0.5).is_some());
    // just past threshold → None
    assert!(find_hovered_points(&layout.point_positions, p0.x + 41.0, 40.0, 0.5).is_none());
}

// ---------- tooltip ----------

#[wasm_bindgen_test]
fn tooltip_flips_when_it_would_overflow() {
    assert_eq!(tooltip_left(100.0, 50.0, 500.0, 12.0), 112.0);
    assert_eq!(tooltip_left(470.0, 50.0, 500.0, 12.0), 408.0);
    assert_eq!(tooltip_left(10.0, 100.0, 50.0, 12.0), 0.0);
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
fn test_draw_multiline_chart() {
    let Some(context) = mock_context() else {
        return;
    };

    let data = sample_data();
    let x_labels = sample_labels();
    let config = LineCurveChartConfig::default();

    let positions = draw_multiline_chart(&context, 800.0, 480.0, &data, &x_labels, &config);

    assert_eq!(positions.len(), 8);
    assert_eq!(positions[0].series_name, "Revenue");
    assert_eq!(positions[4].series_name, "Expenses");
}
