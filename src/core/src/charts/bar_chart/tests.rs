use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ---------- layout tests ----------

#[wasm_bindgen_test]
fn empty_data_produces_no_layout() {
    assert!(compute_layout(500.0, 400.0, &[]).is_none());
}

#[wasm_bindgen_test]
fn layout_has_one_bar_per_point_and_preserves_labels() {
    let data = vec![
        DataPoint::new("A", 10),
        DataPoint::new("B", 20),
        DataPoint::new("C", 15),
    ];
    let layout = compute_layout(500.0, 400.0, &data).unwrap();

    assert_eq!(layout.bar_rects.len(), 3);
    assert_eq!(layout.bar_rects[0].label, "A");
    assert_eq!(layout.bar_rects[0].value, 10);
    assert_eq!(layout.bar_rects[1].label, "B");
    assert_eq!(layout.bar_rects[1].value, 20);
    assert_eq!(layout.bar_rects[2].label, "C");
    assert_eq!(layout.bar_rects[2].value, 15);

    assert_eq!(layout.x_labels.len(), 3);
    assert_eq!(layout.x_labels[0].text, "A");
    assert_eq!(layout.x_labels[1].text, "B");
    assert_eq!(layout.x_labels[2].text, "C");
}

#[wasm_bindgen_test]
fn bar_heights_are_proportional_to_values() {
    let data = vec![
        DataPoint::new("A", 10),
        DataPoint::new("B", 20),
        DataPoint::new("C", 15),
    ];
    let layout = compute_layout(500.0, 400.0, &data).unwrap();

    let a = &layout.bar_rects[0];
    let b = &layout.bar_rects[1];
    let c = &layout.bar_rects[2];

    assert!((b.height / a.height - 2.0).abs() < 1e-9);
    assert!((c.height / a.height - 1.5).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn bars_are_ordered_left_to_right_and_inside_plot_area() {
    let data = vec![
        DataPoint::new("A", 10),
        DataPoint::new("B", 20),
        DataPoint::new("C", 15),
    ];
    let layout = compute_layout(500.0, 400.0, &data).unwrap();

    let a = &layout.bar_rects[0];
    let b = &layout.bar_rects[1];
    let c = &layout.bar_rects[2];

    assert!(a.x < b.x);
    assert!(b.x < c.x);

    for bar in &layout.bar_rects {
        assert!(bar.x >= layout.left_padding);
        assert!(bar.y >= layout.axis_padding - 1e-9);
        assert!(bar.y + bar.height <= 400.0 - layout.axis_padding + 1e-9);
    }
}

#[wasm_bindgen_test]
fn grid_lines_span_the_plot_area() {
    let data = vec![
        DataPoint::new("A", 10),
        DataPoint::new("B", 20),
        DataPoint::new("C", 15),
    ];
    let layout = compute_layout(500.0, 400.0, &data).unwrap();

    assert_eq!(layout.grid_lines.len(), 6);
    assert!((layout.grid_lines[0].value - 0.0).abs() < 1e-9);
    assert!((layout.grid_lines.last().unwrap().value - layout.max_value).abs() < 1e-9);
    // y decreases (screen coords): bottom line is first, top line is last.
    assert!(layout.grid_lines[0].y > layout.grid_lines.last().unwrap().y);
}

#[wasm_bindgen_test]
fn zero_values_do_not_produce_nan() {
    let data = vec![DataPoint::new("A", 0)];
    let layout = compute_layout(500.0, 400.0, &data).unwrap();

    assert!(layout.max_value > 0.0);
    assert!(layout.bar_rects[0].y.is_finite());
    assert!(layout.bar_rects[0].height.is_finite());
}

// ---------- interaction tests ----------

#[wasm_bindgen_test]
fn hit_testing_finds_the_correct_bar() {
    let data = vec![DataPoint::new("A", 10), DataPoint::new("B", 20)];
    let layout = compute_layout(500.0, 400.0, &data).unwrap();
    let rects = &layout.bar_rects;

    let first = &rects[0];
    let found = find_bar_at(
        rects,
        first.x + first.width / 2.0,
        first.y + first.height / 2.0,
    );

    assert_eq!(found.map(|b| b.label.as_str()), Some("A"));
    assert!(find_bar_at(rects, -1.0, -1.0).is_none());
}

#[wasm_bindgen_test]
fn tooltip_flips_when_it_would_overflow() {
    // normal case: fits on the right
    assert_eq!(tooltip_left(100.0, 50.0, 500.0, 10.0), 110.0);
    // overflow: flips to the left of the cursor
    assert_eq!(tooltip_left(470.0, 50.0, 500.0, 10.0), 410.0);
    // even when flipped, clamp to zero
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
fn test_draw_bar_chart() {
    let Some(context) = mock_context() else {
        return;
    };

    let data = vec![
        DataPoint::new("A", 10),
        DataPoint::new("B", 20),
        DataPoint::new("C", 15),
    ];
    let config = BarChartConfig::new("blue", "gray", "black");

    let rects = draw_bar_chart(&context, 500.0, 400.0, &data, &config);

    assert_eq!(rects.len(), 3);
    assert_eq!(rects[0].label, "A");
    assert_eq!(rects[0].value, 10);
}
