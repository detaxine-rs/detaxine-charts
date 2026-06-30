# detaxine-charts
![Stable Version](https://img.shields.io/crates/v/detaxine-charts)
[![Docs.rs](https://docs.rs/detaxine-ui/badge.svg)](https://docs.rs/detaxine-ui)
![Detaxine Charts CI](https://github.com/detaxine-rs/detaxine-charts/actions/workflows/main.yml/badge.svg?branch=)

A high-performance, canvas-based charting library for [Leptos](https://leptos.dev). Every chart is responsive by default, supports tooltips on hover, redraws automatically on window resize, and accepts reactive signals for live data streaming.

**Note**: This crate is **NOW** available for use, all charts are customizable to your liking.

**New/Upcoming Features:**
- [x] Zoomable & pannable CandlestickChart
- [x] Live/streaming data via `use_chart_data` hook
- [ ] Touch support for candlestick zoom/pan
- [ ] Customizable tooltip renderer
- [ ] Toggleable legend
- [ ] Polar Area Chart
- [ ] Radar Chart
- [ ] Scatter Chart

This crate is built using the [Leptos](https://leptos.dev) framework and uses HTML5 canvas to render the charts.

## Features

- [x] BarChart
- [x] PieChart
- [x] DoughnutChart
- [x] LineCurveChart
- [x] CandlestickChart (with zoom & pan)

## Live Demo

[https://detaxine-rs.github.io/detaxine-charts/](https://detaxine-rs.github.io/detaxine-charts/)

## Usage

Add the following to your `Cargo.toml`:

```toml
[dependencies]
detaxine-charts = { version = "0.8.32", features = ["BarChart", "PieChart"] }
```

Available features: `BarChart`, `PieChart`, `DoughnutChart`, `LineCurveChart`, `CandlestickChart`.
Omitting features entirely enables all charts.

## Example

```rust
use leptos::prelude::*;
use detaxine_charts::{
    use_chart_data,
    bar_chart::{BarChart, BarChartConfig, DataPoint},
};

#[component]
fn App() -> impl IntoView {
    let data = use_chart_data(vec![
        DataPoint::new("Jan", 120),
        DataPoint::new("Feb", 85),
        DataPoint::new("Mar", 200),
        DataPoint::new("Apr", 150),
    ]);

    view! {
        <BarChart
            data=data.signal()
            config=BarChartConfig::new("#4f46e5", "#e5e7eb", "#111827")
        />
    }
}
```

## Live/Streaming Data

The `use_chart_data` hook provides an ergonomic API for updating charts reactively:

```rust
use leptos::prelude::*;
use detaxine_charts::{
    use_chart_data,
    candlestick_chart::{CandlestickChart, CandlestickChartConfig, Candle},
};

#[component]
fn LiveChart() -> impl IntoView {
    let candles = use_chart_data(initial_candles());

    // update the current open candle on every tick
    let tick_handle = set_interval_with_handle(move || {
        candles.update_last(fetch_current_candle());
    }, Duration::from_millis(500));

    // close candle and open a new one every minute
    let candle_handle = set_interval_with_handle(move || {
        candles.append(open_new_candle());
        candles.retain_last(500); // rolling window
    }, Duration::from_secs(60));

    on_cleanup(move || {
        if let (Ok(tick_handler), Ok(candle_handler)) = (tick_handle, candle_handle)   {
            tick_handler.clear();
            candle_handler.clear();
        };
    });

    view! {
        <CandlestickChart
            data=candles.signal()
            config=CandlestickChartConfig::default()
        />
    }
}
```

### `use_chart_data` API

| Method | Description |
|---|---|
| `signal()` | Returns the underlying `Signal<Vec<T>>` to pass into a chart |
| `append(item)` | Append a single item |
| `extend(items)` | Append multiple items |
| `update_at(index, item)` | Replace the item at the given index |
| `update_last(item)` | Update the last item - ideal for live candle tick updates |
| `set(items)` | Replace all data at once |
| `remove_at(index)` | Remove the item at the given index |
| `clear()` | Remove all items |
| `retain_last(n)` | Keep only the last `n` items - prevents unbounded memory growth |
| `len()` | Returns the current number of items |
| `is_empty()` | Returns true if there are no items |
| `get_untracked()` | Read current data without tracking reactivity |

## Running the Example

The example is a full trading dashboard showcasing all five charts with realistic market data.

Prerequisites:

```bash
cargo install trunk
cargo install wasm-pack
```

Run the dashboard:

```bash
cd src/core
trunk serve --example dashboard
```

Then open [http://localhost:8080](http://localhost:8080) in your browser.

The dashboard includes:
- **Candlestick Chart** - zoomable/pannable candlestick chart (scroll to zoom, drag to pan)
- **Candlestick Chart - Live Data** — zoomable/pannable candlestick chart (scroll to zoom, drag to pan) with live data streaming
- **Bar Chart** - bar chart with responsive full-width bars
- **Line Chart** - multi-series line chart with area fill
- **Pie Chart** - pie chart with hover tooltips
- **Doughnut Chart** - doughnut chart with hover tooltips

## Design Notes

- **Canvas-based rendering** - all charts draw to an HTML5 `<canvas>` element via `web_sys`, giving full control over pixel output with no DOM overhead
- **Device pixel ratio aware** - canvases are scaled by `window.devicePixelRatio` so charts are sharp on HiDPI and Retina displays
- **Responsive** - each chart listens for `window.resize` and redraws to fit its parent container width automatically
- **HTML tooltip overlay** - tooltips are absolutely positioned `<div>` elements rather than canvas-drawn text, making them easy to style with CSS
- **Geometry-driven hit testing** - after each draw, shape positions are stored and used for precise mouse hit detection on hover
- **Fine-grained reactivity** - built on Leptos signals so only the chart whose data changes redraws, never the whole page

## Versioning

`detaxine-charts` follows Leptos's versioning for major and minor versions.
The patch version is incremented independently for bug fixes and new features.

| detaxine-charts | Leptos |
|---|---|
| 0.8.x | 0.8.x |

## License

This project is licensed under both the MIT license and the Apache License (Version 2.0).

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, shall be dual licensed as above, without any additional terms or conditions.

## Acknowledgements

This project is a spiritual successor to my own [visualize-yew](https://crates.io/crates/visualize-yew) crate, rebuilt from the ground up for [Leptos](https://leptos.dev) with a focus on performance, live data streaming, and a more ergonomic API.
