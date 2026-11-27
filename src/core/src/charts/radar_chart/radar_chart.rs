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
pub struct RadarChartConfig {
    pub show_legend: bool,
    pub show_grid: bool,
    pub num_grid_rings: usize,
    pub grid_color: String,
    pub axis_color: String,
    pub label_color: String,
    pub show_points: bool,
    pub stroke_width: f64,
    pub fill_opacity: f64,
    pub max_value: Option<f64>,
}

impl Default for RadarChartConfig {
    fn default() -> Self {
        Self {
            show_legend: true,
            show_grid: true,
            num_grid_rings: 4,
            grid_color: "#e5e7eb".into(),
            axis_color: "#d1d5db".into(),
            label_color: "#6b7280".into(),
            show_points: true,
            stroke_width: 2.0,
            fill_opacity: 0.2,
            max_value: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    pub name: String,
    pub color: String,
}

impl Series {
    pub fn new(name: &str, color: &str) -> Self {
        Self {
            name: name.into(),
            color: color.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PointPos {
    x: f64,
    y: f64,
    axis_label: String,
    value: f64,
    series_name: String,
    series_color: String,
    series_index: usize,
    axis_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct AxisLabel {
    x: f64,
    y: f64,
    text: String,
    align: &'static str, // "center" | "left" | "right"
}

#[derive(Clone, Debug, PartialEq)]
struct RadarLayout {
    points: Vec<PointPos>,
    grid_radii: Vec<f64>,
    spoke_ends: Vec<(f64, f64)>,
    axis_labels: Vec<AxisLabel>,
    center_x: f64,
    center_y: f64,
    max_radius: f64,
    label_font_size: f64,
    max_value: f64,
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

fn axis_angle(i: usize, num_axes: usize) -> f64 {
    -PI / 2.0 + i as f64 * (2.0 * PI / num_axes as f64)
}

fn label_text_align(cos: f64) -> &'static str {
    if cos.abs() < 0.15 {
        "center"
    } else if cos > 0.0 {
        "left"
    } else {
        "right"
    }
}

#[component]
pub fn RadarChart(
    #[prop(into)] data: MaybeProp<Vec<(Series, Vec<f64>)>>,
    #[prop(into)] axes: MaybeProp<Vec<String>>,
    #[prop(optional, default = Default::default())] config: RadarChartConfig,
) -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let tooltip_ref = NodeRef::<Div>::new();
    let point_positions = StoredValue::new(Vec::<PointPos>::new());
    let config = StoredValue::new(config);
    let container_ref = NodeRef::<Div>::new();

    let legend_meta = Memo::new(move |_| {
        data.get()
            .unwrap_or_default()
            .iter()
            .map(|(s, _)| (s.name.clone(), s.color.clone()))
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
        let chart_axes = axes.get_untracked().unwrap_or_default();

        let positions = draw_radar_chart(
            &context,
            width,
            height,
            &chart_data,
            &chart_axes,
            &config.get_value(),
        );
        point_positions.set_value(positions);
    };

    Effect::new(move |_| {
        let _ = data.get();
        let _ = axes.get();
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

        let points = point_positions.get_value();
        let hovered = find_point_at(&points, x, y, 8.0);

        let tooltip_el: HtmlElement = tooltip.into();
        let style = tooltip_el.style();
        if let Some(point) = hovered {
            tooltip_el.set_inner_html(&format!(
                "<strong>{}</strong><br/>{}: {}",
                point.axis_label,
                point.series_name,
                format_with_commas(point.value, Some(1))
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
                        line-height: 1.6;
                        pointer-events: none;
                        white-space: nowrap;
                    "
                />
            </div>
        </div>
    }
}

fn compute_radar_layout(
    width: f64,
    height: f64,
    data: &[(Series, Vec<f64>)],
    axis_labels: &[String],
    config: &RadarChartConfig,
) -> Option<RadarLayout> {
    let num_axes = axis_labels.len();
    if num_axes < 3 || data.is_empty() || width <= 0.0 || height <= 0.0 {
        return None;
    }

    let label_margin = (height.min(width) * 0.16).clamp(30.0, 60.0);
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let max_radius = (width.min(height) / 2.0 - label_margin).max(10.0);

    let max_value = config.max_value.unwrap_or_else(|| {
        data.iter()
            .flat_map(|(_, values)| values.iter().copied())
            .fold(0.0_f64, f64::max)
            .max(f64::EPSILON)
    });

    let num_rings = config.num_grid_rings.max(1);
    let grid_radii: Vec<f64> = (1..=num_rings)
        .map(|i| max_radius * (i as f64 / num_rings as f64))
        .collect();

    let spoke_ends: Vec<(f64, f64)> = (0..num_axes)
        .map(|i| {
            let angle = axis_angle(i, num_axes);
            (
                center_x + max_radius * angle.cos(),
                center_y + max_radius * angle.sin(),
            )
        })
        .collect();

    let label_font_size = (max_radius * 0.07).clamp(9.0, 12.0);
    let axis_labels_out: Vec<AxisLabel> = axis_labels
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let angle = axis_angle(i, num_axes);
            let label_radius = max_radius + label_font_size + 6.0;
            let cos = angle.cos();
            AxisLabel {
                x: center_x + label_radius * cos,
                y: center_y + label_radius * angle.sin(),
                text: text.clone(),
                align: label_text_align(cos),
            }
        })
        .collect();

    let mut points = Vec::new();
    for (s_idx, (series, values)) in data.iter().enumerate() {
        if values.len() != num_axes {
            continue;
        }
        for (i, value) in values.iter().enumerate() {
            let angle = axis_angle(i, num_axes);
            let ratio = (value.max(0.0) / max_value).min(1.0);
            let r = max_radius * ratio;
            points.push(PointPos {
                x: center_x + r * angle.cos(),
                y: center_y + r * angle.sin(),
                axis_label: axis_labels[i].clone(),
                value: *value,
                series_name: series.name.clone(),
                series_color: series.color.clone(),
                series_index: s_idx,
                axis_index: i,
            });
        }
    }

    Some(RadarLayout {
        points,
        grid_radii,
        spoke_ends,
        axis_labels: axis_labels_out,
        center_x,
        center_y,
        max_radius,
        label_font_size,
        max_value,
    })
}

fn draw_radar_chart(
    context: &CanvasRenderingContext2d,
    width: f64,
    height: f64,
    data: &[(Series, Vec<f64>)],
    axis_labels: &[String],
    config: &RadarChartConfig,
) -> Vec<PointPos> {
    let Some(layout) = compute_radar_layout(width, height, data, axis_labels, config) else {
        return vec![];
    };

    let num_axes = axis_labels.len();
    context.clear_rect(0.0, 0.0, width, height);

    // grid rings
    if config.show_grid {
        context.set_stroke_style_str(&config.grid_color);
        context.set_line_width(1.0);
        for ring_radius in &layout.grid_radii {
            context.begin_path();
            for i in 0..num_axes {
                let angle = axis_angle(i, num_axes);
                let x = layout.center_x + ring_radius * angle.cos();
                let y = layout.center_y + ring_radius * angle.sin();
                if i == 0 {
                    context.move_to(x, y);
                } else {
                    context.line_to(x, y);
                }
            }
            context.close_path();
            context.stroke();
        }
    }

    // spokes
    context.set_stroke_style_str(&config.axis_color);
    context.set_line_width(1.0);
    for (ex, ey) in &layout.spoke_ends {
        context.begin_path();
        context.move_to(layout.center_x, layout.center_y);
        context.line_to(*ex, *ey);
        context.stroke();
    }

    // axis labels
    context.set_fill_style_str(&config.label_color);
    context.set_font(&format!("{}px Arial", layout.label_font_size));
    context.set_text_baseline("middle");
    for label in &layout.axis_labels {
        context.set_text_align(label.align);
        let _ = context.fill_text(&label.text, label.x, label.y);
    }

    // series polygons + points
    for (s_idx, (series, values)) in data.iter().enumerate() {
        if values.len() != num_axes {
            continue;
        }

        let series_points: Vec<&PointPos> = layout
            .points
            .iter()
            .filter(|p| p.series_index == s_idx)
            .collect();
        if series_points.is_empty() {
            continue;
        }

        context.set_stroke_style_str(series.color.as_str());
        context.set_line_width(config.stroke_width);
        context.set_fill_style_str(&with_alpha(&series.color, config.fill_opacity));

        context.begin_path();
        for (i, p) in series_points.iter().enumerate() {
            if i == 0 {
                context.move_to(p.x, p.y);
            } else {
                context.line_to(p.x, p.y);
            }
        }
        context.close_path();
        context.fill();
        context.stroke();

        if config.show_points {
            context.set_fill_style_str(series.color.as_str());
            for p in &series_points {
                context.begin_path();
                let _ = context.arc(p.x, p.y, 3.0, 0.0, 2.0 * PI);
                context.fill();
            }
        }
    }

    layout.points
}

fn find_point_at<'a>(
    points: &'a [PointPos],
    x: f64,
    y: f64,
    hit_radius: f64,
) -> Option<&'a PointPos> {
    points.iter().find(|p| {
        let dx = x - p.x;
        let dy = y - p.y;
        (dx * dx + dy * dy).sqrt() <= hit_radius
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
