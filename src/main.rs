#![deny(dead_code)]
#![deny(unused_imports)]
#![deny(unused_variables)]
#![deny(unused_mut)]

mod app;
mod clipboard;
mod config;
mod fs_tree;
mod monitor;
mod rclone;
mod systemd;
mod ui;
pub mod updater;
pub mod cmd;
pub mod installer;
pub mod term_caps;

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
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};
use futures_util::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::time::interval;

use app::App;
use term_caps::TtyMode;

/// Extracts the global `--tty-mode=auto|on|off` (or `--tty-mode <mode>`)
/// flag. Returns the override plus the remaining args (program name kept at
/// index 0). The flag wins over `dash-config.json` and is never saved.
/// Returns `Err` on an invalid value instead of exiting, so it stays unit
/// testable (`main` turns it into exit code 2).
fn parse_tty_override(args: &[String]) -> Result<(Option<TtyMode>, Vec<String>), String> {
    let valid = || TtyMode::options().join(", ");
    let mut tty_override = None;
    let mut rest = Vec::with_capacity(args.len());
    if let Some(first) = args.first() {
        rest.push(first.clone());
    }
    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        if a == "--tty-mode" {
            i += 1;
            match args.get(i).and_then(|v| TtyMode::parse(v)) {
                Some(m) => tty_override = Some(m),
                None => {
                    return Err(format!("Invalid --tty-mode value: expected one of: {}", valid()));
                }
            }
        } else if let Some(v) = a.strip_prefix("--tty-mode=") {
            match TtyMode::parse(v) {
                Some(m) => tty_override = Some(m),
                None => {
                    return Err(format!(
                        "Invalid --tty-mode value '{}': expected one of: {}",
                        v,
                        valid()
                    ));
                }
            }
        } else {
            rest.push(a.clone());
        }
        i += 1;
    }
    Ok((tty_override, rest))
}

/// Effective TTY mode for CLI (updater) paths: flag first, config fallback.
fn cli_tty_mode(tty_override: Option<TtyMode>) -> TtyMode {
    tty_override.unwrap_or_else(|| config::load_config().tty_mode)
}

