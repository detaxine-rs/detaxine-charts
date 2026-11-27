use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ---------- layout tests ----------

#[wasm_bindgen_test]
fn empty_data_produces_no_layout() {
    assert!(compute_candle_layout(800.0, 480.0, &[]).is_none());
}

#[wasm_bindgen_test]
fn all_zero_candles_produce_no_layout() {
    // Padding is ×1.05 / ×0.95, so only all-zero highs/lows give value_range == 0.
    let data = vec![Candle::new("Mon", 0.0, 0.0, 0.0, 0.0)];
    assert!(compute_candle_layout(800.0, 480.0, &data).is_none());
}

#[wasm_bindgen_test]
fn layout_has_one_position_per_candle_and_preserves_ohlc_labels() {
    let data = vec![
        Candle::new("Mon", 100.0, 115.0, 95.0, 110.0),
        Candle::new("Tue", 110.0, 120.0, 105.0, 108.0),
        Candle::new("Wed", 108.0, 112.0, 100.0, 102.0),
    ];
    let layout = compute_candle_layout(800.0, 480.0, &data).unwrap();

    assert_eq!(layout.candle_positions.len(), 3);
    assert_eq!(layout.candle_positions[0].label, "Mon");
    assert_eq!(layout.candle_positions[1].label, "Tue");
    assert_eq!(layout.candle_positions[2].label, "Wed");

    assert_eq!(layout.candle_positions[0].open, 100.0);
    assert_eq!(layout.candle_positions[0].high, 115.0);
    assert_eq!(layout.candle_positions[0].low, 95.0);
    assert_eq!(layout.candle_positions[0].close, 110.0);
}

#[wasm_bindgen_test]
fn candles_are_ordered_left_to_right_and_inside_plot_area() {
    let data = vec![
        Candle::new("Mon", 100.0, 115.0, 95.0, 110.0),
        Candle::new("Tue", 110.0, 120.0, 105.0, 108.0),
        Candle::new("Wed", 108.0, 112.0, 100.0, 102.0),
    ];
    let layout = compute_candle_layout(800.0, 480.0, &data).unwrap();

    let a = &layout.candle_positions[0];
    let b = &layout.candle_positions[1];
    let c = &layout.candle_positions[2];

    assert!(a.x < b.x);
    assert!(b.x < c.x);

    for pos in &layout.candle_positions {
        assert!(pos.x >= layout.left_padding);
        assert!(pos.y >= layout.axis_padding - 1e-9);
        assert!(pos.y + pos.height <= 480.0 - layout.axis_padding + 1e-9);
    }
}

#[wasm_bindgen_test]
fn body_and_wick_extents_are_consistent() {
    let data = vec![Candle::new("Mon", 100.0, 120.0, 80.0, 110.0)];
    let layout = compute_candle_layout(800.0, 480.0, &data).unwrap();
    let pos = &layout.candle_positions[0];

    // wick extends past body on both sides
    assert!(pos.y < pos.body_top);
    assert!(pos.y + pos.height > pos.body_bottom);
    // body has positive extent
    assert!(pos.body_top < pos.body_bottom);
    // close > open
    assert!(pos.is_bullish);
}

#[wasm_bindgen_test]
fn bearish_candle_when_close_below_open() {
    let data = vec![Candle::new("Mon", 110.0, 120.0, 90.0, 100.0)];
    let layout = compute_candle_layout(800.0, 480.0, &data).unwrap();
    assert!(!layout.candle_positions[0].is_bullish);
}

#[wasm_bindgen_test]
fn grid_lines_span_the_plot_area() {
    let data = vec![
        Candle::new("Mon", 100.0, 115.0, 95.0, 110.0),
        Candle::new("Tue", 110.0, 120.0, 105.0, 108.0),
    ];
    let layout = compute_candle_layout(800.0, 480.0, &data).unwrap();

    assert_eq!(layout.grid_lines.len(), 6);
    assert!((layout.grid_lines[0].value - layout.min_value).abs() < 1e-9);
    assert!((layout.grid_lines.last().unwrap().value - layout.max_value).abs() < 1e-9);
    // y decreases (bottom line first, top line last)
    assert!(layout.grid_lines[0].y > layout.grid_lines.last().unwrap().y);
}

// ---------- interaction tests ----------

#[wasm_bindgen_test]
fn hit_testing_finds_the_correct_candle_by_x() {
    let data = vec![
        Candle::new("Mon", 100.0, 115.0, 95.0, 110.0),
        Candle::new("Tue", 110.0, 120.0, 105.0, 108.0),
        Candle::new("Wed", 108.0, 112.0, 100.0, 102.0),
    ];
    let layout = compute_candle_layout(800.0, 480.0, &data).unwrap();
    let positions = &layout.candle_positions;

    let first = &positions[0];
    let found = find_candle_at(positions, first.x + first.width / 2.0);
    assert_eq!(found.map(|(i, c)| (i, c.label.as_str())), Some((0, "Mon")));

    assert!(find_candle_at(positions, -1.0).is_none());
    assert!(find_candle_at(positions, 10_000.0).is_none());
}

