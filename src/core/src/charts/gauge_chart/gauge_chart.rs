use leptos::{
    html::{Canvas, Div},
    prelude::*,
};
use leptos_use::{UseResizeObserverReturn, use_resize_observer};
use std::f64::consts::PI;
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
    pub sweep_degrees: f64,
    pub zones: Vec<GaugeZone>,
    pub zone_gap_degrees: f64,
    pub track_color: String,
    pub progress_color: String,
    pub needle_color: String,
    pub needle_base_width: f64,
    pub arc_thickness: f64,
    pub show_value: bool,
    pub value_color: Option<String>,
    pub pivot_border_color: String,
    pub unit: String,
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

#[derive(Clone, Debug, PartialEq)]
struct ZoneArc {
    start_angle: f64,
    end_angle: f64,
    is_first: bool,
    is_last: bool,
    color: String,
}

#[derive(Clone, Debug, PartialEq)]
struct NeedleTriangle {
    tip: (f64, f64),
    base_left: (f64, f64),
    base_right: (f64, f64),
}

#[derive(Clone, Debug, PartialEq)]
struct GaugeLayout {
    clamped_value: f64,
    start_angle: f64,
    end_angle: f64,
    value_angle: f64,
    center_x: f64,
    center_y: f64,
    radius: f64,
    pivot_radius: f64,
    value_font_size: f64,
    label_font_size: f64,
    needle: NeedleTriangle,
    zone_arcs: Vec<ZoneArc>,
    use_zone_style: bool,
    active_color: String,
    display_label: Option<String>,
}

fn get_context(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .ok()??
        .dyn_into::<CanvasRenderingContext2d>()
        .ok()
}

fn value_to_angle(value: f64, min: f64, max: f64, start_angle: f64, end_angle: f64) -> f64 {
    let range = (max - min).max(f64::EPSILON);
    let ratio = ((value - min) / range).clamp(0.0, 1.0);
    start_angle + ratio * (end_angle - start_angle)
}

fn find_active_zone(value: f64, zones: &[GaugeZone]) -> Option<&GaugeZone> {
    zones.iter().find(|z| value >= z.from && value <= z.to)
}

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
    #[prop(into)] value: MaybeProp<f64>,
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
        let _ = value.get();
        redraw();
    });

    let redraw_for_observer = redraw.clone();
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

fn compute_zone_arcs(config: &GaugeChartConfig, start_angle: f64, end_angle: f64) -> Vec<ZoneArc> {
    let gap = config.zone_gap_degrees.to_radians();
    let zone_count = config.zones.len();
    let mut result = Vec::new();

    for (i, zone) in config.zones.iter().enumerate() {
        let from = zone.from.clamp(config.min, config.max);
        let to = zone.to.clamp(config.min, config.max);
        if to <= from {
            continue;
        }

        let is_first = i == 0;
        let is_last = i == zone_count - 1;

        let zone_start_raw = value_to_angle(from, config.min, config.max, start_angle, end_angle);
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

        result.push(ZoneArc {
            start_angle: zone_start,
            end_angle: zone_end,
            is_first,
            is_last,
            color: zone.color.clone(),
        });
    }

    result
}

fn compute_gauge_layout(
    width: f64,
    height: f64,
    value: f64,
    config: &GaugeChartConfig,
) -> Option<GaugeLayout> {
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    let clamped_value = value.clamp(config.min, config.max);

    let mid_angle = 3.0 * PI / 2.0;
    let half_sweep = config.sweep_degrees.to_radians() / 2.0;
    let start_angle = mid_angle - half_sweep;
    let end_angle = mid_angle + half_sweep;

    let label_margin = 26.0;
    let value_text_reserve = 56.0;

    let (min_cos, max_cos, min_sin, max_sin) = angle_extent(start_angle, end_angle);

    let radius_from_width = (width - label_margin * 2.0) / (max_cos - min_cos).max(0.01);
    let radius_from_height =
        (height - label_margin * 2.0 - value_text_reserve) / (max_sin - min_sin).max(0.01);
    let radius = radius_from_width.min(radius_from_height).max(10.0);

    let center_x = width / 2.0;
    let center_y = label_margin - radius * min_sin;

    let pivot_radius = (radius * 0.35).clamp(20.0, 70.0);
    let value_font_size = (pivot_radius * 0.55).max(11.0);
    let label_font_size = (pivot_radius * 0.32).max(8.0);

    let value_angle = value_to_angle(
        clamped_value,
        config.min,
        config.max,
        start_angle,
        end_angle,
    );

    let needle_start = pivot_radius;
    let needle_end = radius - config.arc_thickness - 4.0;
    let perp_angle = value_angle + PI / 2.0;
    let half_base = config.needle_base_width / 2.0;

    let tip = (
        center_x + needle_end * value_angle.cos(),
        center_y + needle_end * value_angle.sin(),
    );
    let base_center_x = center_x + needle_start * value_angle.cos();
    let base_center_y = center_y + needle_start * value_angle.sin();
    let base_left = (
        base_center_x + half_base * perp_angle.cos(),
        base_center_y + half_base * perp_angle.sin(),
    );
    let base_right = (
        base_center_x - half_base * perp_angle.cos(),
        base_center_y - half_base * perp_angle.sin(),
    );

    let needle = NeedleTriangle {
        tip,
        base_left,
        base_right,
    };

    let zone_arcs = compute_zone_arcs(config, start_angle, end_angle);
    let use_zone_style = !config.zones.is_empty();

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

    Some(GaugeLayout {
        clamped_value,
        start_angle,
        end_angle,
        value_angle,
        center_x,
        center_y,
        radius,
        pivot_radius,
        value_font_size,
        label_font_size,
        needle,
        zone_arcs,
        use_zone_style,
        active_color,
        display_label,
    })
}

