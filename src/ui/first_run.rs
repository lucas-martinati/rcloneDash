use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, FirstRunField, FirstRunState, FirstRunStep, HitAction, Hitbox, RcloneInstallStatus};
use crate::ui::container::{centered_fixed_rect, render_modal_container, ModalContainerConfig};
use crate::ui::theme::ThemePalette;

pub fn first_run_modal_dimensions(screen: Rect, show_help: bool) -> (u16, u16) {
    let width = if screen.width >= 92 {
        88.min(screen.width.saturating_sub(4))
    } else if screen.width >= 82 {
        80.min(screen.width.saturating_sub(2))
    } else {
        screen.width.saturating_sub(2)
    };
    let height = if show_help { 27 } else { 19 }.min(screen.height.saturating_sub(2));
    (width, height)
}

pub fn render_first_run_modal(
    f: &mut Frame,
    app: &App,
    state: &FirstRunState,
    theme: &ThemePalette,
    hitboxes: &mut Vec<Hitbox>,
) {
    let screen = f.area();
    let (width, height) = first_run_modal_dimensions(screen, state.show_help);
    let area = centered_fixed_rect(width, height, screen);

    let (step_num, step_name) = match state.step {
        FirstRunStep::RcloneCheck => (1, "Rclone Detection"),
        FirstRunStep::GoogleCredentials => (2, "Google Drive API"),
    };

    use crate::ui::keys::KeybindingRegistry;
    let actions: Vec<Vec<Span>> = match state.step {
        FirstRunStep::RcloneCheck => match state.rclone_status {
            RcloneInstallStatus::Installed(_) => vec![
                KeybindingRegistry::format_shortcut_label("↵", "continue", theme.green, Color::White),
            ],
            RcloneInstallStatus::Installing => vec![
                KeybindingRegistry::format_shortcut_label("⏳", "installing...", theme.cyan, Color::White),
            ],
            RcloneInstallStatus::NotInstalled | RcloneInstallStatus::Failed(_) => vec![
                KeybindingRegistry::format_shortcut_label("↵", "install", theme.cyan, Color::White),
                KeybindingRegistry::format_shortcut_label("Tab", "switch", theme.accent, Color::White),
                KeybindingRegistry::format_shortcut_label("Esc", "skip", theme.yellow, Color::White),
            ],
        },
        FirstRunStep::GoogleCredentials => vec![
            KeybindingRegistry::format_shortcut_label("Tab", "navigate", theme.accent, Color::White),
            KeybindingRegistry::format_shortcut_label("Ctrl+V", "paste", theme.cyan, Color::White),
            KeybindingRegistry::format_shortcut_label("↵", "confirm", theme.green, Color::White),
            KeybindingRegistry::format_shortcut_label("?", if state.show_help { "hide guide" } else { "guide" }, theme.yellow, Color::White),
            KeybindingRegistry::format_shortcut_label("Esc", "skip", theme.red, Color::White),
        ],
    };

    let inner = render_modal_container(
        f,
        app,
        theme,
        area,
        ModalContainerConfig {
            title_prefix: "setup wizard",
            title_color: Some(theme.accent),
            title_extra: Some(vec![
                Span::styled(format!(" │ {}", step_name), Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
            ]),
            action_shortcuts: Some(actions),
            counter: Some((step_num, 2)),
            border_color: theme.accent,
            show_close_button: true,
            ..Default::default()
        },
        hitboxes,
    );

    match state.step {
        FirstRunStep::RcloneCheck => render_step_rclone(f, app, state, theme, inner, hitboxes),
        FirstRunStep::GoogleCredentials => render_step_google(f, app, state, theme, inner, hitboxes),
    }
}

fn render_step_rclone(
    f: &mut Frame,
    app: &App,
    state: &FirstRunState,
    theme: &ThemePalette,
    area: Rect,
    hitboxes: &mut Vec<Hitbox>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Intro
            Constraint::Length(5), // Status card
            Constraint::Length(1), // Spacer
            Constraint::Length(2), // Action buttons
            Constraint::Min(2),    // Footer tips
        ])
        .split(area);

    // 1. Intro text with word wrapping
    let intro_text = vec![
        Line::from(Span::styled("Welcome to RcloneDash!", Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD))),
        Line::from("RcloneDash monitors bidirectional syncs and requires rclone on your system."),
    ];
    f.render_widget(Paragraph::new(intro_text).wrap(Wrap { trim: true }), chunks[0]);

    // 2. Status card
    let card_area = chunks[1];
    let (badge_style, badge_text, desc_lines) = match &state.rclone_status {
        RcloneInstallStatus::Installed(ver) => (
            Style::default().fg(Color::Black).bg(theme.green).add_modifier(Modifier::BOLD),
            " ✔ RCLONE DETECTED ",
            vec![
                Line::from(vec![
                    Span::styled("Binary detected: ", Style::default().fg(theme.text_muted)),
                    Span::styled(ver.as_str(), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
                ]),
            ],
        ),
        RcloneInstallStatus::NotInstalled => (
            Style::default().fg(Color::Black).bg(theme.yellow).add_modifier(Modifier::BOLD),
            " ⚠ RCLONE NOT FOUND ",
            vec![
                Line::from(Span::styled("rclone was not found in PATH or ~/.local/bin/rclone.", Style::default().fg(theme.yellow))),
                Line::from("You can install it automatically right now without sudo privileges."),
            ],
        ),
        RcloneInstallStatus::Installing => (
            Style::default().fg(Color::Black).bg(theme.cyan).add_modifier(Modifier::BOLD),
            " ⏳ INSTALLING RCLONE... ",
            vec![
                Line::from(Span::styled("Downloading official precompiled binary to ~/.local/bin/rclone...", Style::default().fg(theme.cyan))),
            ],
        ),
        RcloneInstallStatus::Failed(err) => (
            Style::default().fg(Color::White).bg(theme.red).add_modifier(Modifier::BOLD),
            " ✗ INSTALLATION FAILED ",
            vec![
                Line::from(Span::styled(err.as_str(), Style::default().fg(theme.red))),
            ],
        ),
    };

    let card_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.border_type())
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.card_bg));

    let card_inner = card_block.inner(card_area);
    f.render_widget(card_block, card_area);

    let mut card_content = vec![
        Line::from(Span::styled(badge_text, badge_style)),
    ];
    card_content.extend(desc_lines);
    f.render_widget(Paragraph::new(card_content).wrap(Wrap { trim: true }), card_inner);

    // 3. Action Buttons
    let btn_area = chunks[3];
    let is_installed = matches!(state.rclone_status, RcloneInstallStatus::Installed(_));
    let is_installing = matches!(state.rclone_status, RcloneInstallStatus::Installing);

    let mut btn_spans = Vec::new();

    if is_installed {
        let is_focused = state.active_field == FirstRunField::ContinueButton;
        let style = if is_focused {
            Style::default().fg(Color::Black).bg(theme.green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_bright).bg(theme.border).add_modifier(Modifier::BOLD)
        };
        let btn_text = " [ Next: Configure Google Drive → (Enter) ] ";
        btn_spans.push(Span::styled(btn_text, style));

        hitboxes.push(Hitbox {
            rect: Rect {
                x: btn_area.x,
                y: btn_area.y,
                width: (btn_text.chars().count() as u16).min(btn_area.width),
                height: 1,
            },
            action: HitAction::FirstRunContinue,
        });
    } else if is_installing {
        btn_spans.push(Span::styled(" ⏳ Installing rclone... Please wait ", Style::default().fg(theme.cyan).add_modifier(Modifier::ITALIC)));
    } else {
        // Install button
        let is_install_focused = state.active_field == FirstRunField::InstallRcloneButton;
        let inst_style = if is_install_focused {
            Style::default().fg(Color::Black).bg(theme.cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_bright).bg(theme.border).add_modifier(Modifier::BOLD)
        };
        let inst_text = " [ Install Rclone Now (Enter) ] ";
        btn_spans.push(Span::styled(inst_text, inst_style));
        btn_spans.push(Span::raw("   "));

        hitboxes.push(Hitbox {
            rect: Rect {
                x: btn_area.x,
                y: btn_area.y,
                width: (inst_text.chars().count() as u16).min(btn_area.width),
                height: 1,
            },
            action: HitAction::FirstRunInstall,
        });

        // Skip button
        let is_skip_focused = state.active_field == FirstRunField::SkipRcloneButton;
        let skip_style = if is_skip_focused {
            Style::default().fg(Color::Black).bg(theme.yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_muted).bg(theme.border)
        };
        let skip_text = " [ Skip for now (Esc) ] ";
        btn_spans.push(Span::styled(skip_text, skip_style));

        let skip_x = btn_area.x + inst_text.chars().count() as u16 + 3;
        hitboxes.push(Hitbox {
            rect: Rect {
                x: skip_x,
                y: btn_area.y,
                width: (skip_text.chars().count() as u16).min(btn_area.width.saturating_sub(skip_x - btn_area.x)),
                height: 1,
            },
            action: HitAction::FirstRunSkipRclone,
        });
    }

    f.render_widget(Paragraph::new(Line::from(btn_spans)), btn_area);
}

