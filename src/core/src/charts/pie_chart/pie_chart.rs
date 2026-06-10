use leptos::{
    ev,
    html::{Canvas, Div},
    prelude::*,
};
use std::f64::consts::PI;
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement, wasm_bindgen::JsCast, window,
};

#[derive(Clone, Debug, PartialEq)]
pub struct PieChartConfig {
    pub show_legend: bool,
}

impl Default for PieChartConfig {
    fn default() -> Self {
        Self { show_legend: true }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DataPoint {
    pub name: String,
    pub value: i32,
    pub color: String,
}

impl DataPoint {
    pub fn new(name: &str, value: i32, color: &str) -> Self {
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
    radius: f64,
    center_x: f64,
    center_y: f64,
    name: String,
    value: i32,
}

fn get_context(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .ok()??
        .dyn_into::<CanvasRenderingContext2d>()
        .ok()
}

#[component]
pub fn PieChart(
    /// Accepts a reactive signal — update data and the chart redraws automatically.
    #[prop(into)]
    data: MaybeProp<Vec<DataPoint>>,
    #[prop(optional, default = Default::default())] config: PieChartConfig,
) -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let tooltip_ref = NodeRef::<Div>::new();
    let slice_positions = StoredValue::new(Vec::<SlicePos>::new());
    let config = StoredValue::new(config);

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

        let slices = draw_pie_chart(&context, width, height, &data);
        slice_positions.set_value(slices);
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

        let hovered = slice_positions.get_value().into_iter().find(|s| {
            let dx = lx - s.center_x;
            let dy = ly - s.center_y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > s.radius {
                return false;
            };

            let mut angle = dy.atan2(dx);
            if angle < 0.0 {
                angle += 2.0 * PI;
            }

            angle >= s.start_angle && angle <= s.end_angle
        });

        let tooltip_el: HtmlElement = tooltip.into();
        let style = tooltip_el.style();
        if let Some(slice) = hovered {
            let _ = style.set_property("display", "block");
            let _ = style.set_property("left", &format!("{}px", x + 10.0));
            let _ = style.set_property("top", &format!("{}px", y - 28.0));
            tooltip_el.set_inner_text(&format!("{}: {}", slice.name, slice.value));
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
                    {legend_meta.get().into_iter().map(|(name, color)| view! {
                        <div style="display: flex; flex-direction: row; align-items: center; gap: 2px;">
                            <span style="font-size: 10px;">{name}</span>
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

fn draw_pie_chart(
    context: &CanvasRenderingContext2d,
    width: f64,
    height: f64,
    data: &[DataPoint],
) -> Vec<SlicePos> {
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let radius = (width.min(height) / 2.0) - 5.0;

    let total: f64 = data.iter().map(|d| d.value as f64).sum();
    if total == 0.0 {
        return vec![];
    };

    context.clear_rect(0.0, 0.0, width, height);

    let mut start_angle = 0.0_f64;
    let mut slice_positions = Vec::new();

    for point in data {
        let slice_angle = point.value as f64 / total * 2.0 * PI;
        let end_angle = start_angle + slice_angle;

        context.begin_path();
        context.move_to(center_x, center_y);
        let _ = context.arc(center_x, center_y, radius, start_angle, end_angle);
        context.close_path();
        context.set_fill_style_str(point.color.as_str());
        context.fill();

        slice_positions.push(SlicePos {
            start_angle,
            end_angle,
            radius,
            center_x,
            center_y,
            name: point.name.clone(),
            value: point.value,
        });

        start_angle = end_angle;
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
    fn test_draw_pie_chart() {
        let Some(context) = mock_context() else {
            return;
        };

        let data = vec![
            DataPoint::new("A", 10, "#ff0000"),
            DataPoint::new("B", 20, "#00ff00"),
            DataPoint::new("C", 30, "#0000ff"),
            DataPoint::new("D", 40, "#ffff00"),
        ];

        draw_pie_chart(&context, 800.0, 600.0, &data);
    }
}
