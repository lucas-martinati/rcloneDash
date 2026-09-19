use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::ui::theme::ThemePalette;

#[allow(dead_code)]
pub const SETTINGS_ITEMS_COUNT: usize = 7;

pub fn render_settings_modal(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let screen = f.area();
    let is_wide = screen.width >= 88;
    let logo_h: u16 = if is_wide { 6 } else { 5 };
    let show_logo = screen.height >= 32;
    let box_w = if screen.width >= 96 {
        88.min(screen.width.saturating_sub(4))
    } else if screen.width >= 86 {
        82.min(screen.width.saturating_sub(2))
    } else {
        76.min(screen.width.saturating_sub(2))
    };
    let box_h = if show_logo {
        23.min(screen.height.saturating_sub(logo_h + 3))
    } else {
        23.min(screen.height.saturating_sub(2))
    };

    let total_h = if show_logo { logo_h + 1 + box_h } else { box_h };
    let container_area = centered_fixed_rect(box_w, total_h, screen);

    let area = if show_logo {
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(logo_h),
                Constraint::Length(1),
                Constraint::Length(box_h),
            ])
            .split(container_area);
        crate::ui::menu::render_btop_logo(f, v_chunks[0]);
        let ver_line = Line::from(vec![
            Span::styled("v1.0.0", Style::default().fg(Color::Rgb(165, 170, 185)).add_modifier(Modifier::BOLD | Modifier::ITALIC)),
        ]);
        f.render_widget(Paragraph::new(ver_line).alignment(Alignment::Center), v_chunks[1]);
        v_chunks[2]
    } else {
        container_area
    };

    f.render_widget(Clear, area);

    let bg = app.border_glyphs();
    let total_opts = app.settings_items_count();
    let cur_opt = (app.settings_selected_idx + 1).min(total_opts);

    let (up_col, down_col) = if app.settings_selected_idx == 0 {
        (theme.text_muted, theme.red)
    } else if app.settings_selected_idx >= total_opts.saturating_sub(1) {
        (theme.red, theme.text_muted)
    } else {
        (theme.red, theme.red)
    };

    let mid_cmd = if app.is_editing_setting {
        Span::styled("Esc cancel  ↵ confirm", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))
    } else if app.settings_tab == 0 {
        if app.settings_selected_idx == 6 {
            Span::styled("↵ open full log", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD))
        } else if app.settings_selected_idx == 5 {
            Span::styled("↵ launch resync", Style::default().fg(theme.red).add_modifier(Modifier::BOLD))
        } else if app.settings_selected_idx == 3 || app.settings_selected_idx == 4 {
            Span::styled("↵ edit", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("← change →", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
        }
    } else {
        Span::styled("← change →", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
    };

    // Tab bar title in outer block: tab→   [1 rclone]    2ui
    let tab0_span = if app.settings_tab == 0 {
        Span::styled("[1 rclone]", Style::default().fg(theme.red).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" 1rclone ", Style::default().fg(theme.text_muted))
    };
    let tab1_span = if app.settings_tab == 1 {
        Span::styled("[2 ui]", Style::default().fg(theme.red).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" 2ui ", Style::default().fg(theme.text_muted))
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.border_type())
        .border_style(Style::default().fg(theme.red))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled(bg.top_left, Style::default().fg(theme.red)),
            Span::styled("settings", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}{}", bg.top_right, bg.top_left), Style::default().fg(theme.red)),
            Span::styled("Tab", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled(" ⇆ ", Style::default().fg(Color::White)),
            tab0_span,
            Span::styled("   ", Style::default().fg(theme.border)),
            tab1_span,
            Span::styled(bg.top_right, Style::default().fg(theme.red)),
        ]))
        .title(
            Line::from(vec![
                Span::styled(bg.top_left, Style::default().fg(theme.red)),
                Span::styled("Esc, q", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" close", Style::default().fg(theme.text_bright)),
                Span::styled(bg.top_right, Style::default().fg(theme.red)),
            ])
            .alignment(Alignment::Right),
        )
        .title_bottom(
            Line::from(vec![
                Span::styled(bg.bot_left, Style::default().fg(theme.red)),
                Span::styled("↑", Style::default().fg(up_col).add_modifier(Modifier::BOLD)),
                Span::styled(" select ", Style::default().fg(Color::White)),
                Span::styled("↓", Style::default().fg(down_col).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{}{}", bg.bot_right, bg.bot_left), Style::default().fg(theme.red)),
                mid_cmd,
                Span::styled(bg.bot_right, Style::default().fg(theme.red)),
            ])
            .alignment(Alignment::Left),
        )
        .title_bottom(
            Line::from(vec![
                Span::styled(format!("{} {}/{} {}", bg.horizontal, cur_opt, total_opts, bg.horizontal), Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            ])
            .alignment(Alignment::Right),
        );
    f.render_widget(outer_block, area);

    // Hitbox for "Tab ⇆" (après "settings ┐┌" -> area.x + 12)
    hitboxes.push(Hitbox {
        rect: Rect {
            x: area.x + 12,
            y: area.y,
            width: 6,
            height: 1,
        },
        action: HitAction::SettingsTab((app.settings_tab + 1) % 2),
    });
    // Hitbox for Tab 0: "[1 rclone]" -> start x = area.x + 18, len = 10
    hitboxes.push(Hitbox {
        rect: Rect {
            x: area.x + 18,
            y: area.y,
            width: 10,
            height: 1,
        },
        action: HitAction::SettingsTab(0),
    });
    // Hitbox for Tab 1: "[2 ui]" -> start x = area.x + 31, len = 6
    hitboxes.push(Hitbox {
        rect: Rect {
            x: area.x + 31,
            y: area.y,
            width: 6,
            height: 1,
        },
        action: HitAction::SettingsTab(1),
    });

    hitboxes.push(Hitbox {
        rect: Rect {
            x: area.x + area.width.saturating_sub(14),
            y: area.y,
            width: 12,
            height: 1,
        },
        action: HitAction::CloseModal,
    });

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    if inner.height < 4 || inner.width < 40 {
        return;
    }

    // Découpage horizontal direct pour les deux colonnes
    let left_col_w = 30u16.min(inner.width.saturating_sub(20));
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(left_col_w),
            Constraint::Length(1),
            Constraint::Min(20),
        ])
        .split(inner);

    let left_area = cols[0];
    let sep_area = cols[1];
    let right_area = cols[2];

    // Séparateur vertical btop++
    let sep_lines: Vec<Line> = (0..inner.height)
        .map(|_| Line::from(Span::styled("│", Style::default().fg(theme.border))))
        .collect();
    f.render_widget(Paragraph::new(sep_lines), sep_area);

    // Données des réglages selon l'onglet actif
    let full_sync_display = match app.config.full_sync_interval.as_str() {
        "60" => "1h (Recommended)".to_string(),
        "120" => "2h".to_string(),
        "240" => "4h".to_string(),
        "360" => "6h".to_string(),
        "720" => "12h".to_string(),
        "1440" => "24h (1 day)".to_string(),
        "never" => "Never (Local)".to_string(),
        other => other.to_string(),
    };

    let settings: Vec<(&str, String)> = if app.settings_tab == 0 {
        vec![
            ("Bisync timer interval", app.config.timer_interval.clone()),
            ("Cloud safety net", full_sync_display),
            ("Bandwidth limit", app.config.bwlimit.as_deref().unwrap_or("Disabled").to_string()),
            ("Local directory", app.config.local_dir.clone()),
            ("Remote storage", app.config.remote.clone()),
            ("Full resynchronization", "Run (--resync)".to_string()),
            ("Full rclone log journal", "Open logs (↵)".to_string()),
        ]
    } else {
        vec![
            ("Color theme", app.current_theme.name().to_string()),
            ("Container layout", app.config.container_layout.name().to_string()),
            ("Mid-panel order", app.config.mid_panel_order.name().to_string()),
            ("Border style", app.config.border_style.name().to_string()),
            ("Graph style", app.config.graph_style.name().to_string()),
            ("UI Loop frequency", format!("{} ms", app.tick_rate_ms_live)),
        ]
    };

    // Rendu de la colonne gauche avec défilement fluide
    let mut left_lines = Vec::new();
    let row_height: u16 = 2; // 1 ligne label + 1 ligne valeur
    let visible_count = (left_area.height as usize / row_height as usize).max(1);
    let scroll_offset = if app.settings_selected_idx >= visible_count {
        app.settings_selected_idx - visible_count + 1
    } else {
        0
    };

    for (visible_pos, i) in (scroll_offset..settings.len()).take(visible_count).enumerate() {
        let (label, val) = &settings[i];
        let is_selected = i == app.settings_selected_idx;
        let item_y = left_area.y + (visible_pos as u16 * row_height);

        // Hitbox de sélection de la ligne entière
        hitboxes.push(Hitbox {
            rect: Rect {
                x: left_area.x,
                y: item_y,
                width: left_area.width,
                height: row_height,
            },
            action: HitAction::SettingOption(i),
        });

        // Hitboxes pour les flèches ← et →
        let arrow_y = item_y + 1;
        hitboxes.push(Hitbox {
            rect: Rect { x: left_area.x, y: arrow_y, width: 4, height: 1 },
            action: HitAction::SettingCycle(i, false),
        });
        hitboxes.push(Hitbox {
            rect: Rect { x: left_area.x + left_area.width.saturating_sub(4), y: arrow_y, width: 4, height: 1 },
            action: HitAction::SettingCycle(i, true),
        });

        let w = left_area.width as usize;

        if is_selected {
            // Bandeau de fond coloré btop++ (brun/rouge profond #5A2222)
            let highlight_bg = Color::Rgb(90, 32, 32);

            let line1 = Line::from(vec![
                Span::styled(format!("{:^width$}", label, width = w), Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
            ]);

            let inner_w = w.saturating_sub(4);
            let line2 = if app.settings_tab == 0 {
                if app.is_editing_setting && (i == 3 || i == 4) {
                    let edit_text = format!("{}_", app.setting_edit_buffer);
                    let edit_centered = format!("{:^width$}", edit_text, width = inner_w);
                    Line::from(vec![
                        Span::styled(format!("[{}]", edit_centered), Style::default().fg(Color::Yellow).bg(Color::Rgb(70, 20, 20)).add_modifier(Modifier::BOLD)),
                    ])
                } else if i == 5 || i == 6 {
                    let val_centered = format!("{:^width$}", val, width = inner_w);
                    Line::from(vec![
                        Span::styled(format!("↵ {} ↵", val_centered), Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                    ])
                } else if i == 3 || i == 4 {
                    let val_centered = format!("{:^width$}", val, width = inner_w);
                    Line::from(vec![
                        Span::styled(format!("↵ {} ↵", val_centered), Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                    ])
                } else {
                    let val_centered = format!("{:^width$}", val, width = inner_w);
                    Line::from(vec![
                        Span::styled(format!("← {} →", val_centered), Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                    ])
                }
            } else {
                let val_centered = format!("{:^width$}", val, width = inner_w);
                Line::from(vec![
                    Span::styled(format!("← {} →", val_centered), Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                ])
            };

            left_lines.push(line1);
            left_lines.push(line2);
        } else {
            let line1 = Line::from(vec![
                Span::styled(format!("{:^width$}", label, width = w), Style::default().fg(Color::Rgb(220, 222, 230))),
            ]);
            let line2 = Line::from(vec![
                Span::styled(format!("{:^width$}", val, width = w), Style::default().fg(Color::Rgb(165, 170, 185))),
            ]);
            left_lines.push(line1);
            left_lines.push(line2);
        }
    }

    let left_p = Paragraph::new(left_lines);
    f.render_widget(left_p, left_area);

    // Rendu de la colonne droite : Panneau de description style btop++
    use crate::ui::keys::{KeyAction, KeybindingRegistry};
    let k_files = KeybindingRegistry::get_key_str(KeyAction::Files);
    let k_dec = KeybindingRegistry::get_key_str(KeyAction::DecTickRate);
    let k_inc = KeybindingRegistry::get_key_str(KeyAction::IncTickRate);
    let k_enter = KeybindingRegistry::get_key_str(KeyAction::Validate);
    let k_esc = KeybindingRegistry::get_key_str(KeyAction::CancelEdit);

    let (desc_title, desc_body): (&str, String) = match (app.settings_tab, app.settings_selected_idx) {
        // --- Catégorie 0 : Rclone ---
        (0, 0) => (
            "Bisync timer interval.",
            "Frequency of automatic checks and synchronization managed by systemd.\n\nConfigures how often rclone-bisync.timer wakes up to inspect changes.\n\nAvailable values: 10m, 15m, 30m, 1h, 2h, 4h.\nRecommended: 15m for an ideal balance between responsiveness and CPU usage.".to_string(),
        ),
        (0, 1) => (
            "Cloud safety net.",
            "Maximum time elapsed before running a full bidirectional sync.\n\nEven if no local changes were detected, this safety net ensures files created or updated remotely from another computer or the web interface are retrieved.\n\n'Never' option is available if you only want sync triggered upon local changes.".to_string(),
        ),
        (0, 2) => (
            "Bandwidth limit (bwlimit).",
            "Maximum allowed transfer speed for rclone.\n\nPreserves your internet connection by limiting network bandwidth used by rclone.\n\nSetting stored in bwlimit.env and injected into the systemd service.\nValue 'Disabled' uses 100% of available bandwidth.".to_string(),
        ),
        (0, 3) => (
            "Monitored local directory.",
            format!(
                "Path to the root local directory synchronized with cloud storage.\n\nContains your local data replicated by bisync.\nOpen the file explorer ([{}]) to inspect its tree structure.\n\nPress [{}] to edit the path, then [{}] to confirm or [{}] to cancel.",
                k_files, k_enter, k_enter, k_esc
            ),
        ),
        (0, 4) => (
            "Remote cloud storage.",
            format!(
                "Remote storage name configured in ~/.config/rclone/rclone.conf.\n\nUsed for cloud quota inquiries, remote listings, and bidirectional synchronization.\n\nPress [{}] to edit the name, then [{}] to confirm or [{}] to cancel.",
                k_enter, k_enter, k_esc
            ),
        ),
        (0, 5) => (
            "Full resynchronization (--resync).",
            format!(
                "In case of critical bisync errors or corrupted sync listings, this action rebuilds listing databases by comparing the local directory and Google Drive (keeping the newest files: --resync-mode newer).\n\nPress [{}] to open the confirmation dialog.",
                k_enter
            ),
        ),
        (0, 6) => (
            "Full rclone log journal (rclone-bisync).",
            format!(
                "Opens the complete rclone-bisync systemd journal log in your external viewer (less or configured editor).\n\nAllows navigating the full history, searching text, and inspecting detailed file transfers.\n\nPress [{}] to open the log file.",
                k_enter
            ),
        ),

        // --- Catégorie 1 : UI & Apparence ---
        (1, 0) => (
            "Color theme.",
            "Sets the color theme applied across the entire dashboard.\n\nSupports 6 curated themes for optimal readability:\n• Tokyo Night\n• Catppuccin Mocha\n• Nord Frost\n• Gruvbox Dark\n• Dracula\n• Monokai Pro\n\nEach theme dynamically adapts borders, text, and btop++ gradient charts.".to_string(),
        ),
        (1, 1) => (
            "Container layout.",
            "Reorder the main dashboard containers to match your preferred workflow.\n\nAvailable presets:\n• Default: Storage & Metrics → History & Logs → Recent Files\n• Recent First: Storage & Metrics → Recent Files → History & Logs\n• Logs on Top: History & Logs → Storage & Metrics → Recent Files\n• Inverted: Recent Files → History & Logs → Storage & Metrics\n\nApplies instantly across the entire dashboard.".to_string(),
        ),
        (1, 2) => (
            "Mid-panel order.",
            "Horizontal placement of the middle section containers.\n\nAvailable configurations:\n• History | Logs: Synchronization history on the left, live journal on the right.\n• Logs | History: Live journal on the left, synchronization history on the right.\n\nAll keyboard shortcuts and mouse interactions adapt automatically.".to_string(),
        ),
        (1, 3) => (
            "Border style.",
            "Customize the box-drawing character style for all cards, panels, and modal dialogs.\n\nAvailable styles:\n• Rounded: Modern curved corners (╭───╮)\n• Sharp: Classic crisp square corners (┌───┐)\n• Double: Retro dual-line borders (╔═══╗)\n• Thick: Bold high-contrast borders (┏━━━┓)\n\nPersisted across sessions in dash-config.json.".to_string(),
        ),
        (1, 4) => (
            "Graph style.",
            "Select the glyph set used to render multiline activity and speed sparkline charts.\n\nAvailable styles:\n• Braille (Default): Dense 8-dot braille patterns (⡀⣀⣄⣤⣦⣶⣷⣿)\n• Blocks: Unicode block elements ( ▂▃▄▅▆▇█)\n\nPersisted across sessions in dash-config.json.".to_string(),
        ),
        (1, 5) => (
            "UI Loop frequency (tick rate).",
            format!(
                "Refresh rate of the TUI display engine in milliseconds.\n\nControls smoothness of log scrolling, metrics calculation, and micro-animations.\n\nDirectly synchronized with keys [{}] and [{}] or clicking the frequency widget.",
                k_dec, k_inc
            ),
        ),
        _ => ("Description.", "Select a setting to view its detailed documentation.".to_string()),
    };

    let desc_inner = Rect {
        x: right_area.x + 2,
        y: right_area.y,
        width: right_area.width.saturating_sub(4),
        height: right_area.height,
    };

    let mut desc_lines = vec![
        Line::from(Span::styled(desc_title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
        Line::from(""),
    ];

    for para in desc_body.split('\n') {
        if para.is_empty() {
            desc_lines.push(Line::from(""));
        } else {
            let mut spans = Vec::new();
            let mut rem = para;
            while let Some(start) = rem.find('[') {
                if let Some(end) = rem[start..].find(']') {
                    let end_pos = start + end;
                    if start > 0 {
                        spans.push(Span::styled(&rem[..start], Style::default().fg(Color::Rgb(185, 190, 205))));
                    }
                    let key_text = &rem[start + 1..end_pos];
                    spans.extend(crate::ui::keys::KeybindingRegistry::format_key_badge(key_text, theme));
                    rem = &rem[end_pos + 1..];
                } else {
                    break;
                }
            }
            if !rem.is_empty() {
                spans.push(Span::styled(rem, Style::default().fg(Color::Rgb(185, 190, 205))));
            }
            desc_lines.push(Line::from(spans));
        }
    }

    let right_p = Paragraph::new(desc_lines).wrap(Wrap { trim: true });
    f.render_widget(right_p, desc_inner);
}

fn centered_fixed_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}
