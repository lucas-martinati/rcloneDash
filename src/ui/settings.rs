use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::config;
use crate::ui::container::{centered_fixed_rect, render_modal_container, ModalContainerConfig, NavArrowsConfig};
use crate::ui::theme::ThemePalette;
pub fn render_settings_modal(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let screen = f.area();
    // Dimensions partagées avec compute_active_modal_area (clic-outside/scroll).
    let dims = crate::ui::container::settings_modal_dims(screen);
    let box_w = dims.box_w;
    // 26 lines necessary and sufficient to display all options and descriptions without cutoff
    let box_h = dims.box_h;
    // Logo is only displayed if screen height allows the full modal to fit
    let show_logo = dims.show_logo;
    let logo_h = dims.logo_h;

    let total_h = dims.total_h;
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
        crate::ui::menu::render_logo(f, v_chunks[0]);
        let mut ver_spans = vec![
            Span::styled(format!("v{}", config::APP_VERSION), Style::default().fg(Color::Rgb(165, 170, 185)).add_modifier(Modifier::BOLD | Modifier::ITALIC)),
        ];
        if let Some(newer) = &app.available_update {
            ver_spans.push(Span::raw("  "));
            ver_spans.push(Span::styled(
                format!("(🚀 v{} available)", newer),
                Style::default().fg(Color::Rgb(250, 200, 50)).add_modifier(Modifier::BOLD),
            ));
        }
        let ver_line = Line::from(ver_spans);
        f.render_widget(Paragraph::new(ver_line).alignment(Alignment::Center), v_chunks[1]);
        v_chunks[2]
    } else {
        container_area
    };

    use crate::config::{SettingCategory, SettingId};

    // Settings data based on active tab
    let current_cat = SettingCategory::ALL.get(app.settings_tab).copied().unwrap_or(SettingCategory::Rclone);
    let settings: Vec<(&str, String)> = SettingId::for_category(current_cat)
        .into_iter()
        .map(|s| (s.label(), app.setting_value(s)))
        .collect();

    let total_opts = settings.len();
    let row_height: u16 = 2; // 1 label line + 1 value line
    let visible_count = (box_h.saturating_sub(4) as usize / row_height as usize).max(1);
    let total_pages = total_opts.div_ceil(visible_count);
    let cur_page = (app.settings_selected_idx / visible_count) + 1;
    let page_label = format!("page {}/{}", cur_page, total_pages.max(1));

    let can_up = total_opts > 1 && app.settings_selected_idx > 0;
    let can_down = total_opts > 1 && app.settings_selected_idx < total_opts.saturating_sub(1);

    let mid_cmd: Vec<Span> = if app.is_editing_setting() {
        let mut spans = crate::ui::keys::KeybindingRegistry::format_shortcut_label("Esc", "cancel", theme.red, Color::White);
        spans.push(Span::styled("  ", Style::default()));
        spans.extend(crate::ui::keys::KeybindingRegistry::format_shortcut_label("↵", "confirm", theme.green, Color::White));
        spans
    } else if let Some(setting) = config::SettingId::from_tab_and_idx(app.settings_tab, app.settings_selected_idx) {
        match setting.kind() {
            config::SettingKind::TextInput => {
                crate::ui::keys::KeybindingRegistry::format_shortcut_label("↵", "edit", theme.red, Color::White)
            }
            config::SettingKind::Action => {
                if setting == config::SettingId::LogJournalAction {
                    crate::ui::keys::KeybindingRegistry::format_shortcut_label("↵", "open full log", theme.cyan, Color::White)
                } else {
                    crate::ui::keys::KeybindingRegistry::format_shortcut_label("↵", "launch resync", theme.red, Color::White)
                }
            }
            config::SettingKind::Cycle => {
                vec![
                    Span::styled("←", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                    Span::styled(" change ", Style::default().fg(Color::White)),
                    Span::styled("→", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                ]
            }
        }
    } else {
        vec![
            Span::styled("←", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled(" change ", Style::default().fg(Color::White)),
            Span::styled("→", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        ]
    };

    let inner = render_modal_container(
        f,
        app,
        theme,
        area,
        ModalContainerConfig {
            title_prefix: "settings",
            title_color: Some(theme.red),
            title_extra: None,
            nav_arrows: Some(NavArrowsConfig {
                label: &page_label,
                up_active: can_up,
                down_active: can_down,
            }),
            action_shortcuts: Some(vec![mid_cmd]),
            counter: None,
            border_color: theme.red,
            show_close_button: true,
            ..Default::default()
        },
        hitboxes,
    );

    if inner.height < 6 || inner.width < 40 {
        return;
    }

    let tab_y = inner.y;
    let divider_y = inner.y + 1;
    let content_area = Rect {
        x: inner.x,
        y: inner.y + 2,
        width: inner.width,
        height: inner.height.saturating_sub(2),
    };

    // 1. Render tab bar row at top of inner area: tab→   [rclone]    2ui (btop++ style)
    let mut tab_spans = vec![
        Span::raw(" "),
        Span::styled("tab→", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        Span::raw("   "),
    ];
    let mut tab_x = inner.x + 8;
    for (i, cat) in config::SettingCategory::ALL.iter().enumerate() {
        if i > 0 {
            tab_spans.push(Span::raw("    "));
            tab_x += 4;
        }
        let is_active = i == app.settings_tab;
        let short_name = cat.short_name();
        let num = superscript_digit(i + 1);
        let tab_len = if is_active {
            tab_spans.push(Span::styled("[", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
            tab_spans.push(Span::styled(num, Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
            tab_spans.push(Span::styled(short_name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));
            tab_spans.push(Span::styled("]", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
            (2 + num.chars().count() + short_name.chars().count()) as u16
        } else {
            tab_spans.push(Span::styled(num, Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
            tab_spans.push(Span::styled(short_name, Style::default().fg(Color::Rgb(220, 222, 230))));
            (num.chars().count() + short_name.chars().count()) as u16
        };

        hitboxes.push(Hitbox {
            rect: Rect {
                x: tab_x,
                y: tab_y,
                width: tab_len,
                height: 1,
            },
            action: HitAction::SettingsTab(i),
        });
        tab_x += tab_len;
    }

    // Hitbox for "tab→" prefix to cycle tabs
    hitboxes.push(Hitbox {
        rect: Rect {
            x: inner.x + 1,
            y: tab_y,
            width: 5,
            height: 1,
        },
        action: HitAction::SettingsTab((app.settings_tab + 1) % config::SettingCategory::ALL.len()),
    });

    f.render_widget(Paragraph::new(Line::from(tab_spans)), Rect { x: inner.x, y: tab_y, width: inner.width, height: 1 });

    // 2. Direct horizontal split for the two columns under the divider
    let left_col_w = 34u16.min(content_area.width.saturating_sub(24));
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(left_col_w),
            Constraint::Length(1),
            Constraint::Min(20),
        ])
        .split(content_area);

    let left_area = cols[0];
    let sep_area = cols[1];
    let right_area = cols[2];

    // 3. Render horizontal divider line: ├──────────┬──────────┤ (btop++ style)
    let (h_char, v_char, cross_top, cross_bot, cross_left, cross_right) = match app.config.border_style {
        config::BorderStyleChoice::Double => ("═", "║", "╦", "╩", "╠", "╣"),
        config::BorderStyleChoice::Thick => ("━", "┃", "┳", "┻", "┣", "┫"),
        _ => ("─", "│", "┬", "┴", "├", "┤"),
    };

    // Left junction on outer box border
    f.render_widget(
        Paragraph::new(cross_left).style(Style::default().fg(theme.red)),
        Rect { x: area.x, y: divider_y, width: 1, height: 1 },
    );

    // Horizontal divider inside inner with T-junction at column separator
    let left_w = (sep_area.x.saturating_sub(inner.x)) as usize;
    let right_w = ((inner.x + inner.width).saturating_sub(sep_area.x + 1)) as usize;
    let div_line = Line::from(vec![
        Span::styled(h_char.repeat(left_w), Style::default().fg(theme.red)),
        Span::styled(cross_top, Style::default().fg(theme.red)),
        Span::styled(h_char.repeat(right_w), Style::default().fg(theme.red)),
    ]);
    f.render_widget(Paragraph::new(div_line), Rect { x: inner.x, y: divider_y, width: inner.width, height: 1 });

    // Right junction on outer box border
    f.render_widget(
        Paragraph::new(cross_right).style(Style::default().fg(theme.red)),
        Rect { x: area.x + area.width.saturating_sub(1), y: divider_y, width: 1, height: 1 },
    );

    // Bottom T-junction on bottom border
    f.render_widget(
        Paragraph::new(cross_bot).style(Style::default().fg(theme.red)),
        Rect { x: sep_area.x, y: area.y + area.height.saturating_sub(1), width: 1, height: 1 },
    );

    // Vertical separator down the middle
    let sep_lines: Vec<Line> = (0..content_area.height)
        .map(|_| Line::from(Span::styled(v_char, Style::default().fg(theme.red))))
        .collect();
    f.render_widget(Paragraph::new(sep_lines), sep_area);

    // Render left column with smooth scrolling
    let mut left_lines = Vec::new();
    let scroll_offset = if app.settings_selected_idx >= visible_count {
        app.settings_selected_idx - visible_count + 1
    } else {
        0
    };

    let highlight_bg = Color::Rgb(95, 30, 30); // btop++ dark red / maroon banner

    for (visible_pos, i) in (scroll_offset..settings.len()).take(visible_count).enumerate() {
        let (label, val) = &settings[i];
        let is_selected = i == app.settings_selected_idx;
        let item_y = left_area.y + (visible_pos as u16 * row_height);

        // Hitbox for selecting the entire row
        hitboxes.push(Hitbox {
            rect: Rect {
                x: left_area.x,
                y: item_y,
                width: left_area.width,
                height: row_height,
            },
            action: HitAction::SettingOption(i),
        });

        // Hitboxes for ← and → arrows
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
        let setting_opt = SettingId::from_tab_and_idx(app.settings_tab, i);
        let setting_kind = setting_opt.map(|s| s.kind()).unwrap_or(config::SettingKind::Cycle);

        if is_selected {
            // Choice counter (e.g. " 1/8") for cycle options in btop++ style
            let cur_choice_info = if let Some(choices) = setting_opt.and_then(|s| s.choices()) {
                if !choices.is_empty() {
                    let norm_cur = val.trim().to_lowercase();
                    let pos = choices.iter().position(|c| {
                        let norm_c = c.trim().to_lowercase();
                        norm_c == norm_cur || (c == "Disabled" && (norm_cur.is_empty() || norm_cur == "disabled"))
                    }).unwrap_or(0);
                    format!(" {}/{}", pos + 1, choices.len())
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let label_with_count = format!("{}{}", label, cur_choice_info);
            let line1 = Line::from(vec![
                Span::styled(format!("{:^width$}", label_with_count, width = w), Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
            ]);

            let inner_w = w.saturating_sub(4);
            let line2 = if app.is_editing_setting() && setting_kind == config::SettingKind::TextInput {
                let buf = app.edit_buffer();
                let visible = format_scrolled_input_with_cursor(buf, app.edit_cursor(), inner_w);
                let edit_centered = format!("{:^width$}", visible, width = inner_w);
                Line::from(vec![
                    Span::styled("[ ", Style::default().fg(Color::Yellow).bg(Color::Rgb(70, 20, 20)).add_modifier(Modifier::BOLD)),
                    Span::styled(edit_centered, Style::default().fg(Color::Yellow).bg(Color::Rgb(70, 20, 20)).add_modifier(Modifier::BOLD)),
                    Span::styled(" ]", Style::default().fg(Color::Yellow).bg(Color::Rgb(70, 20, 20)).add_modifier(Modifier::BOLD)),
                ])
            } else {
                let truncated = truncate_chars(val, inner_w);
                let val_centered = format!("{:^width$}", truncated, width = inner_w);
                match setting_kind {
                    config::SettingKind::Cycle => {
                        Line::from(vec![
                            Span::styled("←", Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                            Span::styled(" ", Style::default().bg(highlight_bg)),
                            Span::styled(val_centered, Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                            Span::styled(" ", Style::default().bg(highlight_bg)),
                            Span::styled("→", Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                        ])
                    }
                    config::SettingKind::TextInput => {
                        Line::from(vec![
                            Span::styled("  ", Style::default().bg(highlight_bg)),
                            Span::styled(val_centered, Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                            Span::styled("  ", Style::default().bg(highlight_bg)),
                        ])
                    }
                    config::SettingKind::Action => {
                        Line::from(vec![
                            Span::styled("↵ ", Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                            Span::styled(val_centered, Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                            Span::styled(" ↵", Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                        ])
                    }
                }
            };

            left_lines.push(line1);
            left_lines.push(line2);
        } else {
            let truncated = truncate_chars(val, w);
            let line1 = Line::from(vec![
                Span::styled(format!("{:^width$}", label, width = w), Style::default().fg(Color::White)),
            ]);
            let line2 = Line::from(vec![
                Span::styled(format!("{:^width$}", truncated, width = w), Style::default().fg(Color::Rgb(165, 170, 185))),
            ]);
            left_lines.push(line1);
            left_lines.push(line2);
        }
    }

    let left_p = Paragraph::new(left_lines);
    f.render_widget(left_p, left_area);

    // Render right column: Description panel
    use crate::ui::keys::{KeyAction, KeybindingRegistry};
    let k_files = KeybindingRegistry::get_key_str(KeyAction::Files);
    let k_dec = KeybindingRegistry::get_key_str(KeyAction::DecTickRate);
    let k_inc = KeybindingRegistry::get_key_str(KeyAction::IncTickRate);
    let k_enter = KeybindingRegistry::get_key_str(KeyAction::Validate);
    let k_esc = KeybindingRegistry::get_key_str(KeyAction::CancelEdit);

    let (desc_title, desc_body): (&str, String) = match SettingId::from_tab_and_idx(app.settings_tab, app.settings_selected_idx) {
        Some(setting) if setting.is_cycle() => {
            let choices = setting.choices().unwrap_or_default();
            let current = app.setting_value(setting);
            let header = if setting == SettingId::BandwidthLimit {
                "Available presets:"
            } else {
                "Available options:"
            };
            let extra_hint = if setting == SettingId::StatsInterval {
                format!("\nDirectly synchronized with keys [{}] and [{}] or clicking the frequency widget.", k_dec, k_inc)
            } else {
                String::new()
            };
            (
                setting.desc_title(),
                format!(
                    "{}{}\n\n{}\n{}",
                    setting.desc_intro(),
                    extra_hint,
                    header,
                    config::format_setting_options_list(&choices, &current)
                ),
            )
        }
        Some(SettingId::LocalDirectory) => (
            SettingId::LocalDirectory.desc_title(),
            format!(
                "{}\nOpen the file explorer ([{}]) to inspect its tree structure.\n\nPress [{}] to edit the path, then [{}] to confirm or [{}] to cancel.\n\nCurrent path:\n  ▶ {} (active)",
                SettingId::LocalDirectory.desc_intro(),
                k_files, k_enter, k_enter, k_esc, app.config.local_dir
            ),
        ),
        Some(SettingId::RemoteStorage) => (
            SettingId::RemoteStorage.desc_title(),
            format!(
                "{}\n\nPress [{}] to edit the name, then [{}] to confirm or [{}] to cancel.\n\nCurrent remote:\n  ▶ {} (active)",
                SettingId::RemoteStorage.desc_intro(),
                k_enter, k_enter, k_esc, app.config.remote
            ),
        ),
        Some(SettingId::ResyncAction) => (
            SettingId::ResyncAction.desc_title(),
            format!(
                "{}\n\nPress [{}] to open the confirmation dialog.",
                SettingId::ResyncAction.desc_intro(),
                k_enter
            ),
        ),
        Some(SettingId::LogJournalAction) => (
            SettingId::LogJournalAction.desc_title(),
            format!(
                "{}\n\nPress [{}] to open the log file.",
                SettingId::LogJournalAction.desc_intro(),
                k_enter
            ),
        ),
        Some(SettingId::GoogleClientId) => {
            let (google_id, _) = config::read_rclone_credentials(&app.config.remote);
            let cur = google_id.as_deref().unwrap_or("(default / unset)");
            (
                SettingId::GoogleClientId.desc_title(),
                format!(
                    "{}\n\nPress [{}] to edit, then [{}] to confirm or [{}] to cancel.\nPress [Ctrl+V] to paste from clipboard.\n\nCurrent Client ID:\n  ▶ {}",
                    SettingId::GoogleClientId.desc_intro(),
                    k_enter, k_enter, k_esc, cur
                ),
            )
        }
        Some(SettingId::GoogleClientSecret) => {
            let (_, google_secret) = config::read_rclone_credentials(&app.config.remote);
            let cur_display = if google_secret.is_some() { "•••••••••••• (configured)" } else { "(default / unset)" };
            (
                SettingId::GoogleClientSecret.desc_title(),
                format!(
                    "{}\n\nPress [{}] to edit, then [{}] to confirm or [{}] to cancel.\nPress [Ctrl+V] to paste from clipboard.\n\nCurrent Client Secret:\n  ▶ {}",
                    SettingId::GoogleClientSecret.desc_intro(),
                    k_enter, k_enter, k_esc, cur_display
                ),
            )
        }
        _ => ("Description.", "Select a setting to view its detailed documentation.".to_string()),
    };

    let desc_inner = Rect {
        x: right_area.x + 2,
        y: right_area.y,
        width: right_area.width.saturating_sub(4),
        height: right_area.height,
    };

    let formatted_title = if desc_title.ends_with('.') {
        desc_title.to_string()
    } else {
        format!("{}.", desc_title)
    };

    let mut desc_lines = vec![
        Line::from(Span::styled(formatted_title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
        Line::from(""),
    ];

    for para in desc_body.split('\n') {
        if para.is_empty() {
            desc_lines.push(Line::from(""));
        } else if para.starts_with("  ▶ ") || para.starts_with("  • ") {
            desc_lines.push(parse_option_line(para, theme));
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

    // Smart auto-scroll if display height is heavily constrained
    let scroll_y = if desc_lines.len() > desc_inner.height as usize {
        let active_line_idx = desc_lines.iter().position(|l| {
            l.spans.iter().any(|s| s.content.contains('▶') || s.content.contains("(active)"))
        }).unwrap_or(0);

        let max_scroll = (desc_lines.len() - desc_inner.height as usize) as u16;
        if active_line_idx as u16 >= desc_inner.height {
            (active_line_idx as u16 - desc_inner.height + 1).min(max_scroll)
        } else {
            0
        }
    } else {
        0
    };

    let right_p = Paragraph::new(desc_lines).wrap(Wrap { trim: true }).scroll((scroll_y, 0));
    f.render_widget(right_p, desc_inner);
}

/// Parses and styles an options line (1 or multiple columns) preserving alignment and colors.
fn parse_option_line(line: &str, theme: &ThemePalette) -> Line<'static> {
    let mut spans = Vec::new();
    let mut rem = line;

    while !rem.is_empty() {
        let next_bullet = rem.match_indices("▶ ")
            .chain(rem.match_indices("• "))
            .min_by_key(|&(idx, _)| idx);

        if let Some((pos, bullet_str)) = next_bullet {
            if pos > 0 {
                spans.push(Span::raw(rem[..pos].to_string()));
            }
            let is_active = bullet_str == "▶ ";
            let bullet_style = if is_active {
                Style::default().fg(theme.green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_muted)
            };
            spans.push(Span::styled(bullet_str.to_string(), bullet_style));

            rem = &rem[pos + bullet_str.len()..];

            let next_end = rem.match_indices("▶ ")
                .chain(rem.match_indices("• "))
                .min_by_key(|&(idx, _)| idx)
                .map(|(idx, _)| idx)
                .unwrap_or(rem.len());

            let item_part = &rem[..next_end];
            rem = &rem[next_end..];

            if is_active {
                if let Some((name, after)) = item_part.split_once(" (active)") {
                    spans.push(Span::styled(name.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));
                    spans.push(Span::styled(" (active)".to_string(), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)));
                    if !after.is_empty() {
                        spans.push(Span::raw(after.to_string()));
                    }
                } else {
                    let trimmed = item_part.trim_end();
                    let spaces = &item_part[trimmed.len()..];
                    spans.push(Span::styled(trimmed.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));
                    if !spaces.is_empty() {
                        spans.push(Span::raw(spaces.to_string()));
                    }
                }
            } else {
                let trimmed = item_part.trim_end();
                let spaces = &item_part[trimmed.len()..];
                spans.push(Span::styled(trimmed.to_string(), Style::default().fg(Color::Rgb(185, 190, 205))));
                if !spaces.is_empty() {
                    spans.push(Span::raw(spaces.to_string()));
                }
            }
        } else {
            spans.push(Span::raw(rem.to_string()));
            break;
        }
    }

    Line::from(spans)
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count > max_chars {
        let prefix: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}…", prefix)
    } else {
        s.to_string()
    }
}


pub fn format_scrolled_input_with_cursor(buf: &str, cursor_pos: usize, max_w: usize) -> String {
    let chars: Vec<char> = buf.chars().collect();
    let cursor_pos = cursor_pos.min(chars.len());
    let mut with_cursor: Vec<char> = Vec::with_capacity(chars.len() + 1);
    for (i, &c) in chars.iter().enumerate() {
        if i == cursor_pos {
            with_cursor.push('_');
        }
        with_cursor.push(c);
    }
    if cursor_pos == chars.len() {
        with_cursor.push('_');
    }

    let total = with_cursor.len();
    if total <= max_w {
        with_cursor.into_iter().collect()
    } else if max_w <= 3 {
        with_cursor.into_iter().take(max_w).collect()
    } else {
        let half = (max_w.saturating_sub(2)) / 2;
        let start = if cursor_pos <= half {
            0
        } else if cursor_pos + half >= total {
            total.saturating_sub(max_w.saturating_sub(1))
        } else {
            cursor_pos - half
        };
        let end = (start + max_w.saturating_sub(2)).min(total);

        let mut res = String::new();
        if start > 0 {
            res.push('…');
        }
        for &ch in &with_cursor[start..end] {
            res.push(ch);
        }
        if end < total {
            res.push('…');
        }
        res
    }
}

fn superscript_digit(n: usize) -> &'static str {
    match n {
        1 => "¹",
        2 => "²",
        3 => "³",
        4 => "⁴",
        5 => "⁵",
        6 => "⁶",
        7 => "⁷",
        8 => "⁸",
        9 => "⁹",
        _ => "",
    }
}