fn draw_gauge_chart(
    context: &CanvasRenderingContext2d,
    width: f64,
    height: f64,
    value: f64,
    config: &GaugeChartConfig,
) {
    let Some(layout) = compute_gauge_layout(width, height, value, config) else {
        return;
    };

    context.clear_rect(0.0, 0.0, width, height);
    context.set_line_width(config.arc_thickness);

    if layout.use_zone_style {
        let cap_angle = 0.001;
        for arc in &layout.zone_arcs {
            context.set_stroke_style_str(&arc.color);
            context.set_line_cap("butt");
            context.begin_path();
            let _ = context.arc(
                layout.center_x,
                layout.center_y,
                layout.radius,
                arc.start_angle,
                arc.end_angle,
            );
            context.stroke();

            if arc.is_first {
                context.set_line_cap("round");
                context.begin_path();
                let _ = context.arc(
                    layout.center_x,
                    layout.center_y,
                    layout.radius,
                    arc.start_angle,
                    arc.start_angle + cap_angle,
                );
                context.stroke();
            }

            if arc.is_last {
                context.set_line_cap("round");
                context.begin_path();
                let _ = context.arc(
                    layout.center_x,
                    layout.center_y,
                    layout.radius,
                    arc.end_angle - cap_angle,
                    arc.end_angle,
                );
                context.stroke();
            }
        }
    } else {
        context.set_line_cap("round");
        context.set_stroke_style_str(&config.track_color);
        context.begin_path();
        let _ = context.arc(
            layout.center_x,
            layout.center_y,
            layout.radius,
            layout.start_angle,
            layout.end_angle,
        );
        context.stroke();

        context.set_stroke_style_str(&config.progress_color);
        context.begin_path();
        let _ = context.arc(
            layout.center_x,
            layout.center_y,
            layout.radius,
            layout.start_angle,
            layout.value_angle,
        );
        context.stroke();
    }

    // needle
    context.set_fill_style_str(&config.needle_color);
    context.begin_path();
    context.move_to(layout.needle.tip.0, layout.needle.tip.1);
    context.line_to(layout.needle.base_left.0, layout.needle.base_left.1);
    context.line_to(layout.needle.base_right.0, layout.needle.base_right.1);
    context.close_path();
    context.fill();

    // pivot fill
    context.set_fill_style_str("white");
    context.begin_path();
    let _ = context.arc(
        layout.center_x,
        layout.center_y,
        layout.pivot_radius,
        0.0,
        2.0 * PI,
    );
    context.fill();

    // pivot outline
    context.set_stroke_style_str(&config.pivot_border_color);
    context.set_line_width(1.5);
    context.begin_path();
    let _ = context.arc(
        layout.center_x,
        layout.center_y,
        layout.pivot_radius,
        0.0,
        2.0 * PI,
    );
    context.stroke();

    if config.show_value {
        context.set_fill_style_str(&layout.active_color);
        context.set_text_align("center");
        context.set_text_baseline("middle");
        context.set_font(&format!("bold {}px Arial", layout.value_font_size));

        let value_text = if config.unit.is_empty() {
            format_with_commas(layout.clamped_value, Some(0))
        } else {
            format!(
                "{} {}",
                format_with_commas(layout.clamped_value, Some(0)),
                config.unit
            )
        };
        let _ = context.fill_text(&value_text, layout.center_x, layout.center_y);
    }

    if config.show_value {
        if let Some(label) = &layout.display_label {
            context.set_font(&format!("{}px Arial", layout.label_font_size));
            context.set_fill_style_str(&layout.active_color);
            context.set_text_align("center");
            context.set_text_baseline("middle");
            let _ = context.fill_text(
                label,
                layout.center_x,
                layout.center_y + layout.pivot_radius + layout.label_font_size + 4.0,
            );
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