#[wasm_bindgen_test]
fn tooltip_flips_when_it_would_overflow() {
    assert_eq!(tooltip_left(100.0, 50.0, 500.0, 10.0), 110.0);
    assert_eq!(tooltip_left(470.0, 50.0, 500.0, 10.0), 410.0);
    assert_eq!(tooltip_left(10.0, 100.0, 50.0, 10.0), 0.0);
}

// ---------- padding consistency ----------

#[wasm_bindgen_test]
fn layout_paddings_match_compute_paddings() {
    let data = vec![Candle::new("Mon", 100.0, 115.0, 95.0, 110.0)];
    let layout = compute_candle_layout(800.0, 480.0, &data).unwrap();
    let (left, right) = compute_paddings(800.0, 480.0);
    assert_eq!(layout.left_padding, left);
    assert_eq!(layout.axis_padding, right);
}

#[wasm_bindgen_test]
fn zoom_pivots_at_plot_edges() {
    let (left, right) = compute_paddings(800.0, 480.0);

    // Zooming at the left edge of the plot keeps `start` fixed.
    let (new_start, _) = compute_zoom(left, 800.0, 480.0, 100, 10, 60, -1);
    assert_eq!(new_start, 10);

    // Zooming at the right edge of the plot keeps `end` fixed.
    let plot_right_edge = 800.0 - right;
    let (_, new_end) = compute_zoom(plot_right_edge, 800.0, 480.0, 100, 10, 60, -1);
    assert_eq!(new_end, 60);
}

#[wasm_bindgen_test]
fn zoom_in_reduces_visible_count() {
    let (start, end) = compute_zoom(400.0, 800.0, 480.0, 100, 10, 60, -1);
    assert_eq!(end - start, 49);
    assert!(start <= end);
    assert!(end <= 100);
}

#[wasm_bindgen_test]
fn zoom_out_increases_visible_count() {
    let (start, end) = compute_zoom(400.0, 800.0, 480.0, 100, 10, 60, 1);
    assert_eq!(end - start, 51);
    assert!(end <= 100);
}

#[wasm_bindgen_test]
fn zoom_respects_minimum_visible_count() {
    let (start, end) = compute_zoom(400.0, 800.0, 480.0, 100, 10, 12, -1);
    assert_eq!(end - start, 2);
}

#[wasm_bindgen_test]
fn zoom_never_exceeds_total() {
    let (start, end) = compute_zoom(400.0, 800.0, 480.0, 20, 0, 20, 1);
    assert!(end <= 20);
    assert!(start <= end);
}

#[wasm_bindgen_test]
fn pan_shifts_view_by_candle_width() {
    // chart_width ≈ 682.4, visible = 30 → candle_width ≈ 22.75.
    // Drag ~22.75px right shifts start by -1.
    let (new_start, new_end) = compute_pan(300.0, 322.75, 800.0, 480.0, 100, 50, 80);
    assert_eq!(new_start, 49);
    assert_eq!(new_end, 79);
}

#[wasm_bindgen_test]
fn pan_clamps_at_both_edges() {
    // Drag far left (mouse moves left) → view moves right → clamps at end.
    let (new_start, new_end) = compute_pan(2000.0, 0.0, 800.0, 480.0, 100, 10, 40);
    assert_eq!(new_start, 70);
    assert_eq!(new_end, 100);

    // Drag far right (mouse moves right) → view moves left → clamps at start.
    let (new_start, new_end) = compute_pan(0.0, 2000.0, 800.0, 480.0, 100, 10, 40);
    assert_eq!(new_start, 0);
    assert_eq!(new_end, 30);
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
fn test_draw_candlestick_chart() {
    let Some(context) = mock_context() else {
        return;
    };

    let data = vec![
        Candle::new("Mon", 100.0, 115.0, 95.0, 110.0),
        Candle::new("Tue", 110.0, 120.0, 105.0, 108.0),
        Candle::new("Wed", 108.0, 112.0, 100.0, 102.0),
        Candle::new("Thu", 102.0, 118.0, 101.0, 115.0),
        Candle::new("Fri", 115.0, 125.0, 113.0, 120.0),
    ];
    let config = CandlestickChartConfig::default();

    let positions = draw_candlestick_chart(&context, 800.0, 480.0, &data, &config);

    assert_eq!(positions.len(), 5);
    assert_eq!(positions[0].label, "Mon");
    assert_eq!(positions[4].label, "Fri");
}
