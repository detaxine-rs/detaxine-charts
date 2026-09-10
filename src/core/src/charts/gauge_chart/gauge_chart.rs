use leptos::{
    html::{Canvas, Div},
    prelude::*,
};
use leptos_use::{UseResizeObserverReturn, use_resize_observer};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, wasm_bindgen::JsCast, window};

use crate::utils::number_format::format_with_commas;

#[derive(Clone, Debug, PartialEq)]
pub struct GaugeZone {
    pub from: f64,
    pub to: f64,
    pub color: String,
    pub label: Option<String>,
}

impl GaugeZone {
    pub fn new(from: f64, to: f64, color: &str, label: &str) -> Self {
        Self {
            from,
            to,
            color: color.into(),
            label: Some(label.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GaugeChartConfig {
    pub min: f64,
    pub max: f64,
    /// Total arc sweep in degrees, centered at the top (12 o'clock).
    /// 180.0 gives a classic half-circle; 270.0 (the default) gives the
    /// wider speedometer/credit-score style with an open gap at the bottom.
    pub sweep_degrees: f64,
    /// Colored bands. When empty, the gauge renders as a single-color
    /// progress arc instead.
    pub zones: Vec<GaugeZone>,
    /// Visual gap between adjacent zones, in degrees.
    pub zone_gap_degrees: f64,
    pub track_color: String,
    pub progress_color: String,
    pub needle_color: String,
    /// Width of the needle's base, in pixels. The needle tapers from this
    /// width down to a point at the tip.
    pub needle_base_width: f64,
    pub arc_thickness: f64,
    pub show_value: bool,
    /// Overrides the auto-detected zone color for the value text.
    pub value_color: Option<String>,
    pub pivot_border_color: String,
    pub unit: String,
    /// Static label shown under the value (e.g. "Credit Score"). If left
    /// empty and zones are configured, the currently active zone's label
    /// (e.g. "EXCELLENT") is shown instead.
    pub label: String,
}

impl Default for GaugeChartConfig {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            sweep_degrees: 270.0,
            zones: vec![],
            zone_gap_degrees: 0.5,
            track_color: "#e5e7eb".into(),
            progress_color: "#4f46e5".into(),
            needle_color: "#111827".into(),
            needle_base_width: 10.0,
            arc_thickness: 26.0,
            show_value: true,
            value_color: None,
            pivot_border_color: "#d1d5db".into(),
            unit: String::new(),
            label: String::new(),
        }
    }
}

fn get_context(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .ok()??
        .dyn_into::<CanvasRenderingContext2d>()
        .ok()
}

/// Maps a value within [min, max] to an angle along the gauge sweep,
/// which is always centered on the top (12 o'clock, angle `3*PI/2`).
fn value_to_angle(value: f64, min: f64, max: f64, start_angle: f64, end_angle: f64) -> f64 {
    let range = (max - min).max(f64::EPSILON);
    let ratio = ((value - min) / range).clamp(0.0, 1.0);
    start_angle + ratio * (end_angle - start_angle)
}

fn find_active_zone(value: f64, zones: &[GaugeZone]) -> Option<&GaugeZone> {
    zones.iter().find(|z| value >= z.from && value <= z.to)
}

/// Samples the angle range to find the bounding box multipliers
/// (min/max cos and sin) so the gauge can be sized to fit any sweep,
/// including sweeps wider than 180° that dip below the horizontal.
fn angle_extent(start: f64, end: f64) -> (f64, f64, f64, f64) {
    let samples = 180;
    let mut min_cos = start.cos().min(end.cos());
    let mut max_cos = start.cos().max(end.cos());
    let mut min_sin = start.sin().min(end.sin());
    let mut max_sin = start.sin().max(end.sin());

    for i in 0..=samples {
        let t = start + (end - start) * (i as f64 / samples as f64);
        min_cos = min_cos.min(t.cos());
        max_cos = max_cos.max(t.cos());
        min_sin = min_sin.min(t.sin());
        max_sin = max_sin.max(t.sin());
    }

    (min_cos, max_cos, min_sin, max_sin)
}

#[component]
pub fn GaugeChart(
    /// Accepts a reactive signal — update the value and the needle redraws to match.
    #[prop(into)]
    value: MaybeProp<f64>,
    #[prop(optional, default = Default::default())] config: GaugeChartConfig,
) -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let config = StoredValue::new(config);
    let container_ref = NodeRef::<Div>::new();

    let redraw = move || {
        let Some(canvas) = canvas_ref.get() else {
            return;
        };
        let canvas: HtmlCanvasElement = canvas.into();
        let Some(context) = get_context(&canvas) else {
            return;
        };
        let Some(win) = window() else { return };

        let device_pixel_ratio = win.device_pixel_ratio();
        let Some(parent) = canvas.parent_element() else {
            return;
        };
        let width = parent.client_width() as f64;
        let height = width * 0.75;

        if width < 1.0 || height < 1.0 {
            return;
        }

        canvas.set_width((width * device_pixel_ratio) as u32);
        canvas.set_height((height * device_pixel_ratio) as u32);

        if context.reset_transform().is_err() {
            return;
        };
        if context
            .scale(device_pixel_ratio, device_pixel_ratio)
            .is_err()
        {
            return;
        };

        let current_value = value.get_untracked().unwrap_or(0.0);
        draw_gauge_chart(&context, width, height, current_value, &config.get_value());
    };

    Effect::new(move |_| {
        let _ = value.get(); // tracked
        redraw();
    });

    let redraw_for_observer = redraw.clone(); // redraw needs to be Fn, not FnOnce — see note below
    let UseResizeObserverReturn { stop, .. } =
        use_resize_observer(container_ref, move |_entries, _observer| {
            redraw_for_observer();
        });

    on_cleanup(move || {
        stop();
    });

    view! {
        <div node_ref=container_ref style="width: 100%;">
            <canvas node_ref=canvas_ref style="width: 100%; height: 100%;"></canvas>
        </div>
    }
}

fn draw_gauge_chart(
    context: &CanvasRenderingContext2d,
    width: f64,
    height: f64,
    value: f64,
    config: &GaugeChartConfig,
) {
    let clamped_value = value.clamp(config.min, config.max);

    let mid_angle = 3.0 * std::f64::consts::PI / 2.0; // 12 o'clock, fixed center of the sweep
    let half_sweep = config.sweep_degrees.to_radians() / 2.0;
    let start_angle = mid_angle - half_sweep;
    let end_angle = mid_angle + half_sweep;

    let label_margin = 26.0;
    let value_text_reserve = 56.0; // accommodates the largest realistic pivot+label size

    let (min_cos, max_cos, min_sin, max_sin) = angle_extent(start_angle, end_angle);

    let radius_from_width = (width - label_margin * 2.0) / (max_cos - min_cos).max(0.01);
    let radius_from_height =
        (height - label_margin * 2.0 - value_text_reserve) / (max_sin - min_sin).max(0.01);
    let radius = radius_from_width.min(radius_from_height).max(10.0);

    let center_x = width / 2.0;
    let center_y = label_margin - radius * min_sin;

    context.clear_rect(0.0, 0.0, width, height);
    context.set_line_width(config.arc_thickness);

    if config.zones.is_empty() {
        // progress style — track plus single-color fill up to the value,
        // rounded at both true ends since there are no internal seams
        context.set_line_cap("round");
        context.set_stroke_style_str(&config.track_color);
        context.begin_path();
        let _ = context.arc(center_x, center_y, radius, start_angle, end_angle);
        context.stroke();

        let value_angle = value_to_angle(
            clamped_value,
            config.min,
            config.max,
            start_angle,
            end_angle,
        );
        context.set_stroke_style_str(&config.progress_color);
        context.begin_path();
        let _ = context.arc(center_x, center_y, radius, start_angle, value_angle);
        context.stroke();
    } else {
        // zone style — colored bands with a small gap between each.
        // Every zone is stroked with a sharp "butt" cap; a separate tiny
        // rounded stroke is added only at the two true outer ends of the
        // whole sweep, since canvas can't apply different caps to each
        // end of a single stroke.
        let gap = config.zone_gap_degrees.to_radians();
        let zone_count = config.zones.len();
        let cap_angle = 0.001; // just enough angle to render a round cap, not a visible arc

        for (i, zone) in config.zones.iter().enumerate() {
            let from = zone.from.clamp(config.min, config.max);
            let to = zone.to.clamp(config.min, config.max);
            if to <= from {
                continue;
            }

            let is_first = i == 0;
            let is_last = i == zone_count - 1;

            let zone_start_raw =
                value_to_angle(from, config.min, config.max, start_angle, end_angle);
            let zone_end_raw = value_to_angle(to, config.min, config.max, start_angle, end_angle);

            let zone_start = if is_first {
                zone_start_raw
            } else {
                zone_start_raw + gap / 2.0
            };
            let zone_end = if is_last {
                zone_end_raw
            } else {
                zone_end_raw - gap / 2.0
            };

            if zone_end <= zone_start {
                continue;
            }

            context.set_stroke_style_str(&zone.color);

            // main band — sharp butt cap on both ends
            context.set_line_cap("butt");
            context.begin_path();
            let _ = context.arc(center_x, center_y, radius, zone_start, zone_end);
            context.stroke();

            // round cap added only at the gauge's true start
            if is_first {
                context.set_line_cap("round");
                context.begin_path();
                let _ = context.arc(
                    center_x,
                    center_y,
                    radius,
                    zone_start,
                    zone_start + cap_angle,
                );
                context.stroke();
            }

            // round cap added only at the gauge's true end
            if is_last {
                context.set_line_cap("round");
                context.begin_path();
                let _ = context.arc(center_x, center_y, radius, zone_end - cap_angle, zone_end);
                context.stroke();
            }
        }
    }

    // pivot circle sized relative to the gauge radius so it scales on small screens
    let pivot_radius = (radius * 0.35).clamp(20.0, 70.0);
    let value_font_size = (pivot_radius * 0.55).max(11.0);
    let label_font_size = (pivot_radius * 0.32).max(8.0);

    // needle — a narrow triangle that starts from the edge of the pivot
    // circle (rather than its center) and points out to the arc, so the
    // pivot circle reads as the axis the needle rotates around
    let needle_angle = value_to_angle(
        clamped_value,
        config.min,
        config.max,
        start_angle,
        end_angle,
    );
    let needle_start = pivot_radius;
    let needle_end = radius - config.arc_thickness - 4.0;
    let perp_angle = needle_angle + std::f64::consts::PI / 2.0;
    let half_base = config.needle_base_width / 2.0;

    let tip_x = center_x + needle_end * needle_angle.cos();
    let tip_y = center_y + needle_end * needle_angle.sin();
    let base_center_x = center_x + needle_start * needle_angle.cos();
    let base_center_y = center_y + needle_start * needle_angle.sin();
    let base_left_x = base_center_x + half_base * perp_angle.cos();
    let base_left_y = base_center_y + half_base * perp_angle.sin();
    let base_right_x = base_center_x - half_base * perp_angle.cos();
    let base_right_y = base_center_y - half_base * perp_angle.sin();

    context.set_fill_style_str(&config.needle_color);
    context.begin_path();
    context.move_to(tip_x, tip_y);
    context.line_to(base_left_x, base_left_y);
    context.line_to(base_right_x, base_right_y);
    context.close_path();
    context.fill();

    // pivot circle — thin outlined ring the needle visually rotates around,
    // drawn after the needle so it cleanly covers the needle's wide base
    context.set_fill_style_str("white");
    context.begin_path();
    let _ = context.arc(
        center_x,
        center_y,
        pivot_radius,
        0.0,
        2.0 * std::f64::consts::PI,
    );
    context.fill();

    context.set_stroke_style_str(&config.pivot_border_color);
    context.set_line_width(1.5);
    context.begin_path();
    let _ = context.arc(
        center_x,
        center_y,
        pivot_radius,
        0.0,
        2.0 * std::f64::consts::PI,
    );
    context.stroke();

    // resolve which color and label to show: an explicit override takes
    // precedence, otherwise fall back to the active zone, otherwise the
    // plain progress color and the static config label
    let active_zone = find_active_zone(clamped_value, &config.zones);
    let active_color = active_zone
        .map(|z| z.color.clone())
        .or_else(|| config.value_color.clone())
        .unwrap_or_else(|| config.progress_color.clone());

    let display_label = if !config.label.is_empty() {
        Some(config.label.clone())
    } else {
        active_zone.and_then(|z| z.label.clone())
    };

    if config.show_value {
        context.set_fill_style_str(&active_color);
        context.set_text_align("center");
        context.set_text_baseline("middle");
        context.set_font(&format!("bold {}px Arial", value_font_size));

        let value_text = if config.unit.is_empty() {
            format_with_commas(clamped_value, Some(0))
        } else {
            format!(
                "{} {}",
                format_with_commas(clamped_value, Some(0)),
                config.unit
            )
        };
        let _ = context.fill_text(&value_text, center_x, center_y);
    }

    if config.show_value {
        if let Some(label) = display_label {
            context.set_font(&format!("{}px Arial", label_font_size));
            context.set_fill_style_str(&active_color);
            context.set_text_align("center");
            context.set_text_baseline("middle");
            let _ = context.fill_text(
                &label,
                center_x,
                center_y + pivot_radius + label_font_size + 4.0,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

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
    fn test_draw_gauge_chart_credit_score_style() {
        let Some(context) = mock_context() else {
            return;
        };

        let config = GaugeChartConfig {
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
        };

        draw_gauge_chart(&context, 500.0, 380.0, 800.0, &config);
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

        let config = GaugeChartConfig {
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
        };

        draw_gauge_chart(&context, 180.0, 140.0, 800.0, &config);
    }
}
