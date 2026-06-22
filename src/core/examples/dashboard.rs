use std::time::Duration;

use detaxine_charts::{
    bar_chart::{BarChart, BarChartConfig, DataPoint as BarPoint},
    candlestick_chart::{Candle, CandlestickChart, CandlestickChartConfig},
    charts::{
        gauge_chart::gauge_chart::{GaugeChart, GaugeChartConfig, GaugeZone},
        polar_area_chart::polar_area_chart::{
            DataPoint as PolarPoint, PolarAreaChart, PolarAreaChartConfig,
        },
        radar_chart::radar_chart::{RadarChart, RadarChartConfig, Series as RadarSeries},
    },
    doughnut_chart::{DoughnutChart, DoughnutChartConfig},
    line_chart::{
        DataPoint as LinePoint, LineCurveChart, LineCurveChartConfig, Series as LineSeries,
    },
    pie_chart::{DataPoint as PiePoint, PieChart, PieChartConfig},
    use_chart_data,
};
use leptos::html::*;
use leptos::prelude::*;
use web_sys::js_sys::Math;

// ── Initial data for charts ─────────────────────────────────────
fn initial_candles() -> Vec<Candle> {
    vec![
        Candle::new("Mar 1", 172.30, 174.50, 170.80, 173.20),
        Candle::new("Mar 2", 173.20, 176.80, 172.50, 176.10),
        Candle::new("Mar 3", 176.10, 177.30, 173.40, 174.00),
        Candle::new("Mar 4", 174.00, 175.20, 171.60, 172.10),
        Candle::new("Mar 5", 172.10, 173.80, 169.90, 170.50),
        Candle::new("Mar 8", 170.50, 171.20, 165.30, 166.00),
        Candle::new("Mar 9", 166.00, 167.50, 162.80, 163.40),
        Candle::new("Mar 10", 163.40, 164.20, 159.60, 160.10),
        Candle::new("Mar 11", 160.10, 163.50, 158.90, 162.80),
        Candle::new("Mar 12", 162.80, 165.40, 161.20, 164.50),
        Candle::new("Mar 15", 164.50, 168.90, 163.80, 167.70),
        Candle::new("Mar 16", 167.70, 170.20, 166.50, 169.40),
        Candle::new("Mar 17", 169.40, 171.80, 168.10, 168.90),
        Candle::new("Mar 18", 168.90, 170.50, 167.30, 169.80),
        Candle::new("Mar 19", 169.80, 172.40, 169.10, 171.60),
        Candle::new("Mar 22", 171.60, 176.30, 171.20, 175.80),
        Candle::new("Mar 23", 175.80, 179.50, 174.90, 178.40),
        Candle::new("Mar 24", 178.40, 181.20, 177.30, 180.50),
        Candle::new("Mar 25", 180.50, 182.80, 178.60, 179.20),
        Candle::new("Mar 26", 179.20, 180.10, 175.40, 176.00),
        Candle::new("Mar 29", 176.00, 178.30, 173.50, 174.20),
        Candle::new("Mar 30", 174.20, 175.80, 170.90, 171.50),
        Candle::new("Mar 31", 171.50, 174.60, 170.20, 173.80),
    ]
}

fn initial_volume() -> Vec<BarPoint> {
    vec![
        BarPoint::new("Mar 1", 82_400),
        BarPoint::new("Mar 2", 91_200),
        BarPoint::new("Mar 3", 78_900),
        BarPoint::new("Mar 4", 95_600),
        BarPoint::new("Mar 5", 88_100),
        BarPoint::new("Mar 8", 120_300),
        BarPoint::new("Mar 9", 145_700),
        BarPoint::new("Mar 10", 162_400),
        BarPoint::new("Mar 11", 138_900),
        BarPoint::new("Mar 12", 110_200),
        BarPoint::new("Mar 15", 98_700),
        BarPoint::new("Mar 16", 87_300),
        BarPoint::new("Mar 17", 76_500),
        BarPoint::new("Mar 18", 82_100),
        BarPoint::new("Mar 19", 91_400),
        BarPoint::new("Mar 22", 134_600),
        BarPoint::new("Mar 23", 158_200),
        BarPoint::new("Mar 24", 172_900),
        BarPoint::new("Mar 25", 143_500),
        BarPoint::new("Mar 26", 119_800),
        BarPoint::new("Mar 29", 102_300),
        BarPoint::new("Mar 30", 94_700),
        BarPoint::new("Mar 31", 88_500),
    ]
}

