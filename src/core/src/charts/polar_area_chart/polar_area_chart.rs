use leptos::{
    ev,
    html::{Canvas, Div},
    prelude::*,
};
use leptos_use::{UseResizeObserverReturn, use_resize_observer};
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement, wasm_bindgen::JsCast, window,
};

use crate::utils::number_format::format_with_commas;

#[derive(Clone, Debug, PartialEq)]
pub struct PolarAreaChartConfig {
    pub show_legend: bool,
    pub show_grid: bool,
    pub num_grid_rings: usize,
    pub grid_color: String,
    pub label_color: String,
    /// Opacity applied to each slice's fill, from 0.0 (fully transparent)
    /// to 1.0 (fully opaque). Slightly transparent fills make overlapping
    /// gridlines and the segment outlines easier to see.
    pub fill_opacity: f64,
}

impl Default for PolarAreaChartConfig {
    fn default() -> Self {
        Self {
            show_legend: true,
            show_grid: true,
            num_grid_rings: 4,
            grid_color: "#e5e7eb".into(),
            label_color: "#6b7280".into(),
            fill_opacity: 0.85,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DataPoint {
    pub name: String,
    pub value: f64,
    pub color: String,
}

impl DataPoint {
    pub fn new(name: &str, value: f64, color: &str) -> Self {
        Self {
            name: name.into(),
            value,
            color: color.into(),
        }
    }
}

#[derive(Clone, Debug)]
struct SlicePos {
    start_angle: f64,
    end_angle: f64,
    slice_radius: f64,
    center_x: f64,
    center_y: f64,
    name: String,
    value: f64,
}

fn get_context(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .ok()??
        .dyn_into::<CanvasRenderingContext2d>()
        .ok()
}

/// Applies an alpha channel to a hex color string (e.g. "#4f46e5" + 0.85
/// becomes "#4f46e5d9"), used to fade slice fills without needing the
/// caller to pre-format colors with alpha themselves.
fn with_alpha(hex_color: &str, opacity: f64) -> String {
    let alpha_byte = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("{}{:02x}", hex_color, alpha_byte)
}

#[component]
pub fn PolarAreaChart(
    /// Accepts a reactive signal — update data and the chart redraws automatically.
    #[prop(into)]
    data: MaybeProp<Vec<DataPoint>>,
    #[prop(optional, default = Default::default())] config: PolarAreaChartConfig,
) -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let tooltip_ref = NodeRef::<Div>::new();
    let slice_positions = StoredValue::new(Vec::<SlicePos>::new());
    let config = StoredValue::new(config);
    let container_ref = NodeRef::<Div>::new();

    let legend_meta = Memo::new(move |_| {
        data.get()
            .unwrap_or_default()
            .iter()
            .map(|d| (d.name.clone(), d.color.clone()))
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

        let Some(chart_data) = data.get_untracked() else {
            return;
        };

        let slices =
            draw_polar_area_chart(&context, width, height, &chart_data, &config.get_value());
        slice_positions.set_value(slices);
    };

    Effect::new(move |_| {
        let _ = data.get(); // tracked
        redraw();
    });

    let redraw_for_observer = redraw.clone(); // redraw needs to be Fn, not FnOnce — see note below
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

        let hovered = slice_positions.get_value().into_iter().find(|s| {
            let dx = x - s.center_x;
            let dy = y - s.center_y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > s.slice_radius {
                return false;
            }

            let mut angle = dy.atan2(dx);
            if angle < 0.0 {
                angle += 2.0 * std::f64::consts::PI;
            }

            if s.start_angle <= s.end_angle {
                angle >= s.start_angle && angle <= s.end_angle
            } else {
                // slice wraps across the 0/2π boundary (e.g. crosses 3 o'clock)
                angle >= s.start_angle || angle <= s.end_angle
            }
        });

        let tooltip_el: HtmlElement = tooltip.into();
        let style = tooltip_el.style();
        if let Some(slice) = hovered {
            tooltip_el.set_inner_text(&format!(
                "{}: {}",
                slice.name,
                format_with_commas(slice.value, Some(0))
            ));
            let _ = style.set_property("display", "block");

            let tooltip_width = tooltip_el.offset_width() as f64;
            let canvas_width = canvas.client_width() as f64;
            let gap = 10.0;
            let left = if x + gap + tooltip_width > canvas_width {
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

    let canvas_mouseleave_handler = move |_: ev::MouseEvent| {
        if let Some(tooltip) = tooltip_ref.get() {
            let tooltip_el: HtmlElement = tooltip.into();
            let _ = tooltip_el.style().set_property("display", "none");
        }
    };

    on_cleanup(move || {
        stop();
    });

    view! {
        <div style="width: 100%; height: 100%; display: flex; flex-direction: column;">
            {move || show_legend().then(|| view! {
                <div style="display: flex; flex-direction: row; gap: 5px; flex-wrap: wrap; margin-bottom: 4px;">
                    {legend_meta.get().into_iter().map(|(name, color)| view! {
                        <div style="display: flex; flex-direction: row; align-items: center; gap: 2px;">
                            <span style="font-size: 10px;">{name}</span>
                            <div style=format!("background-color: {}; width: 10px; height: 10px; display: inline-block;", color)></div>
                        </div>
                    }).collect_view()}
                </div>
            })}
            <div node_ref=container_ref style="position: relative; flex: 1; min-height: 0;">
                <canvas
                    node_ref=canvas_ref
                    style="width: 100%; height: 100%;"
                    on:mousemove=canvas_mousemove_handler
                    on:mouseleave=canvas_mouseleave_handler
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
                        white-space: nowrap;
                    "
                />
            </div>
        </div>
    }
}

fn draw_polar_area_chart(
    context: &CanvasRenderingContext2d,
    width: f64,
    height: f64,
    data: &[DataPoint],
    config: &PolarAreaChartConfig,
) -> Vec<SlicePos> {
    if data.is_empty() {
        return vec![];
    };

    let label_margin = (height.min(width) * 0.12).clamp(20.0, 40.0);
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let max_radius = (width.min(height) / 2.0 - label_margin).max(10.0);

    let Some(max_value) = data.iter().map(|d| d.value).reduce(f64::max) else {
        return vec![];
    };
    if max_value <= 0.0 {
        return vec![];
    }

    context.clear_rect(0.0, 0.0, width, height);

    // grid rings — concentric circles giving a visual reference for
    // how each slice's radius maps back to a value
    if config.show_grid {
        let num_rings = config.num_grid_rings.max(1);
        context.set_stroke_style_str(&config.grid_color);
        context.set_line_width(1.0);

        for i in 1..=num_rings {
            let ring_radius = max_radius * (i as f64 / num_rings as f64);
            context.begin_path();
            let _ = context.arc(
                center_x,
                center_y,
                ring_radius,
                0.0,
                2.0 * std::f64::consts::PI,
            );
            context.stroke();
        }

        // ring value labels, placed along a fixed vertical axis above center
        let label_font_size = (max_radius * 0.08).clamp(8.0, 11.0);
        context.set_fill_style_str(&config.label_color);
        context.set_font(&format!("{}px Arial", label_font_size));
        context.set_text_align("center");
        context.set_text_baseline("middle");
        for i in 1..=num_rings {
            let ring_radius = max_radius * (i as f64 / num_rings as f64);
            let ring_value = max_value * (i as f64 / num_rings as f64);
            let _ = context.fill_text(
                &format_with_commas(ring_value, Some(0)),
                center_x,
                center_y - ring_radius,
            );
        }
    }

    let num_slices = data.len();
    let slice_angle = 2.0 * std::f64::consts::PI / num_slices as f64;
    let gap = (slice_angle * 0.04).min(0.04); // small visual gap between adjacent slices

    let mut slice_positions = Vec::new();

    for (i, point) in data.iter().enumerate() {
        let raw_start = -std::f64::consts::PI / 2.0 + i as f64 * slice_angle;
        let raw_end = raw_start + slice_angle;
        let start_angle = raw_start + gap / 2.0;
        let end_angle = raw_end - gap / 2.0;

        // square-root scaling — radius represents the value, but since a
        // slice's visual area grows with radius squared, scaling radius
        // linearly with value would make larger values look exaggerated
        let ratio = (point.value.max(0.0) / max_value).sqrt();
        let slice_radius = max_radius * ratio;

        context.set_fill_style_str(&with_alpha(&point.color, config.fill_opacity));
        context.begin_path();
        context.move_to(center_x, center_y);
        let _ = context.arc(center_x, center_y, slice_radius, start_angle, end_angle);
        context.close_path();
        context.fill();

        context.set_stroke_style_str(&point.color);
        context.set_line_width(1.5);
        context.stroke();

        slice_positions.push(SlicePos {
            start_angle: if start_angle < 0.0 {
                start_angle + 2.0 * std::f64::consts::PI
            } else {
                start_angle
            },
            end_angle: if end_angle < 0.0 {
                end_angle + 2.0 * std::f64::consts::PI
            } else {
                end_angle
            },
            slice_radius,
            center_x,
            center_y,
            name: point.name.clone(),
            value: point.value,
        });
    }

    slice_positions
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
    fn test_draw_polar_area_chart() {
        let Some(context) = mock_context() else {
            return;
        };

        let data = vec![
            DataPoint::new("Mon", 12.0, "#4f46e5"),
            DataPoint::new("Tue", 19.0, "#e11d48"),
            DataPoint::new("Wed", 7.0, "#0891b2"),
            DataPoint::new("Thu", 15.0, "#16a34a"),
            DataPoint::new("Fri", 22.0, "#d97706"),
        ];

        draw_polar_area_chart(
            &context,
            500.0,
            500.0,
            &data,
            &PolarAreaChartConfig::default(),
        );
    }

    #[wasm_bindgen_test]
    fn test_draw_polar_area_chart_empty() {
        let Some(context) = mock_context() else {
            return;
        };

        draw_polar_area_chart(
            &context,
            500.0,
            500.0,
            &[],
            &PolarAreaChartConfig::default(),
        );
    }
}
