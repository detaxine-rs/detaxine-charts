use leptos::{
    ev,
    html::{Canvas, Div},
    prelude::*,
};
use leptos_use::{UseResizeObserverReturn, use_resize_observer};
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement, wasm_bindgen::JsCast, window,
};

use crate::utils::number_format::{format_int_with_commas, format_short_number, nice_ceiling};

#[derive(Clone, Debug, PartialEq)]
pub struct LineCurveChartConfig {
    pub show_grid: bool,
    pub show_legend: bool,
    pub show_inflection_points: bool,
    pub show_x_axis: bool,
    pub show_y_axis: bool,
    pub show_x_axis_labels: bool,
    pub show_y_axis_labels: bool,
    pub stroke_width: f64,
    pub show_area_chart: bool,
    pub x_axis_title: String,
    pub y_axis_title: String,
}

impl Default for LineCurveChartConfig {
    fn default() -> Self {
        Self {
            show_grid: true,
            show_legend: true,
            show_inflection_points: true,
            show_x_axis: true,
            show_y_axis: true,
            show_x_axis_labels: true,
            show_y_axis_labels: true,
            stroke_width: 2.0,
            show_area_chart: false,
            x_axis_title: String::new(),
            y_axis_title: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DataPoint {
    pub y: i64,
}

impl DataPoint {
    pub fn new(y: i64) -> Self {
        Self { y }
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
    label: String,
    value: i64,
    series_name: String,
    series_color: String,
    series_index: usize,
    point_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct GridLine {
    y: f64,
    value: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct MultilineLayout {
    point_positions: Vec<PointPos>,
    grid_lines: Vec<GridLine>,
    left_padding: f64,
    axis_padding: f64,
    point_spacing: f64,
    num_points: usize,
    max_value: f64,
}

fn get_context(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .ok()??
        .dyn_into::<CanvasRenderingContext2d>()
        .ok()
}

#[component]
pub fn LineCurveChart(
    #[prop(into)] data: MaybeProp<Vec<(Series, Vec<DataPoint>)>>,
    #[prop(into)] x: MaybeProp<Vec<String>>,
    #[prop(optional, default = Default::default())] config: LineCurveChartConfig,
) -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let tooltip_ref = NodeRef::<Div>::new();
    let crosshair_ref = NodeRef::<Div>::new();
    let point_positions = StoredValue::new(Vec::<PointPos>::new());
    let config = StoredValue::new(config);
    let container_ref = NodeRef::<Div>::new();

    let series_meta = Memo::new(move |_| {
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
        let height = width * 0.6;

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

        let positions = draw_multiline_chart(
            &context,
            width,
            height,
            &data,
            &x.get_untracked().unwrap_or_default(),
            &config.get_value(),
        );
        point_positions.set_value(positions);
    };

    Effect::new(move |_| {
        let _ = data.get();
        let _ = x.get();
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
        let Some(crosshair) = crosshair_ref.get() else {
            return;
        };

        let rect = canvas.get_bounding_client_rect();
        let x = e.client_x() as f64 - rect.left();
        let y = e.client_y() as f64 - rect.top();

        let positions = point_positions.get_value();

        let tooltip_el: HtmlElement = tooltip.into();
        let crosshair_el: HtmlElement = crosshair.into();
        let tooltip_style = tooltip_el.style();
        let crosshair_style = crosshair_el.style();

        let hovered = find_hovered_points(&positions, x, 40.0, 0.5);
        let Some((closest_x, matched)) = hovered else {
            let _ = tooltip_style.set_property("display", "none");
            let _ = crosshair_style.set_property("display", "none");
            return;
        };

        // crosshair
        let _ = crosshair_style.set_property("display", "block");
        let _ = crosshair_style.set_property("left", &format!("{}px", closest_x));
        let _ = crosshair_style.set_property("height", &format!("{}px", canvas.client_height()));

        // tooltip content
        let label_point = matched[0];
        let mut html = format!("<strong>{}</strong>", label_point.label);
        for p in &matched {
            html.push_str(&format!(
                "<br/><span style=\"color:{}\">●</span> {}: {}",
                p.series_color,
                p.series_name,
                format_int_with_commas(p.value)
            ));
        }

        let _ = tooltip_style.set_property("display", "block");
        tooltip_el.set_inner_html(&html);

        let tooltip_width = tooltip_el.offset_width() as f64;
        let canvas_width = canvas.client_width() as f64;
        // NOTE: gap 12.0 / top offset -10.0 differ from other charts (10.0 / -28.0).
        let left = tooltip_left(closest_x, tooltip_width, canvas_width, 12.0);
        let _ = tooltip_style.set_property("left", &format!("{}px", left));
        let _ = tooltip_style.set_property("top", &format!("{}px", y - 10.0));
    };

    let canvas_mouseleave_handler = move |_: ev::MouseEvent| {
        if let Some(tooltip) = tooltip_ref.get() {
            let tooltip_el: HtmlElement = tooltip.into();
            let _ = tooltip_el.style().set_property("display", "none");
        }
        if let Some(crosshair) = crosshair_ref.get() {
            let crosshair_el: HtmlElement = crosshair.into();
            let _ = crosshair_el.style().set_property("display", "none");
        }
    };

    on_cleanup(move || {
        stop();
    });

    view! {
        <div style="width: 100%;">
            {move || show_legend().then(|| view! {
                <div style="display: flex; flex-direction: row; gap: 5px; flex-wrap: wrap; margin-bottom: 4px;">
                    {series_meta.get().into_iter().map(|(name, color)| view! {
                        <div style="display: flex; flex-direction: row; align-items: center; gap: 2px;">
                            <span style="font-size: 10px;">{name}</span>
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
                    on:mouseleave=canvas_mouseleave_handler
                ></canvas>
                <div
                    node_ref=crosshair_ref
                    style="
                        position: absolute;
                        top: 0;
                        display: none;
                        width: 0;
                        border-left: 1px dashed #9ca3af;
                        pointer-events: none;
                    "
                />
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

fn compute_multiline_layout(
    width: f64,
    height: f64,
    data: &[(Series, Vec<DataPoint>)],
    x_labels: &[String],
) -> Option<MultilineLayout> {
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    let max_raw = data
        .iter()
        .flat_map(|(_, points)| points.iter().map(|d| d.y))
        .max()?;
    let max_value = nice_ceiling(max_raw as f64 * 1.0);

    let first = data.first()?;
    let num_points = first.1.len();
    if num_points < 2 {
        return None;
    }

    let axis_padding = (height * 0.12).clamp(28.0, 50.0);
    let left_padding = (width * 0.12).clamp(35.0, 50.0);
    let available_height = height - axis_padding * 2.0;
    let point_spacing = (width - left_padding - axis_padding) / (num_points as f64 - 1.0);

    let num_grid_lines = 5;
    let step_value = max_value / num_grid_lines as f64;
    let step_height = available_height / num_grid_lines as f64;

    let grid_lines: Vec<GridLine> = (0..=num_grid_lines)
        .map(|i| GridLine {
            y: height - axis_padding - i as f64 * step_height,
            value: (i as f64 * step_value).round(),
        })
        .collect();

    let mut point_positions = Vec::new();
    for (s_idx, (series, points)) in data.iter().enumerate() {
        for (p_idx, datapoint) in points.iter().enumerate() {
            let x = left_padding + p_idx as f64 * point_spacing;
            let y = height - axis_padding - (datapoint.y as f64 / max_value) * available_height;

            point_positions.push(PointPos {
                x,
                y,
                label: x_labels.get(p_idx).cloned().unwrap_or_default(),
                value: datapoint.y,
                series_name: series.name.clone(),
                series_color: series.color.clone(),
                series_index: s_idx,
                point_index: p_idx,
            });
        }
    }

    Some(MultilineLayout {
        point_positions,
        grid_lines,
        left_padding,
        axis_padding,
        point_spacing,
        num_points,
        max_value,
    })
}

fn draw_multiline_chart(
    context: &CanvasRenderingContext2d,
    width: f64,
    height: f64,
    data: &[(Series, Vec<DataPoint>)],
    x_labels: &[String],
    config: &LineCurveChartConfig,
) -> Vec<PointPos> {
    let Some(layout) = compute_multiline_layout(width, height, data, x_labels) else {
        return vec![];
    };

    let y_label_font_size = (height * 0.045).clamp(9.0, 12.0);
    let x_label_font_size = (height * 0.04).clamp(8.0, 11.0);
    let title_font_size = (height * 0.045).clamp(10.0, 13.0);

    context.clear_rect(0.0, 0.0, width, height);

    if config.show_x_axis {
        context.set_stroke_style_str("#cccccc");
        context.set_line_width(1.0);
        context.begin_path();
        context.move_to(layout.left_padding, height - layout.axis_padding);
        context.line_to(width, height - layout.axis_padding);
        context.stroke();
    }

    if config.show_y_axis {
        context.set_stroke_style_str("#cccccc");
        context.set_line_width(1.0);
        context.begin_path();
        context.move_to(layout.left_padding, 0.0);
        context.line_to(layout.left_padding, height - layout.axis_padding);
        context.stroke();
    }

    context.set_stroke_style_str("#cccccc");
    context.set_line_width(1.0);
    context.set_fill_style_str("black");
    context.set_text_align("right");
    context.set_text_baseline("middle");
    context.set_font(&format!("{}px Arial", y_label_font_size));

    for line in &layout.grid_lines {
        if config.show_grid {
            context.begin_path();
            context.move_to(layout.left_padding, line.y);
            context.line_to(width, line.y);
            context.stroke();
        }
        if config.show_y_axis_labels {
            let _ = context.fill_text(
                &format_short_number(line.value),
                layout.left_padding - 10.0,
                line.y,
            );
        }
    }

    for (s_idx, (series, points)) in data.iter().enumerate() {
        let series_points: Vec<&PointPos> = layout
            .point_positions
            .iter()
            .filter(|p| p.series_index == s_idx)
            .collect();
        if series_points.is_empty() {
            continue;
        }

        context.set_stroke_style_str(series.color.as_str());
        context.set_line_width(config.stroke_width);

        context.begin_path();
        context.move_to(series_points[0].x, series_points[0].y);
        for i in 1..series_points.len() {
            let prev = series_points[i - 1];
            let curr = series_points[i];
            let ctrl1_x = prev.x + layout.point_spacing / 3.0;
            let ctrl1_y = prev.y;
            let ctrl2_x = curr.x - layout.point_spacing / 3.0;
            let ctrl2_y = curr.y;
            context.bezier_curve_to(ctrl1_x, ctrl1_y, ctrl2_x, ctrl2_y, curr.x, curr.y);
        }
        context.stroke();

        if config.show_area_chart {
            context.line_to(
                layout.left_padding + (points.len() as f64 - 1.0) * layout.point_spacing,
                height - layout.axis_padding,
            );
            context.line_to(layout.left_padding, height - layout.axis_padding);
            context.close_path();
            let fill_color = format!("{}33", &series.color);
            context.set_fill_style_str(&fill_color);
            context.fill();
        }

        if config.show_inflection_points {
            context.set_fill_style_str(series.color.as_str());
            for point in &series_points {
                context.begin_path();
                let _ = context.arc(point.x, point.y, 3.0, 0.0, std::f64::consts::PI * 2.0);
                context.fill();
            }
        }
    }

    if config.show_x_axis_labels {
        context.set_fill_style_str("black");
        context.set_text_align("right");
        context.set_text_baseline("middle");
        context.set_font(&format!("{}px Arial", x_label_font_size));
        for (i, x_label) in x_labels.iter().enumerate() {
            let x = layout.left_padding + i as f64 * layout.point_spacing;
            let y = height - layout.axis_padding / 2.0;
            context.save();
            let _ = context.translate(x, y);
            let _ = context.rotate(-std::f64::consts::PI / 4.0);
            let _ = context.fill_text(x_label.as_str(), 0.0, 0.0);
            context.restore();
        }
    }

    if !config.x_axis_title.is_empty() {
        context.set_text_align("center");
        context.set_font(&format!("bold {}px Arial", title_font_size));
        let _ = context.fill_text(
            &config.x_axis_title,
            (layout.left_padding + width) / 2.0,
            height - layout.axis_padding / 4.0,
        );
    }

    if !config.y_axis_title.is_empty() {
        context.set_text_align("center");
        context.set_text_baseline("middle");
        context.set_font(&format!("bold {}px Arial", title_font_size));
        context.save();
        let _ = context.rotate(-std::f64::consts::PI / 2.0);
        let _ = context.fill_text(
            &config.y_axis_title,
            -(height / 2.0),
            layout.left_padding / 4.0,
        );
        context.restore();
    }

    layout.point_positions
}

fn find_hovered_points<'a>(
    positions: &'a [PointPos],
    cursor_x: f64,
    hit_threshold: f64,
    epsilon: f64,
) -> Option<(f64, Vec<&'a PointPos>)> {
    let first = positions.first()?;

    let mut closest_x = first.x;
    let mut min_dist = (closest_x - cursor_x).abs();
    for p in positions {
        let d = (p.x - cursor_x).abs();
        if d < min_dist {
            min_dist = d;
            closest_x = p.x;
        }
    }

    if min_dist > hit_threshold {
        return None;
    }

    let matched: Vec<&PointPos> = positions
        .iter()
        .filter(|p| (p.x - closest_x).abs() < epsilon)
        .collect();

    if matched.is_empty() {
        None
    } else {
        Some((closest_x, matched))
    }
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
