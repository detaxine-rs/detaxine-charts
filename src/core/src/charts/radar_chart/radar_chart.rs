use leptos::{
    ev,
    html::{Canvas, Div},
    prelude::*,
};
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
    /// Opacity applied to each series' fill, from 0.0 (no fill) to 1.0
    /// (fully opaque). A semi-transparent fill lets overlapping series
    /// remain distinguishable.
    pub fill_opacity: f64,
    /// Maximum value represented by the outer ring. When `None`, it's
    /// derived automatically from the largest value across all series.
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

#[derive(Clone, Debug)]
struct PointPos {
    x: f64,
    y: f64,
    axis_label: String,
    value: f64,
    series_name: String,
    series_color: String,
}

fn get_context(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .ok()??
        .dyn_into::<CanvasRenderingContext2d>()
        .ok()
}

/// Applies an alpha channel to a hex color string, used to fade series
/// fills without requiring the caller to pre-format colors with alpha.
fn with_alpha(hex_color: &str, opacity: f64) -> String {
    let alpha_byte = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("{}{:02x}", hex_color, alpha_byte)
}

#[component]
pub fn RadarChart(
    /// Reactive signal — update data and the chart redraws automatically.
    /// Each tuple is one series plotted across every axis.
    #[prop(into)]
    data: MaybeProp<Vec<(Series, Vec<f64>)>>,
    /// Reactive signal — the axis labels, one per spoke (e.g. skill names).
    #[prop(into)]
    axes: MaybeProp<Vec<String>>,
    #[prop(optional, default = Default::default())] config: RadarChartConfig,
) -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let tooltip_ref = NodeRef::<Div>::new();
    let point_positions = StoredValue::new(Vec::<PointPos>::new());
    let config = StoredValue::new(config);

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
        let parent_height = parent.client_height() as f64;
        let height = if parent_height > 50.0 {
            parent_height
        } else {
            width * 0.8
        };

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
        let _ = data.get(); // tracked
        let _ = axes.get(); // tracked
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

        let hit_radius = 8.0;
        let hovered = point_positions.get_value().into_iter().find(|p| {
            let dx = lx - p.x;
            let dy = ly - p.y;
            (dx * dx + dy * dy).sqrt() <= hit_radius
        });

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
        resize_listener.remove();
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
            <div style="position: relative; flex: 1; min-height: 0;">
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

fn draw_radar_chart(
    context: &CanvasRenderingContext2d,
    width: f64,
    height: f64,
    data: &[(Series, Vec<f64>)],
    axis_labels: &[String],
    config: &RadarChartConfig,
) -> Vec<PointPos> {
    let num_axes = axis_labels.len();
    if num_axes < 3 || data.is_empty() {
        return vec![];
    };

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

    context.clear_rect(0.0, 0.0, width, height);

    let angle_for = |i: usize| {
        -std::f64::consts::PI / 2.0 + i as f64 * (2.0 * std::f64::consts::PI / num_axes as f64)
    };

    // grid rings — concentric polygons (not circles) since a radar chart's
    // axes are discrete spokes, not a continuous radial scale
    if config.show_grid {
        let num_rings = config.num_grid_rings.max(1);
        context.set_stroke_style_str(&config.grid_color);
        context.set_line_width(1.0);

        for ring in 1..=num_rings {
            let ring_radius = max_radius * (ring as f64 / num_rings as f64);
            context.begin_path();
            for i in 0..num_axes {
                let angle = angle_for(i);
                let x = center_x + ring_radius * angle.cos();
                let y = center_y + ring_radius * angle.sin();
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

    // spokes — one line from center to each axis label
    context.set_stroke_style_str(&config.axis_color);
    context.set_line_width(1.0);
    for i in 0..num_axes {
        let angle = angle_for(i);
        context.begin_path();
        context.move_to(center_x, center_y);
        context.line_to(
            center_x + max_radius * angle.cos(),
            center_y + max_radius * angle.sin(),
        );
        context.stroke();
    }

    // axis labels, placed just outside the outer ring at each spoke's angle
    let label_font_size = (max_radius * 0.07).clamp(9.0, 12.0);
    context.set_fill_style_str(&config.label_color);
    context.set_font(&format!("{}px Arial", label_font_size));
    context.set_text_baseline("middle");
    for (i, label) in axis_labels.iter().enumerate() {
        let angle = angle_for(i);
        let label_radius = max_radius + label_font_size + 6.0;
        let lx = center_x + label_radius * angle.cos();
        let ly = center_y + label_radius * angle.sin();

        // align text away from the center so labels don't overlap the chart,
        // based on which side of the vertical axis the spoke falls on
        let cos = angle.cos();
        context.set_text_align(if cos.abs() < 0.15 {
            "center"
        } else if cos > 0.0 {
            "left"
        } else {
            "right"
        });

        let _ = context.fill_text(label, lx, ly);
    }

    let mut point_positions = Vec::new();

    for (series, values) in data {
        if values.len() != num_axes {
            continue;
        }

        context.set_stroke_style_str(series.color.as_str());
        context.set_line_width(config.stroke_width);
        context.set_fill_style_str(&with_alpha(&series.color, config.fill_opacity));

        context.begin_path();
        for i in 0..num_axes {
            let angle = angle_for(i);
            let ratio = (values[i].max(0.0) / max_value).min(1.0);
            let r = max_radius * ratio;
            let x = center_x + r * angle.cos();
            let y = center_y + r * angle.sin();

            if i == 0 {
                context.move_to(x, y);
            } else {
                context.line_to(x, y);
            }
        }
        context.close_path();
        context.fill();
        context.stroke();

        for i in 0..num_axes {
            let angle = angle_for(i);
            let ratio = (values[i].max(0.0) / max_value).min(1.0);
            let r = max_radius * ratio;
            let x = center_x + r * angle.cos();
            let y = center_y + r * angle.sin();

            if config.show_points {
                context.set_fill_style_str(series.color.as_str());
                context.begin_path();
                let _ = context.arc(x, y, 3.0, 0.0, 2.0 * std::f64::consts::PI);
                context.fill();
            }

            point_positions.push(PointPos {
                x,
                y,
                axis_label: axis_labels.get(i).cloned().unwrap_or_default(),
                value: values[i],
                series_name: series.name.clone(),
                series_color: series.color.clone(),
            });
        }
    }

    point_positions
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
    fn test_draw_radar_chart() {
        let Some(context) = mock_context() else {
            return;
        };

        let axes = vec![
            "Speed".to_string(),
            "Power".to_string(),
            "Defense".to_string(),
            "Agility".to_string(),
            "Stamina".to_string(),
        ];

        let data = vec![
            (
                Series::new("Player A", "#4f46e5"),
                vec![80.0, 65.0, 70.0, 90.0, 60.0],
            ),
            (
                Series::new("Player B", "#e11d48"),
                vec![60.0, 85.0, 75.0, 55.0, 80.0],
            ),
        ];

        draw_radar_chart(
            &context,
            500.0,
            500.0,
            &data,
            &axes,
            &RadarChartConfig::default(),
        );
    }

    #[wasm_bindgen_test]
    fn test_draw_radar_chart_too_few_axes() {
        let Some(context) = mock_context() else {
            return;
        };

        let axes = vec!["A".to_string(), "B".to_string()];
        let data = vec![(Series::new("X", "#4f46e5"), vec![1.0, 2.0])];

        draw_radar_chart(
            &context,
            500.0,
            500.0,
            &data,
            &axes,
            &RadarChartConfig::default(),
        );
    }
}
