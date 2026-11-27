use crate::utils::number_format::{format_int_with_commas, format_short_number, nice_ceiling};
use leptos::{
    ev,
    html::{Canvas, Div},
    prelude::*,
};
use leptos_use::{UseResizeObserverReturn, use_resize_observer};
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement, wasm_bindgen::JsCast, window,
};

#[derive(Clone, Debug, PartialEq)]
pub struct BarChartConfig {
    pub bar_color: String,
    pub grid_color: String,
    pub axis_color: String,
}

impl Default for BarChartConfig {
    fn default() -> Self {
        Self {
            bar_color: "blue".into(),
            grid_color: "#cccccc".into(),
            axis_color: "black".into(),
        }
    }
}

impl BarChartConfig {
    pub fn new(bar_color: &str, grid_color: &str, axis_color: &str) -> Self {
        Self {
            bar_color: bar_color.into(),
            grid_color: grid_color.into(),
            axis_color: axis_color.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DataPoint {
    pub name: String,
    pub value: i64,
}

impl DataPoint {
    pub fn new(name: &str, value: i64) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct BarRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    label: String,
    value: i64,
}

#[derive(Clone, Debug, PartialEq)]
struct GridLine {
    y: f64,
    value: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct XLabel {
    text: String,
    x: f64,
    y: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct ChartLayout {
    bar_rects: Vec<BarRect>,
    grid_lines: Vec<GridLine>,
    x_labels: Vec<XLabel>,
    left_padding: f64,
    axis_padding: f64,
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
pub fn BarChart(
    /// Accepts a reactive signal — update data and the chart redraws automatically.
    #[prop(into)]
    data: MaybeProp<Vec<DataPoint>>,
    #[prop(optional, default = Default::default())] config: BarChartConfig,
) -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let tooltip_ref = NodeRef::<Div>::new();
    let bar_rects = StoredValue::new(Vec::<BarRect>::new());
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

        let rects = draw_bar_chart(&context, width, height, &data, &config.get_value());
        bar_rects.set_value(rects);
    };

    // effect — redraw when data changes
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
        let rects = bar_rects.get_value();

        let hovered = find_bar_at(rects.as_slice(), x, y);

        let tooltip_el: HtmlElement = tooltip.into();
        let style = tooltip_el.style();
        if let Some(bar) = hovered {
            tooltip_el.set_inner_text(&format!(
                "{}: {}",
                bar.label,
                format_int_with_commas(bar.value)
            ));
            let _ = style.set_property("display", "block");

            // measure after content is set so offset_width reflects the new content
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

fn compute_layout(width: f64, height: f64, data: &[DataPoint]) -> Option<ChartLayout> {
    if data.is_empty() || width <= 0.0 || height <= 0.0 {
        return None;
    }

    let axis_padding = (height * 0.12).clamp(28.0, 50.0);
    let left_padding = (width * 0.12).clamp(35.0, 50.0);

    let bar_padding = 0.3;
    let num_bars = data.len() as f64;
    let slot_width = (width - left_padding) / num_bars;
    let bar_width = slot_width * (1.0 - bar_padding);
    let bar_spacing = slot_width * bar_padding;

    let max_raw = data.iter().map(|p| p.value).max()?;
    let max_value = if max_raw <= 0 {
        1.0
    } else {
        nice_ceiling(max_raw as f64)
    };

    let available_height = height - axis_padding * 2.0;
    let num_grid_lines = 5;
    let step_value = max_value / num_grid_lines as f64;
    let step_height = available_height / num_grid_lines as f64;

    let grid_lines = (0..=num_grid_lines)
        .map(|i| GridLine {
            y: height - axis_padding - i as f64 * step_height,
            value: i as f64 * step_value,
        })
        .collect();

    let bar_rects = data
        .iter()
        .enumerate()
        .map(|(i, point)| {
            let x = left_padding + i as f64 * slot_width + bar_spacing / 2.0;
            let y = height - axis_padding - point.value as f64 * (available_height / max_value);
            let bar_height = height - axis_padding - y;

            BarRect {
                x,
                y,
                width: bar_width,
                height: bar_height,
                label: point.name.clone(),
                value: point.value,
            }
        })
        .collect();

    let x_labels = data
        .iter()
        .enumerate()
        .map(|(i, point)| XLabel {
            text: point.name.clone(),
            x: left_padding + i as f64 * slot_width + slot_width / 2.0,
            y: height - axis_padding / 2.0,
        })
        .collect();

    Some(ChartLayout {
        bar_rects,
        grid_lines,
        x_labels,
        left_padding,
        axis_padding,
        max_value,
    })
}

fn draw_bar_chart(
    context: &CanvasRenderingContext2d,
    width: f64,
    height: f64,
    data: &[DataPoint],
    config: &BarChartConfig,
) -> Vec<BarRect> {
    let Some(layout) = compute_layout(width, height, data) else {
        return vec![];
    };

    context.clear_rect(0.0, 0.0, width, height);

    // grid
    context.set_stroke_style_str(&config.grid_color);
    context.set_line_width(1.0);
    context.set_fill_style_str(&config.axis_color);
    context.set_text_align("right");
    context.set_text_baseline("middle");
    context.set_font(&format!("{}px Arial", (height * 0.045).clamp(9.0, 12.0)));

    for line in &layout.grid_lines {
        context.begin_path();
        context.move_to(layout.left_padding, line.y);
        context.line_to(width, line.y);
        context.stroke();
        let _ = context.fill_text(
            &format_short_number(line.value),
            layout.left_padding - 10.0,
            line.y,
        );
    }

    // bars
    context.set_fill_style_str(&config.bar_color);
    for bar in &layout.bar_rects {
        context.fill_rect(bar.x, bar.y, bar.width, bar.height);
    }

    // x labels
    context.set_fill_style_str(&config.axis_color);
    context.set_text_align("right");
    context.set_text_baseline("middle");
    context.set_font(&format!("{}px Arial", (height * 0.04).clamp(8.0, 11.0)));

    for label in &layout.x_labels {
        context.save();
        let _ = context.translate(label.x, label.y);
        let _ = context.rotate(-std::f64::consts::PI / 4.0);
        let _ = context.fill_text(&label.text, 0.0, 0.0);
        context.restore();
    }

    layout.bar_rects
}

fn find_bar_at(rects: &[BarRect], x: f64, y: f64) -> Option<&BarRect> {
    rects
        .iter()
        .find(|b| x >= b.x && x <= b.x + b.width && y >= b.y && y <= b.y + b.height)
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