/// Style for CLI (updater) paths: flag first, config fallback, and an
/// explicit `--tty-mode=off` forces colors even when piped or NO_COLOR
/// (like `ls --color=always`).
fn cli_style(tty_override: Option<TtyMode>) -> updater::ConsoleStyle {
    let mode = cli_tty_mode(tty_override);
    let forced = tty_override == Some(TtyMode::Off);
    updater::ConsoleStyle::from_caps(term_caps::TermCaps::resolve_forced(mode, forced))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Handle command-line arguments (before initializing TUI / raw mode)
    let args: Vec<String> = std::env::args().collect();
    let (tty_override, args) = match parse_tty_override(&args) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
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
                println!("  --update-demo         Preview the updater interface with a simulated update");
                println!("  --tty-mode=MODE       TTY compatibility (btop++ style): auto (detect), on (16 colors + ASCII), off (full color + Unicode). Overrides dash-config.json.");
                return Ok(());
            }
            "--version" | "-v" | "-V" => {
                println!("rcloneDash v{}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--check-update" => {
                let style = cli_style(tty_override);
                let g = style.caps.glyphs();
                println!("  {}{} {}", g.corner_tl, g.hline, style.bold_red("rcloneDash Update Check"));
                println!("  {}", g.vline);
                println!("  {}  Checking for updates...", style.bold_cyan(g.diamond));
                match updater::check_for_updates().await {
                    Ok(Some(info)) => {
                        println!("  {}  Current version: {}", g.vline, style.gray(&format!("v{}", info.current_version)));
                        println!("  {}  Latest version:  {}", g.vline, style.bold(&format!("v{}", info.latest_version)));
                        println!("  {}", g.vline);
                        println!("  {}{} {}", g.corner_bl, g.hline, style.bold_green(&format!("{} Update available: v{}! Run: rclonedash --update", g.rocket, info.latest_version)));
                    }
                    Ok(None) => {
                        println!("  {}  v{} is already installed", g.vline, env!("CARGO_PKG_VERSION"));
                        println!("  {}", g.vline);
                        println!("  {}{} {}", g.corner_bl, g.hline, style.bold_green(&format!("{} rcloneDash is up to date.", g.ok_emoji)));
                    }
                    Err(e) => {
                        println!("  {}", g.vline);
                        eprintln!("  {}{} {}", g.corner_bl, g.hline, style.bold_red(&format!("{}  Could not check for updates: {}", g.warn, e)));
                    }
                }
                return Ok(());
            }
            "--update" | "-u" => {
                let style = cli_style(tty_override);
                let g = style.caps.glyphs();
                println!("  {}{} {}", g.corner_tl, g.hline, style.bold_red("rcloneDash Updater"));
                println!("  {}", g.vline);
                println!("  {}  Checking for latest release...", style.bold_cyan(g.diamond));
                match updater::check_for_updates().await {
                    Ok(Some(info)) => {
                        println!("  {}  Found {} (current: {})", g.vline, style.bold(&format!("v{}", info.latest_version)), style.gray(&format!("v{}", info.current_version)));
                        println!("  {}", g.vline);
                        match updater::download_and_install_update(&info, style.caps).await {
                            Ok(()) => {
                                println!("  {}", g.vline);
                                println!("  {}{} {}", g.corner_bl, g.hline, style.bold_green(&format!("{} Successfully updated to v{}!", g.spark, info.latest_version)));
                            }
                            Err(e) => {
                                println!("  {}", g.vline);
                                eprintln!("  {}{} {}", g.corner_bl, g.hline, style.bold_red(&format!("{} Update failed: {}", g.fail_emoji, e)));
                                std::process::exit(1);
                            }
                        }
                    }
                    Ok(None) => {
                        println!("  {}  v{} is already installed", g.vline, env!("CARGO_PKG_VERSION"));
                        println!("  {}", g.vline);
                        println!("  {}{} {}", g.corner_bl, g.hline, style.bold_green(&format!("{} rcloneDash is already on the latest version.", g.ok_emoji)));
                    }
                    Err(e) => {
                        println!("  {}", g.vline);
                        eprintln!("  {}{} {}", g.corner_bl, g.hline, style.bold_red(&format!("{} Update check failed: {}", g.fail_emoji, e)));
                        std::process::exit(1);
                    }
                }
                return Ok(());
            }
            "--update-demo" => {
                updater::demo_update_flow(cli_style(tty_override).caps).await;
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
        leave_tui_mode_best_effort(&mut io::stdout());
        default_panic_hook(panic_info);
    }));

    // 2. Initialize terminal with mouse capture, enhanced keyboard protocol, and RAII Drop guard
    let mut stdout = io::stdout();
    enter_tui_mode(&mut stdout)?;
    let mut tui_guard = TerminalSessionGuard { active: true };
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // 3. Initialize application state
    let mut app = App::new();
    if let Some(mode) = tty_override {
        app.apply_tty_override(mode);
    }
    if force_first_run {
        app.open_first_run();
    }

    // 4. Async event streams and intervals
    let mut reader = EventStream::new();
    let mut data_interval = interval(app.stats_interval_duration());
    let mut ui_interval = interval(Duration::from_millis(50));
    let mut last_drawn_sec = current_epoch_secs();

    // Initial immediate render
    terminal.draw(|f| ui::render(f, &mut app))?;

    // 5. Main event loop (keyboard + mouse + background events + adaptive timer ticks)
    while app.running {
        tokio::select! {
            _ = data_interval.tick() => {
                app.on_tick().await;
                last_drawn_sec = current_epoch_secs();
                terminal.draw(|f| ui::render(f, &mut app))?;
            }
            Some(bg_ev) = app.recv_bg_event() => {
                app.handle_bg_event(bg_ev);
                app.drain_bg_events();
                if app.running {
                    last_drawn_sec = current_epoch_secs();
                    terminal.draw(|f| ui::render(f, &mut app))?;
                }
            }
            _ = ui_interval.tick() => {
                if app.running {
                    let now_sec = current_epoch_secs();
                    let toast_expired = app.toast.as_ref().is_some_and(|(_, t)| t.elapsed().as_secs() > 4);
                    if toast_expired {
                        app.toast = None;
                    }
                    // Redraw at 20 FPS during active sync animations, or once per wall-clock second when idle
                    if app.is_syncing() || toast_expired || now_sec != last_drawn_sec {
                        last_drawn_sec = now_sec;
                        terminal.draw(|f| ui::render(f, &mut app))?;
                    }
                }
            }
            maybe_event = reader.next() => {
                if let Some(Ok(event)) = maybe_event {
                    let mut needs_draw = handle_single_event(event, &mut app, &mut terminal, &mut tui_guard)?;

                    // Prevent backlog during fast mouse wheel / trackpad scrolling
                    // Drain all immediately available events in the buffer without overwriting the Tokio waker
                    while let Ok(Some(Ok(buffered_event))) = tokio::time::timeout(Duration::ZERO, reader.next()).await {
                        if handle_single_event(buffered_event, &mut app, &mut terminal, &mut tui_guard)? {
                            needs_draw = true;
                        }
                    }

                    if needs_draw && app.running {
                        last_drawn_sec = current_epoch_secs();
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

    // 6. Clean and symmetric terminal restoration (including PopKeyboardEnhancementFlags)
    tui_guard.active = false;
    leave_tui_mode(terminal.backend_mut())?;
    terminal.show_cursor()?;

    Ok(())
}

/// RAII guard ensuring the terminal leaves raw/alternate/enhanced-keyboard mode
/// even if `main` exits early via `?`.
struct TerminalSessionGuard {
    active: bool,
}

impl Drop for TerminalSessionGuard {
    fn drop(&mut self) {
        if self.active {
            leave_tui_mode_best_effort(&mut io::stdout());
        }
    }
}

fn current_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn enter_tui_mode<W: io::Write>(out: &mut W) -> io::Result<()> {
    enable_raw_mode()?;
    let _ = execute!(
        out,
        EnterAlternateScreen,
        SetTitle("RcloneDash"),
        EnableMouseCapture,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        )
    );
    Ok(())
}

fn leave_tui_mode<W: io::Write>(out: &mut W) -> io::Result<()> {
    let _ = execute!(out, PopKeyboardEnhancementFlags);
    disable_raw_mode()?;
    execute!(out, DisableMouseCapture, LeaveAlternateScreen)
}

fn leave_tui_mode_best_effort<W: io::Write>(out: &mut W) {
    let _ = disable_raw_mode();
    let _ = execute!(
        out,
        PopKeyboardEnhancementFlags,
        DisableMouseCapture,
        LeaveAlternateScreen,
        crossterm::cursor::Show
    );
}

fn suspend_tui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    guard: &mut TerminalSessionGuard,
) -> io::Result<()> {
    guard.active = false;
    leave_tui_mode(terminal.backend_mut())?;
    terminal.show_cursor()
}

fn resume_tui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    guard: &mut TerminalSessionGuard,
) -> io::Result<()> {
    enter_tui_mode(terminal.backend_mut())?;
    guard.active = true;
    terminal.clear()
}

fn secure_temp_log_path() -> std::path::PathBuf {
    let base_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir);
    base_dir.join(format!("rclonedash-full-{}.log", std::process::id()))
}

