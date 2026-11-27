use leptos::{
    ev,
    html::{Canvas, Div},
    prelude::*,
};
use leptos_use::{UseResizeObserverReturn, use_resize_observer};
use std::f64::consts::PI;
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement, wasm_bindgen::JsCast, window,
};

use crate::utils::number_format::format_int_with_commas;

#[derive(Clone, Debug, PartialEq)]
pub struct DoughnutChartConfig {
    pub show_legend: bool,
}

impl Default for DoughnutChartConfig {
    fn default() -> Self {
        Self { show_legend: true }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct SegmentPos {
    start_angle: f64,
    end_angle: f64,
    radius: f64,
    center_x: f64,
    center_y: f64,
    label: String,
    value: i64,
    color: String,
}

#[derive(Clone, Debug, PartialEq)]
struct DoughnutLayout {
    segments: Vec<SegmentPos>,
    center_x: f64,
    center_y: f64,
    radius: f64,
    inner_radius: f64,
    total: f64,
}

fn get_context(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .ok()??
        .dyn_into::<CanvasRenderingContext2d>()
        .ok()
}

#[component]
pub fn DoughnutChart(
    #[prop(into)] data: MaybeProp<Vec<(String, i64, String)>>,
    #[prop(optional, default = Default::default())] config: DoughnutChartConfig,
) -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let tooltip_ref = NodeRef::<Div>::new();
    let segment_positions = StoredValue::new(Vec::<SegmentPos>::new());
    let config = StoredValue::new(config);
    let container_ref = NodeRef::<Div>::new();

    let legend_meta = Memo::new(move |_| {
        data.get()
            .unwrap_or_default()
            .iter()
            .map(|(label, _, color)| (label.clone(), color.clone()))
            .collect::<Vec<_>>()
    });

    let show_legend = move || config.get_value().show_legend;

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
        let height = width * 0.8;

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

        let Some(data) = data.get_untracked() else {
            return;
        };

        let segments = draw_doughnut_chart(&context, width, height, &data);
        segment_positions.set_value(segments);
    };

    Effect::new(move |_| {
        let _ = data.get();
        redraw();
    });

    let redraw_for_observer = redraw.clone();
    let UseResizeObserverReturn { stop, .. } =
        use_resize_observer(container_ref, move |_entries, _observer| {
            redraw_for_observer();
        });

    let canvas_mousemove_handler = move |e: ev::MouseEvent| {
        let Some(canvas) = canvas_ref.get() else {
            return;
        };
        let canvas: HtmlCanvasElement = canvas.into();
        let Some(tooltip) = tooltip_ref.get() else {
            return;
        };

        let rect = canvas.get_bounding_client_rect();
        let x = e.client_x() as f64 - rect.left();
        let y = e.client_y() as f64 - rect.top();

        let segments = segment_positions.get_value();
        let hovered = find_segment_at(&segments, x, y);

        let tooltip_el: HtmlElement = tooltip.into();
        let style = tooltip_el.style();
        if let Some(seg) = hovered {
            let _ = style.set_property("display", "block");
            tooltip_el.set_inner_text(&format!(
                "{}: {}",
                seg.label,
                format_int_with_commas(seg.value)
            ));

            let tooltip_width = tooltip_el.offset_width() as f64;
            let canvas_width = canvas.client_width() as f64;
            let left = tooltip_left(x, tooltip_width, canvas_width, 10.0);

            let _ = style.set_property("left", &format!("{}px", left));
            let _ = style.set_property("top", &format!("{}px", y - 28.0));
        } else {
            let _ = style.set_property("display", "none");
        }
    };

    on_cleanup(move || {
        stop();
    });

