use leptos::{
    ev,
    html::{Canvas, Div},
    prelude::*,
};
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

#[derive(Clone, Debug)]
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

fn get_context(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .ok()??
        .dyn_into::<CanvasRenderingContext2d>()
        .ok()
}

#[component]
pub fn DoughnutChart(
    /// Accepts a reactive signal — update data and the chart redraws automatically.
    /// Each tuple is (label, value, color).
    #[prop(into)]
    data: MaybeProp<Vec<(String, i64, String)>>,
    #[prop(optional, default = Default::default())] config: DoughnutChartConfig,
) -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let tooltip_ref = NodeRef::<Div>::new();
    let segment_positions = StoredValue::new(Vec::<SegmentPos>::new());
    let config = StoredValue::new(config);

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
        let _ = data.get(); // tracked
        redraw();
    });

    let resize_listener = window_event_listener(ev::resize, move |_| {
        redraw();
    });

    let canvas_mousemove_handler = move |e: ev::MouseEvent| {
        let Some(canvas) = canvas_ref.get() else {
            return;
        };
        let canvas: HtmlCanvasElement = canvas.into();
        let Some(tooltip) = tooltip_ref.get() else {
            return;
        };
        let Some(win) = window() else { return };

        let rect = canvas.get_bounding_client_rect();
        let x = e.client_x() as f64 - rect.left();
        let y = e.client_y() as f64 - rect.top();

        let device_pixel_ratio = win.device_pixel_ratio();
        let scale_x = canvas.client_width() as f64 / canvas.width() as f64 * device_pixel_ratio;
        let scale_y = canvas.client_height() as f64 / canvas.height() as f64 * device_pixel_ratio;
        let lx = x * scale_x;
        let ly = y * scale_y;

        let hovered = segment_positions.get_value().into_iter().find(|s| {
            let dx = lx - s.center_x;
            let dy = ly - s.center_y;
            let dist = (dx * dx + dy * dy).sqrt();
            let inner_radius = s.radius * 0.5;

            if dist < inner_radius || dist > s.radius {
                return false;
            };

            let mut angle = dy.atan2(dx);
            if angle < -PI / 2.0 {
                angle += 2.0 * PI;
            }

            angle >= s.start_angle && angle <= s.end_angle
        });

        let tooltip_el: HtmlElement = tooltip.into();
        let style = tooltip_el.style();
        if let Some(seg) = hovered {
            let _ = style.set_property("display", "block");
            tooltip_el.set_inner_text(&format!(
                "{}: {}",
                seg.label,
                format_int_with_commas(seg.value)
            ));

            // measure after content is set so offset_width reflects the new content
            let tooltip_width = tooltip_el.offset_width() as f64;
            let canvas_width = canvas.client_width() as f64;
            let gap = 10.0;

            let left = if x + gap + tooltip_width > canvas_width {
                // flip to the left side of the cursor
                (x - gap - tooltip_width).max(0.0)
            } else {
                x + gap
            };

            let _ = style.set_property("left", &format!("{}px", left));
            let _ = style.set_property("top", &format!("{}px", y - 28.0));
        } else {
            let _ = style.set_property("display", "none");
        }
    };

    on_cleanup(move || {
        resize_listener.remove();
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
            <div style="position: relative;">
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

fn draw_doughnut_chart(
    context: &CanvasRenderingContext2d,
    width: f64,
    height: f64,
    data: &[(String, i64, String)],
) -> Vec<SegmentPos> {
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let radius = (width.min(height) / 2.0) * 0.85;
    let inner_radius = radius * 0.5;

    let total: f64 = data.iter().map(|(_, value, _)| *value as f64).sum();
    if total == 0.0 {
        return vec![];
    };

    context.clear_rect(0.0, 0.0, width, height);

    let mut start_angle = -PI / 2.0;
    let mut segment_positions = Vec::new();

    for (label, value, color) in data {
        let sweep_angle = (*value as f64 / total) * 2.0 * PI;
        let end_angle = start_angle + sweep_angle;

        // fill segment
        context.begin_path();
        context.set_fill_style_str(color.as_str());
        context.move_to(center_x, center_y);
        let _ = context.arc(center_x, center_y, radius, start_angle, end_angle);
        context.line_to(center_x, center_y);
        context.fill();
        context.close_path();

        // outline segment
        context.begin_path();
        context.set_stroke_style_str("white");
        context.set_line_width(2.0);
        context.move_to(center_x, center_y);
        let _ = context.arc(center_x, center_y, radius, start_angle, end_angle);
        context.line_to(center_x, center_y);
        context.stroke();
        context.close_path();

        // cut inner hole
        context.begin_path();
        let _ = context.set_global_composite_operation("destination-out");
        let _ = context.arc(center_x, center_y, inner_radius, 0.0, 2.0 * PI);
        context.fill();
        context.close_path();
        let _ = context.set_global_composite_operation("source-over");

        segment_positions.push(SegmentPos {
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

    segment_positions
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
    fn test_draw_doughnut_chart() {
        let Some(context) = mock_context() else {
            return;
        };

        let data = vec![
            ("A".to_string(), 10, "#ff0000".to_string()),
            ("B".to_string(), 20, "#00ff00".to_string()),
            ("C".to_string(), 30, "#0000ff".to_string()),
        ];

        draw_doughnut_chart(&context, 500.0, 500.0, &data);
    }
}
