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

use crate::utils::number_format::format_with_commas;

#[derive(Clone, Debug, PartialEq)]
pub struct PolarAreaChartConfig {
    pub show_legend: bool,
    pub show_grid: bool,
    pub num_grid_rings: usize,
    pub grid_color: String,
    pub label_color: String,
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

#[derive(Clone, Debug, PartialEq)]
struct SlicePos {
    start_angle: f64,
    end_angle: f64,
    slice_radius: f64,
    center_x: f64,
    center_y: f64,
    name: String,
    value: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct GridRing {
    radius: f64,
    value: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct PolarLayout {
    slices: Vec<SlicePos>,
    grid_rings: Vec<GridRing>,
    center_x: f64,
    center_y: f64,
    max_radius: f64,
    max_value: f64,
    label_font_size: f64,
}

fn get_context(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .ok()??
        .dyn_into::<CanvasRenderingContext2d>()
        .ok()
}

fn with_alpha(hex_color: &str, opacity: f64) -> String {
    let alpha_byte = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("{}{:02x}", hex_color, alpha_byte)
}

fn normalize_angle(angle: f64, min: f64) -> f64 {
    if angle < min { angle + 2.0 * PI } else { angle }
}

fn angle_in_slice(angle: f64, start: f64, end: f64) -> bool {
    if start <= end {
        angle >= start && angle <= end
    } else {
        // slice wraps across the 0/2π boundary (e.g. crosses 3 o'clock)
        angle >= start || angle <= end
    }
}

#[component]
pub fn PolarAreaChart(
    #[prop(into)] data: MaybeProp<Vec<DataPoint>>,
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

        let slices = slice_positions.get_value();
        let hovered = find_slice_at(&slices, x, y);

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
            let left = tooltip_left(x, tooltip_width, canvas_width, 10.0);

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

fn compute_polar_layout(
    width: f64,
    height: f64,
    data: &[DataPoint],
    config: &PolarAreaChartConfig,
) -> Option<PolarLayout> {
    if data.is_empty() || width <= 0.0 || height <= 0.0 {
        return None;
    }

    let label_margin = (height.min(width) * 0.12).clamp(20.0, 40.0);
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let max_radius = (width.min(height) / 2.0 - label_margin).max(10.0);

    let max_value = data.iter().map(|d| d.value).reduce(f64::max)?;
    if max_value <= 0.0 {
        return None;
    }

    let num_slices = data.len();
    let slice_angle = 2.0 * PI / num_slices as f64;
    let gap = (slice_angle * 0.04).min(0.04);

    let mut slices = Vec::with_capacity(num_slices);
    for (i, point) in data.iter().enumerate() {
        let raw_start = -PI / 2.0 + i as f64 * slice_angle;
        let raw_end = raw_start + slice_angle;
        let start_angle = normalize_angle(raw_start + gap / 2.0, 0.0);
        let end_angle = normalize_angle(raw_end - gap / 2.0, 0.0);

        // square-root scaling — a slice's visual area grows with radius squared,
        // so scaling radius linearly with value would exaggerate larger values
        let ratio = (point.value.max(0.0) / max_value).sqrt();
        let slice_radius = max_radius * ratio;

        slices.push(SlicePos {
            start_angle,
            end_angle,
            slice_radius,
            center_x,
            center_y,
            name: point.name.clone(),
            value: point.value,
        });
    }

    let num_rings = config.num_grid_rings.max(1);
    let grid_rings: Vec<GridRing> = (1..=num_rings)
        .map(|i| GridRing {
            radius: max_radius * (i as f64 / num_rings as f64),
            value: max_value * (i as f64 / num_rings as f64),
        })
        .collect();

    let label_font_size = (max_radius * 0.08).clamp(8.0, 11.0);

    Some(PolarLayout {
        slices,
        grid_rings,
        center_x,
        center_y,
        max_radius,
        max_value,
        label_font_size,
    })
}

fn draw_polar_area_chart(
    context: &CanvasRenderingContext2d,
    width: f64,
    height: f64,
    data: &[DataPoint],
    config: &PolarAreaChartConfig,
) -> Vec<SlicePos> {
    let Some(layout) = compute_polar_layout(width, height, data, config) else {
        return vec![];
    };

    context.clear_rect(0.0, 0.0, width, height);

    if config.show_grid {
        context.set_stroke_style_str(&config.grid_color);
        context.set_line_width(1.0);
        for ring in &layout.grid_rings {
            context.begin_path();
            let _ = context.arc(layout.center_x, layout.center_y, ring.radius, 0.0, 2.0 * PI);
            context.stroke();
        }

        context.set_fill_style_str(&config.label_color);
        context.set_font(&format!("{}px Arial", layout.label_font_size));
        context.set_text_align("center");
        context.set_text_baseline("middle");
        for ring in &layout.grid_rings {
            let _ = context.fill_text(
                &format_with_commas(ring.value, Some(0)),
                layout.center_x,
                layout.center_y - ring.radius,
            );
        }
    }

    for (i, slice) in layout.slices.iter().enumerate() {
        // color comes from the caller's data (layout doesn't carry it);
        // safe today because slices is built 1:1 from data by index.
        let color = &data[i].color;

        context.set_fill_style_str(&with_alpha(color, config.fill_opacity));
        context.begin_path();
        context.move_to(layout.center_x, layout.center_y);
        let _ = context.arc(
            layout.center_x,
            layout.center_y,
            slice.slice_radius,
            slice.start_angle,
            slice.end_angle,
        );
        context.close_path();
        context.fill();

        context.set_stroke_style_str(color);
        context.set_line_width(1.5);
        context.stroke();
    }

    layout.slices
}

fn find_slice_at<'a>(slices: &'a [SlicePos], x: f64, y: f64) -> Option<&'a SlicePos> {
    slices.iter().find(|s| {
        let dx = x - s.center_x;
        let dy = y - s.center_y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > s.slice_radius {
            return false;
        }
        let angle = normalize_angle(dy.atan2(dx), 0.0);
        angle_in_slice(angle, s.start_angle, s.end_angle)
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
