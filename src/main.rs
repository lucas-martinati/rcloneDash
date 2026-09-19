mod app;
mod clipboard;
mod config;
mod fs_tree;
mod monitor;
mod systemd;
mod ui;

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
    // 1. Hook de panic pour restaurer le terminal et la souris en cas d'erreur
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

    // 2. Initialisation du terminal avec capture souris et clavier enrichi (btop++ style)
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

    // 3. Initialisation de l'application
    let mut app = App::new();

    // 4. Flux d'événements asynchrones
    let mut reader = EventStream::new();
    let tick_ms = app.config.tick_rate_ms.unwrap_or(250);
    let mut data_interval = interval(Duration::from_millis(tick_ms));
    let mut ui_interval = interval(Duration::from_millis(50));

    // Premier rendu immédiat
    terminal.draw(|f| ui::render(f, &mut app))?;

    // 5. Boucle d'événements (clavier + souris + ticks)
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

                    // Évite l'engorgement lors de défilements rapides (molette souris / trackpad)
                    // Draine tous les événements déjà disponibles dans le tampon sans écraser le waker Tokio
                    while let Ok(Some(Ok(buffered_event))) = tokio::time::timeout(Duration::ZERO, reader.next()).await {
                        if handle_single_event(buffered_event, &mut app, &mut terminal)? {
                            needs_draw = true;
                        }
                    }

                    if needs_draw && app.running {
                        terminal.draw(|f| ui::render(f, &mut app))?;
                    }
                }
                if app.tick_rate_changed {
                    app.tick_rate_changed = false;
                    data_interval = interval(Duration::from_millis(app.tick_rate_ms_live));
                }
            }
        }
    }

    // 6. Restauration propre et silencieuse du terminal
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
    // Récupérer les logs complets via journalctl si possible
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

    let pager = std::env::var("PAGER")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "less".to_string());
    if pager == "less" {
        let _ = std::process::Command::new("less").arg("-R").arg(log_path).status();
    } else {
        let _ = std::process::Command::new(&pager).arg(log_path).status();
    }

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

                    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
                    let path = config::filters_file();
                    let _ = std::process::Command::new(&editor).arg(&path).status();

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