    view! {
        <div style="width: 100%;">
            {move || show_legend().then(|| view! {
                <div style="display: flex; flex-direction: row; gap: 5px; flex-wrap: wrap; margin-bottom: 4px;">
                    {legend_meta.get().into_iter().map(|(label, color)| view! {
                        <div style="display: flex; flex-direction: row; align-items: center; gap: 2px;">
                            <span style="font-size: 10px;">{label}</span>
                            <div style=format!("background-color: {}; width: 10px; height: 10px; display: inline-block;", color)></div>
                        </div>
                    }).collect_view()}
                </div>
            })}
            <div node_ref=container_ref style="position: relative;">
                <canvas
                    node_ref=canvas_ref
                    style="width: 100%; height: 100%;"
                    on:mousemove=canvas_mousemove_handler
                ></canvas>
                <div
                    node_ref=tooltip_ref
                    style="
                        position: absolute;
                        display: none;
                        background: rgba(0,0,0,0.75);
                        color: white;
                        padding: 4px 8px;
                        border-radius: 4px;
                        font-size: 13px;
                        pointer-events: none;
                    "
                />
            </div>
        </div>
    }
}

fn compute_doughnut_layout(
    width: f64,
    height: f64,
    data: &[(String, i64, String)],
) -> Option<DoughnutLayout> {
    if data.is_empty() || width <= 0.0 || height <= 0.0 {
        return None;
    }

    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let radius = (width.min(height) / 2.0) * 0.85;
    let inner_radius = radius * 0.5;

    let total: f64 = data.iter().map(|(_, value, _)| *value as f64).sum();
    if total == 0.0 {
        return None;
    }

    let mut start_angle = -PI / 2.0;
    let mut segments = Vec::with_capacity(data.len());

    for (label, value, color) in data {
        let sweep_angle = (*value as f64 / total) * 2.0 * PI;
        let end_angle = start_angle + sweep_angle;

        segments.push(SegmentPos {
            start_angle,
            end_angle,
            radius,
            center_x,
            center_y,
            label: label.clone(),
            value: *value,
            color: color.clone(),
        });

        start_angle = end_angle;
    }

    Some(DoughnutLayout {
        segments,
        center_x,
        center_y,
        radius,
        inner_radius,
        total,
    })
}

fn draw_doughnut_chart(
    context: &CanvasRenderingContext2d,
    width: f64,
    height: f64,
    data: &[(String, i64, String)],
) -> Vec<SegmentPos> {
    let Some(layout) = compute_doughnut_layout(width, height, data) else {
        return vec![];
    };

    context.clear_rect(0.0, 0.0, width, height);

    for seg in &layout.segments {
        // fill segment
        context.begin_path();
        context.set_fill_style_str(&seg.color);
        context.move_to(seg.center_x, seg.center_y);
        let _ = context.arc(
            seg.center_x,
            seg.center_y,
            seg.radius,
            seg.start_angle,
            seg.end_angle,
        );
        context.line_to(seg.center_x, seg.center_y);
        context.fill();
        context.close_path();

        // outline segment
        context.begin_path();
        context.set_stroke_style_str("white");
        context.set_line_width(2.0);
        context.move_to(seg.center_x, seg.center_y);
        let _ = context.arc(
            seg.center_x,
            seg.center_y,
            seg.radius,
            seg.start_angle,
            seg.end_angle,
        );
        context.line_to(seg.center_x, seg.center_y);
        context.stroke();
        context.close_path();

        // cut inner hole
        context.begin_path();
        let _ = context.set_global_composite_operation("destination-out");
        let _ = context.arc(
            seg.center_x,
            seg.center_y,
            layout.inner_radius,
            0.0,
            2.0 * PI,
        );
        context.fill();
        context.close_path();
        let _ = context.set_global_composite_operation("source-over");
    }

    layout.segments
}

fn normalize_angle(angle: f64) -> f64 {
    if angle < -PI / 2.0 {
        angle + 2.0 * PI
    } else {
        angle
    }
}

fn find_segment_at<'a>(segments: &'a [SegmentPos], x: f64, y: f64) -> Option<&'a SegmentPos> {
    segments.iter().find(|s| {
        let dx = x - s.center_x;
        let dy = y - s.center_y;
        let dist = (dx * dx + dy * dy).sqrt();
        let inner_radius = s.radius * 0.5;

        if dist < inner_radius || dist > s.radius {
            return false;
        }

        let angle = normalize_angle(dy.atan2(dx));
        angle >= s.start_angle && angle <= s.end_angle
    })
}

fn tooltip_left(x: f64, tooltip_width: f64, canvas_width: f64, gap: f64) -> f64 {
    if x + gap + tooltip_width > canvas_width {
        (x - gap - tooltip_width).max(0.0)
    } else {
        x + gap
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
