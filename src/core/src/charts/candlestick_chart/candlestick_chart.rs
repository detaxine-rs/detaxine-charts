use leptos::{
    ev,
    html::{Canvas, Div},
    prelude::*,
};
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement, wasm_bindgen::JsCast, window,
};

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

#[derive(Clone, Debug)]
struct CandlePos {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    label: String,
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
    /// Accepts a reactive signal — push new candles and the chart updates automatically.
    #[prop(into)]
    data: MaybeProp<Vec<Candle>>,
    #[prop(optional, default = Default::default())] config: CandlestickChartConfig,
) -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let tooltip_ref = NodeRef::<Div>::new();
    let candle_positions = StoredValue::new(Vec::<CandlePos>::new());
    let config = StoredValue::new(config);

    // view window — initialised from current data length, clamped on each update
    let view_start = RwSignal::new(0usize);
    let view_end = RwSignal::new(data.get_untracked().unwrap_or_default().len());

    // track whether the view is pinned to the latest candle
    // when true, new candles scroll the view automatically
    let pinned_to_latest = RwSignal::new(true);

    // drag state
    let is_dragging = StoredValue::new(false);
    let drag_start_x = StoredValue::new(0.0f64);
    let drag_start_view = StoredValue::new((0usize, 0usize));

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

        let Some(all_data) = data.get() else {
            return;
        };
        let total = all_data.len();

        // clamp view window to current data length
        let start = view_start.get().min(total.saturating_sub(1));
        let end = view_end.get().min(total);
        let (start, end) = if start >= end {
            (end.saturating_sub(1), end)
        } else {
            (start, end)
        };

        let data_slice = all_data[start..end].to_vec();
        let config = config.get_value();

        let candles = draw_candlestick_chart(&context, width, height, &data_slice, &config);
        candle_positions.set_value(candles);
    };

    // effect 1 — handle pinning when new data arrives
    Effect::new(move |_| {
        let total = data.get().unwrap_or_default().len(); // tracked

        if pinned_to_latest.get_untracked() {
            let visible = view_end.get_untracked() - view_start.get_untracked();
            let new_end = total;
            let new_start = new_end.saturating_sub(visible);
            view_start.set(new_start);
            view_end.set(new_end);
        }
    });

    // effect 2 — redraw when view window changes
    Effect::new(move |_| {
        let _ = view_start.get(); // tracked
        let _ = view_end.get(); // tracked
        redraw();
    });

    let resize_listener = window_event_listener(ev::resize, move |_| {
        redraw();
    });

    // zoom
    let canvas_wheel_handler = move |e: ev::WheelEvent| {
        let Some(canvas) = canvas_ref.get() else {
            return;
        };
        let canvas: HtmlCanvasElement = canvas.into();

        let rect = canvas.get_bounding_client_rect();
        let x = e.client_x() as f64 - rect.left();
        let axis_padding = 60.0;
        let chart_width = canvas.client_width() as f64 - axis_padding * 2.0;

        let total = data.get_untracked().unwrap_or_default().len();
        let start = view_start.get_untracked();
        let end = view_end.get_untracked();
        let visible = end - start;

        let mouse_ratio = ((x - axis_padding) / chart_width).clamp(0.0, 1.0);
        let center = start + (mouse_ratio * visible as f64) as usize;

        let delta_y = e.delta_y().signum() as isize;
        let new_visible = ((visible as isize + delta_y).max(2) as usize).min(total);

        let new_start = center.saturating_sub((mouse_ratio * new_visible as f64) as usize);
        let new_end = (new_start + new_visible).min(total);
        let new_start = new_end.saturating_sub(new_visible);

        // unpin from latest if user zooms away from the edge
        if new_end < total {
            pinned_to_latest.set(false);
        } else {
            pinned_to_latest.set(true);
        }

        view_start.set(new_start);
        view_end.set(new_end);

        e.prevent_default();
    };

    // pan — mousedown
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

    // pan + tooltip — mousemove
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

        if is_dragging.get_value() {
            let total = data.get_untracked().unwrap_or_default().len();
            let axis_padding = 60.0;
            let chart_width = canvas.client_width() as f64 - axis_padding * 2.0;
            let (drag_start, drag_end) = drag_start_view.get_value();
            let visible = drag_end - drag_start;
            let candle_width = chart_width / visible as f64;
            let delta_candles = ((drag_start_x.get_value() - x) / candle_width).round() as isize;

            let new_start =
                (drag_start as isize + delta_candles).clamp(0, (total - visible) as isize) as usize;
            let new_end = new_start + visible;

            // unpin if user drags away from the latest candle
            if new_end < total {
                pinned_to_latest.set(false);
            } else {
                pinned_to_latest.set(true);
            }

            view_start.set(new_start);
            view_end.set(new_end);
        }

        // tooltip hit-test
        let device_pixel_ratio = win.device_pixel_ratio();
        let scale_x = canvas.client_width() as f64 / canvas.width() as f64 * device_pixel_ratio;
        let scale_y = canvas.client_height() as f64 / canvas.height() as f64 * device_pixel_ratio;
        let lx = x * scale_x;
        let ly = y * scale_y;

        let hovered = candle_positions
            .get_value()
            .into_iter()
            .find(|c| lx >= c.x && lx <= c.x + c.width && ly >= c.y && ly <= c.y + c.height);

        let tooltip_el: HtmlElement = tooltip.into();
        let style = tooltip_el.style();
        if let Some(candle) = hovered {
            let _ = style.set_property("display", "block");
            let _ = style.set_property("left", &format!("{}px", x + 10.0));
            let _ = style.set_property("top", &format!("{}px", y - 28.0));
            tooltip_el.set_inner_html(&format!(
                "<strong>{}</strong><br/>O: {:.2}  H: {:.2}  L: {:.2}  C: {:.2}",
                candle.label, candle.open, candle.high, candle.low, candle.close,
            ));
        } else {
            let _ = style.set_property("display", "none");
        }
    };

    let canvas_mouseup_handler = move |_| {
        is_dragging.set_value(false);
    };

    on_cleanup(move || {
        resize_listener.remove();
    });

    view! {
        <div style="width: 100%;">
            <div style="position: relative;">
                <canvas
                    node_ref=canvas_ref
                    style="width: 100%; height: 100%; cursor: grab;"
                    on:wheel=canvas_wheel_handler
                    on:mousedown=canvas_mousedown_handler
                    on:mousemove=canvas_mousemove_handler
                    on:mouseup=canvas_mouseup_handler
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

fn draw_candlestick_chart(
    context: &CanvasRenderingContext2d,
    width: f64,
    height: f64,
    data: &Vec<Candle>,
    config: &CandlestickChartConfig,
) -> Vec<CandlePos> {
    if data.is_empty() {
        return vec![];
    };

    let axis_padding = 60.0;
    let candle_margin = 0.2;

    let max_value = data
        .iter()
        .map(|c| c.high)
        .fold(f64::NEG_INFINITY, f64::max)
        * 1.05;
    let min_value = data.iter().map(|c| c.low).fold(f64::INFINITY, f64::min) * 0.95;
    let value_range = max_value - min_value;

    if value_range == 0.0 {
        return vec![];
    };

    let chart_width = width - axis_padding * 2.0;
    let chart_height = height - axis_padding * 2.0;
    let slot_width = chart_width / data.len() as f64;
    let candle_width = (slot_width * (1.0 - candle_margin)).max(0.5);

    let to_y = |value: f64| -> f64 {
        height - axis_padding - ((value - min_value) / value_range) * chart_height
    };

    context.clear_rect(0.0, 0.0, width, height);

    let num_grid_lines = 5;
    let step_value = (max_value - min_value) / num_grid_lines as f64;
    let step_height = chart_height / num_grid_lines as f64;

    context.set_line_width(1.0);
    context.set_fill_style_str("black");
    context.set_text_align("right");
    context.set_text_baseline("middle");

    for i in 0..=num_grid_lines {
        let y = height - axis_padding - i as f64 * step_height;
        let label = min_value + i as f64 * step_value;

        if config.show_grid {
            context.set_stroke_style_str("#e5e7eb");
            context.begin_path();
            context.move_to(axis_padding, y);
            context.line_to(width - axis_padding, y);
            context.stroke();
        }

        let _ = context.fill_text(&format!("{:.2}", label), axis_padding - 8.0, y);
    }

    context.set_stroke_style_str("#e5e7eb");
    context.begin_path();
    context.move_to(axis_padding, height - axis_padding);
    context.line_to(width - axis_padding, height - axis_padding);
    context.stroke();

    let mut candle_positions = Vec::new();

    for (i, candle) in data.iter().enumerate() {
        let slot_x = axis_padding + i as f64 * slot_width;
        let candle_x = slot_x + (slot_width - candle_width) / 2.0;
        let wick_x = candle_x + candle_width / 2.0;

        let is_bullish = candle.close >= candle.open;
        let color = if is_bullish {
            config.bullish_color.as_str()
        } else {
            config.bearish_color.as_str()
        };

        let body_top = to_y(candle.open.max(candle.close));
        let body_bottom = to_y(candle.open.min(candle.close));
        let body_height = (body_bottom - body_top).max(1.0);

        context.set_stroke_style_str(config.wick_color.as_str());
        context.set_line_width(1.0_f64.min(candle_width));
        context.begin_path();
        context.move_to(wick_x, to_y(candle.high));
        context.line_to(wick_x, body_top);
        context.stroke();

        context.begin_path();
        context.move_to(wick_x, body_bottom);
        context.line_to(wick_x, to_y(candle.low));
        context.stroke();

        if candle_width < 2.0 {
            context.set_stroke_style_str(color);
            context.set_line_width(candle_width);
            context.begin_path();
            context.move_to(wick_x, body_top);
            context.line_to(wick_x, body_bottom);
            context.stroke();
        } else {
            context.set_fill_style_str(color);
            context.fill_rect(candle_x, body_top, candle_width, body_height);
            context.set_stroke_style_str(color);
            context.set_line_width(1.0);
            context.stroke_rect(candle_x, body_top, candle_width, body_height);
        }

        if slot_width > 30.0 {
            context.set_fill_style_str("black");
            context.set_text_align("right");
            context.set_text_baseline("middle");
            context.save();
            let _ = context.translate(wick_x, height - axis_padding / 2.0);
            let _ = context.rotate(-std::f64::consts::PI / 4.0);
            let _ = context.fill_text(&candle.label, 0.0, 0.0);
            context.restore();
        }

        candle_positions.push(CandlePos {
            x: candle_x,
            y: to_y(candle.high),
            width: candle_width,
            height: to_y(candle.low) - to_y(candle.high),
            open: candle.open,
            high: candle.high,
            low: candle.low,
            close: candle.close,
            label: candle.label.clone(),
        });
    }

    candle_positions
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
    fn test_draw_candlestick_chart() {
        let Some(context) = mock_context() else {
            return;
        };

        let data = vec![
            Candle::new("Mon", 100.0, 115.0, 95.0, 110.0),
            Candle::new("Tue", 110.0, 120.0, 105.0, 108.0),
            Candle::new("Wed", 108.0, 112.0, 100.0, 102.0),
            Candle::new("Thu", 102.0, 118.0, 101.0, 115.0),
            Candle::new("Fri", 115.0, 125.0, 113.0, 120.0),
        ];
        let config = CandlestickChartConfig::default();

        draw_candlestick_chart(&context, 800.0, 480.0, &data, &config);
    }
}