fn write_private_temp_file(path: &std::path::Path, bytes: &[u8]) {
    use std::io::Write;
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    if let Ok(mut f) = opts.open(path) {
        let _ = f.write_all(bytes);
    }
}

fn open_full_logs(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut app::App,
    guard: &mut TerminalSessionGuard,
) -> Result<(), Box<dyn std::error::Error>> {
    suspend_tui(terminal, guard)?;

    let log_path = secure_temp_log_path();
    // Retrieve complete logs via journalctl if possible
    let output = std::process::Command::new("journalctl")
        .args(["--user", "-u", "rclone-bisync", "--no-pager", "-n", "10000"])
        .output();
    match output {
        Ok(out) if !out.stdout.is_empty() => {
            write_private_temp_file(&log_path, &out.stdout);
        }
        _ => {
            let text = app.live.log_lines.iter().cloned().collect::<Vec<_>>().join("\n");
            write_private_temp_file(&log_path, text.as_bytes());
        }
    }

    let _ = cmd::open_in_pager(&log_path);
    let _ = std::fs::remove_file(&log_path);

    resume_tui(terminal, guard)?;

    app.set_toast("✔ Full logs viewer closed");
    Ok(())
}

fn handle_single_event(
    event: Event,
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    guard: &mut TerminalSessionGuard,
) -> Result<bool, Box<dyn std::error::Error>> {
    match event {
        Event::Key(key) => {
            if key.kind == crossterm::event::KeyEventKind::Press
                || key.kind == crossterm::event::KeyEventKind::Repeat
            {
                let action = app.handle_key(key);
                if action == app::Action::OpenEditor {
                    suspend_tui(terminal, guard)?;

                    let path = config::filters_file();
                    let _ = cmd::open_in_editor(&path);

                    resume_tui(terminal, guard)?;

                    app.filters = config::read_filters();
                    app.reload_files();
                    app.set_toast(format!("✔ {} reloaded!", config::filters_display_name()));
                } else if action == app::Action::OpenFullLogs {
                    open_full_logs(terminal, app, guard)?;
                }
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Event::Mouse(mouse) => {
            let action = app.handle_mouse(mouse);
            if action == app::Action::OpenFullLogs {
                open_full_logs(terminal, app, guard)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_parse_tty_override_forms() {
        // `=` form keeps the other args in order.
        let (mode, rest) = parse_tty_override(&argv(&["rclonedash", "--tty-mode=on", "--update"])).unwrap();
        assert_eq!(mode, Some(TtyMode::On));
        assert_eq!(rest, argv(&["rclonedash", "--update"]));

        // Space form.
        let (mode, rest) = parse_tty_override(&argv(&["rclonedash", "--tty-mode", "off"])).unwrap();
        assert_eq!(mode, Some(TtyMode::Off));
        assert_eq!(rest, argv(&["rclonedash"]));

        // Last flag wins.
        let (mode, _) =
            parse_tty_override(&argv(&["rclonedash", "--tty-mode=on", "--tty-mode=auto"])).unwrap();
        assert_eq!(mode, Some(TtyMode::Auto));

        // Invalid values and a missing value are errors (main exits 2).
        assert!(parse_tty_override(&argv(&["rclonedash", "--tty-mode=bogus"])).is_err());
        assert!(parse_tty_override(&argv(&["rclonedash", "--tty-mode"])).is_err());

        // No flag: passthrough.
        let (mode, rest) = parse_tty_override(&argv(&["rclonedash", "--update"])).unwrap();
        assert_eq!(mode, None);
        assert_eq!(rest, argv(&["rclonedash", "--update"]));
    }
}
