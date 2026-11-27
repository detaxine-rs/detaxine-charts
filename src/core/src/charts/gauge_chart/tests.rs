use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ---------- value_to_angle ----------

#[wasm_bindgen_test]
fn value_to_angle_endpoints() {
    let start = -PI / 2.0;
    let end = PI / 2.0;
    assert!((value_to_angle(0.0, 0.0, 100.0, start, end) - start).abs() < 1e-9);
    assert!((value_to_angle(50.0, 0.0, 100.0, start, end)).abs() < 1e-9);
    assert!((value_to_angle(100.0, 0.0, 100.0, start, end) - end).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn value_to_angle_clamps_outside_range() {
    let start = -PI / 2.0;
    let end = PI / 2.0;
    assert!((value_to_angle(-50.0, 0.0, 100.0, start, end) - start).abs() < 1e-9);
    assert!((value_to_angle(200.0, 0.0, 100.0, start, end) - end).abs() < 1e-9);
}

// ---------- find_active_zone ----------

#[wasm_bindgen_test]
fn find_active_zone_boundaries_are_inclusive() {
    let zones = vec![
        GaugeZone::new(0.0, 50.0, "#red", "LOW"),
        GaugeZone::new(50.0, 100.0, "#blue", "HIGH"),
    ];
    // value at a shared boundary matches the first zone in array order
    assert_eq!(
        find_active_zone(50.0, &zones).map(|z| z.label.as_deref()),
        Some(Some("LOW"))
    );
    assert_eq!(
        find_active_zone(0.0, &zones).map(|z| z.label.as_deref()),
        Some(Some("LOW"))
    );
    assert_eq!(
        find_active_zone(100.0, &zones).map(|z| z.label.as_deref()),
        Some(Some("HIGH"))
    );
}

#[wasm_bindgen_test]
fn find_active_zone_returns_none_when_no_match() {
    let zones = vec![GaugeZone::new(10.0, 50.0, "#red", "LOW")];
    assert!(find_active_zone(5.0, &zones).is_none());
    assert!(find_active_zone(60.0, &zones).is_none());
    assert!(find_active_zone(50.0, &[]).is_none());
}

// ---------- angle_extent ----------

#[wasm_bindgen_test]
fn angle_extent_covers_full_circle() {
    let (min_cos, max_cos, min_sin, max_sin) = angle_extent(0.0, 2.0 * PI);
    assert!((min_cos - (-1.0)).abs() < 1e-9);
    assert!((max_cos - 1.0).abs() < 1e-9);
    assert!((min_sin - (-1.0)).abs() < 1e-9);
    assert!((max_sin - 1.0).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn angle_extent_covers_270_sweep() {
    let start = 3.0 * PI / 4.0;
    let end = 9.0 * PI / 4.0;
    let (min_cos, max_cos, min_sin, max_sin) = angle_extent(start, end);
    assert!((min_cos - (-1.0)).abs() < 1e-9);
    assert!((max_cos - 1.0).abs() < 1e-9);
    assert!((min_sin - (-1.0)).abs() < 1e-9);
    // max_sin = sin(135°) = sin(45°) = sqrt(2)/2
    assert!((max_sin - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn angle_extent_for_upper_hemisphere() {
    let (min_cos, max_cos, min_sin, max_sin) = angle_extent(0.0, PI);
    assert!((min_cos - (-1.0)).abs() < 1e-9);
    assert!((max_cos - 1.0).abs() < 1e-9);
    assert!((min_sin - 0.0).abs() < 1e-9);
    assert!((max_sin - 1.0).abs() < 1e-9);
}

// ---------- layout: needle geometry ----------

#[wasm_bindgen_test]
fn layout_needle_points_along_value_angle_at_min_mid_max() {
    let config = GaugeChartConfig::default();
    let min = compute_gauge_layout(500.0, 375.0, 0.0, &config).unwrap();
    let mid = compute_gauge_layout(500.0, 375.0, 50.0, &config).unwrap();
    let max = compute_gauge_layout(500.0, 375.0, 100.0, &config).unwrap();

    // min (angle 135°): needle points to lower-left
    assert!(min.needle.tip.0 < min.center_x);
    assert!(min.needle.tip.1 > min.center_y);

    // mid (angle 270°): needle points straight up
    assert!((mid.needle.tip.0 - mid.center_x).abs() < 1e-9);
    assert!(mid.needle.tip.1 < mid.center_y);

    // max (angle 45°): needle points to lower-right
    assert!(max.needle.tip.0 > max.center_x);
    assert!(max.needle.tip.1 > max.center_y);
}

#[wasm_bindgen_test]
fn layout_needle_geometry_is_consistent() {
    let config = GaugeChartConfig::default();
    let layout = compute_gauge_layout(500.0, 375.0, 50.0, &config).unwrap();

    // tip is at (radius - arc_thickness - 4) from center
    let dx = layout.needle.tip.0 - layout.center_x;
    let dy = layout.needle.tip.1 - layout.center_y;
    let tip_dist = (dx * dx + dy * dy).sqrt();
    let expected_tip = layout.radius - config.arc_thickness - 4.0;
    assert!((tip_dist - expected_tip).abs() < 1e-9);

    // base center is at pivot_radius from center
    let bcx = (layout.needle.base_left.0 + layout.needle.base_right.0) / 2.0;
    let bcy = (layout.needle.base_left.1 + layout.needle.base_right.1) / 2.0;
    let bdx = bcx - layout.center_x;
    let bdy = bcy - layout.center_y;
    let base_dist = (bdx * bdx + bdy * bdy).sqrt();
    assert!((base_dist - layout.pivot_radius).abs() < 1e-9);
}

// ---------- layout: sizing and clamping ----------

#[wasm_bindgen_test]
fn layout_centers_chart_horizontally() {
    let config = GaugeChartConfig::default();
    let layout = compute_gauge_layout(500.0, 375.0, 50.0, &config).unwrap();
    assert!((layout.center_x - 250.0).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn layout_clamps_value_outside_range() {
    let config = GaugeChartConfig::default();
    let below = compute_gauge_layout(500.0, 375.0, -50.0, &config).unwrap();
    let above = compute_gauge_layout(500.0, 375.0, 200.0, &config).unwrap();
    assert!((below.clamped_value - 0.0).abs() < 1e-9);
    assert!((above.clamped_value - 100.0).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn layout_radius_respects_minimum_floor_on_tiny_canvas() {
    let config = GaugeChartConfig::default();
    let layout = compute_gauge_layout(50.0, 40.0, 50.0, &config).unwrap();
    assert!((layout.radius - 10.0).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn layout_pivot_radius_clamps_at_bounds() {
    let config = GaugeChartConfig::default();
    let tiny = compute_gauge_layout(50.0, 40.0, 50.0, &config).unwrap();
    assert!((tiny.pivot_radius - 20.0).abs() < 1e-9);
    let huge = compute_gauge_layout(5000.0, 3750.0, 50.0, &config).unwrap();
    assert!((huge.pivot_radius - 70.0).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn layout_returns_none_for_zero_dimensions() {
    let config = GaugeChartConfig::default();
    assert!(compute_gauge_layout(0.0, 375.0, 50.0, &config).is_none());
    assert!(compute_gauge_layout(500.0, 0.0, 50.0, &config).is_none());
    assert!(compute_gauge_layout(-1.0, -1.0, 50.0, &config).is_none());
}

// ---------- layout: style selection ----------

#[wasm_bindgen_test]
fn layout_uses_progress_style_when_zones_empty() {
    let config = GaugeChartConfig::default();
    let layout = compute_gauge_layout(500.0, 375.0, 50.0, &config).unwrap();
    assert!(layout.zone_arcs.is_empty());
    assert!(!layout.use_zone_style);
}

#[wasm_bindgen_test]
fn layout_uses_zone_style_when_zones_configured() {
    let config = credit_score_config();
    let layout = compute_gauge_layout(500.0, 375.0, 700.0, &config).unwrap();
    assert_eq!(layout.zone_arcs.len(), 5);
    assert!(layout.use_zone_style);
}

// ---------- active color resolution ----------
// NOTE: `value_color` is documented as "overrides the auto-detected zone
// color for the value text", but the current implementation gives
// `active_zone.color` priority over `value_color`. Tests below lock in
// the current behavior — flip the first one if precedence is corrected.

#[wasm_bindgen_test]
fn active_color_prefers_zone_color_over_value_color() {
    let config = GaugeChartConfig {
        zones: vec![GaugeZone::new(0.0, 50.0, "#zone", "LOW")],
        value_color: Some("#override".to_string()),
        progress_color: "#progress".to_string(),
        ..Default::default()
    };
    let layout = compute_gauge_layout(500.0, 375.0, 25.0, &config).unwrap();
    assert_eq!(layout.active_color, "#zone");
}

#[wasm_bindgen_test]
fn active_color_falls_back_to_value_color_without_zone() {
    let config = GaugeChartConfig {
        zones: vec![GaugeZone::new(0.0, 50.0, "#zone", "LOW")],
        value_color: Some("#override".to_string()),
        progress_color: "#progress".to_string(),
        ..Default::default()
    };
    // value 75 does not match any zone
    let layout = compute_gauge_layout(500.0, 375.0, 75.0, &config).unwrap();
    assert_eq!(layout.active_color, "#override");
}

#[wasm_bindgen_test]
fn active_color_falls_back_to_progress_color() {
    let config = GaugeChartConfig {
        progress_color: "#progress".to_string(),
        ..Default::default()
    };
    let layout = compute_gauge_layout(500.0, 375.0, 50.0, &config).unwrap();
    assert_eq!(layout.active_color, "#progress");
}

// ---------- display label resolution ----------

#[wasm_bindgen_test]
fn display_label_resolution() {
    // config.label wins over zone label
    let with_config = GaugeChartConfig {
        zones: vec![GaugeZone::new(0.0, 100.0, "#c", "ZONE")],
        label: "CONFIG".to_string(),
        ..Default::default()
    };
    let layout = compute_gauge_layout(500.0, 375.0, 50.0, &with_config).unwrap();
    assert_eq!(layout.display_label.as_deref(), Some("CONFIG"));

    // falls back to active zone's label
    let zone_only = GaugeChartConfig {
        zones: vec![GaugeZone::new(0.0, 100.0, "#c", "ZONE")],
        ..Default::default()
    };
    let layout = compute_gauge_layout(500.0, 375.0, 50.0, &zone_only).unwrap();
    assert_eq!(layout.display_label.as_deref(), Some("ZONE"));

    // nothing to show
    let default = GaugeChartConfig::default();
    let layout = compute_gauge_layout(500.0, 375.0, 50.0, &default).unwrap();
    assert!(layout.display_label.is_none());
}

// ---------- zone arcs ----------

#[wasm_bindgen_test]
fn zone_arcs_apply_half_gap_to_interior_edges() {
    let config = GaugeChartConfig {
        min: 0.0,
        max: 100.0,
        zone_gap_degrees: 2.0,
        zones: vec![
            GaugeZone::new(0.0, 33.0, "#a", "A"),
            GaugeZone::new(33.0, 66.0, "#b", "B"),
            GaugeZone::new(66.0, 100.0, "#c", "C"),
        ],
        ..Default::default()
    };
    let layout = compute_gauge_layout(500.0, 375.0, 50.0, &config).unwrap();
    let arcs = &layout.zone_arcs;
    assert_eq!(arcs.len(), 3);

    let gap = 2.0_f64.to_radians();
    // interior gaps are exactly zone_gap_degrees wide
    assert!((arcs[1].start_angle - arcs[0].end_angle - gap).abs() < 1e-9);
    assert!((arcs[2].start_angle - arcs[1].end_angle - gap).abs() < 1e-9);
    // outer ends are flush with the sweep boundaries
    assert!((arcs[0].start_angle - layout.start_angle).abs() < 1e-9);
    assert!((arcs[2].end_angle - layout.end_angle).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn zone_arcs_mark_first_and_last_by_original_index() {
    let config = GaugeChartConfig {
        min: 0.0,
        max: 100.0,
        zones: vec![
            GaugeZone::new(0.0, 33.0, "#a", "A"),
            GaugeZone::new(33.0, 66.0, "#b", "B"),
            GaugeZone::new(66.0, 100.0, "#c", "C"),
        ],
        ..Default::default()
    };
    let layout = compute_gauge_layout(500.0, 375.0, 50.0, &config).unwrap();
    let arcs = &layout.zone_arcs;
    assert!(arcs[0].is_first);
    assert!(!arcs[1].is_first);
    assert!(!arcs[1].is_last);
    assert!(arcs[2].is_last);
}

#[wasm_bindgen_test]
fn zone_arcs_skip_zones_clamped_to_empty() {
    let config = GaugeChartConfig {
        min: 0.0,
        max: 100.0,
        zones: vec![
            GaugeZone::new(0.0, 50.0, "#a", "A"),
            GaugeZone::new(150.0, 200.0, "#b", "B"), // clamps to [100, 100] → skipped
            GaugeZone::new(50.0, 100.0, "#c", "C"),
        ],
        ..Default::default()
    };
    let layout = compute_gauge_layout(500.0, 375.0, 50.0, &config).unwrap();
    assert_eq!(layout.zone_arcs.len(), 2);
    assert_eq!(layout.zone_arcs[0].color, "#a");
    assert_eq!(layout.zone_arcs[1].color, "#c");
    // style decision tracks config.zones.len(), not rendered arcs
    assert!(layout.use_zone_style);
}

#[wasm_bindgen_test]
fn zone_arcs_skip_when_gap_collapses_zone() {
    let config = GaugeChartConfig {
        min: 0.0,
        max: 100.0,
        zone_gap_degrees: 10.0,
        zones: vec![
            GaugeZone::new(0.0, 10.0, "#a", "A"),
            GaugeZone::new(50.0, 51.0, "#b", "B"), // ~2.7° arc, 10° gap swamps it
            GaugeZone::new(90.0, 100.0, "#c", "C"),
        ],
        ..Default::default()
    };
    let layout = compute_gauge_layout(500.0, 375.0, 50.0, &config).unwrap();
    assert_eq!(layout.zone_arcs.len(), 2);
    assert_eq!(layout.zone_arcs[0].color, "#a");
    assert_eq!(layout.zone_arcs[1].color, "#c");
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

fn credit_score_config() -> GaugeChartConfig {
    GaugeChartConfig {
        min: 300.0,
        max: 850.0,
        zones: vec![
            GaugeZone::new(300.0, 579.0, "#e11d48", "VERY POOR"),
            GaugeZone::new(579.0, 669.0, "#d97706", "POOR"),
            GaugeZone::new(669.0, 739.0, "#eab308", "FAIR"),
            GaugeZone::new(739.0, 799.0, "#84cc16", "GOOD"),
            GaugeZone::new(799.0, 850.0, "#16a34a", "EXCELLENT"),
        ],
        ..Default::default()
    }
}

#[wasm_bindgen_test]
fn test_draw_gauge_chart_credit_score_style() {
    let Some(context) = mock_context() else {
        return;
    };
    draw_gauge_chart(&context, 500.0, 380.0, 800.0, &credit_score_config());
}

#[wasm_bindgen_test]
fn test_draw_gauge_chart_progress_style() {
    let Some(context) = mock_context() else {
        return;
    };
    let config = GaugeChartConfig {
        label: "Battery".to_string(),
        unit: "%".to_string(),
        ..Default::default()
    };
    draw_gauge_chart(&context, 500.0, 380.0, 72.0, &config);
}

#[wasm_bindgen_test]
fn test_draw_gauge_chart_small_canvas() {
    let Some(context) = mock_context() else {
        return;
    };
    draw_gauge_chart(&context, 180.0, 140.0, 800.0, &credit_score_config());
}