fn initial_metrics() -> Vec<(LineSeries, Vec<LinePoint>)> {
    vec![
        (
            LineSeries::new("Revenue", "#4f46e5"),
            vec![
                LinePoint::new(142_000),
                LinePoint::new(158_000),
                LinePoint::new(149_000),
                LinePoint::new(163_000),
                LinePoint::new(171_000),
                LinePoint::new(168_000),
                LinePoint::new(175_000),
            ],
        ),
        (
            LineSeries::new("Expenses", "#e11d48"),
            vec![
                LinePoint::new(98_000),
                LinePoint::new(104_000),
                LinePoint::new(99_000),
                LinePoint::new(112_000),
                LinePoint::new(108_000),
                LinePoint::new(115_000),
                LinePoint::new(110_000),
            ],
        ),
    ]
}

fn initial_x_labels() -> Vec<String> {
    vec![
        "Mon".into(),
        "Tue".into(),
        "Wed".into(),
        "Thu".into(),
        "Fri".into(),
        "Sat".into(),
        "Sun".into(),
    ]
}

fn portfolio_allocation() -> Vec<PiePoint> {
    vec![
        PiePoint::new("Tech", 42, "#4f46e5"),
        PiePoint::new("Healthcare", 18, "#0891b2"),
        PiePoint::new("Finance", 15, "#16a34a"),
        PiePoint::new("Energy", 12, "#d97706"),
        PiePoint::new("Consumer", 8, "#e11d48"),
        PiePoint::new("Other", 5, "#9333ea"),
    ]
}

fn sector_exposure() -> Vec<(String, i64, String)> {
    vec![
        ("US Equities".into(), 45, "#4f46e5".into()),
        ("International".into(), 25, "#0891b2".into()),
        ("Fixed Income".into(), 15, "#16a34a".into()),
        ("Commodities".into(), 8, "#d97706".into()),
        ("Cash".into(), 7, "#9333ea".into()),
    ]
}

