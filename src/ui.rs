use crate::app::App;
use crate::processes::ProcessMonitor;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Row, Sparkline, Table},
};

// Helper to generate a rainbow color based on a tick
fn get_rainbow_color(tick: u64, offset: u64) -> Color {
    let f = 0.1;
    let i = (tick + offset) as f64;
    let r = (f * i + 0.0).sin() * 127.0 + 128.0;
    let g = (f * i + 2.0).sin() * 127.0 + 128.0;
    let b = (f * i + 4.0).sin() * 127.0 + 128.0;
    Color::Rgb(r as u8, g as u8, b as u8)
}

// Helper for a pulsing neon color
fn get_neon_pulse(tick: u64, base_color: (u8, u8, u8)) -> Color {
    let (r, g, b) = base_color;
    let pulse = (tick as f64 * 0.1).sin().abs(); // 0.0 to 1.0
    let factor = 0.5 + (pulse * 0.5); // 0.5 to 1.0

    Color::Rgb(
        (r as f64 * factor) as u8,
        (g as f64 * factor) as u8,
        (b as f64 * factor) as u8,
    )
}

fn bytes_to_mb(bytes: u64) -> u64 {
    bytes / 1024 / 1024
}

fn unit_label(unit: crate::parser::SizeUnit) -> &'static str {
    match unit {
        crate::parser::SizeUnit::KB => "KB",
        crate::parser::SizeUnit::MB => "MB",
    }
}

pub fn ui(f: &mut Frame, app: &App) {
    // Switch views based on view_mode
    match app.view_mode {
        crate::app::ViewMode::Dashboard => render_dashboard(f, app),
        crate::app::ViewMode::Processes => render_processes_view(f, app),
        crate::app::ViewMode::GpuEngines => render_gpu_engines_view(f, app),
        crate::app::ViewMode::Clocks => render_clocks_view(f, app),
    }

    // Always render help overlay if shown
    if app.show_help {
        render_help(f);
    }
}