fn render_step_google(
    f: &mut Frame,
    app: &App,
    state: &FirstRunState,
    theme: &ThemePalette,
    area: Rect,
    hitboxes: &mut Vec<Hitbox>,
) {
    let constraints = if state.show_help {
        vec![
            Constraint::Length(4), // Explanatory banner
            Constraint::Length(3), // Client ID input
            Constraint::Length(3), // Client Secret input
            Constraint::Length(2), // Action buttons
            Constraint::Min(6),    // Help card
        ]
    } else {
        vec![
            Constraint::Length(4), // Explanatory banner
            Constraint::Length(3), // Client ID input
            Constraint::Length(3), // Client Secret input
            Constraint::Length(2), // Action buttons
            Constraint::Min(2),    // Shortcut tips
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    // 1. Explanatory banner with text wrapping
    let quota_info = vec![
        Line::from(Span::styled("Why use your own Google Cloud Console credentials?", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
        Line::from("• By default, rclone uses a shared public ID causing 403 Rate Limit errors."),
        Line::from("• A custom Client ID gives you a dedicated personal quota (1,000 req/100s)."),
    ];
    f.render_widget(Paragraph::new(quota_info).wrap(Wrap { trim: true }), chunks[0]);

    // 2. Client ID input box
    let id_area = chunks[1];
    let is_id_active = state.active_field == FirstRunField::ClientIdInput;
    let id_border_color = if is_id_active { theme.accent } else { theme.border };

    let id_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.border_type())
        .border_style(Style::default().fg(id_border_color))
        .title(Span::styled(" Google Client ID ", Style::default().fg(if is_id_active { theme.accent } else { theme.text_muted }).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(theme.card_bg));

    let id_inner = id_block.inner(id_area);
    f.render_widget(id_block, id_area);

    let id_disp = if state.client_id.is_empty() {
        if is_id_active {
            Line::from(Span::styled("_ (paste with Ctrl+V)", Style::default().fg(theme.accent)))
        } else {
            Line::from(Span::styled("(paste your Client ID here or press Enter to skip)", Style::default().fg(theme.text_muted).add_modifier(Modifier::ITALIC)))
        }
    } else {
        let max_w = id_inner.width as usize;
        let text = if is_id_active {
            crate::ui::settings::format_scrolled_input_with_cursor(&state.client_id, state.client_id_cursor, max_w)
        } else {
            state.client_id.clone()
        };
        Line::from(Span::styled(text, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)))
    };
    f.render_widget(Paragraph::new(id_disp), id_inner);

    hitboxes.push(Hitbox {
        rect: id_area,
        action: HitAction::FirstRunClientId,
    });

    // 3. Client Secret input box
    let sec_area = chunks[2];
    let is_sec_active = state.active_field == FirstRunField::ClientSecretInput;
    let sec_border_color = if is_sec_active { theme.accent } else { theme.border };

    let sec_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.border_type())
        .border_style(Style::default().fg(sec_border_color))
        .title(Span::styled(" Google Client Secret ", Style::default().fg(if is_sec_active { theme.accent } else { theme.text_muted }).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(theme.card_bg));

    let sec_inner = sec_block.inner(sec_area);
    f.render_widget(sec_block, sec_area);

    let sec_disp = if state.client_secret.is_empty() {
        if is_sec_active {
            Line::from(Span::styled("_ (paste with Ctrl+V)", Style::default().fg(theme.accent)))
        } else {
            Line::from(Span::styled("(paste your Client Secret here)", Style::default().fg(theme.text_muted).add_modifier(Modifier::ITALIC)))
        }
    } else {
        let max_w = sec_inner.width as usize;
        let text = if is_sec_active {
            crate::ui::settings::format_scrolled_input_with_cursor(&state.client_secret, state.client_secret_cursor, max_w)
        } else {
            "•".repeat(state.client_secret.chars().count())
        };
        Line::from(Span::styled(text, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)))
    };
    f.render_widget(Paragraph::new(sec_disp), sec_inner);

    hitboxes.push(Hitbox {
        rect: sec_area,
        action: HitAction::FirstRunClientSecret,
    });

    // 4. Action buttons (compact & responsive)
    let btn_area = chunks[3];
    let is_save_focused = state.active_field == FirstRunField::SaveCredentialsButton;
    let is_skip_focused = state.active_field == FirstRunField::SkipCredentialsButton;
    let is_help_focused = state.active_field == FirstRunField::ToggleHelpButton;

    let save_style = if is_save_focused {
        Style::default().fg(Color::Black).bg(theme.green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text_bright).bg(theme.border).add_modifier(Modifier::BOLD)
    };

    let skip_style = if is_skip_focused {
        Style::default().fg(Color::Black).bg(theme.yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text_muted).bg(theme.border)
    };

    let help_style = if is_help_focused {
        Style::default().fg(Color::Black).bg(theme.cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.cyan).bg(theme.border)
    };

    let btn_save = " [ Save & Complete Setup (Enter) ] ";
    let btn_skip = " [ Skip for now (Esc) ] ";
    let btn_help = if state.show_help { " [ Hide Guide (?/h) ] " } else { " [ Show Guide (?/h) ] " };

    let save_w = btn_save.chars().count() as u16;
    let skip_w = btn_skip.chars().count() as u16;
    let help_w = btn_help.chars().count() as u16;
    let spacing: u16 = 2;

    let btn_line = Line::from(vec![
        Span::styled(btn_save, save_style),
        Span::raw("  "),
        Span::styled(btn_skip, skip_style),
        Span::raw("  "),
        Span::styled(btn_help, help_style),
    ]);

    f.render_widget(Paragraph::new(btn_line), btn_area);

    // Hitboxes for buttons with exact dynamic coordinates
    let x_save = btn_area.x;
    let x_skip = x_save + save_w + spacing;
    let x_help = x_skip + skip_w + spacing;

    hitboxes.push(Hitbox {
        rect: Rect {
            x: x_save,
            y: btn_area.y,
            width: save_w.min(btn_area.width),
            height: 1,
        },
        action: HitAction::FirstRunSaveCredentials,
    });

    if x_skip < btn_area.x + btn_area.width {
        hitboxes.push(Hitbox {
            rect: Rect {
                x: x_skip,
                y: btn_area.y,
                width: skip_w.min((btn_area.x + btn_area.width).saturating_sub(x_skip)),
                height: 1,
            },
            action: HitAction::FirstRunSkipCredentials,
        });
    }

    if x_help < btn_area.x + btn_area.width {
        hitboxes.push(Hitbox {
            rect: Rect {
                x: x_help,
                y: btn_area.y,
                width: help_w.min((btn_area.x + btn_area.width).saturating_sub(x_help)),
                height: 1,
            },
            action: HitAction::FirstRunToggleHelp,
        });
    }

    // 5. Help card (when toggled)
    if state.show_help {
        let help_area = chunks[4];
        let help_block = Block::default()
            .borders(Borders::ALL)
            .border_type(app.border_type())
            .border_style(Style::default().fg(theme.cyan))
            .title(Span::styled(" 📖 Google Cloud Console Guide ", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)))
            .style(Style::default().bg(theme.card_bg));

        let help_inner = help_block.inner(help_area);
        f.render_widget(help_block, help_area);

        let guide_lines = vec![
            Line::from("1. Open https://console.cloud.google.com and create a project."),
            Line::from("2. Go to 'APIs & Services' > 'Library', search 'Google Drive API' & click Enable."),
            Line::from("3. In 'OAuth consent screen', choose 'External' and configure basic info."),
            Line::from("4. In 'Credentials' > 'Create Credentials' > 'OAuth client ID'."),
            Line::from("5. Select 'Desktop app', copy the generated Client ID and Client Secret!"),
        ];
        f.render_widget(Paragraph::new(guide_lines).wrap(Wrap { trim: true }), help_inner);
    }
}