// Component
#[component]
pub fn App() -> impl IntoView {
    let candles_ref: NodeRef<Div> = NodeRef::new();
    let live_ref: NodeRef<Div> = NodeRef::new();
    let bar_ref: NodeRef<Div> = NodeRef::new();
    let line_ref: NodeRef<Div> = NodeRef::new();
    let pie_ref: NodeRef<Div> = NodeRef::new();
    let doughnut_ref: NodeRef<Div> = NodeRef::new();
    let progress_gauge_ref: NodeRef<Div> = NodeRef::new();
    let zone_gauge_ref: NodeRef<Div> = NodeRef::new();
    let polar_area_ref: NodeRef<Div> = NodeRef::new();

    let drawer_open = RwSignal::new(false);

    let scroll_to = |node_ref: NodeRef<Div>, close_drawer: RwSignal<bool>| {
        move |_| {
            close_drawer.set(false);
            if let Some(el) = node_ref.get() {
                el.scroll_into_view_with_bool(true);
            }
        }
    };

    let candles = use_chart_data(initial_candles());
    let volume = use_chart_data(initial_volume());
    let metrics = use_chart_data(initial_metrics());
    let x_labels = RwSignal::new(initial_x_labels());
    let allocation = use_chart_data(portfolio_allocation());
    let exposure = use_chart_data(sector_exposure());

    let live_candles = use_chart_data(vec![
        Candle::new("09:00", 172.30, 174.50, 170.80, 173.20),
        Candle::new("09:01", 173.20, 176.80, 172.50, 176.10),
        Candle::new("09:02", 176.10, 177.30, 173.40, 174.00),
    ]);
    let live_candles_signal = live_candles.signal();

    let append_handle = set_interval_with_handle(
        move || {
            let current = live_candles_signal.get().clone();
            let last = current
                .last()
                .cloned()
                .unwrap_or(Candle::new("", 100.0, 105.0, 95.0, 100.0));
            let open = last.close;
            let close = open + (Math::random() - 0.5) * 4.0;
            let high = open.max(close) + Math::random() * 2.0;
            let low = (open.min(close) - Math::random() * 2.0).max(1.0);
            let label = format!("09:{:02}", current.len());

            live_candles.append(Candle::new(&label, open, high, low, close));
            live_candles.retain_last(500);
        },
        Duration::from_secs(1),
    );

    on_cleanup(move || {
        if let Ok(handle) = append_handle {
            handle.clear();
        }
    });

    let nav_items = vec![
        ("Candlestick", candles_ref),
        ("Live Candlestick", live_ref),
        ("Bar Chart", bar_ref),
        ("Line Chart", line_ref),
        ("Pie Chart", pie_ref),
        ("Doughnut Chart", doughnut_ref),
        ("Gauge Chart - Progress", progress_gauge_ref),
        ("Gauge Chart - Zones", zone_gauge_ref),
        ("Polar Area Chart", polar_area_ref),
    ];

    view! {
        <div class="dashboard">
            // Topbar
            <header class="topbar">
                <button class="topbar-button" on:click=move |_| drawer_open.set(true)>
                    <svg width="20" height="20" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M4 6h16M4 12h16M4 18h16"/>
                    </svg>
                </button>
                <div>
                    <div class="topbar-title">"detaxine-charts"</div>
                    <div class="topbar-subtitle">"Live Demo"</div>
                </div>
            </header>

            // Drawer Backdrop
            <div
                class="drawer-backdrop"
                class:open=move || drawer_open.get()
                on:click=move |_| drawer_open.set(false)
            />

            // Drawer
            <div class="drawer" class:open=move || drawer_open.get()>
                <div class="drawer-header">
                    <div class="drawer-title">"CHARTS"</div>
                    <button class="topbar-button" on:click=move |_| drawer_open.set(false)>
                        <svg width="18" height="18" fill="none" stroke="currentColor" stroke-width="3" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>
                <nav>
                    {nav_items.clone().into_iter().map(|(label, node_ref)| {
                        let label = label.to_string();
                        view! {
                            <button
                                class="nav-button"
                                on:click=scroll_to(node_ref, drawer_open)
                            >
                                {label}
                            </button>
                        }
                    }).collect::<Vec<_>>()}
                </nav>
            </div>

            <div class="layout">
                // Main Content
                <main class="main">
                    <div class="content">
                        <div class="header">
                            <h1>"Detaxine Charts"</h1>
                            <p>"High-performance canvas charts for Leptos with support for fine-grained reactivity."</p>
                        </div>

                        <div class="card" node_ref=candles_ref>
                            <div class="section-header">
                                <h2 class="section-title">"Candlestick Chart"</h2>
                            </div>
                            <div class="chart-container">
                                <CandlestickChart data=candles.signal() config=CandlestickChartConfig::default() />
                            </div>
                        </div>

                        <div class="card" node_ref=live_ref>
                            <div class="section-header">
                                <h2 class="section-title">"Live Candlestick (Real-time)"</h2>
                            </div>
                            <div class="chart-container">
                                <CandlestickChart data=live_candles_signal.clone() config=CandlestickChartConfig::default() />
                            </div>
                        </div>

                        <div class="card" node_ref=bar_ref>
                            <div class="section-header">
                                <h2 class="section-title">"Bar Chart — Volume"</h2>
                            </div>
                            <div class="chart-container">
                                <BarChart data=volume.signal() config=BarChartConfig::new("#6366f1", "#e5e7eb", "#111827") />
                            </div>
                        </div>

                        <div class="card" node_ref=line_ref>
                            <div class="section-header">
                                <h2 class="section-title">"Line Chart — Revenue vs Expenses"</h2>
                            </div>
                            <div class="chart-container">
                                <LineCurveChart
                                    data=metrics.signal()
                                    x=x_labels
                                    config=LineCurveChartConfig {
                                        show_area_chart: true,
                                        x_axis_title: "Day".to_string(),
                                        y_axis_title: "USD".to_string(),
                                        ..Default::default()
                                    }
                                />
                            </div>
                        </div>

                        <div class="card" node_ref=pie_ref>
                            <div class="section-header">
                                <h2 class="section-title">"Pie Chart — Portfolio Allocation"</h2>
                            </div>
                            <div class="chart-container">
                                <PieChart data=allocation.signal() config=PieChartConfig { show_legend: true } />
                            </div>
                        </div>

                        <div class="card" node_ref=doughnut_ref>
                            <div class="section-header">
                                <h2 class="section-title">"Doughnut Chart — Sector Exposure"</h2>
                            </div>
                            <div class="chart-container">
                                <DoughnutChart data=exposure.signal() config=DoughnutChartConfig { show_legend: true } />
                            </div>
                        </div>

                        <div class="card" node_ref=progress_gauge_ref>
                            <div class="section-header">
                                <h2 class="section-title">"Gauge Chart - Progress Style"</h2>
                            </div>
                            <div class="chart-container">
                                <GaugeChart
                                    value=Signal::derive(move || 72.0)
                                    config=GaugeChartConfig {
                                        min: 0.0,
                                        max: 100.0,
                                        label: "Battery".to_string(),
                                        unit: "%".to_string(),
                                        ..Default::default()
                                    }
                                />
                            </div>
                        </div>

                        <div class="card" node_ref=zone_gauge_ref>
                            <div class="section-header">
                                <h2 class="section-title">"Gauge Chart - Zone Style"</h2>
                            </div>
                            <div class="chart-container">
                                <GaugeChart
                                    value=Signal::derive(move || 800.0)
                                    config=GaugeChartConfig {
                                        min: 300.0,
                                        max: 850.0,
                                        zones: vec![
                                            GaugeZone::new(300.0, 579.0, "#e11d48", "VERY POOR"),
                                            GaugeZone::new(579.0, 669.0, "#d97706", "POOR"),
                                            GaugeZone::new(669.0, 739.0, "#eab308", "FAIR"),
                                            GaugeZone::new(739.0, 799.0, "#84cc16", "GOOD"),
                                            GaugeZone::new(799.0, 850.0, "#16a34a", "EXCELLENT"),
                                        ],
                                        ..Default::default()
                                    }
                                />
                            </div>
                        </div>

                        <div class="card" node_ref=polar_area_ref>
                            <div class="section-header">
                                <h2 class="section-title">"Polar Area Chart"</h2>
                            </div>
                            <div class="chart-container">
                                <PolarAreaChart
                                    data=use_chart_data(vec![
                                        PolarPoint::new("Mon", 12.0, "#4f46e5"),
                                        PolarPoint::new("Tue", 19.0, "#e11d48"),
                                        PolarPoint::new("Wed", 7.0, "#0891b2"),
                                        PolarPoint::new("Thu", 15.0, "#16a34a"),
                                        PolarPoint::new("Fri", 22.0, "#d97706"),
                                    ]).signal()
                                    config=PolarAreaChartConfig::default()
                                />
                            </div>
                        </div>

                        <div class="card" node_ref=polar_area_ref>
                            <div class="section-header">
                                <h2 class="section-title">"Radar Chart"</h2>
                            </div>
                            <div class="chart-container">
                                <RadarChart
                                    axes=use_chart_data(vec![
                                        "Speed".to_string(),
                                        "Power".to_string(),
                                        "Defense".to_string(),
                                        "Agility".to_string(),
                                        "Stamina".to_string(),
                                    ]).signal()
                                    data=use_chart_data(vec![
                                        (RadarSeries::new("Player A", "#4f46e5"), vec![80.0, 65.0, 70.0, 90.0, 60.0]),
                                        (RadarSeries::new("Player B", "#e11d48"), vec![60.0, 85.0, 75.0, 55.0, 80.0]),
                                    ]).signal()
                                    config=RadarChartConfig::default()
                                />
                            </div>
                        </div>

                        <div class="footer-note">
                            "Built with Leptos • Canvas rendered • Real-time updates • Fully responsive"
                        </div>
                    </div>
                </main>
            </div>
        </div>
    }
}

fn main() {
    mount_to_body(|| view! { <App /> });
}