fn render_dashboard(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),  // Header
                Constraint::Length(3),  // Source/Trends/Conn
                Constraint::Length(4),  // RAM/SWAP/IRAM
                Constraint::Length(12), // CPU/GPU/Engines
                Constraint::Min(0),     // Temps/Power
            ]
            .as_ref(),
        )
        .split(f.area());

    // Animated Border Color
    let border_color = get_rainbow_color(app.tick_count, 0);

    // Header
    let title_color = get_rainbow_color(app.tick_count, 10);
    let header_text = Line::from(vec![
        Span::styled(
            "JetsonScope ",
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("TUI ", Style::default().fg(Color::White)),
        Span::styled(
            format!(
                "- {}",
                app.latest_stats
                    .timestamp
                    .clone()
                    .unwrap_or_else(|| "awaiting data".to_string())
            ),
            Style::default().fg(Color::Gray),
        ),
    ]);

    let header = Paragraph::new(header_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(if app.connection_status.contains("demo") || app.connection_status.contains("sintético") {
                    "⚠ MODO DEMO (Datos Sintéticos) ⚠"
                } else {
                    "System Status"
                }),
        )
        .style(Style::default().fg(if app.connection_status.contains("demo") || app.connection_status.contains("sintético") {
            Color::Yellow
        } else {
            Color::Cyan
        }));
    f.render_widget(header, chunks[0]);

    // Source + Trends
    render_trends(f, chunks[1], app, border_color);

    // RAM & SWAP
    let mem_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(40),
                Constraint::Percentage(40),
                Constraint::Percentage(20),
            ]
            .as_ref(),
        )
        .split(chunks[2]);

    let (ram_used_mb, ram_total_mb, ram_ratio, ram_unit) = app
        .latest_stats
        .ram
        .as_ref()
        .map(|ram| {
            let used = bytes_to_mb(ram.used_bytes);
            let total = bytes_to_mb(ram.total_bytes);
            let ratio = if ram.total_bytes == 0 {
                0.0
            } else {
                ram.used_bytes as f64 / ram.total_bytes as f64
            };
            (used, total, ratio, unit_label(ram.unit))
        })
        .unwrap_or((0, 0, 0.0, "MB"));

    // Neon Green for RAM
    let ram_color = get_neon_pulse(app.tick_count, (0, 255, 0));
    let ram_gauge = Gauge::default()
        .block(
            Block::default()
                .title("RAM")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .gauge_style(Style::default().fg(ram_color))
        .ratio(ram_ratio)
        .label(format!("{ram_used_mb}/{ram_total_mb} {ram_unit}"));
    f.render_widget(ram_gauge, mem_chunks[0]);

    let (swap_used_mb, swap_total_mb, swap_ratio, swap_unit) = app
        .latest_stats
        .swap
        .as_ref()
        .map(|swap| {
            let used = bytes_to_mb(swap.used_bytes);
            let total = bytes_to_mb(swap.total_bytes);
            let ratio = if swap.total_bytes == 0 {
                0.0
            } else {
                swap.used_bytes as f64 / swap.total_bytes as f64
            };
            (used, total, ratio, unit_label(swap.unit))
        })
        .unwrap_or((0, 0, 0.0, "MB"));

    // Neon Yellow for SWAP
    let swap_color = get_neon_pulse(app.tick_count, (255, 255, 0));
    let swap_gauge = Gauge::default()
        .block(
            Block::default()
                .title("SWAP")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .gauge_style(Style::default().fg(swap_color))
        .ratio(swap_ratio)
        .label(format!("{swap_used_mb}/{swap_total_mb} {swap_unit}"));
    f.render_widget(swap_gauge, mem_chunks[1]);

    // IRAM + MTS + LFB overview
    let iram_text = app
        .latest_stats
        .iram
        .as_ref()
        .map(|iram| {
            format!(
                "IRAM: {}/{} {}\nLFB: {} MB",
                bytes_to_mb(iram.used_bytes),
                bytes_to_mb(iram.total_bytes),
                unit_label(iram.unit),
                bytes_to_mb(iram.lfb_bytes.unwrap_or_default())
            )
        })
        .unwrap_or_else(|| "IRAM: n/a\nLFB: n/a".to_string());

    let lfb_text = app
        .latest_stats
        .ram
        .as_ref()
        .and_then(|ram| ram.largest_free_block.as_ref())
        .map(|lfb| match lfb {
            crate::parser::LargestFreeBlock::Blocks { count, size_bytes } => {
                format!("RAM LFB: {}x{} MB", count, bytes_to_mb(*size_bytes))
            }
            crate::parser::LargestFreeBlock::Size { size_bytes } => {
                format!("RAM LFB: {} MB", bytes_to_mb(*size_bytes))
            }
        })
        .unwrap_or_else(|| "RAM LFB: n/a".to_string());

    let swap_cached = app
        .latest_stats
        .swap
        .as_ref()
        .and_then(|swap| swap.cached_bytes)
        .map(|val| format!("SWAP cached: {} MB", bytes_to_mb(val)))
        .unwrap_or_else(|| "SWAP cached: -".to_string());

    let mts_text = app
        .latest_stats
        .mts
        .as_ref()
        .map(|mts| format!("MTS fg/bg: {}%/{}%", mts.fg_percent, mts.bg_percent))
        .unwrap_or_else(|| "MTS: -".to_string());

    // Lightweight clocks/engines summary (EMC/GR3D/NVENC/NVDEC)
    let mut engine_summary = Vec::new();
    for name in ["EMC", "GR3D", "MC", "AXI", "NVENC", "NVDEC"].iter() {
        if let Some(stat) = app.latest_stats.engines.get(&name.to_string()) {
            let usage = stat
                .usage_percent
                .map(|v| format!("{v}%"))
                .or_else(|| stat.raw_value.map(|v| v.to_string()))
                .unwrap_or_else(|| "-".to_string());
            let freq = stat
                .freq_mhz
                .map(|v| format!("{v}MHz"))
                .or_else(|| stat.raw_value.map(|v| format!("{v}MHz")))
                .unwrap_or_else(|| "-".to_string());
            engine_summary.push(format!("{name}: {usage} @ {freq}"));
        }
    }
    let engine_text = if engine_summary.is_empty() {
        "Engines: n/a".to_string()
    } else {
        engine_summary.join(" | ")
    };

    let mem_info = Paragraph::new(vec![
        Line::from(iram_text.clone()),
        Line::from(lfb_text.clone()),
        Line::from(swap_cached.clone()),
        Line::from(engine_text),
        Line::from(Span::styled(
            &mts_text,
            Style::default().fg(Color::LightCyan),
        )),
    ])
    .block(
        Block::default()
            .title("Mem/Engines")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(mem_info, mem_chunks[2]);

    // CPU & GPU
    let cpu_gpu_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
        .split(chunks[3]);

    let cpu_block = Block::default()
        .title("CPU")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    f.render_widget(cpu_block, cpu_gpu_chunks[0]);

    if !app.latest_stats.cpus.is_empty() {
        let inner_area = cpu_gpu_chunks[0].inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 1,
        });

        let core_constraints: Vec<Constraint> = (0..app.latest_stats.cpus.len())
            .map(|_| Constraint::Length(1))
            .collect();

        let core_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(core_constraints)
            .split(inner_area);

        for (i, core) in app.latest_stats.cpus.iter().enumerate() {
            if i < core_chunks.len() {
                let load = core.load_percent.unwrap_or(0);
                let freq = core.freq_mhz.unwrap_or(0);
                let label = format!("Core {}: {}% @ {}MHz", i, load, freq);
                let ratio = load as f64 / 100.0;

                // Color based on load (Green -> Yellow -> Red) but neon
                let core_color = if load < 50 {
                    Color::Rgb(0, 255, 255) // Cyan
                } else if load < 80 {
                    Color::Rgb(255, 255, 0) // Yellow
                } else {
                    Color::Rgb(255, 0, 255) // Magenta/Red
                };

                let gauge = Gauge::default()
                    .gauge_style(Style::default().fg(core_color))
                    .ratio(ratio)
                    .label(label);
                f.render_widget(gauge, core_chunks[i]);
            }
        }
    }

    // GPU and engine frequencies
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(cpu_gpu_chunks[1]);

    let gpu_load = app.latest_stats.gpu_usage().unwrap_or(0);
    let gpu_ratio = gpu_load as f64 / 100.0;
    // Neon Magenta for GPU
    let gpu_color = get_neon_pulse(app.tick_count, (255, 0, 255));
    let gpu_gauge = Gauge::default()
        .block(
            Block::default()
                .title("GPU")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .gauge_style(Style::default().fg(gpu_color))
        .ratio(gpu_ratio)
        .label(format!("{gpu_load}%"));
    f.render_widget(gpu_gauge, right_chunks[0]);

    // Engine table (EMC, NVENC, NVDEC, etc.)
    let mut engines: Vec<(&String, &crate::parser::EngineStat)> =
        app.latest_stats.engines.iter().collect();
    engines.sort_by(|a, b| a.0.cmp(b.0));
    let engine_rows: Vec<Row> = engines
        .into_iter()
        .map(|(name, stat)| {
            let usage = stat
                .usage_percent
                .map(|v| format!("{v}%"))
                .or_else(|| stat.raw_value.map(|v| v.to_string()))
                .unwrap_or_else(|| "-".to_string());
            let freq = stat
                .freq_mhz
                .map(|v| format!("{v} MHz"))
                .or_else(|| stat.raw_value.map(|v| format!("{v} MHz")))
                .unwrap_or_else(|| "-".to_string());
            Row::new(vec![
                Span::styled(name.to_string(), Style::default().fg(Color::Magenta)),
                Span::styled(usage, Style::default().fg(Color::White)),
                Span::styled(freq, Style::default().fg(Color::Gray)),
            ])
        })
        .collect();
    let engine_table = Table::new(
        engine_rows,
        [
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ],
    )
    .block(
        Block::default()
            .title("Engines")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    )
    .header(
        Row::new(vec!["Name", "Usage", "Freq"]).style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    );
    f.render_widget(engine_table, right_chunks[1]);

    // Temps & Power
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[4]);

    // Temps Table
    let mut temps: Vec<(&String, &f32)> = app.latest_stats.temps.iter().collect();
    temps.sort_by(|a, b| a.0.cmp(b.0));
    let temp_rows: Vec<Row> = temps
        .iter()
        .map(|(k, v)| {
            let color = if **v > 80.0 {
                Color::Red
            } else if **v > 60.0 {
                Color::Yellow
            } else {
                Color::Green
            };
            Row::new(vec![
                Span::styled((*k).to_string(), Style::default().fg(Color::Cyan)),
                Span::styled(format!("{:.1}C", v), Style::default().fg(color)),
            ])
        })
        .collect();
    let temp_table = Table::new(
        temp_rows,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .block(
        Block::default()
            .title("Temperatures")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    )
    .header(
        Row::new(vec!["Sensor", "Temp"]).style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    );
    f.render_widget(temp_table, bottom_chunks[0]);

    // Power Table
    let mut power_entries: Vec<(&String, &crate::parser::PowerRail)> =
        app.latest_stats.power.iter().collect();
    power_entries.sort_by(|a, b| a.0.cmp(b.0));
    let power_rows: Vec<Row> = power_entries
        .iter()
        .map(|(k, rail)| {
            Row::new(vec![
                Span::styled((*k).to_string(), Style::default().fg(Color::Magenta)),
                Span::styled(
                    format!("{}mW", rail.current_mw),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{}mW", rail.average_mw),
                    Style::default().fg(Color::Gray),
                ),
            ])
        })
        .collect();
    let power_table = Table::new(
        power_rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ],
    )
    .block(
        Block::default()
            .title("Power")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    )
    .header(
        Row::new(vec!["Rail", "Current", "Avg"]).style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    );
    f.render_widget(power_table, bottom_chunks[1]);

    if app.show_help {
        render_help(f);
    }
}

