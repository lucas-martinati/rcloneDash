use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::ui::theme::ThemePalette;

pub const SETTINGS_ITEMS_COUNT: usize = 7;

pub fn render_settings_modal(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let screen = f.area();
    let is_wide = screen.width >= 86;
    let logo_h: u16 = if is_wide { 6 } else { 5 };
    let show_logo = screen.height >= 28;
    let box_w = if is_wide { 86.min(screen.width) } else { 78.min(screen.width) };
    let box_h = if show_logo {
        20.min(screen.height.saturating_sub(logo_h + 3))
    } else {
        22.min(screen.height.saturating_sub(2))
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

    let cur_opt = app.settings_selected_idx + 1;

    let (up_col, down_col) = if app.settings_selected_idx == 0 {
        (theme.text_muted, theme.red)
    } else if app.settings_selected_idx >= SETTINGS_ITEMS_COUNT.saturating_sub(1) {
        (theme.red, theme.text_muted)
    } else {
        (theme.red, theme.red)
    };

    // En-tête btop++ avec coin supérieur gauche ┌[options]┐
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(theme.red))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┌[", Style::default().fg(theme.red)),
            Span::styled("options", Style::default().fg(Color::Rgb(220, 220, 220))),
            Span::styled("]┐", Style::default().fg(theme.red)),
        ]))
        .title_bottom(
            Line::from(vec![
                Span::styled("┘", Style::default().fg(theme.red)),
                Span::styled("↑", Style::default().fg(up_col).add_modifier(Modifier::BOLD)),
                Span::styled(" select ", Style::default().fg(Color::White)),
                Span::styled("↓", Style::default().fg(down_col).add_modifier(Modifier::BOLD)),
                Span::styled("└┘", Style::default().fg(theme.red)),
                Span::styled("← modifier →", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled("└┘", Style::default().fg(theme.red)),
                Span::styled("↵ enregistrer", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
                Span::styled("└┘", Style::default().fg(theme.red)),
                Span::styled("Esc fermer", Style::default().fg(Color::Rgb(160, 165, 180))),
                Span::styled("└", Style::default().fg(theme.red)),
                Span::styled(format!(" ─── {}/{} ", cur_opt, SETTINGS_ITEMS_COUNT), Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            ])
            .alignment(Alignment::Right),
        );
    f.render_widget(outer_block, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    if inner.height < 4 || inner.width < 40 {
        return;
    }

    // Découpage horizontal direct pour les deux colonnes (sans onglets ni séparateur horizontal)
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

    // Données des réglages
    let full_sync_display = match app.config.full_sync_interval.as_str() {
        "60" => "1h (Recommandé)".to_string(),
        "120" => "2h".to_string(),
        "240" => "4h".to_string(),
        "360" => "6h".to_string(),
        "720" => "12h".to_string(),
        "1440" => "24h (1 jour)".to_string(),
        "never" => "Jamais (Local)".to_string(),
        other => other.to_string(),
    };

    let settings = [
        ("Color theme", app.current_theme.name().to_string()),
        ("Intervalle timer bisync", app.config.timer_interval.clone()),
        ("Filet de sécurité Cloud", full_sync_display),
        ("Limite bande passante", app.config.bwlimit.as_deref().unwrap_or("Désactivé").to_string()),
        ("Fréquence UI", format!("{} ms", app.tick_rate_ms_live)),
        ("Dossier local", app.config.local_dir.clone()),
        ("Remote distant", app.config.remote.clone()),
    ];

    // Rendu de la colonne gauche
    let mut left_lines = Vec::new();
    let row_height = 2; // 1 ligne label + 1 ligne valeur

    for (i, (label, val)) in settings.iter().enumerate() {
        let is_selected = i == app.settings_selected_idx;
        let item_y = left_area.y + (i as u16 * row_height);

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

            // Flèche gauche et droite intégrées dans la ligne de valeur style btop++
            let inner_w = w.saturating_sub(4);
            let val_centered = format!("{:^width$}", val, width = inner_w);
            let line2 = Line::from(vec![
                Span::styled(format!("← {} →", val_centered), Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
            ]);

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
    let (desc_title, desc_body) = match app.settings_selected_idx {
        0 => (
            "Color theme.",
            "Définit le jeu de couleurs appliqué à l'ensemble du tableau de bord.\n\nPrend en charge 6 thèmes soignés pour une lisibilité optimale :\n• Tokyo Night\n• Catppuccin Mocha\n• Nord Frost\n• Gruvbox Dark\n• Dracula\n• Monokai Pro\n\nChaque thème adapte dynamiquement les bordures, textes et graphiques à dégradé btop++."
        ),
        1 => (
            "Intervalle timer bisync.",
            "Fréquence de vérification et de synchronisation automatique par systemd.\n\nConfigure la fréquence à laquelle rclone-bisync.timer se réveille pour inspecter les modifications.\n\nValeurs disponibles : 10m, 15m, 30m, 1h, 2h, 4h.\nRecommandé : 15m pour un équilibre idéal entre rapidité et consommation processeur."
        ),
        2 => (
            "Filet de sécurité Cloud.",
            "Délai maximal avant d'exécuter une synchronisation bidirectionnelle complète.\n\nMême si aucune modification locale n'a été détectée, ce filet garantit la récupération de tous les fichiers créés ou modifiés depuis un autre poste ou sur le cloud.\n\nOption 'Jamais' disponible pour une synchronisation déclenchée uniquement lors de modifications locales."
        ),
        3 => (
            "Limite de bande passante (bwlimit).",
            "Vitesse maximale autorisée pour les transferts rclone.\n\nPermet de préserver votre connexion Internet en limitant le débit réseau utilisé par rclone.\n\nParamètre enregistré dans bwlimit.env et injecté dans le service systemd.\nValeur 'Désactivé' pour exploiter 100% de la bande passante."
        ),
        4 => (
            "Fréquence de boucle UI (tick rate).",
            "Vitesse de rafraîchissement du moteur d'affichage TUI en millisecondes.\n\nContrôle la fluidité du défilement des logs, du calcul des métriques et des micro-animations.\n\nDirectement synchronisé avec les touches [+] et [-] ou clics sur le widget de fréquence."
        ),
        5 => (
            "Dossier local surveillé.",
            "Chemin vers le répertoire local racine synchronisé avec le stockage cloud.\n\nContient vos données réelles répliquées par bisync.\nConsultez l'explorateur de fichiers (touche 'f') pour explorer son arborescence."
        ),
        6 => (
            "Remote cloud distant.",
            "Identifiant du stockage distant configuré dans ~/.config/rclone/rclone.conf.\n\nUtilisé pour les requêtes de quota, de listing distant et de synchronisation bidirectionnelle."
        ),
        _ => ("Description.", "Sélectionnez un paramètre pour afficher son aide détaillée."),
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
            desc_lines.push(Line::from(Span::styled(para, Style::default().fg(Color::Rgb(185, 190, 205)))));
        }
    }

    let right_p = Paragraph::new(desc_lines);
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
