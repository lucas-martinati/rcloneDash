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
    let mut tick_rate = interval(Duration::from_millis(tick_ms));

    // Premier rendu immédiat
    terminal.draw(|f| ui::render(f, &mut app))?;

    // 5. Boucle d'événements (clavier + souris + ticks)
    while app.running {
        tokio::select! {
            _ = tick_rate.tick() => {
                app.on_tick().await;
                terminal.draw(|f| ui::render(f, &mut app))?;
            }
            maybe_event = reader.next() => {
                if let Some(Ok(event)) = maybe_event {
                    match event {
                        Event::Key(key) => {
                            if key.kind == crossterm::event::KeyEventKind::Press {
                                let action = app.handle_key(key);
                                if action == app::Action::OpenEditor {
                                    // Suspendre temporairement le TUI et la souris pour l'éditeur
                                    disable_raw_mode()?;
                                    execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags, DisableMouseCapture, LeaveAlternateScreen)?;
                                    terminal.show_cursor()?;

                                    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
                                    let path = config::filters_file();
                                    let _ = std::process::Command::new(&editor).arg(&path).status();

                                    enable_raw_mode()?;
                                    execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture, PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES))?;
                                    terminal.clear()?;

                                    app.filters = config::read_filters();
                                    app.reload_files();
                                    app.set_toast("✔ Fichier gdrive-filters.txt rechargé !");
                                }
                                terminal.draw(|f| ui::render(f, &mut app))?;
                            } else if key.kind == crossterm::event::KeyEventKind::Release {
                                terminal.draw(|f| ui::render(f, &mut app))?;
                            }
                        }
                        Event::Mouse(mouse) => {
                            let action = app.handle_mouse(mouse);
                            if action == app::Action::OpenEditor {
                                // Cas où un clic déclencherait l'éditeur
                            }
                            terminal.draw(|f| ui::render(f, &mut app))?;
                        }
                        _ => {}
                    }
                }
                if app.tick_rate_changed {
                    app.tick_rate_changed = false;
                    tick_rate = interval(Duration::from_millis(app.tick_rate_ms_live));
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