fn render_trends(f: &mut Frame, area: ratatui::layout::Rect, app: &App, border_color: Color) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Length(22),
                Constraint::Length(34),
                Constraint::Min(0),
            ]
            .as_ref(),
        )
        .split(area);

    let source = Paragraph::new(format!(
        "Source: {} | {}",
        app.source_label, app.connection_status
    ))
    .block(
        Block::default()
            .title(format!("Fuente/Conexión [{}]", app.connection_status))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(connection_color(&app.connection_status))),
    )
    .style(Style::default().fg(connection_color(&app.connection_status)));
    f.render_widget(source, chunks[0]);

    // Controls status (read-only for now)
    let ctrl = app.control.status();
    let ctrl_lines = vec![
        Line::from(format!(
            "available: {}",
            if ctrl.available { "yes" } else { "no" }
        )),
        Line::from(format!(
            "jetson_clocks: {}",
            ctrl.jetson_clocks
                .map(|v| if v { "on" } else { "off" })
                .unwrap_or("n/a")
        )),
        Line::from(format!(
            "nvpmodel: {}",
            ctrl.nvpmodel.clone().unwrap_or_else(|| "n/a".to_string())
        )),
        Line::from(format!(
            "fan: {}",
            ctrl.fan.clone().unwrap_or_else(|| "n/a".to_string())
        )),
        Line::from(format!(
            "modes: {}",
            if ctrl.nvpmodel_modes.is_empty() {
                "n/a".to_string()
            } else {
                ctrl.nvpmodel_modes.join(", ")
            }
        )),
        Line::from(ctrl.note.clone()),
        Line::from(ctrl.last_error.clone().unwrap_or_else(|| "OK".to_string())),
    ];
    let ctrl_widget = Paragraph::new(ctrl_lines).block(
        Block::default()
            .title("Controles (c/m/f)")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(ctrl_widget, chunks[1]);

    let trend_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(chunks[2]);

    // Filter history by time window
    use std::time::Instant;
    let now = Instant::now();
    let window_secs = app.history_window.duration_secs();
    let window_label = app.history_window.label();
    
    let filter_by_window = |data: &std::collections::VecDeque<(Instant, f64)>| -> Vec<u64> {
        data.iter()
            .filter(|(timestamp, _)| now.duration_since(*timestamp).as_secs() <= window_secs)
            .map(|(_, value)| *value as u64)
            .collect()
    };
    
    let ram_data = filter_by_window(&app.history.ram);
    let gpu_data = filter_by_window(&app.history.gpu);
    let cpu_data = filter_by_window(&app.history.cpu);

    let sparkline_ram = Sparkline::default()
        .block(
            Block::default()
                .title(format!("RAM [{}]", window_label))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .data(&ram_data)
        .style(Style::default().fg(Color::Green));

    let sparkline_gpu = Sparkline::default()
        .block(
            Block::default()
                .title(format!("GPU [{}]", window_label))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .data(&gpu_data)
        .style(Style::default().fg(Color::Magenta));

    let sparkline_cpu = Sparkline::default()
        .block(
            Block::default()
                .title(format!("CPU avg [{}]", window_label))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .data(&cpu_data)
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(sparkline_ram, trend_chunks[0]);
    f.render_widget(sparkline_gpu, trend_chunks[1]);
    f.render_widget(sparkline_cpu, trend_chunks[2]);
}

#[allow(dead_code)]
fn history_to_u64(data: &[f64]) -> Vec<u64> {
    data.iter()
        .map(|v| if *v < 0.0 { 0 } else { v.round() as u64 })
        .collect()
}

fn connection_color(status: &str) -> Color {
    if status.contains("conectado") && !status.contains("offline") {
        Color::Green
    } else if status.contains("reintentando") || status.contains("timeout") {
        Color::Yellow
    } else if status.contains("offline") || status.contains("error") {
        Color::Red
    } else if status.contains("demo") || status.contains("sintético") {
        Color::Gray
    } else {
        Color::Cyan // conectando
    }
}

fn render_help(f: &mut Frame) {
    let area = centered_rect(70, 60, f.area());
    let help_text = vec![
        Line::from("Teclas:"),
        Line::from("  q: salir"),
        Line::from("  h: toggle ayuda"),
        Line::from("  v: ciclo de vista (Dashboard/Procesos/GPU/Clocks)"),
        Line::from("  s: ordenar procesos (CPU/Mem)"),
        Line::from("  r: reconectar al socket"),
        Line::from(""),
        Line::from("Controles (requieren daemon):"),
        Line::from("  c: toggle jetson_clocks"),
        Line::from("  m: cambiar nvpmodel"),
        Line::from("  f: fan 80% (demo)"),
        Line::from(""),
        Line::from("Conexión:"),
        Line::from("  Socket: /tmp/jetsonscope.sock (legacy: /tmp/tegrastats.sock)"),
        Line::from("  Fallback: modo sintético si socket no disponible"),
        Line::from("  Estados: conectado (verde), reintentando (amarillo),"),
        Line::from("           offline (rojo), demo (gris)"),
    ];
    let block = Block::default()
        .title("Ayuda")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let para = Paragraph::new(help_text).block(block).style(Style::default().fg(Color::White));
    f.render_widget(Clear, area);
    f.render_widget(para, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    let vertical = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1]);

    vertical[1]
}

fn render_processes_view(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Process table
        ])
        .split(f.area());

    // Header
    let border_color = get_rainbow_color(app.tick_count, 0);
    let header = Paragraph::new("Vista de Procesos - Top CPU/Memoria")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title("Procesos"),
        )
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(header, chunks[0]);

    // Process table
    let mut monitor = ProcessMonitor::new();
    let top_processes = monitor.top_processes(15, app.process_sort_by_mem);
    
    let rows: Vec<Row> = top_processes
        .iter()
        .map(|p| {
            let cpu_color = if p.cpu_usage > 50.0 {
                Color::Red
            } else if p.cpu_usage > 25.0 {
                Color::Yellow
            } else {
                Color::Green
            };
            
            Row::new(vec![
                Span::styled(p.pid.to_string(), Style::default().fg(Color::Cyan)),
                Span::styled(p.name.clone(), Style::default().fg(Color::White)),
                Span::styled(format!("{:.1}%", p.cpu_usage), Style::default().fg(cpu_color)),
                Span::styled(format!("{} MB", p.memory_kb / 1024), Style::default().fg(Color::Magenta)),
                Span::styled(p.user.clone().unwrap_or_else(|| "-".to_string()), Style::default().fg(Color::Gray)),
                Span::styled(
                    p.threads
                        .map(|t| t.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(Color::Gray),
                ),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Percentage(32),
            Constraint::Length(10),
            Constraint::Length(14),
            Constraint::Length(10),
            Constraint::Length(8),
        ],
    )
    .block(
        Block::default()
            .title("Top Procesos")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    )
    .header(
        Row::new(vec![
            "PID",
            "Nombre",
            if app.process_sort_by_mem { "CPU (▲)" } else { "CPU" },
            if app.process_sort_by_mem { "Memoria (▼)" } else { "Memoria" },
            "UID",
            "Threads",
        ])
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    );
    
    f.render_widget(table, chunks[1]);
}

fn render_gpu_engines_view(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Engines grid
        ])
        .split(f.area());

    // Header
    let border_color = get_rainbow_color(app.tick_count, 0);
    let header = Paragraph::new("Vista de GPU Engines - Frecuencias y Uso")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title("GPU Engines"),
        )
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(header, chunks[0]);

    // Engines grid
    let mut engines: Vec<(&String, &crate::parser::EngineStat)> =
        app.latest_stats.engines.iter().collect();
    engines.sort_by(|a, b| a.0.cmp(b.0));

    // Create grid layout
    let num_engines = engines.len();
    let rows = (num_engines + 1) / 2; // 2 columns
    let mut constraints = vec![];
    for _ in 0..rows {
        constraints.push(Constraint::Length(5));
    }
    
    let row_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(chunks[1]);

    for (i, (name, stat)) in engines.iter().enumerate() {
        let row_idx = i / 2;
        let col_idx = i % 2;
        
        if row_idx >= row_chunks.len() {
            break;
        }
        
        let col_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(row_chunks[row_idx]);
        
        let area = col_chunks[col_idx];
        
        let usage = stat.usage_percent.unwrap_or(0);
        let freq = stat.freq_mhz.map(|f| format!("{} MHz", f))
            .or_else(|| stat.raw_value.map(|v| v.to_string()))
            .unwrap_or_else(|| "-".to_string());
        
        let color = if usage > 75 {
            Color::Red
        } else if usage > 50 {
            Color::Yellow
        } else {
            Color::Green
        };
        
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title(format!("{} ({})", name, freq))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .gauge_style(Style::default().fg(color))
            .ratio(usage as f64 / 100.0)
            .label(format!("{}%", usage));
        
        f.render_widget(gauge, area);
    }
}

