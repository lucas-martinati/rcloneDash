use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::ui::theme::ThemePalette;

/// Renders the 1-line dynamic pulse bar (ligne de vie) across the dashboard.
/// - In IDLE mode: displays countdown progress to next sync.
/// - In SYNC mode: displays an animated roving beam (indeterminate) or live transfer progress.
pub fn render_pulse_line(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    if area.width < 10 || area.height < 1 {
        return;
    }

    let is_syncing = app.is_syncing();
    let max_w = area.width as usize;

    let mut line_spans = Vec::new();

    let fill_char = match app.config.graph_style {
        crate::config::GraphStyleChoice::Braille => "⣿",
        crate::config::GraphStyleChoice::Blocks => "█",
    };
    let (beam_low, beam_mid, beam_high) = match app.config.graph_style {
        crate::config::GraphStyleChoice::Braille => ("⣤", "⣶", "⣿"),
        crate::config::GraphStyleChoice::Blocks => ("▒", "▓", "█"),
    };

    if is_syncing {
        let live_pct = app.live.overall_progress_pct();

        // 1. Left prefix
        let prefix_style = Style::default().fg(theme.green).add_modifier(Modifier::BOLD);
        line_spans.push(Span::styled("⚡ Sync: ", prefix_style));

        let has_transfer_info = app.live.phase_index >= 3
            && (app.live.transfer.pct > 0 || app.live.transfer.files_total > 0 || !app.live.transfer.speed.is_empty());

        if has_transfer_info {
            // Live transfer with known progress percentage
            if max_w >= 60 && app.live.phase_index >= 3 && app.live.transfer.files_total > 0 {
                let files_str = format!("{}/{} files ", app.live.transfer.files_done, app.live.transfer.files_total);
                line_spans.push(Span::styled(files_str, Style::default().fg(theme.text_bright)));
            }

            let show_speed = max_w >= 70
                && app.live.phase_index == 3
                && !app.live.transfer.speed.is_empty()
                && app.live.transfer.speed != "0 B/s"
                && app.live.transfer.speed != "0/s";

            let right_text = if show_speed {
                format!(" {}% ({})", live_pct, app.live.transfer.speed)
            } else {
                format!(" {}%", live_pct)
            };

            let left_w = Line::from(line_spans.clone()).width();
            let right_w = Span::raw(&right_text).width();
            let brackets_w = 2; // '[' and ']'

            if max_w > left_w + right_w + brackets_w + 3 {
                let bar_w = max_w - left_w - right_w - brackets_w;
                let filled = (((live_pct as f64) / 100.0 * bar_w as f64).round() as usize).min(bar_w);

                line_spans.push(Span::styled("[", Style::default().fg(theme.border)));
                for i in 0..bar_w {
                    if i < filled {
                        line_spans.push(Span::styled(fill_char, Style::default().fg(theme.green).add_modifier(Modifier::BOLD)));
                    } else {
                        line_spans.push(Span::styled("·", Style::default().fg(theme.separator)));
                    }
                }
                line_spans.push(Span::styled("]", Style::default().fg(theme.border)));
                line_spans.push(Span::styled(right_text, Style::default().fg(theme.green).add_modifier(Modifier::BOLD)));
            } else if max_w > left_w + right_w {
                // Not enough room for bar, just show right text
                line_spans.push(Span::styled(right_text, Style::default().fg(theme.green).add_modifier(Modifier::BOLD)));
            }
        } else {
            // Indeterminate: listing / diffing / scanning phase (when searching / before transfer info is available)
            let status_msg = if max_w < 50 {
                "scanning... "
            } else {
                "scanning & diffing... "
            };
            line_spans.push(Span::styled(status_msg, Style::default().fg(theme.text_bright)));

            let elapsed_str = if app.live.transfer.elapsed.is_empty() {
                "0s".to_string()
            } else {
                app.live.transfer.elapsed.clone()
            };
            let right_text = if max_w < 60 {
                format!(" ⏱ {}", elapsed_str)
            } else {
                format!(" ⏱ {} elapsed", elapsed_str)
            };

            let left_w = Line::from(line_spans.clone()).width();
            let right_w = Span::raw(&right_text).width();
            let brackets_w = 2; // '[' and ']'

            if max_w > left_w + right_w + brackets_w + 3 {
                let bar_w = max_w - left_w - right_w - brackets_w;

                // Animated roving wave across the track with symmetric tiers & smooth color/opacity gradient
                let t = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() / 70) as usize;

                let beam_len = (bar_w / 4).clamp(3, 10);
                let cycle_steps = bar_w + beam_len;
                let head = (t % cycle_steps) as isize;

                line_spans.push(Span::styled("[", Style::default().fg(theme.border)));
                for i in 0..bar_w {
                    let pos = i as isize;
                    let start = head - beam_len as isize;
                    if pos >= start && pos < head {
                        let rel = pos - start;
                        let center = (beam_len as f64 - 1.0) / 2.0;
                        let dist = if center > 0.0 {
                            ((rel as f64 - center).abs() / center).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let intensity = 1.0 - dist; // 0.0 at outer edges, 1.0 at center

                        let stops = [
                            (0.0, crate::ui::theme::color_with_opacity(theme.green, 0.45, Some(theme.separator))),
                            (0.40, theme.green),
                            (1.0, crate::ui::theme::lerp_color(theme.green, ratatui::style::Color::White, 0.35)),
                        ];
                        let slot_color = crate::ui::theme::gradient_multi_stop(&stops, intensity);

                        let (char_to_use, is_bold) = if intensity < 0.35 {
                            (beam_low, false)
                        } else if intensity < 0.70 {
                            (beam_mid, true)
                        } else {
                            (beam_high, true)
                        };

                        let mut style = Style::default().fg(slot_color);
                        if is_bold {
                            style = style.add_modifier(Modifier::BOLD);
                        }
                        line_spans.push(Span::styled(char_to_use, style));
                    } else {
                        line_spans.push(Span::styled("·", Style::default().fg(theme.separator)));
                    }
                }
                line_spans.push(Span::styled("]", Style::default().fg(theme.border)));
                line_spans.push(Span::styled(right_text, Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)));
            } else if max_w > left_w + right_w {
                line_spans.push(Span::styled(right_text, Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)));
            }
        }
    } else {
        // IDLE mode: Countdown to next scheduled sync
        let is_disabled = app.service_info.timer_left.eq_ignore_ascii_case("disabled")
            || app.config.timer_interval.eq_ignore_ascii_case("never");

        if is_disabled {
            line_spans.push(Span::styled("⚡ Sync timer: ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)));
            line_spans.push(Span::styled("Paused ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)));
            if max_w >= 50 {
                line_spans.push(Span::styled("(manual sync only)", Style::default().fg(theme.text_muted)));
            }
        } else {
            let left_str = if app.service_info.timer_left.is_empty() {
                "—".to_string()
            } else {
                app.service_info.timer_left.clone()
            };

            line_spans.push(Span::styled("⚡ Next sync: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
            line_spans.push(Span::styled(format!("{} ", left_str), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)));

            let right_text = if max_w >= 50 {
                format!(" (every {})", app.config.timer_interval)
            } else {
                format!(" ({})", app.config.timer_interval)
            };

            let left_w = Line::from(line_spans.clone()).width();
            let right_w = Span::raw(&right_text).width();
            let brackets_w = 2; // '[' and ']'

            if max_w > left_w + right_w + brackets_w + 3 {
                let bar_w = max_w - left_w - right_w - brackets_w;
                let ratio = app.timer_progress().unwrap_or(0.0);
                let filled = ((ratio * bar_w as f64).round() as usize).min(bar_w);
                let bar_color = if ratio >= 0.85 {
                    theme.green
                } else {
                    theme.accent
                };

                line_spans.push(Span::styled("[", Style::default().fg(theme.border)));
                for i in 0..bar_w {
                    if i < filled {
                        let slot_color = if filled >= 3 && i == filled - 1 {
                            crate::ui::theme::lerp_color(bar_color, ratatui::style::Color::White, 0.45)
                        } else if filled >= 3 && i == filled - 2 {
                            crate::ui::theme::lerp_color(bar_color, ratatui::style::Color::White, 0.20)
                        } else if filled == 2 && i == 1 {
                            crate::ui::theme::lerp_color(bar_color, ratatui::style::Color::White, 0.35)
                        } else {
                            bar_color
                        };
                        line_spans.push(Span::styled(fill_char, Style::default().fg(slot_color).add_modifier(Modifier::BOLD)));
                    } else {
                        line_spans.push(Span::styled("·", Style::default().fg(theme.separator)));
                    }
                }
                line_spans.push(Span::styled("]", Style::default().fg(theme.border)));
                line_spans.push(Span::styled(right_text, Style::default().fg(theme.text_muted)));
            } else if max_w > left_w + brackets_w + 3 {
                let bar_w = max_w - left_w - brackets_w;
                let ratio = app.timer_progress().unwrap_or(0.0);
                let filled = ((ratio * bar_w as f64).round() as usize).min(bar_w);
                let bar_color = if ratio >= 0.85 {
                    theme.green
                } else {
                    theme.accent
                };

                line_spans.push(Span::styled("[", Style::default().fg(theme.border)));
                for i in 0..bar_w {
                    if i < filled {
                        let slot_color = if filled >= 3 && i == filled - 1 {
                            crate::ui::theme::lerp_color(bar_color, ratatui::style::Color::White, 0.45)
                        } else if filled >= 3 && i == filled - 2 {
                            crate::ui::theme::lerp_color(bar_color, ratatui::style::Color::White, 0.20)
                        } else if filled == 2 && i == 1 {
                            crate::ui::theme::lerp_color(bar_color, ratatui::style::Color::White, 0.35)
                        } else {
                            bar_color
                        };
                        line_spans.push(Span::styled(fill_char, Style::default().fg(slot_color).add_modifier(Modifier::BOLD)));
                    } else {
                        line_spans.push(Span::styled("·", Style::default().fg(theme.separator)));
                    }
                }
                line_spans.push(Span::styled("]", Style::default().fg(theme.border)));
            }
        }
    }

    let p = Paragraph::new(Line::from(line_spans));
    f.render_widget(p, area);
}
