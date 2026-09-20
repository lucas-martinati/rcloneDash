#![deny(dead_code)]
#![deny(unused_imports)]
#![deny(unused_variables)]
#![deny(unused_mut)]

mod app;
mod clipboard;
mod config;
mod fs_tree;
mod monitor;
mod systemd;
mod ui;
pub mod updater;
pub mod cmd;
pub mod installer;

use std::io;
use std::panic;
use std::time::Duration;

use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::time::interval;

use app::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Handle command-line arguments (before initializing TUI / raw mode)
    let args: Vec<String> = std::env::args().collect();
    let force_first_run = args.iter().any(|a| a == "--first-run" || a == "--wizard");

    if args.len() > 1 && !force_first_run {
        match args[1].as_str() {
            "--help" | "-h" => {
                println!("rcloneDash - Terminal Dashboard for Rclone\n");
                println!("Usage: rclonedash [OPTIONS]\n");
                println!("Options:");
                println!("  -h, --help            Print help information");
                println!("  -v, -V, --version     Print version information");
                println!("  --first-run, --wizard Run initial setup wizard (Google credentials & rclone check)");
                println!("  --check-update        Check if a newer version is available");
                println!("  -u, --update          Update rcloneDash to the latest release");
                return Ok(());
            }
            "--version" | "-v" | "-V" => {
                println!("rcloneDash v{}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--check-update" => {
                println!("Checking for updates...");
                match updater::check_for_updates().await {
                    Ok(Some(info)) => {
                        println!("🎉 A new version of rcloneDash is available: v{} (current: v{})", info.latest_version, info.current_version);
                        println!("Run `rclonedash --update` or `sudo rclonedash --update` to install it.");
                    }
                    Ok(None) => {
                        println!("✅ rcloneDash is up to date (v{}).", env!("CARGO_PKG_VERSION"));
                    }
                    Err(e) => {
                        eprintln!("⚠️  Could not check for updates: {}", e);
                    }
                }
                return Ok(());
            }
            "--update" | "-u" => {
                println!("Checking for latest release...");
                match updater::check_for_updates().await {
                    Ok(Some(info)) => {
                        println!("Downloading and installing rcloneDash v{}...", info.latest_version);
                        match updater::download_and_install_update(&info).await {
                            Ok(()) => {
                                println!("✨ Successfully updated to v{}!", info.latest_version);
                            }
                            Err(e) => {
                                eprintln!("❌ Update failed: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                    Ok(None) => {
                        println!("✅ rcloneDash is already on the latest version (v{}).", env!("CARGO_PKG_VERSION"));
                    }
                    Err(e) => {
                        eprintln!("❌ Update check failed: {}", e);
                        std::process::exit(1);
                    }
                }
                return Ok(());
            }
            unknown => {
                eprintln!("Unknown argument: {}\nRun `rclonedash --help` for usage.", unknown);
                std::process::exit(1);
            }
        }
    }

    // 1. Panic hook to cleanly restore terminal and mouse capture on crash
    let default_panic_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            PopKeyboardEnhancementFlags,
            DisableMouseCapture,
            LeaveAlternateScreen
        );
        default_panic_hook(panic_info);
    }));

    // 2. Initialize terminal with mouse capture and enhanced keyboard protocol
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let _ = execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        )
    );
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // 3. Initialize application state
    let mut app = App::new();
    if force_first_run {
        app.open_first_run();
    }

    // 4. Async event streams and intervals
    let mut reader = EventStream::new();
    let mut data_interval = interval(app.stats_interval_duration());
    let mut ui_interval = interval(Duration::from_millis(50));

    // Initial immediate render
    terminal.draw(|f| ui::render(f, &mut app))?;

    // 5. Main event loop (keyboard + mouse + timer ticks)
    while app.running {
        tokio::select! {
            _ = data_interval.tick() => {
                app.on_tick().await;
                terminal.draw(|f| ui::render(f, &mut app))?;
            }
            _ = ui_interval.tick() => {
                if app.running {
                    terminal.draw(|f| ui::render(f, &mut app))?;
                }
            }
            maybe_event = reader.next() => {
                if let Some(Ok(event)) = maybe_event {
                    let mut needs_draw = handle_single_event(event, &mut app, &mut terminal)?;

                    // Prevent backlog during fast mouse wheel / trackpad scrolling
                    // Drain all immediately available events in the buffer without overwriting the Tokio waker
                    while let Ok(Some(Ok(buffered_event))) = tokio::time::timeout(Duration::ZERO, reader.next()).await {
                        if handle_single_event(buffered_event, &mut app, &mut terminal)? {
                            needs_draw = true;
                        }
                    }

                    if needs_draw && app.running {
                        terminal.draw(|f| ui::render(f, &mut app))?;
                    }
                }
                if app.stats_interval_changed {
                    app.stats_interval_changed = false;
                    data_interval = interval(app.stats_interval_duration());
                }
            }
        }
    }

    // 6. Clean and silent terminal restoration
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), DisableMouseCapture, LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn open_full_logs(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut app::App,
) -> Result<(), Box<dyn std::error::Error>> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        PopKeyboardEnhancementFlags,
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    let log_path = "/tmp/rclone-bisync-full.log";
    // Retrieve complete logs via journalctl if possible
    let output = std::process::Command::new("journalctl")
        .args(["--user", "-u", "rclone-bisync", "--no-pager", "-n", "10000"])
        .output();
    if let Ok(out) = output {
        if !out.stdout.is_empty() {
            let _ = std::fs::write(log_path, &out.stdout);
        } else {
            let text = app.live.log_lines.iter().cloned().collect::<Vec<_>>().join("\n");
            let _ = std::fs::write(log_path, text);
        }
    } else {
        let text = app.live.log_lines.iter().cloned().collect::<Vec<_>>().join("\n");
        let _ = std::fs::write(log_path, text);
    }

    let _ = cmd::open_in_pager(std::path::Path::new(log_path));

    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        )
    )?;
    terminal.clear()?;

    app.set_toast("✔ Full logs viewer closed");
    Ok(())
}

fn handle_single_event(
    event: Event,
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<bool, Box<dyn std::error::Error>> {
    match event {
        Event::Key(key) => {
            if key.kind == crossterm::event::KeyEventKind::Press
                || key.kind == crossterm::event::KeyEventKind::Repeat
            {
                let action = app.handle_key(key);
                if action == app::Action::OpenEditor {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        PopKeyboardEnhancementFlags,
                        DisableMouseCapture,
                        LeaveAlternateScreen
                    )?;
                    terminal.show_cursor()?;

                    let path = config::filters_file();
                    let _ = cmd::open_in_editor(&path);

                    enable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        EnterAlternateScreen,
                        EnableMouseCapture,
                        PushKeyboardEnhancementFlags(
                            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                                | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                        )
                    )?;
                    terminal.clear()?;

                    app.filters = config::read_filters();
                    app.reload_files();
                    app.set_toast("✔ gdrive-filters.txt reloaded!");
                } else if action == app::Action::OpenFullLogs {
                    open_full_logs(terminal, app)?;
                }
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Event::Mouse(mouse) => {
            let action = app.handle_mouse(mouse);
            if action == app::Action::OpenFullLogs {
                open_full_logs(terminal, app)?;
            }
            Ok(true)
        }
        Event::Resize(_, _) => {
            terminal.autoresize()?;
            Ok(true)
        }
        _ => Ok(false),
    }
}
