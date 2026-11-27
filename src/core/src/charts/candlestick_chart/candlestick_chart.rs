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
pub struct CandlestickChartConfig {
    pub bullish_color: String,
    pub bearish_color: String,
    pub wick_color: String,
    pub show_grid: bool,
}

impl Default for CandlestickChartConfig {
    fn default() -> Self {
        Self {
            bullish_color: "#16a34a".into(),
            bearish_color: "#e11d48".into(),
            wick_color: "#6b7280".into(),
            show_grid: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Candle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub label: String,
}

impl Candle {
    pub fn new(label: &str, open: f64, high: f64, low: f64, close: f64) -> Self {
        Self {
            open,
            high,
            low,
            close,
            label: label.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct CandlePos {
    x: f64,
    y: f64, // top of wick = to_y(high)
    width: f64,
    height: f64,      // wick span = to_y(low) - to_y(high)
    body_top: f64,    // to_y(open.max(close))
    body_bottom: f64, // to_y(open.min(close))
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    label: String,
    is_bullish: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct GridLine {
    y: f64,
    value: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct CandleChartLayout {
    candle_positions: Vec<CandlePos>,
    grid_lines: Vec<GridLine>,
    left_padding: f64,
    axis_padding: f64,
    slot_width: f64,
    min_value: f64,
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
pub fn CandlestickChart(
    #[prop(into)] data: MaybeProp<Vec<Candle>>,
    #[prop(optional, default = Default::default())] config: CandlestickChartConfig,
) -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let tooltip_ref = NodeRef::<Div>::new();
    let crosshair_ref = NodeRef::<Div>::new();
    let candle_positions = StoredValue::new(Vec::<CandlePos>::new());
    let config = StoredValue::new(config);
    let container_ref = NodeRef::<Div>::new();

    let view_start = RwSignal::new(0usize);
    let view_end = RwSignal::new(data.get_untracked().unwrap_or_default().len());
    let pinned_to_latest = RwSignal::new(true);

    let is_dragging = StoredValue::new(false);
    let drag_start_x = StoredValue::new(0.0f64);
    let drag_start_view = StoredValue::new((0usize, 0usize));

    let hovered_index = RwSignal::new(None::<usize>);
    let mouse_x = StoredValue::new(0.0f64);

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
        let Some((width, height)) = canvas_dimensions(&canvas) else {
            return;
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

        let Some(all_data) = data.get() else { return };
        let total = all_data.len();

        let start = view_start.get().min(total.saturating_sub(1));
        let end = view_end.get().min(total);
        let (start, end) = if start >= end {
            (end.saturating_sub(1), end)
        } else {
            (start, end)
        };

        let data_slice = &all_data[start..end];
        let config_val = config.get_value();

        let candles = draw_candlestick_chart(&context, width, height, data_slice, &config_val);
        candle_positions.set_value(candles.clone());

        // Keep tooltip & crosshair in sync with fresh data while hovering
        if let Some(idx) = hovered_index.get_untracked() {
            if let Some(candle) = candles.get(idx) {
                if let Some(tooltip) = tooltip_ref.get_untracked() {
                    let tooltip_el: HtmlElement = tooltip.into();
                    tooltip_el.set_inner_html(&format!(
                        "<strong>{}</strong><br/>O: {:.2}  H: {:.2}  L: {:.2}  C: {:.2}",
                        candle.label, candle.open, candle.high, candle.low, candle.close,
                    ));

                    let canvas_width = canvas.client_width() as f64;
                    let tooltip_width = tooltip_el.offset_width() as f64;
                    let cursor_x = mouse_x.get_value();
                    let left = tooltip_left(cursor_x, tooltip_width, canvas_width, 10.0);
                    let _ = tooltip_el
                        .style()
                        .set_property("left", &format!("{}px", left));
                }
                if let Some(crosshair) = crosshair_ref.get_untracked() {
                    let crosshair_el: HtmlElement = crosshair.into();
                    let crosshair_x = candle.x + candle.width / 2.0;
                    let _ = crosshair_el
                        .style()
                        .set_property("left", &format!("{}px", crosshair_x));
                }
            } else {
                hovered_index.set(None);
                if let Some(tooltip) = tooltip_ref.get_untracked() {
                    let _: HtmlElement = tooltip.into();
                }
            }
        }
    };

    Effect::new(move |_| {
        let total = data.get().unwrap_or_default().len();
        if pinned_to_latest.get_untracked() {
            let visible = view_end.get_untracked() - view_start.get_untracked();
            let new_end = total;
            let new_start = new_end.saturating_sub(visible);
            view_start.set(new_start);
            view_end.set(new_end);
        }
    });

    Effect::new(move |_| {
        let _ = view_start.get();
        let _ = view_end.get();
        redraw();
    });

    let redraw_for_observer = redraw.clone();
    let UseResizeObserverReturn { stop, .. } =
        use_resize_observer(container_ref, move |_entries, _observer| {
            redraw_for_observer();
        });

    let canvas_wheel_handler = move |e: ev::WheelEvent| {
        let Some(canvas) = canvas_ref.get() else {
            return;
        };
        let canvas: HtmlCanvasElement = canvas.into();

        let rect = canvas.get_bounding_client_rect();
        let x = e.client_x() as f64 - rect.left();

        let total = data.get_untracked().unwrap_or_default().len();
        let start = view_start.get_untracked();
        let end = view_end.get_untracked();
        let delta_sign = e.delta_y().signum() as isize;

        let Some((canvas_width, canvas_height)) = canvas_dimensions(&canvas) else {
            return;
        };
        let (new_start, new_end) = compute_zoom(
            x,
            canvas_width,
            canvas_height,
            total,
            start,
            end,
            delta_sign,
        );

        pinned_to_latest.set(new_end >= total);
        view_start.set(new_start);
        view_end.set(new_end);
        e.prevent_default();
    };

    let canvas_mousedown_handler = move |e: ev::MouseEvent| {
        let Some(canvas) = canvas_ref.get() else {
            return;
        };
        let canvas: HtmlCanvasElement = canvas.into();
        let rect = canvas.get_bounding_client_rect();
        let x = e.client_x() as f64 - rect.left();

        is_dragging.set_value(true);
        drag_start_x.set_value(x);
        drag_start_view.set_value((view_start.get_untracked(), view_end.get_untracked()));
    };

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

        if is_dragging.get_value() {
            let total = data.get_untracked().unwrap_or_default().len();
            let (drag_start, drag_end) = drag_start_view.get_value();

            if let Some((width, height)) = canvas_dimensions(&canvas) {
                let (new_start, new_end) = compute_pan(
                    drag_start_x.get_value(),
                    x,
                    width,
                    height,
                    total,
                    drag_start,
                    drag_end,
                );
                pinned_to_latest.set(new_end >= total);
                view_start.set(new_start);
                view_end.set(new_end);
            }
        }

        let positions = candle_positions.get_value();
        let hovered = find_candle_at(&positions, x);

        let tooltip_el: HtmlElement = tooltip.into();
        let crosshair_el: HtmlElement = crosshair.into();
        let tooltip_style = tooltip_el.style();
        let crosshair_style = crosshair_el.style();

        if let Some((idx, candle)) = hovered {
            hovered_index.set(Some(idx));
            mouse_x.set_value(x);

            tooltip_el.set_inner_html(&format!(
                "<strong>{}</strong><br/>O: {}  H: {}  L: {}  C: {}",
                candle.label,
                format_with_commas(candle.open, Some(2)),
                format_with_commas(candle.high, Some(2)),
                format_with_commas(candle.low, Some(2)),
                format_with_commas(candle.close, Some(2)),
            ));
            let _ = tooltip_style.set_property("display", "block");

            let tooltip_width = tooltip_el.offset_width() as f64;
            let canvas_width = canvas.client_width() as f64;
            let left = tooltip_left(x, tooltip_width, canvas_width, 10.0);
            let _ = tooltip_style.set_property("left", &format!("{}px", left));
            let _ = tooltip_style.set_property("top", &format!("{}px", y - 28.0));

            let crosshair_x = candle.x + candle.width / 2.0;
            let _ = crosshair_style.set_property("display", "block");
            let _ = crosshair_style.set_property("left", &format!("{}px", crosshair_x));
            let _ =
                crosshair_style.set_property("height", &format!("{}px", canvas.client_height()));
        } else {
            hovered_index.set(None);
            let _ = tooltip_style.set_property("display", "none");
            let _ = crosshair_style.set_property("display", "none");
        }
    };

    let canvas_mouseleave_handler = move |_: ev::MouseEvent| {
        is_dragging.set_value(false);
        hovered_index.set(None);
        if let Some(tooltip) = tooltip_ref.get() {
            let tooltip_el: HtmlElement = tooltip.into();
            let _ = tooltip_el.style().set_property("display", "none");
        }
        if let Some(crosshair) = crosshair_ref.get() {
            let crosshair_el: HtmlElement = crosshair.into();
            let _ = crosshair_el.style().set_property("display", "none");
        }
    };

    let canvas_mouseup_handler = move |_| {
        is_dragging.set_value(false);
    };

    on_cleanup(move || {
        stop();
    });

    view! {
        <div style="width: 100%;">
            <div node_ref=container_ref style="position: relative;">
                <canvas
                    node_ref=canvas_ref
                    style="width: 100%; height: 100%; cursor: grab;"
                    on:wheel=canvas_wheel_handler
                    on:mousedown=canvas_mousedown_handler
                    on:mousemove=canvas_mousemove_handler
                    on:mouseup=canvas_mouseup_handler
                    on:mouseleave=canvas_mouseleave_handler
                />
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
                        pointer-events: none;
                        line-height: 1.6;
                        white-space: nowrap;
                    "
                />
            </div>
            <div style="display: flex; gap: 8px; margin-top: 6px; font-size: 12px; color: #6b7280;">
                <span>"Scroll to zoom"</span>
                <span>"·"</span>
                <span>"Drag to pan"</span>
                <span>"·"</span>
                {move || {
                    let start = view_start.get();
                    let end = view_end.get();
                    let total = data.get().unwrap_or_default().len();
                    format!("Showing {} of {} candles", end - start, total)
                }}
                <span>"·"</span>
                {move || if pinned_to_latest.get() {
                    "● Live"
                } else {
                    "○ Paused"
                }}
            </div>
        </div>
    }
}

/// Dimensions the chart lays out against — must match what `redraw` uses,
/// or interaction math (zoom pivot, drag sensitivity) drifts from the drawing.
fn canvas_dimensions(canvas: &HtmlCanvasElement) -> Option<(f64, f64)> {
    let parent = canvas.parent_element()?;
    let width = parent.client_width() as f64;
    let height = width * 0.6;
    if width < 1.0 || height < 1.0 {
        return None;
    }
    Some((width, height))
}

fn compute_paddings(width: f64, height: f64) -> (f64, f64) {
    let axis_padding = (height * 0.12).clamp(28.0, 60.0);
    let left_padding = (width * 0.1).clamp(40.0, 60.0);
    (left_padding, axis_padding)
}

fn compute_candle_layout(width: f64, height: f64, data: &[Candle]) -> Option<CandleChartLayout> {
    if data.is_empty() || width <= 0.0 || height <= 0.0 {
        return None;
    }

    let (left_padding, axis_padding) = compute_paddings(width, height);

    let candle_margin = 0.2;

    let max_value = data
        .iter()
        .map(|c| c.high)
        .fold(f64::NEG_INFINITY, f64::max)
        * 1.05;
    let min_value = data.iter().map(|c| c.low).fold(f64::INFINITY, f64::min) * 0.95;
    let value_range = max_value - min_value;

    if value_range == 0.0 {
        return None;
    }

    let chart_width = width - left_padding - axis_padding;
    let chart_height = height - axis_padding * 2.0;
    let slot_width = chart_width / data.len() as f64;
    let candle_width = (slot_width * (1.0 - candle_margin)).max(0.5);

    let to_y = |value: f64| -> f64 {
        height - axis_padding - ((value - min_value) / value_range) * chart_height
    };

    let num_grid_lines = 5;
    let step_value = (max_value - min_value) / num_grid_lines as f64;
    let step_height = chart_height / num_grid_lines as f64;

    let grid_lines = (0..=num_grid_lines)
        .map(|i| GridLine {
            y: height - axis_padding - i as f64 * step_height,
            value: min_value + i as f64 * step_value,
        })
        .collect();

    let candle_positions = data
        .iter()
        .enumerate()
        .map(|(i, candle)| {
            let slot_x = left_padding + i as f64 * slot_width;
            let candle_x = slot_x + (slot_width - candle_width) / 2.0;
            CandlePos {
                x: candle_x,
                y: to_y(candle.high),
                width: candle_width,
                height: to_y(candle.low) - to_y(candle.high),
                body_top: to_y(candle.open.max(candle.close)),
                body_bottom: to_y(candle.open.min(candle.close)),
                open: candle.open,
                high: candle.high,
                low: candle.low,
                close: candle.close,
                label: candle.label.clone(),
                is_bullish: candle.close >= candle.open,
            }
        })
        .collect();

    Some(CandleChartLayout {
        candle_positions,
        grid_lines,
        left_padding,
        axis_padding,
        slot_width,
        min_value,
        max_value,
    })
}

fn draw_candlestick_chart(
    context: &CanvasRenderingContext2d,
    width: f64,
    height: f64,
    data: &[Candle],
    config: &CandlestickChartConfig,
) -> Vec<CandlePos> {
    let Some(layout) = compute_candle_layout(width, height, data) else {
        return vec![];
    };

    let y_label_font_size = (height * 0.045).clamp(9.0, 12.0);
    let x_label_font_size = (height * 0.04).clamp(8.0, 11.0);

    context.clear_rect(0.0, 0.0, width, height);

    context.set_line_width(1.0);
    context.set_fill_style_str("black");
    context.set_text_align("right");
    context.set_text_baseline("middle");
    context.set_font(&format!("{}px Arial", y_label_font_size));

    for line in &layout.grid_lines {
        if config.show_grid {
            context.set_stroke_style_str("#e5e7eb");
            context.begin_path();
            context.move_to(layout.left_padding, line.y);
            context.line_to(width - layout.axis_padding, line.y);
            context.stroke();
        }

        let _ = context.fill_text(
            &format!("{:.2}", line.value),
            layout.left_padding - 8.0,
            line.y,
        );
    }

    context.set_stroke_style_str("#e5e7eb");
    context.begin_path();
    context.move_to(layout.left_padding, height - layout.axis_padding);
    context.line_to(width - layout.axis_padding, height - layout.axis_padding);
    context.stroke();

    for pos in &layout.candle_positions {
        let color = if pos.is_bullish {
            config.bullish_color.as_str()
        } else {
            config.bearish_color.as_str()
        };
        let wick_x = pos.x + pos.width / 2.0;
        let body_height = (pos.body_bottom - pos.body_top).max(1.0);

        context.set_stroke_style_str(config.wick_color.as_str());
        context.set_line_width(1.0_f64.min(pos.width));
        context.begin_path();
        context.move_to(wick_x, pos.y);
        context.line_to(wick_x, pos.body_top);
        context.stroke();

        context.begin_path();
        context.move_to(wick_x, pos.body_bottom);
        context.line_to(wick_x, pos.y + pos.height);
        context.stroke();

        if pos.width < 2.0 {
            context.set_stroke_style_str(color);
            context.set_line_width(pos.width);
            context.begin_path();
            context.move_to(wick_x, pos.body_top);
            context.line_to(wick_x, pos.body_bottom);
            context.stroke();
        } else {
            context.set_fill_style_str(color);
            context.fill_rect(pos.x, pos.body_top, pos.width, body_height);
            context.set_stroke_style_str(color);
            context.set_line_width(1.0);
            context.stroke_rect(pos.x, pos.body_top, pos.width, body_height);
        }

        if layout.slot_width > 30.0 {
            context.set_fill_style_str("black");
            context.set_text_align("right");
            context.set_text_baseline("middle");
            context.set_font(&format!("{}px Arial", x_label_font_size));
            context.save();
            let _ = context.translate(wick_x, height - layout.axis_padding / 2.0);
            let _ = context.rotate(-std::f64::consts::PI / 4.0);
            let _ = context.fill_text(&pos.label, 0.0, 0.0);
            context.restore();
        }
    }

    layout.candle_positions
}

fn find_candle_at(positions: &[CandlePos], x: f64) -> Option<(usize, &CandlePos)> {
    positions
        .iter()
        .enumerate()
        .find(|(_, c)| x >= c.x && x <= c.x + c.width)
}

fn tooltip_left(x: f64, tooltip_width: f64, canvas_width: f64, gap: f64) -> f64 {
    if x + gap + tooltip_width > canvas_width {
        (x - gap - tooltip_width).max(0.0)
    } else {
        x + gap
    }
}

fn compute_zoom(
    x: f64,
    canvas_width: f64,
    canvas_height: f64,
    total: usize,
    start: usize,
    end: usize,
    delta_sign: isize,
) -> (usize, usize) {
    let (left_padding, right_padding) = compute_paddings(canvas_width, canvas_height);
    let chart_width = canvas_width - left_padding - right_padding;
    let visible = end.saturating_sub(start);

    let mouse_ratio = ((x - left_padding) / chart_width).clamp(0.0, 1.0);
    let center = start + (mouse_ratio * visible as f64) as usize;

    let new_visible = ((visible as isize + delta_sign).max(2) as usize).min(total);

    let new_start = center.saturating_sub((mouse_ratio * new_visible as f64) as usize);
    let new_end = (new_start + new_visible).min(total);
    let new_start = new_end.saturating_sub(new_visible);

    (new_start, new_end)
}

fn compute_pan(
    drag_start_x: f64,
    current_x: f64,
    canvas_width: f64,
    canvas_height: f64,
    total: usize,
    drag_start: usize,
    drag_end: usize,
) -> (usize, usize) {
    let visible = drag_end.saturating_sub(drag_start);
    let (left_padding, right_padding) = compute_paddings(canvas_width, canvas_height);
    let chart_width = canvas_width - left_padding - right_padding;
    if visible == 0 || chart_width <= 0.0 {
        return (drag_start, drag_end);
    }

    let candle_width = chart_width / visible as f64;
    let delta_candles = ((drag_start_x - current_x) / candle_width).round() as isize;
    let max_start = total.saturating_sub(visible);
    let new_start = (drag_start as isize + delta_candles).clamp(0, max_start as isize) as usize;
    let new_end = new_start + visible;

    (new_start, new_end)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