fn render_clocks_view(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(6),  // CPU clusters
            Constraint::Length(6),  // EMC/MC/AXI
            Constraint::Length(6),  // GPU/GR3D
            Constraint::Min(0),     // Controls/governors
        ])
        .split(f.area());

    let border_color = get_rainbow_color(app.tick_count, 0);
    let header = Paragraph::new("Clocks & Governors")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title("Clocks/Perf"),
        )
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(header, chunks[0]);

    // CPU clusters overview
    let cpu_loads: Vec<_> = app.latest_stats.cpus.iter().map(|c| c.load_percent.unwrap_or(0)).collect();
    let cpu_freqs: Vec<_> = app.latest_stats.cpus.iter().map(|c| c.freq_mhz.unwrap_or(0)).collect();
    let cpu_lines = vec![
        Line::from(format!("Cores: {}", app.latest_stats.cpus.len())),
        Line::from(format!("Avg load: {:.1}%", if cpu_loads.is_empty() { 0.0 } else { cpu_loads.iter().sum::<u32>() as f64 / cpu_loads.len() as f64 })),
        Line::from(format!("Max freq: {} MHz", cpu_freqs.iter().max().cloned().unwrap_or(0))),
        Line::from(format!(
            "Governor: {}",
            app.control
                .status()
                .cpu_governor
                .clone()
                .unwrap_or_else(|| "n/a".to_string())
        )),
    ];
    let cpu_block = Paragraph::new(cpu_lines)
        .block(
            Block::default()
                .title("CPU Clusters")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        );
    f.render_widget(cpu_block, chunks[1]);

    // EMC/MC/AXI
    let mut emc_lines = Vec::new();
    for name in ["EMC", "MC", "AXI"].iter() {
        if let Some(stat) = app.latest_stats.engines.get(*name) {
            let usage = stat.usage_percent.map(|v| format!("{v}% ")).unwrap_or_default();
            let freq = stat.freq_mhz.map(|v| format!("{v} MHz")).unwrap_or_else(|| "-".to_string());
            emc_lines.push(Line::from(format!("{name}: {usage}{freq}")));
        }
    }
    if emc_lines.is_empty() {
        emc_lines.push(Line::from("No EMC/MC/AXI data"));
    }
    let emc_block = Paragraph::new(emc_lines).block(
        Block::default()
            .title("Memory/Bus Clocks")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(emc_block, chunks[2]);

    // GPU/Engines (GR3D + video/vision)
    let mut eng_lines = Vec::new();
    for name in ["GR3D", "NVENC", "NVDEC", "NVJPG", "NVJPG1", "VIC", "OFA", "ISP", "NVCSI"].iter() {
        if let Some(stat) = app.latest_stats.engines.get(*name) {
            let usage = stat.usage_percent.map(|v| format!("{v}% ")).unwrap_or_else(|| "off ".to_string());
            let freq = stat.freq_mhz.map(|v| format!("{v} MHz")).unwrap_or_else(|| "-".to_string());
            eng_lines.push(Line::from(format!("{name}: {usage}{freq}")));
        }
    }
    if eng_lines.is_empty() {
        eng_lines.push(Line::from("No engine data"));
    }
    let eng_block = Paragraph::new(eng_lines).block(
        Block::default()
            .title("GPU/Media Engines")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(eng_block, chunks[3]);

    // Controls/governors summary
    let ctrl = app.control.status();
    let ctrl_lines = vec![
        Line::from(format!("jetson_clocks: {}", ctrl.jetson_clocks.map(|v| if v { "on" } else { "off" }).unwrap_or("n/a"))),
        Line::from(format!("nvpmodel: {}", ctrl.nvpmodel.clone().unwrap_or_else(|| "n/a".to_string()))),
        Line::from(format!("fan: {}", ctrl.fan.clone().unwrap_or_else(|| "n/a".to_string()))),
        Line::from(format!("supports: fan={} nvpmodel={} jetson_clocks={}", ctrl.supports_fan, ctrl.supports_nvpmodel, ctrl.supports_jetson_clocks)),
    ];
    let ctrl_block = Paragraph::new(ctrl_lines).block(
        Block::default()
            .title("Controls")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(ctrl_block, chunks[4]);
}
