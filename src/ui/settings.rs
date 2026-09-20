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
    let is_wide = screen.width >= 88;
    let logo_h: u16 = if is_wide { 6 } else { 5 };
    // Logo is only displayed if screen height allows the full modal to fit
    let show_logo = screen.height >= 35;
    let box_w = if screen.width >= 96 {
        88.min(screen.width.saturating_sub(4))
    } else if screen.width >= 86 {
        82.min(screen.width.saturating_sub(2))
    } else {
        76.min(screen.width.saturating_sub(2))
    };

    // Dynamic height calculation to prevent any overflow
    let max_avail_h = if show_logo {
        screen.height.saturating_sub(logo_h + 3)
    } else {
        screen.height.saturating_sub(2)
    };
    // 26 lines necessary and sufficient to display all options and descriptions without cutoff
    let box_h = 26u16.min(max_avail_h).max(18);

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

    let bg = app.border_glyphs();
    let total_opts = app.settings_items_count();
    let cur_opt = (app.settings_selected_idx + 1).min(total_opts);

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

    // Tab bar title in outer block: tab→   [1 rclone]    2ui
    let mut title_extra = vec![
        Span::styled(format!("{}{}", bg.top_right, bg.top_left), Style::default().fg(theme.red)),
        Span::styled("Tab", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        Span::styled(" ⇆ ", Style::default().fg(Color::White)),
    ];
    for (i, cat) in config::SettingCategory::ALL.iter().enumerate() {
        if i > 0 {
            title_extra.push(Span::styled("   ", Style::default().fg(theme.border)));
        }
        let is_active = i == app.settings_tab;
        let label = format!("{}{}", i + 1, cat.short_name());
        if is_active {
            title_extra.push(Span::styled(format!("[{}]", label), Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
        } else {
            title_extra.push(Span::styled(format!(" {} ", label), Style::default().fg(theme.text_muted)));
        }
    }

    let inner = render_modal_container(
        f,
        app,
        theme,
        area,
        ModalContainerConfig {
            title_prefix: "settings",
            title_color: Some(theme.red),
            title_extra: Some(title_extra),
            nav_arrows: Some(NavArrowsConfig {
                label: "select",
                up_active: can_up,
                down_active: can_down,
            }),
            action_shortcuts: Some(vec![mid_cmd]),
            counter: Some((cur_opt, total_opts)),
            border_color: theme.red,
            show_close_button: true,
            ..Default::default()
        },
        hitboxes,
    );

    // Hitbox for "Tab ⇆" (after "settings ┐┌" -> area.x + 12)
    hitboxes.push(Hitbox {
        rect: Rect {
            x: area.x + 12,
            y: area.y,
            width: 6,
            height: 1,
        },
        action: HitAction::SettingsTab((app.settings_tab + 1) % config::SettingCategory::ALL.len()),
    });
    // Dynamic hitboxes for individual tabs
    let mut tab_x = area.x + 18;
    for (i, cat) in config::SettingCategory::ALL.iter().enumerate() {
        let label = format!("{}{}", i + 1, cat.short_name());
        let tab_len = (label.chars().count() + 2) as u16;
        hitboxes.push(Hitbox {
            rect: Rect {
                x: tab_x,
                y: area.y,
                width: tab_len,
                height: 1,
            },
            action: HitAction::SettingsTab(i),
        });
        tab_x += tab_len + 3; // +3 for spacing "   "
    }

    if inner.height < 4 || inner.width < 40 {
        return;
    }

    // Direct horizontal split for the two columns
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

    // Vertical separator
    let sep_lines: Vec<Line> = (0..inner.height)
        .map(|_| Line::from(Span::styled("│", Style::default().fg(theme.border))))
        .collect();
    f.render_widget(Paragraph::new(sep_lines), sep_area);

    use crate::config::{SettingCategory, SettingId};

    // Settings data based on active tab
    let current_cat = SettingCategory::ALL.get(app.settings_tab).copied().unwrap_or(SettingCategory::Rclone);
    let settings: Vec<(&str, String)> = SettingId::for_category(current_cat)
        .into_iter()
        .map(|s| (s.label(), app.setting_value(s)))
        .collect();

    // Render left column with smooth scrolling
    let mut left_lines = Vec::new();
    let row_height: u16 = 2; // 1 label line + 1 value line
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
            // Highlight background banner (deep red/brown #5A2222)
            let highlight_bg = Color::Rgb(90, 32, 32);

            let line1 = Line::from(vec![
                Span::styled(format!("{:^width$}", label, width = w), Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
            ]);

            let inner_w = w.saturating_sub(4);
            let line2 = if app.is_editing_setting() && setting_kind == config::SettingKind::TextInput {
                let buf = app.edit_buffer();
                let visible = format_scrolled_input_with_cursor(buf, app.edit_cursor(), inner_w);
                let edit_centered = format!("{:^width$}", visible, width = inner_w);
                Line::from(vec![
                    Span::styled(format!("[{}]", edit_centered), Style::default().fg(Color::Yellow).bg(Color::Rgb(70, 20, 20)).add_modifier(Modifier::BOLD)),
                ])
            } else {
                let truncated = truncate_chars(val, inner_w);
                let val_centered = format!("{:^width$}", truncated, width = inner_w);
                match setting_kind {
                    config::SettingKind::TextInput | config::SettingKind::Action => {
                        Line::from(vec![
                            Span::styled(format!("↵ {} ↵", val_centered), Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                        ])
                    }
                    config::SettingKind::Cycle => {
                        Line::from(vec![
                            Span::styled(format!("← {} →", val_centered), Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                        ])
                    }
                }
            };

            left_lines.push(line1);
            left_lines.push(line2);
        } else {
            let truncated = truncate_chars(val, w);
            let line1 = Line::from(vec![
                Span::styled(format!("{:^width$}", label, width = w), Style::default().fg(Color::Rgb(220, 222, 230))),
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

    let mut desc_lines = vec![
        Line::from(Span::styled(desc_title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
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


