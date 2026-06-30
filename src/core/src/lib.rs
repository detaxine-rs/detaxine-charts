//! # Detaxine Charts
//!
//! `detaxine-charts` is a high-performance, canvas-based charting library for
//! [Leptos](https://leptos.dev). Every chart is responsive by default, supports
//! tooltips on hover, and redraws automatically on window resize.
//!
//! ## Available Charts
//!
//! | Component | Description |
//! |---|---|
//! | [`BarChart`] | Vertical bar chart with configurable colors |
//! | [`PieChart`] | Classic pie chart with hit-tested hover tooltips |
//! | [`DoughnutChart`] | Pie chart with a hollow center |
//! | [`LineCurveChart`] | Multi-series line chart with optional bezier curves and area fill |
//! | [`CandlestickChart`] | OHLC candlestick chart with zoom and pan |
//!
//!
//! ## Live Demo
//!
//! [https://elonaire.github.io/detaxine-charts/](https://elonaire.github.io/detaxine-charts/)
//!
//! ## Usage
//!
//! Add the crate to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! detaxine-charts = "0.8.33"
//! ```
//!
//! Each chart is behind a feature flag so you only compile what you need:
//!
//! ```toml
//! [dependencies]
//! detaxine-charts = { version = "0.8.33", features = ["BarChart", "LineCurveChart"] }
//! ```
//!
//! Available features: `BarChart`, `PieChart`, `DoughnutChart`, `LineCurveChart`, `CandlestickChart`.
//! Omitting features entirely enables all charts.
//!
//! ## Quick Start
//!
//! ```rust
//! use leptos::prelude::*;
//! use detaxine_charts::{bar_chart::{BarChart, BarChartConfig, DataPoint}, use_chart_data};
//!
//! #[component]
//! fn App() -> impl IntoView {
//!     view! {
//!         <BarChart
//!             data=use_chart_data(vec![
//!                 DataPoint::new("Jan", 120),
//!                 DataPoint::new("Feb", 85),
//!                 DataPoint::new("Mar", 200),
//!             ]).signal()
//!             config=BarChartConfig::new("#4f46e5", "#e5e7eb", "#111827")
//!         />
//!     }
//! }
//! ```
//!
//! ## Chart Examples
//!
//! ### [`BarChart`]
//!
//! ```rust
//! use leptos::prelude::*;
//! use detaxine_charts::{bar_chart::{BarChart, BarChartConfig, DataPoint}, use_chart_data};
//!
//! #[component]
//! fn MyBarChart() -> impl IntoView {
//!     view! {
//!         <BarChart
//!             data=use_chart_data(vec![
//!                 DataPoint::new("Q1", 42000),
//!                 DataPoint::new("Q2", 58000),
//!                 DataPoint::new("Q3", 51000),
//!                 DataPoint::new("Q4", 73000),
//!             ]).signal()
//!             config=BarChartConfig::new("#4f46e5", "#e5e7eb", "#111827")
//!         />
//!     }
//! }
//! ```
//!
//! ### [`PieChart`]
//!
//! ```rust
//! use leptos::prelude::*;
//! use detaxine_charts::{pie_chart::{PieChart, PieChartConfig, DataPoint}, use_chart_data};
//!
//! #[component]
//! fn MyPieChart() -> impl IntoView {
//!     view! {
//!         <PieChart
//!             data=use_chart_data(vec![
//!                 DataPoint::new("Housing",       35, "#4f46e5"),
//!                 DataPoint::new("Food",          20, "#e11d48"),
//!                 DataPoint::new("Transport",     15, "#0891b2"),
//!                 DataPoint::new("Savings",        8, "#9333ea"),
//!             ]).signal()
//!             config=PieChartConfig { show_legend: true }
//!         />
//!     }
//! }
//! ```
//!
//! ### [`DoughnutChart`]
//!
//! ```rust
//! use leptos::prelude::*;
//! use detaxine_charts::{doughnut_chart::{DoughnutChart, DoughnutChartConfig}, use_chart_data};
//!
//! #[component]
//! fn MyDoughnutChart() -> impl IntoView {
//!     view! {
//!         <DoughnutChart
//!             data=use_chart_data(vec![
//!                 ("Housing".to_string(),   35, "#4f46e5".to_string()),
//!                 ("Food".to_string(),      20, "#e11d48".to_string()),
//!                 ("Transport".to_string(), 15, "#0891b2".to_string()),
//!                 ("Savings".to_string(),    8, "#9333ea".to_string()),
//!             ]).signal()
//!             config=DoughnutChartConfig { show_legend: true }
//!         />
//!     }
//! }
//! ```
//!
//! ### [`LineCurveChart`]
//!
//! ```rust
//! use leptos::prelude::*;
//! use detaxine_charts::{line_chart::{LineCurveChart, LineCurveChartConfig, DataPoint, Series}, use_chart_data};
//!
//! #[component]
//! fn MyLineChart() -> impl IntoView {
//!     view! {
//!         <LineCurveChart
//!             data=use_chart_data(vec![
//!                 (
//!                     Series::new("Revenue", "#4f46e5"),
//!                     vec![
//!                         DataPoint::new(120),
//!                         DataPoint::new(85),
//!                         DataPoint::new(200),
//!                         DataPoint::new(150),
//!                     ],
//!                 ),
//!                 (
//!                     Series::new("Expenses", "#e11d48"),
//!                     vec![
//!                         DataPoint::new(80),
//!                         DataPoint::new(90),
//!                         DataPoint::new(110),
//!                         DataPoint::new(95),
//!                     ],
//!                 ),
//!             ]).signal()
//!             x=use_chart_data(vec![
//!                 "Jan".to_string(), "Feb".to_string(),
//!                 "Mar".to_string(), "Apr".to_string(),
//!             ]).signal()
//!             config=LineCurveChartConfig {
//!                 show_area_chart: true,
//!                 x_axis_title: "Month".to_string(),
//!                 y_axis_title: "Amount ($)".to_string(),
//!                 ..Default::default()
//!             }
//!         />
//!     }
//! }
//! ```
//!
//! ### [`CandlestickChart`]
//!
//! ```rust
//! use leptos::prelude::*;
//! use detaxine_charts::{candlestick_chart::{CandlestickChart, CandlestickChartConfig, Candle}, use_chart_data};
//!
//! #[component]
//! fn MyCandlestickChart() -> impl IntoView {
//!     view! {
//!         <CandlestickChart
//!             data=use_chart_data(vec![
//!                 Candle::new("Mon", 172.30, 174.50, 170.80, 173.20),
//!                 Candle::new("Tue", 173.20, 176.80, 172.50, 176.10),
//!                 Candle::new("Wed", 176.10, 177.30, 173.40, 174.00),
//!                 Candle::new("Thu", 174.00, 175.20, 171.60, 172.10),
//!                 Candle::new("Fri", 172.10, 173.80, 169.90, 170.50),
//!             ]).signal()
//!             config=CandlestickChartConfig {
//!                 bullish_color: "#16a34a".to_string(),
//!                 bearish_color: "#e11d48".to_string(),
//!                 wick_color:    "#6b7280".to_string(),
//!                 show_grid: true,
//!             }
//!         />
//!     }
//! }
//! ```
//!
//! ## Design Notes
//!
//! - **Canvas-based rendering** - all charts draw to an HTML5 `<canvas>` element
//!   via `web_sys`, giving full control over pixel output with no DOM overhead.
//! - **Device pixel ratio aware** - canvases are scaled by `window.devicePixelRatio`
//!   so charts are sharp on HiDPI and Retina displays.
//! - **Responsive** - each chart listens for `window.resize` and redraws to fit
//!   its parent container width automatically.
//! - **HTML tooltip overlay** - tooltips are absolutely positioned `<div>` elements
//!   rather than canvas-drawn text, making them easy to style with CSS.
//! - **Geometry-driven hit testing** - after each draw, shape positions are stored
//!   in a [`StoredValue`](leptos::StoredValue) and used for precise mouse hit
//!   detection on hover.

pub mod charts;
pub mod utils;

pub use utils::hooks::use_chart_data::use_chart_data;

#[cfg(feature = "BarChart")]
pub use charts::bar_chart::bar_chart;
#[cfg(feature = "CandlestickChart")]
pub use charts::candlestick_chart::candlestick_chart;
#[cfg(feature = "DoughnutChart")]
pub use charts::doughnut_chart::doughnut_chart;
#[cfg(feature = "LineCurveChart")]
pub use charts::line_chart::line_chart;
#[cfg(feature = "PieChart")]
pub use charts::pie_chart::pie_chart;
