use super::*;

use crate::monitor::history::{PastRun, RunStatus};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    #[tokio::test]
    async fn test_mouse_click_hitboxes() {
        let backend = TestBackend::new(130, 35);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        // Rendu pour générer les hitboxes précises
        terminal.draw(|f| crate::ui::render(f, &mut app)).unwrap();
        assert!(!app.hit_mgr.dashboard.is_empty(), "Les hitboxes doivent être enregistrées");

        // Trouver la hitbox du bouton Fichiers (Files) dans le footer
        let files_hb = app.hit_mgr.dashboard.iter().find(|h| h.action == HitAction::ButtonFiles);
        assert!(files_hb.is_some(), "Le bouton Files doit être présent");
        let hb = files_hb.unwrap();

        // Clic au centre de la hitbox Files
        let click_x = hb.rect.x + hb.rect.width / 2;
        let click_y = hb.rect.y + hb.rect.height / 2;

        let mouse_event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: click_x,
            row: click_y,
            modifiers: KeyModifiers::NONE,
        };

        app.handle_mouse(mouse_event);
        assert_eq!(app.modal, Modal::Files, "Le clic doit basculer la modal Fichiers");

        // Clic à nouveau pour fermer
        app.handle_mouse(mouse_event);
        assert_eq!(app.modal, Modal::None, "Le second clic doit fermer la modal Fichiers");

        // Tester également l'ouverture du Menu et le clic sur Option 0 (Options / Settings)
        app.modal = Modal::Menu;
        app.menu_selected_idx = 0;
        terminal.draw(|f| crate::ui::render(f, &mut app)).unwrap();
        let menu_opt0_hb = app.hit_mgr.modal.iter().find(|h| h.action == HitAction::MenuOption(0));
        assert!(menu_opt0_hb.is_some(), "L'option Menu 0 (Settings) doit avoir sa hitbox");
        let mhb = menu_opt0_hb.unwrap();
        let menu_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: mhb.rect.x + mhb.rect.width / 2,
            row: mhb.rect.y + mhb.rect.height / 2,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse(menu_click);
        assert_eq!(app.modal, Modal::Settings, "Le clic sur MenuOption(0) doit ouvrir Settings");
    }

    #[tokio::test]
    async fn test_mouse_scroll_logs() {
        let mut app = App::new();
        app.live.log_lines = (1..=30).map(|i| format!("log {}", i)).collect();
        app.logs_viewport_height = 10;
        app.hit_mgr.dashboard.push(Hitbox {
            rect: Rect { x: 50, y: 5, width: 50, height: 15 },
            action: HitAction::LogsArea,
        });

        assert!(app.auto_scroll);
        // Molette vers le haut dans les logs
        let scroll_up = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 60,
            row: 10,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse(scroll_up);
        assert!(!app.auto_scroll, "Le défilement vers le haut doit désactiver l'auto-scroll");
        assert_eq!(app.logs_scroll, 1);

        // Molette vers le bas dans les logs
        let scroll_down = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 60,
            row: 10,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse(scroll_down);
        assert_eq!(app.logs_scroll, 0);
        assert!(app.auto_scroll, "Le défilement tout en bas doit réactiver l'auto-scroll");
    }

    #[tokio::test]
    async fn test_resize_resilience() {
        let sizes = [(40, 15), (60, 20), (80, 24), (100, 30), (140, 45), (200, 60)];
        let modals = vec![
            Modal::None,
            Modal::Menu,
            Modal::Help,
            Modal::Settings,
            Modal::Filters,
            Modal::Files,
            Modal::ConfirmResync,
            Modal::ConfirmCancel,
            Modal::ConfirmDelete("/test/file.txt".to_string()),
            Modal::HistoryDetails(0),
            Modal::DryRun,
        ];

        for (w, h) in sizes {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new();

            // Add sample run for HistoryDetails
            app.past_runs.push(PastRun {
                id: 1,
                date: "2026-09-17".to_string(),
                time: "22:00".to_string(),
                duration: "1m 30s".to_string(),
                status: RunStatus::Success,
                files_copied: vec!["file1.txt".to_string()],
                files_modified: vec![],
                files_deleted: vec![],
                synced_files: vec![("new".to_string(), "file1.txt".to_string(), "22:00".to_string())],
                errors: vec![],
            });

            for modal in &modals {
                app.modal = modal.clone();
                let res = terminal.draw(|f| crate::ui::render(f, &mut app));
                assert!(res.is_ok(), "Render failed for size {}x{} with modal {:?}", w, h, modal);
            }
        }
    }

    #[tokio::test]
    async fn test_menu_keyboard_and_mouse() {
        let mut app = App::new();
        // Pressing Esc on dashboard opens Menu
        let esc_event = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_key(esc_event);
        assert_eq!(app.modal, Modal::Menu);

        // First option is Options (Settings)
        assert_eq!(app.menu_selected_idx, 0);

        // Pressing Down selects second option (Help)
        let down_event = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        app.handle_key(down_event);
        assert_eq!(app.menu_selected_idx, 1);

        // Pressing Enter opens Help modal
        let enter_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        app.handle_key(enter_event);
        assert_eq!(app.modal, Modal::Help);

        // Pressing Esc closes Help modal
        app.handle_key(esc_event);
        assert_eq!(app.modal, Modal::None);
    }

    #[tokio::test]
    async fn test_stats_interval_stepping() {
        let mut app = App::new();
        app.config.stats_interval = "3s".to_string();

        // '-' decreases interval: 3s -> 2s -> 1s
        let minus_event = KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE);
        app.handle_key(minus_event);
        assert_eq!(app.config.stats_interval, "2s");

        app.handle_key(minus_event);
        assert_eq!(app.config.stats_interval, "1s");

        // '+' increases interval: 1s -> 2s -> 3s
        let plus_event = KeyEvent::new(KeyCode::Char('+'), KeyModifiers::NONE);
        app.handle_key(plus_event);
        assert_eq!(app.config.stats_interval, "2s");
        assert!(app.stats_interval_changed);

        app.handle_key(plus_event);
        assert_eq!(app.config.stats_interval, "3s");
    }

    #[tokio::test]
    async fn test_settings_never_cycle() {
        let mut app = App::new();
        app.settings_tab = 0;
        app.settings_selected_idx = 1; // full_sync_interval
        app.config.full_sync_interval = "1440".to_string();

        // Cycling forward from 1440 must reach "never"
        app.cycle_setting(true);
        assert_eq!(app.config.full_sync_interval, "never");

        // Cycling forward from "never" must wrap around to "60"
        app.cycle_setting(true);
        assert_eq!(app.config.full_sync_interval, "60");
    }

    #[tokio::test]
    async fn test_dry_run_and_cancel_modals() {
        let mut app = App::new();

        // Trigger dry-run modal
        app.start_dry_run();
        assert_eq!(app.modal, Modal::DryRun);
        assert!(app.dry_run_running);

        // Esc closes DryRun modal
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_key(esc);
        assert_eq!(app.modal, Modal::None);

        // 'c' opens ConfirmCancel modal if syncing
        app.live.is_syncing = true;
        let c_key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        app.handle_key(c_key);
        assert_eq!(app.modal, Modal::ConfirmCancel);

        // 'n' cancels ConfirmCancel modal
        let n_key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
        app.handle_key(n_key);
        assert_eq!(app.modal, Modal::None);
    }

    #[tokio::test]
    async fn test_filter_modal_keyboard_and_mouse_scroll() {
        let mut app = App::new();
        app.filters = (1..=30).map(|i| format!("- /exclude_pattern_{}/**", i)).collect();
        app.modal = Modal::Filters;
        app.filter_viewport_height = 10;
        app.selected_filter_idx = 0;
        app.filter_scroll_offset = 0;

        // Register a Hitbox for FilterArea
        app.hit_mgr.dashboard.push(Hitbox {
            rect: Rect { x: 10, y: 10, width: 40, height: 15 },
            action: HitAction::FilterArea,
        });

        // 1. Keyboard Down navigation
        let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        for _ in 0..12 {
            app.handle_key(down_key);
        }
        assert_eq!(app.selected_filter_idx, 12);
        assert!(app.filter_scroll_offset > 0, "filter_scroll_offset must have scrolled down");

        // 2. Keyboard PageUp
        let page_up = KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE);
        app.handle_key(page_up);
        assert_eq!(app.selected_filter_idx, 2);
        assert_eq!(app.filter_scroll_offset, 2);

        // 3. Mouse Wheel ScrollDown over FilterArea
        let scroll_down = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 15,
            row: 15,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse(scroll_down);
        assert_eq!(app.selected_filter_idx, 3);

        // 4. Mouse Wheel ScrollUp over FilterArea
        let scroll_up = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 15,
            row: 15,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse(scroll_up);
        assert_eq!(app.selected_filter_idx, 2);

        // 5. Left Click on FilterRow
        app.hit_mgr.dashboard.push(Hitbox {
            rect: Rect { x: 10, y: 12, width: 40, height: 1 },
            action: HitAction::FilterRow(20),
        });
        let click_row = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 15,
            row: 12,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse(click_row);
        assert_eq!(app.selected_filter_idx, 20);
        assert!(app.filter_scroll_offset >= 11, "filter_scroll_offset must be updated to keep row 20 visible");
    }

    #[tokio::test]
    async fn test_toggle_ctrl_mode() {
        let mut app = App::new();
        assert!(!app.ctrl_mode);

        // Click on ToggleCtrlMode hitbox toggles ctrl_mode
        app.hit_mgr.dashboard.push(Hitbox {
            rect: Rect { x: 30, y: 40, width: 15, height: 1 },
            action: HitAction::ToggleCtrlMode,
        });
        let click_ctrl = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 35,
            row: 40,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse(click_ctrl);
        assert!(app.ctrl_mode);
        assert!(app.toast.is_some());

        // Clicking again toggles back off
        app.handle_mouse(click_ctrl);
        assert!(!app.ctrl_mode);

        // Key Ctrl+X toggles on
        let ctrl_x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
        app.handle_key(ctrl_x);
        assert!(app.ctrl_mode);

        // Key Ctrl+X toggles off
        app.handle_key(ctrl_x);
        assert!(!app.ctrl_mode);

        // Raw 0x18 keycode also works even while filtering
        app.is_filtering_recent = true;
        let raw_ctrl_x = KeyEvent::new(KeyCode::Char('\u{18}'), KeyModifiers::NONE);
        app.handle_key(raw_ctrl_x);
        assert!(app.ctrl_mode);
        app.handle_key(raw_ctrl_x);
        assert!(!app.ctrl_mode);
    }

    #[tokio::test]
    async fn test_stats_interval_stepping_and_settings_harmony() {
        let mut app = App::new();
        app.config.stats_interval = "2s".to_string();

        // Step up
        app.step_stats_interval(true);
        assert_eq!(app.config.stats_interval, "3s");

        // Step down
        app.step_stats_interval(false);
        assert_eq!(app.config.stats_interval, "2s");

        // Cycle via settings
        app.settings_tab = 0;
        app.settings_selected_idx = 3; // Rclone stats interval
        app.cycle_setting(true);
        assert_eq!(app.config.stats_interval, "3s");
    }

    #[tokio::test]
    async fn test_recent_filter_interactive() {
        let mut app = App::new();
        app.past_runs.clear();
        app.live.synced_files.clear();
        app.past_runs.push(crate::monitor::PastRun {
            id: 1,
            date: "2026-09-18".into(),
            time: "10:00".into(),
            duration: "5s".into(),
            status: RunStatus::Success,
            files_copied: vec!["Documents/alpha.txt".into(), "Pictures/photo.jpg".into()],
            files_modified: vec!["Code/main.rs".into()],
            files_deleted: vec![],
            errors: vec![],
            synced_files: vec![],
        });

        // Focus RecentFiles and press '/' to start filtering
        app.focused_panel = FocusedPanel::RecentFiles;
        let slash_key = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
        app.handle_key(slash_key);
        assert!(app.is_filtering_recent);

        // Type "photo"
        for c in "photo".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(app.recent_filter, "photo");
        let filtered = app.get_all_recent_files();
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].1.contains("photo.jpg"));

        // Press Enter to confirm filter
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!app.is_filtering_recent);
        assert_eq!(app.recent_filter, "photo");

        // Focus and press '/', then Esc to clear
        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.is_filtering_recent);
        assert!(app.recent_filter.is_empty());
        assert_eq!(app.get_all_recent_files().len(), 3);
    }

    #[tokio::test]
    async fn test_total_history_runs_with_live_sync() {
        let mut app = App::new();
        app.service_info.state = ServiceState::Idle;
        app.live.is_syncing = false;
        app.past_runs.clear();
        assert_eq!(app.total_history_runs(), 0);

        app.past_runs.push(crate::monitor::PastRun {
            id: 1,
            date: "2026-09-18".into(),
            time: "10:00".into(),
            duration: "5s".into(),
            status: RunStatus::Success,
            files_copied: vec![],
            files_modified: vec![],
            files_deleted: vec![],
            errors: vec![],
            synced_files: vec![],
        });
        assert_eq!(app.total_history_runs(), 1);

        app.live.is_syncing = true;
        assert_eq!(app.total_history_runs(), 2);
    }

    #[tokio::test]
    async fn test_confirm_sync_and_dry_run_modals() {
        let mut app = App::new();

        // 1. Appuyer sur 's' -> ouvre la confirmation de sync
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::ConfirmSync);

        // 'n' ou Esc annule
        app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        // 2. Appuyer sur 'd' -> ouvre la modal DryRun directement
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::DryRun);

        // Esc annule
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);
    }

    #[tokio::test]
    async fn test_unambiguous_global_shortcuts() {
        let mut app = App::new();
        // Quand on est sur le panel RecentFiles, les touches globales ne doivent pas être capturées par d'anciennes actions
        app.focused_panel = FocusedPanel::RecentFiles;

        app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::Settings);
        app.modal = Modal::None;

        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::DryRun);
        app.modal = Modal::None;

        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
        assert!(app.is_filtering_recent, "'f' sur RecentFiles doit activer le filtre");
        app.is_filtering_recent = false;

        // 'b' ouvre la modal Files (explorateur)
        app.focused_panel = FocusedPanel::History;
        app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::Files);
        app.modal = Modal::None;

        // 'f' active le filtre même depuis un autre panel et focus RecentFiles
        app.focused_panel = FocusedPanel::History;
        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
        assert!(app.is_filtering_recent);
        assert_eq!(app.focused_panel, FocusedPanel::RecentFiles);
        app.is_filtering_recent = false;

        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::ConfirmSync);
        app.modal = Modal::None;
    }

    #[test]
    fn test_grayscale_palette() {
        let palette = crate::ui::theme::ThemeChoice::TokyoNight.palette();
        let gray = palette.to_grayscale();

        // Vérifier que chaque couleur RGB est en nuances de gris (r == g == b)
        if let ratatui::style::Color::Rgb(r, g, b) = gray.accent {
            assert_eq!(r, g);
            assert_eq!(g, b);
        } else {
            panic!("Expected RGB color");
        }

        if let ratatui::style::Color::Rgb(r, g, b) = gray.red {
            assert_eq!(r, g);
            assert_eq!(g, b);
        } else {
            panic!("Expected RGB color");
        }
    }

    #[tokio::test]
    async fn test_initial_unselected_and_scroll() {
        let mut app = App::new();
        app.service_info.state = ServiceState::Idle;
        app.live.is_syncing = false;
        assert_eq!(app.recent_selected_idx, None);
        assert_eq!(app.selected_run_idx, None);

        // Add 5 past runs
        app.past_runs.clear();
        for i in 0..5 {
            app.past_runs.push(crate::monitor::PastRun {
                id: i,
                date: "2026-09-18".into(),
                time: format!("10:0{}", i),
                duration: "2s".into(),
                status: RunStatus::Success,
                files_copied: vec![],
                files_modified: vec![],
                files_deleted: vec![],
                errors: vec![],
                synced_files: vec![],
            });
        }

        // Viewport of 3 items
        app.history_viewport_height = 3;
        app.focused_panel = FocusedPanel::History;

        // Down from None -> Some(0)
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.selected_run_idx, Some(0));
        assert_eq!(app.history_scroll_offset, 0);

        // Up from 0 -> None (disappears)
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.selected_run_idx, None);

        // Down again -> Some(0)
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.selected_run_idx, Some(0));

        // Down: content remains below (max_offset = 5 - 3 = 2), so scroll_offset increments and selected_idx increments
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.history_scroll_offset, 1);
        assert_eq!(app.selected_run_idx, Some(1));
        // Visual position = 1 - 1 = 0 (fixed at the top of the viewport!)

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.history_scroll_offset, 2);
        assert_eq!(app.selected_run_idx, Some(2));
        // Visual position = 2 - 2 = 0 (still fixed!)

        // Now max_offset (2) is reached. Down moves the cursor towards the end
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.history_scroll_offset, 2);
        assert_eq!(app.selected_run_idx, Some(3));

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.history_scroll_offset, 2);
        assert_eq!(app.selected_run_idx, Some(4));

        // Clamped at end (cannot scroll past 4)
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.history_scroll_offset, 2);
        assert_eq!(app.selected_run_idx, Some(4));
    }

    #[tokio::test]
    async fn test_stats_interval_boundaries() {
        let mut app = App::new();
        app.config.stats_interval = "1s".to_string();
        assert!(!app.can_dec_stats_interval());
        assert!(app.can_inc_stats_interval());

        app.config.stats_interval = "30s".to_string();
        assert!(app.can_dec_stats_interval());
        assert!(!app.can_inc_stats_interval());

        app.config.stats_interval = "5s".to_string();
        assert!(app.can_dec_stats_interval());
        assert!(app.can_inc_stats_interval());
    }

    #[tokio::test]
    async fn test_settings_resync_modal_trigger() {
        let mut app = App::new();
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 6; // Resynchronisation complète

        // Pressing Enter must open ConfirmResync
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::ConfirmResync);

        // Annuler avec Esc
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        // Rouvrir et tester avec Flèche Droite
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 6;
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::ConfirmResync);
    }

    #[tokio::test]
    async fn test_files_browser_left_right_navigation() {
        let mut app = App::new();
        app.modal = Modal::Files;
        app.file_entries = vec![
            crate::fs_tree::FileEntry {
                name: "dossier_a".into(),
                rel_path: "dossier_a".into(),
                is_dir: true,
                size: 0,
                mtime: "".into(),
                ignored: false,
            },
            crate::fs_tree::FileEntry {
                name: "fichier_b.txt".into(),
                rel_path: "fichier_b.txt".into(),
                is_dir: false,
                size: 100,
                mtime: "".into(),
                ignored: false,
            },
        ];
        app.file_selected_idx = 0; // dossier_a

        // Flèche droite sur un dossier entre dans le dossier
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(app.file_current_rel, "dossier_a");

        // Flèche gauche remonte au parent
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.file_current_rel, "");
    }

    #[test]
    fn test_normalize_display_path() {
        use crate::ui::dashboard::normalize_display_path;
        assert_eq!(
            normalize_display_path("Images/Screenshot From 2026-09-18 08-08-09.png"),
            "Images/Screenshot from 2026-09-18 08-08-09.png"
        );
        assert_eq!(
            normalize_display_path("Images/Screenshot from 2026-09-03 08-56-39.png"),
            "Images/Screenshot from 2026-09-03 08-56-39.png"
        );
        assert_eq!(
            normalize_display_path("From zero to hero.pdf"),
            "from zero to hero.pdf"
        );
    }

    #[test]
    fn test_split_path() {
        use crate::ui::dashboard::split_path;
        assert_eq!(
            split_path("Images/Screenshots/Screenshot.png"),
            ("Images/Screenshots", "Screenshot.png")
        );
        assert_eq!(
            split_path("Screenshot.png"),
            ("", "Screenshot.png")
        );
        assert_eq!(
            split_path("/home/user/document.pdf"),
            ("home/user", "document.pdf")
        );
        assert_eq!(
            split_path("folder/subfolder/file.txt/"),
            ("folder/subfolder", "file.txt")
        );
        assert_eq!(
            split_path(""),
            ("", "")
        );
    }

    #[test]
    fn test_color_gradient_and_opacity() {
        use ratatui::style::Color;
        use crate::ui::theme::{lerp_color, color_with_opacity, gradient_multi_stop};

        // 1. Lerp test
        let c1 = Color::Rgb(0, 0, 0);
        let c2 = Color::Rgb(200, 100, 50);
        assert_eq!(lerp_color(c1, c2, 0.0), Color::Rgb(0, 0, 0));
        assert_eq!(lerp_color(c1, c2, 1.0), Color::Rgb(200, 100, 50));
        assert_eq!(lerp_color(c1, c2, 0.5), Color::Rgb(100, 50, 25));

        // 2. Opacity test
        let col = Color::Rgb(100, 200, 100);
        let bg = Color::Rgb(0, 0, 0);
        assert_eq!(color_with_opacity(col, 1.0, Some(bg)), Color::Rgb(100, 200, 100));
        assert_eq!(color_with_opacity(col, 0.0, Some(bg)), Color::Rgb(0, 0, 0));

        // 3. Multi-stop gradient test
        let stops = [
            (0.0, Color::Rgb(0, 0, 255)),
            (0.5, Color::Rgb(0, 255, 0)),
            (1.0, Color::Rgb(255, 0, 0)),
        ];
        assert_eq!(gradient_multi_stop(&stops, 0.0), Color::Rgb(0, 0, 255));
        assert_eq!(gradient_multi_stop(&stops, 0.5), Color::Rgb(0, 255, 0));
        assert_eq!(gradient_multi_stop(&stops, 1.0), Color::Rgb(255, 0, 0));
        assert_eq!(gradient_multi_stop(&stops, 0.25), Color::Rgb(0, 128, 128));
    }



    #[tokio::test]
    async fn test_universal_q_closes_modals() {
        let mut app = App::new();

        // 1. Files modal se ferme avec 'q'
        app.modal = Modal::Files;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        // 2. Filters modal se ferme avec 'q'
        app.modal = Modal::Filters;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        // 3. Settings modal se ferme avec 'q'
        app.modal = Modal::Settings;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        // 4. Help modal se ferme avec 'q'
        app.modal = Modal::Help;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        // 5. Confirm modals se ferment avec 'q'
        app.modal = Modal::ConfirmSync;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        app.modal = Modal::ConfirmResync;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        app.modal = Modal::ConfirmCancel;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        // 6. Menu principal : 'q' quitte l'application
        app.modal = Modal::Menu;
        app.running = true;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(!app.running, "Dans le Menu, 'q' doit quitter l'application");

        // 7. Modal::None : 'q' quitte l'application
        app.modal = Modal::None;
        app.running = true;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(!app.running, "Sur le Dashboard, 'q' doit quitter l'application");
    }

    #[tokio::test]
    async fn test_settings_interactive_string_edit() {
        let mut app = App::new();
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 4; // Local directory
        app.config.local_dir = "/home/user/drive".to_string();

        // Press Enter to enter editing mode
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.is_editing_setting());
        assert_eq!(app.edit_buffer(), "/home/user/drive");

        // Saisie de caractères : "/sub"
        for c in "/sub".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(app.edit_buffer(), "/home/user/drive/sub");

        // Backspace
        app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.edit_buffer(), "/home/user/drive/su");

        // Cancel with Esc: modifications are discarded, original value preserved
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.is_editing_setting());
        assert_eq!(app.config.local_dir, "/home/user/drive");

        // Enter editing with 'e', modify and validate with Enter
        app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(app.is_editing_setting());
        app.edit_state = EditState::Setting { tab: 0, index: 4, buffer: "/home/new/path".to_string(), cursor: 14 };
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!app.is_editing_setting());
        assert_eq!(app.config.local_dir, "/home/new/path");

        // Test sur le remote (option 5) : par exemple "GoogleDrive:"
        app.settings_tab = 0;
        app.settings_selected_idx = 5;
        app.config.remote = "GoogleDrive:".to_string();
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)); // Flèche droite active l'édition
        assert!(app.is_editing_setting());
        assert_eq!(app.edit_buffer(), "GoogleDrive:");

        // L'utilisateur ne modifie rien à la chaîne et sort : "GoogleDrive:" reste inchangé
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.is_editing_setting());
        assert_eq!(app.config.remote, "GoogleDrive:");
    }

    #[tokio::test]
    async fn test_settings_text_input_cursor_navigation() {
        let mut app = App::new();
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 4; // LocalDirectory
        app.start_editing_setting();
        assert!(app.is_editing_setting());

        // Start buffer
        app.edit_state = EditState::Setting {
            tab: 0,
            index: 3,
            buffer: "hello".to_string(),
            cursor: 5,
        };
        assert_eq!(app.edit_cursor(), 5);

        // Move cursor left twice
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.edit_cursor(), 4);
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.edit_cursor(), 3);

        // Insert 'X' at cursor 3 -> "helXlo"
        app.handle_key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE));
        assert_eq!(app.edit_buffer(), "helXlo");
        assert_eq!(app.edit_cursor(), 4);

        // Home key
        app.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
        assert_eq!(app.edit_cursor(), 0);

        // Delete at cursor 0 (deletes 'h') -> "elXlo"
        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(app.edit_buffer(), "elXlo");
        assert_eq!(app.edit_cursor(), 0);

        // End key
        app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
        assert_eq!(app.edit_cursor(), 5);

        // Backspace at end (deletes 'o') -> "elXl"
        app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.edit_buffer(), "elXl");
        assert_eq!(app.edit_cursor(), 4);

        // Cancel with Esc
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.is_editing_setting());
    }

    #[tokio::test]
    async fn test_google_credentials_settings_editing() {
        let mut app = App::new();
        app.config.remote = "TestGoogleRemote:".to_string();
        app.modal = Modal::Settings;
        app.settings_tab = 0;

        // Option 8: GoogleClientId
        app.settings_selected_idx = 8;
        assert_eq!(config::SettingId::from_tab_and_idx(0, 8), Some(config::SettingId::GoogleClientId));
        assert!(config::SettingId::GoogleClientId.is_text_input());

        // Press Enter to start editing
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.is_editing_setting());

        // Set buffer to test client id and commit
        if let Some(buf) = app.edit_buffer_mut() {
            buf.clear();
            buf.push_str("123456789.apps.googleusercontent.com");
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!app.is_editing_setting());

        // Option 9: GoogleClientSecret
        app.settings_selected_idx = 9;
        assert_eq!(config::SettingId::from_tab_and_idx(0, 9), Some(config::SettingId::GoogleClientSecret));
        assert!(config::SettingId::GoogleClientSecret.is_text_input());

        // Press 'e' to start editing
        app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(app.is_editing_setting());

        // Test cancel_setting_edit
        app.cancel_setting_edit();
        assert!(!app.is_editing_setting());

        // Test open_first_run
        app.open_first_run();
        assert!(matches!(app.modal, Modal::FirstRun(_)));

        // Clean up test remote credentials
        let _ = config::write_rclone_credentials("TestGoogleRemote:", "", "");
        let _ = config::write_rclone_credentials("GoogleDrive:", "", "");
    }

    #[tokio::test]
    async fn test_scrollbar_mouse_interaction() {
        let mut app = App::new();
        app.logs_viewport_height = 5;
        app.live.log_lines = (0..20).map(|i| format!("log line {}", i)).collect();

        // 1. Clic sur flèche haut scrollbar logs
        assert_eq!(app.logs_scroll, 0);
        assert!(app.auto_scroll);
        app.apply_scrollbar_step(ScrollbarTarget::Logs, true);
        assert_eq!(app.logs_scroll, 1);
        assert!(!app.auto_scroll);

        // 2. Clic sur flèche bas scrollbar logs
        app.apply_scrollbar_step(ScrollbarTarget::Logs, false);
        assert_eq!(app.logs_scroll, 0);
        assert!(app.auto_scroll);

        // 3. Saut / Glissement (drag) sur la piste scrollbar
        // Piste de hauteur 10, offset 5 -> milieu
        app.apply_scrollbar_drag_to_row(ScrollbarTarget::Logs, 5, 0, 10, 20, 5, 1);
        assert!(app.logs_scroll > 0);
        assert!(!app.auto_scroll);

        // 4. Test ScrollbarTarget::History step & drag
        app.past_runs = (0..10).map(|i| crate::monitor::history::PastRun {
            id: i,
            date: "2026-09-18".into(),
            time: "12:00:00".into(),
            duration: "5s".into(),
            status: crate::monitor::history::RunStatus::Success,
            files_copied: vec![],
            files_modified: vec![],
            files_deleted: vec![],
            synced_files: vec![],
            errors: vec![],
        }).collect();
        app.history_viewport_height = 4;
        app.selected_run_idx = Some(0);

        app.apply_scrollbar_step(ScrollbarTarget::History, false);
        assert_eq!(app.selected_run_idx, Some(1));

        app.apply_scrollbar_step(ScrollbarTarget::History, true);
        assert_eq!(app.selected_run_idx, Some(0));

        app.apply_scrollbar_drag_to_row(ScrollbarTarget::History, 9, 0, 10, 10, 4, 1);
        assert_eq!(app.selected_run_idx, Some(9));
    }

    #[test]
    fn test_keybinding_registry() {
        use crate::ui::keys::{KeyAction, KeybindingRegistry};
        use crate::ui::theme::ThemeChoice;

        assert_eq!(KeybindingRegistry::get_key_str(KeyAction::Files), "b");
        assert_eq!(KeybindingRegistry::get_key_str(KeyAction::DecTickRate), "-");
        assert_eq!(KeybindingRegistry::get_key_str(KeyAction::IncTickRate), "+");
        assert_eq!(KeybindingRegistry::get_key_str(KeyAction::Validate), "Enter");
        assert_eq!(KeybindingRegistry::get_key_str(KeyAction::CancelEdit), "Esc");

        let theme = ThemeChoice::TokyoNight.palette();
        let badge = KeybindingRegistry::format_key_badge("Ctrl+X", &theme);
        assert_eq!(badge.len(), 3);
        assert_eq!(badge[0].content, " [");
        assert_eq!(badge[1].content, " Ctrl+X ");
        assert_eq!(badge[2].content, "] ");
    }

    #[tokio::test]
    async fn test_recent_files_scroll_with_small_viewport() {
        let mut app = App::new();
        // Remplir 10 fichiers récents via past_runs
        app.past_runs = vec![crate::monitor::history::PastRun {
            id: 1,
            date: "2026-09-18".into(),
            time: "12:00:00".into(),
            duration: "5s".into(),
            status: crate::monitor::history::RunStatus::Success,
            files_copied: (0..10).map(|i| format!("file_{}.txt", i)).collect(),
            files_modified: vec![],
            files_deleted: vec![],
            synced_files: vec![],
            errors: vec![],
        }];

        // Simuler un conteneur rétréci par la synchronisation en cours (viewport de 2 lignes)
        let vp = 2;
        app.recent_viewport_height = vp;
        app.focused_panel = FocusedPanel::RecentFiles;

        // Premier appui sur Flèche Bas : sélectionne l'élément 0
        app.scroll_recent_down(vp);
        assert_eq!(app.recent_selected_idx, Some(0));
        assert_eq!(app.recent_scroll_offset, 0);

        // Défiler vers le bas à travers tous les éléments
        for expected_idx in 1..10 {
            app.scroll_recent_down(vp);
            assert_eq!(app.recent_selected_idx, Some(expected_idx));

            let sel = app.recent_selected_idx.unwrap();
            // L'élément sélectionné ne doit JAMAIS dépasser la fenêtre visible
            assert!(
                sel >= app.recent_scroll_offset,
                "L'élément sélectionné ({}) ne doit pas être inférieur à l'offset ({})",
                sel,
                app.recent_scroll_offset
            );
            assert!(
                sel < app.recent_scroll_offset + vp,
                "L'élément sélectionné ({}) ne doit pas dépasser le conteneur (offset {} + vp {})",
                sel,
                app.recent_scroll_offset,
                vp
            );
        }

        // Vérifier que le tout dernier élément (index 9) est bien atteint et visible
        assert_eq!(app.recent_selected_idx, Some(9));
        assert_eq!(app.recent_scroll_offset, 8); // Affiche les éléments 8 et 9 dans le conteneur de 2 lignes

        // Défiler vers le haut
        for expected_idx in (0..9).rev() {
            app.scroll_recent_up(vp);
            assert_eq!(app.recent_selected_idx, Some(expected_idx));

            let sel = app.recent_selected_idx.unwrap();
            assert!(
                sel >= app.recent_scroll_offset,
                "En remontant, l'élément sélectionné ({}) doit rester >= offset ({})",
                sel,
                app.recent_scroll_offset
            );
            assert!(
                sel < app.recent_scroll_offset + vp,
                "En remontant, l'élément sélectionné ({}) doit rester < offset {} + vp {}",
                sel,
                app.recent_scroll_offset,
                vp
            );
        }
    }

    #[tokio::test]
    async fn test_wrap_text_no_crop() {
        let long_line = "2026-09-18 13:40:12 ERROR : Failed to copy /home/user/very/long/path/to/my/awesome/document_with_special_data.pdf: corrupted file size mismatch 1243 vs 1255";
        let wrapped = crate::ui::dashboard::wrap_text(long_line, 50, 45);
        assert!(wrapped.len() >= 3);
        for (i, line) in wrapped.iter().enumerate() {
            let limit = if i == 0 { 50 } else { 45 };
            assert!(line.chars().count() <= limit, "Ligne {} dépasse la limite {}: {}", i, limit, line);
        }

        // Test de sécurité UTF-8 (aucun panic avec caractères accentués ou emojis)
        let utf8_line = "🚨 Événement critique détecté dans le répertoire /données/système/sécurité_avancée/fichier_spécial.json";
        let wrapped_utf8 = crate::ui::dashboard::wrap_text(utf8_line, 30, 25);
        assert!(wrapped_utf8.len() >= 2);
        for line in &wrapped_utf8 {
            assert!(!line.is_empty());
        }
    }

    #[tokio::test]
    async fn test_clipboard_copy_logs_and_history_errors() {
        let mut app = App::new();

        // 1. Copie des logs
        app.live.log_lines.push_back("2026-09-18 13:40:12 INFO : Démarrage".into());
        app.live.log_lines.push_back("2026-09-18 13:40:15 NOTICE : Synchro terminée".into());
        app.copy_logs_to_clipboard();
        assert!(app.toast.is_some());
        let (msg, _) = app.toast.as_ref().unwrap();
        assert!(msg.contains("2 log lines copied"));

        // 2. Copie des erreurs d'un run
        app.past_runs = vec![crate::monitor::history::PastRun {
            id: 42,
            date: "2026-09-18".into(),
            time: "13:30:00".into(),
            duration: "12s".into(),
            status: crate::monitor::history::RunStatus::Failed,
            files_copied: vec![],
            files_modified: vec![],
            files_deleted: vec![],
            synced_files: vec![],
            errors: vec![
                "Failed to sync /docs: Google Drive quota exceeded".into(),
                "Connection timeout after 30s".into(),
            ],
        }];

        app.copy_history_errors(0);
        let (msg, _) = app.toast.as_ref().unwrap();
        assert!(msg.contains("2 error(s) from run #42"));

        // 3. Raccourci y / c dans Modal::HistoryDetails
        app.modal = Modal::HistoryDetails(0);
        let action = app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('y'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(action, Action::None);
        let (msg, _) = app.toast.as_ref().unwrap();
        assert!(msg.contains("2 error(s) from run #42"));

        let action_c = app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('c'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(action_c, Action::None);
        assert!(app.toast.is_some());

        // 4. Raccourci y sur le dashboard quand Logs est focalisé
        app.modal = Modal::None;
        app.focused_panel = FocusedPanel::Logs;
        let action_dash_y = app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('y'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(action_dash_y, Action::None);
        let (msg, _) = app.toast.as_ref().unwrap();
        assert!(msg.contains("log lines copied"));
    }

    #[tokio::test]
    async fn test_sync_progress_and_history_details_states() {
        let mut app = App::new();

        // 1. Test overall_progress_pct
        app.live.is_syncing = true;
        app.live.phase_index = 0; // Listings
        assert_eq!(app.live.overall_progress_pct(), 10);
        app.live.phase_index = 1; // Diffs locaux
        assert_eq!(app.live.overall_progress_pct(), 25);
        app.live.phase_index = 2; // Diffs distants
        assert_eq!(app.live.overall_progress_pct(), 50);
        app.live.phase_index = 3; // Application (sans transfer stats: 70)
        assert_eq!(app.live.overall_progress_pct(), 70);
        app.live.transfer.pct = 85;
        // With explicit rclone transfer percentage:
        assert_eq!(app.live.overall_progress_pct(), 85);
        app.live.transfer.pct = 0;
        app.live.phase_index = 4; // Mise à jour
        assert_eq!(app.live.overall_progress_pct(), 95);
        app.live.phase_index = 5; // Terminé
        assert_eq!(app.live.overall_progress_pct(), 100);

        // 2. Test is_syncing and panel hiding
        app.live.is_syncing = true;
        app.live.phase_index = 2;
        assert!(app.is_syncing());

        // When phase reaches 5 (Terminé), is_syncing must be false
        app.live.phase_index = 5;
        assert!(!app.is_syncing());

        // When systemd service is Idle, is_syncing and transfer are reset
        app.live.is_syncing = true;
        app.live.phase_index = 3;
        app.live.transfer.pct = 50;
        app.service_info.state = crate::systemd::ServiceState::Idle;
        if app.service_info.state == crate::systemd::ServiceState::Idle || app.service_info.state == crate::systemd::ServiceState::Failed {
            app.live.is_syncing = false;
            app.live.transfer = crate::monitor::parser::TransferStats::default();
        }
        assert!(!app.is_syncing());
        assert_eq!(app.live.transfer.pct, 0);

        // 3. Test HistoryDetails modal command availability
        // Run with NO errors and NO files
        app.past_runs = vec![crate::monitor::history::PastRun {
            id: 1,
            date: "2026-09-18".into(),
            time: "14:00:00".into(),
            duration: "5s".into(),
            status: crate::monitor::history::RunStatus::Success,
            files_copied: vec![],
            files_modified: vec![],
            files_deleted: vec![],
            synced_files: vec![],
            errors: vec![],
        }];

        app.toast = None;
        app.modal = Modal::HistoryDetails(0);

        // Keys y/c should NOT copy anything (disabled / unusable)
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('y'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(app.toast.is_none());

        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('c'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(app.toast.is_none());

        // Key d and Enter should NOT attempt to open files
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('d'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(app.toast.is_none());

        // Key q closes the modal
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('q'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.modal, Modal::None);
    }

    #[tokio::test]
    async fn test_scrollbar_cursor_hiding_and_open_commands() {
        let mut app = App::new();

        // 1. Test is_dragging_scrollbar
        assert!(!app.is_dragging_scrollbar(ScrollbarTarget::History));
        app.active_scrollbar_drag = Some((ScrollbarTarget::History, 5, 20, 50, 10, 0));
        assert!(app.is_dragging_scrollbar(ScrollbarTarget::History));
        assert!(!app.is_dragging_scrollbar(ScrollbarTarget::RecentFiles));
        app.active_scrollbar_drag = None;
        assert!(!app.is_dragging_scrollbar(ScrollbarTarget::History));

        // 2. Test apply_scrollbar_drag_to_row positions cursor at top or bottom boundary
        app.history_viewport_height = 5;
        // Total 20 runs, viewport 5 -> max_offset = 15
        // Jump to bottom (row 9, top 0, height 10 -> ratio 1.0)
        app.apply_scrollbar_drag_to_row(ScrollbarTarget::History, 9, 0, 10, 20, 5, 1);
        assert_eq!(app.history_scroll_offset, 15);
        // Cursor at bottom of visible items: 15 + 5 - 1 = 19
        assert_eq!(app.selected_run_idx, Some(19));

        // Jump to top (row 0, top 0, height 10 -> ratio 0.0)
        app.apply_scrollbar_drag_to_row(ScrollbarTarget::History, 0, 0, 10, 20, 5, 1);
        assert_eq!(app.history_scroll_offset, 0);
        // Cursor at top of visible items: 0
        assert_eq!(app.selected_run_idx, Some(0));

        // 3. Test open file / folder command in history details
        app.past_runs = vec![crate::monitor::history::PastRun {
            id: 1,
            date: "2026-09-18".into(),
            time: "14:00:00".into(),
            duration: "5s".into(),
            status: crate::monitor::history::RunStatus::Success,
            files_copied: vec!["file1.txt".into()],
            files_modified: vec![],
            files_deleted: vec![],
            synced_files: vec![("new".into(), "file1.txt".into(), "14:00".into())],
            errors: vec![],
        }];
        app.modal = Modal::HistoryDetails(0);
        app.history_selected_file_idx = 0;
        app.ctrl_mode = false;

        // Enter in file mode calls open_selected_file
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(app.toast.is_some());

        // Enter with Ctrl or ctrl_mode = true calls open_history_folder
        app.ctrl_mode = true;
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));
        let (msg, _) = app.toast.as_ref().unwrap();
        assert!(msg.contains("Parent folder") || msg.contains("Unable") || msg.contains("Opened"));
    }

    #[tokio::test]
    async fn test_log_filter_tabs_and_shortcuts() {
        let mut app = App::new();
        assert_eq!(app.log_filter, LogFilter::All);

        // Test next / prev
        assert_eq!(app.log_filter.next(), LogFilter::Files);
        assert_eq!(app.log_filter.next().next(), LogFilter::Problems);
        assert_eq!(app.log_filter.next().next().next(), LogFilter::All);
        assert_eq!(app.log_filter.prev(), LogFilter::Problems);

        // Matching logic
        let file_log = "2026-09-18 12:00:00 NOTICE: test.txt: Copied (new)";
        let err_log = "2026-09-18 12:00:00 ERROR: failed to copy: network error";
        let info_log = "2026-09-18 12:00:00 INFO: starting sync";

        assert!(LogFilter::All.matches(file_log));
        assert!(LogFilter::All.matches(err_log));
        assert!(LogFilter::All.matches(info_log));

        assert!(LogFilter::Files.matches(file_log));
        assert!(!LogFilter::Files.matches(err_log));
        assert!(!LogFilter::Files.matches(info_log));

        assert!(!LogFilter::Problems.matches(file_log));
        assert!(LogFilter::Problems.matches(err_log));
        assert!(!LogFilter::Problems.matches(info_log));

        // Keyboard switching when focused on Logs panel
        app.focused_panel = FocusedPanel::Logs;
        app.auto_scroll = true;

        // Right arrow moves to next tab
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Right,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.log_filter, LogFilter::Files);
        assert!(app.auto_scroll); // auto-scroll preserved!

        // Left arrow moves to previous tab
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Left,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.log_filter, LogFilter::All);

        // 'f' cycles filter tab
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('f'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.log_filter, LogFilter::Files);

        // 'f' again cycles to Problems
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('f'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.log_filter, LogFilter::Problems);

        // Mouse click on selector left/right arrows
        app.hit_mgr.dashboard = vec![
            Hitbox {
                rect: ratatui::layout::Rect { x: 10, y: 10, width: 1, height: 1 },
                action: HitAction::LogFilterPrev,
            },
            Hitbox {
                rect: ratatui::layout::Rect { x: 12, y: 10, width: 1, height: 1 },
                action: HitAction::LogFilterNext,
            },
        ];
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 10,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert_eq!(app.log_filter, LogFilter::Files);

        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 12,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert_eq!(app.log_filter, LogFilter::Problems);

        // Setting 7 opens full logs
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 7;
        let action = app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(action, Action::OpenFullLogs);

        // Clean up: reset to All
        app.set_log_filter(LogFilter::All);
    }

    #[tokio::test]
    async fn test_new_appearance_and_layout_settings_cycle() {
        let mut app = App::new();
        app.settings_tab = 1;
        app.config.container_layout = crate::config::ContainerLayout::Default;
        app.config.mid_panel_order = crate::config::MidPanelOrder::HistoryLogs;
        app.config.border_style = crate::config::BorderStyleChoice::Rounded;
        app.config.graph_style = crate::config::GraphStyleChoice::Blocks;

        // 1. ContainerLayout (idx 1)
        app.settings_selected_idx = 1;
        assert_eq!(app.config.container_layout, crate::config::ContainerLayout::Default);
        app.cycle_setting(true);
        assert_eq!(app.config.container_layout, crate::config::ContainerLayout::RecentFirst);
        app.cycle_setting(false);
        assert_eq!(app.config.container_layout, crate::config::ContainerLayout::Default);

        // 2. MidPanelOrder (idx 2)
        app.settings_selected_idx = 2;
        assert_eq!(app.config.mid_panel_order, crate::config::MidPanelOrder::HistoryLogs);
        app.cycle_setting(true);
        assert_eq!(app.config.mid_panel_order, crate::config::MidPanelOrder::LogsHistory);
        app.cycle_setting(true);
        assert_eq!(app.config.mid_panel_order, crate::config::MidPanelOrder::HistoryLogs);

        // 3. BorderStyle (idx 3)
        app.settings_selected_idx = 3;
        assert_eq!(app.config.border_style, crate::config::BorderStyleChoice::Rounded);
        assert_eq!(app.border_type(), BorderType::Rounded);
        app.cycle_setting(true);
        assert_eq!(app.config.border_style, crate::config::BorderStyleChoice::Sharp);
        assert_eq!(app.border_type(), BorderType::Plain);

        // 4. GraphStyle (idx 4)
        app.settings_selected_idx = 4;
        assert_eq!(app.config.graph_style, crate::config::GraphStyleChoice::Blocks);
        app.cycle_setting(true);
        assert_eq!(app.config.graph_style, crate::config::GraphStyleChoice::Braille);
        app.cycle_setting(true);
        assert_eq!(app.config.graph_style, crate::config::GraphStyleChoice::Blocks);
    }

    #[tokio::test]
    async fn test_settings_tab_switching() {
        let mut app = App::new();
        app.modal = Modal::Settings;
        assert_eq!(app.settings_tab, 0);
        assert_eq!(app.settings_selected_idx, 0);

        // Press Tab to switch to Tab 1 (UI)
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.settings_tab, 1);
        assert_eq!(app.settings_selected_idx, 0);

        // Press Tab again to switch back to Tab 0 (Rclone)
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.settings_tab, 0);

        // Direct switch with key '2'
        app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
        assert_eq!(app.settings_tab, 1);

        // Direct switch with key '1'
        app.handle_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
        assert_eq!(app.settings_tab, 0);
    }

    #[tokio::test]
    async fn test_filter_type_cycling() {
        let mut app = App::new();
        app.modal = Modal::Filters;
        app.filters = vec![
            "- /my_folder/**".to_string(),
            "+ *.pdf".to_string(),
            "# ignored comment".to_string(),
        ];
        app.selected_filter_idx = 0;

        // '-' cycles to '#'
        app.cycle_filter_type(0);
        assert_eq!(app.filters[0], "# /my_folder/**");

        // '#' cycles to raw rule
        app.cycle_filter_type(0);
        assert_eq!(app.filters[0], "/my_folder/**");

        // raw rule cycles to '+'
        app.cycle_filter_type(0);
        assert_eq!(app.filters[0], "+ /my_folder/**");

        // '+' cycles back to '-'
        app.cycle_filter_type(0);
        assert_eq!(app.filters[0], "- /my_folder/**");

        // Test via key 't'
        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
        assert_eq!(app.filters[0], "# /my_folder/**");

        // Test via key ' ' (Space)
        app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(app.filters[0], "/my_folder/**");
    }

    #[tokio::test]
    async fn test_filter_inline_editing() {
        let mut app = App::new();
        app.modal = Modal::Filters;
        app.filters = vec![
            "- /test/**".to_string(),
            "+ *.txt".to_string(),
        ];
        app.selected_filter_idx = 0;

        // Press 'e' or Enter to start editing
        app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(app.is_editing_filter());
        assert!(!app.is_adding_filter());
        assert_eq!(app.edit_buffer(), "- /test/**");

        // Type additional characters
        app.handle_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
        assert_eq!(app.edit_buffer(), "- /test/**1");

        // Backspace
        app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.edit_buffer(), "- /test/**");

        // Commit with Enter
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!app.is_editing_filter());
        assert_eq!(app.filters[0], "- /test/**");

        // Start editing and cancel with Esc
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.is_editing_filter());
        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(app.edit_buffer(), "- /test/**x");
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.is_editing_filter());
        assert_eq!(app.filters[0], "- /test/**"); // Reverted!
    }

    #[tokio::test]
    async fn test_filter_adding_and_deleting() {
        let mut app = App::new();
        app.modal = Modal::Filters;
        app.filters = vec![
            "- /old/**".to_string(),
        ];
        app.selected_filter_idx = 0;

        // Press 'a' to add a new filter
        app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        assert!(app.is_editing_filter());
        assert!(app.is_adding_filter());
        assert_eq!(app.edit_buffer(), "- ");

        // Type rule content
        for c in "custom/**".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(app.edit_buffer(), "- custom/**");

        // Commit with Enter
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!app.is_editing_filter());
        assert_eq!(app.filters.len(), 2);
        assert_eq!(app.filters[1], "- custom/**");
        assert_eq!(app.selected_filter_idx, 1);

        // Delete with 'd'
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(app.filters.len(), 1);
        assert_eq!(app.filters[0], "- /old/**");
        assert_eq!(app.selected_filter_idx, 0);
    }

    #[tokio::test]
    async fn test_filter_mouse_actions() {
        let mut app = App::new();
        app.modal = Modal::Filters;
        app.filters = vec![
            "- /row0/**".to_string(),
            "+ /row1/**".to_string(),
        ];
        app.selected_filter_idx = 0;

        // HitAction::FilterAdd
        app.hit_mgr.dashboard = vec![
            Hitbox {
                rect: ratatui::layout::Rect { x: 50, y: 10, width: 10, height: 1 },
                action: HitAction::FilterAdd,
            },
        ];
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 52,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert!(app.is_editing_filter());
        assert!(app.is_adding_filter());

        app.cancel_filter_edit();

        // HitAction::FilterDelete
        app.hit_mgr.dashboard = vec![
            Hitbox {
                rect: ratatui::layout::Rect { x: 50, y: 12, width: 10, height: 1 },
                action: HitAction::FilterDelete(1),
            },
        ];
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 52,
            row: 12,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert_eq!(app.filters.len(), 1);
        assert_eq!(app.filters[0], "- /row0/**");
    }

    #[tokio::test]
    async fn test_timer_interval_options_cycle_forward_and_backward() {
        let mut app = App::new();
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 0; // timer_interval
        app.config.timer_interval = "10min".to_string();

        // Forward cycle: 10min -> 15min -> 30min -> 1h -> 2h -> 4h -> 10min
        app.cycle_setting(true);
        assert_eq!(app.config.timer_interval, "15min");
        app.cycle_setting(true);
        assert_eq!(app.config.timer_interval, "30min");
        app.cycle_setting(true);
        assert_eq!(app.config.timer_interval, "1h");
        app.cycle_setting(true);
        assert_eq!(app.config.timer_interval, "2h"); // Visited 2h!
        app.cycle_setting(true);
        assert_eq!(app.config.timer_interval, "4h"); // Visited 4h!
        app.cycle_setting(true);
        assert_eq!(app.config.timer_interval, "10min");

        // Backward cycle: 10min -> 4h -> 2h -> 1h -> 30min -> 15min -> 10min
        app.cycle_setting(false);
        assert_eq!(app.config.timer_interval, "4h");
        app.cycle_setting(false);
        assert_eq!(app.config.timer_interval, "2h");
        app.cycle_setting(false);
        assert_eq!(app.config.timer_interval, "1h");
        app.cycle_setting(false);
        assert_eq!(app.config.timer_interval, "30min");
        app.cycle_setting(false);
        assert_eq!(app.config.timer_interval, "15min");
        app.cycle_setting(false);
        assert_eq!(app.config.timer_interval, "10min");
    }

    #[tokio::test]
    async fn test_click_outside_modal_dismissal() {
        let mut app = App::new();
        app.modal = Modal::Settings;
        app.hit_mgr.active_modal_area = Some(ratatui::layout::Rect { x: 20, y: 10, width: 40, height: 20 });

        // Background hitbox that shouldn't be clicked when modal is open
        app.hit_mgr.dashboard = vec![
            Hitbox {
                rect: ratatui::layout::Rect { x: 5, y: 5, width: 10, height: 1 },
                action: HitAction::ButtonSync,
            },
        ];

        // Click outside the active modal area at (5, 5)
        let action = app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 5,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });

        // The modal should be dismissed, and ButtonSync must NOT be executed!
        assert_eq!(app.modal, Modal::None);
        assert_eq!(action, Action::None);

        // Re-open modal and click inside active_modal_area at (25, 15)
        app.modal = Modal::Settings;
        app.hit_mgr.active_modal_area = Some(ratatui::layout::Rect { x: 20, y: 10, width: 40, height: 20 });
        let action2 = app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 25,
            row: 15,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        // Modal is NOT dismissed by click inside
        assert_eq!(app.modal, Modal::Settings);
        assert_eq!(action2, Action::None);
    }

    #[tokio::test]
    async fn test_modal_scroll_isolation() {
        let mut app = App::new();
        app.modal = Modal::Filters;
        app.filters = vec![
            "- /row0/**".to_string(),
            "+ /row1/**".to_string(),
            "- /row2/**".to_string(),
        ];
        app.selected_filter_idx = 0;
        app.logs_scroll = 0;

        // ScrollDown while Filters modal is open
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollDown,
            column: 10,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });

        // Filters selection scrolled to 1, while background logs remained 0!
        assert_eq!(app.selected_filter_idx, 1);
        assert_eq!(app.logs_scroll, 0);

        // ScrollUp while Filters modal is open
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollUp,
            column: 10,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert_eq!(app.selected_filter_idx, 0);
        assert_eq!(app.logs_scroll, 0);
    }

    #[tokio::test]
    async fn test_universal_scroll_primitives_and_scrollbar_harmony() {
        // 1. Direct unit test of scroll_list_step
        let mut sel = 0;
        let mut offset = 0;
        let total = 20;
        let vp = 5;

        // Scroll down 4 times (within viewport)
        for expected in 1..=4 {
            App::scroll_list_step(&mut sel, &mut offset, total, vp, false);
            assert_eq!(sel, expected);
            assert_eq!(offset, 0);
        }

        // Scroll down 5th time (hits viewport bottom, offset shifts)
        App::scroll_list_step(&mut sel, &mut offset, total, vp, false);
        assert_eq!(sel, 5);
        assert_eq!(offset, 1);

        // Scroll up
        App::scroll_list_step(&mut sel, &mut offset, total, vp, true);
        assert_eq!(sel, 4);
        assert_eq!(offset, 1);

        // 2. Direct unit test of scroll_list_jump
        App::scroll_list_jump(&mut sel, &mut offset, total, vp, 1.0);
        assert_eq!(offset, 15); // total(20) - vp(5) = 15
        assert_eq!(sel, 19);

        App::scroll_list_jump(&mut sel, &mut offset, total, vp, 0.0);
        assert_eq!(offset, 0);
        assert_eq!(sel, 0);

        // 3. Test ScrollbarTarget::Filters step and jump via App
        let mut app = App::new();
        app.filters = (0..20).map(|i| format!("- rule_{}", i)).collect();
        app.filter_viewport_height = 5;
        app.selected_filter_idx = 0;
        app.filter_scroll_offset = 0;

        app.apply_scrollbar_step(ScrollbarTarget::Filters, false);
        assert_eq!(app.selected_filter_idx, 1);

        app.apply_scrollbar_drag_to_row(ScrollbarTarget::Filters, 5, 0, 10, 20, 5, 1);
        assert!(app.filter_scroll_offset > 0);
        assert!(app.selected_filter_idx >= app.filter_scroll_offset);
    }

    #[tokio::test]
    async fn test_scrollbar_drag_mouse_tracking_precision() {
        let mut app = App::new();
        app.modal = Modal::Filters;
        app.filters = (0..50).map(|i| format!("- rule_{}", i)).collect();
        app.filter_viewport_height = 10;
        app.selected_filter_idx = 0;
        app.filter_scroll_offset = 0;

        let total = 50;
        let visible = 10;
        let top_y = 5;
        let track_height = 20;

        // 1. Initial click on thumb (thumb size = round(10/50 * 20) = 4, thumb at 0..4)
        // Click at row 6 -> click_offset = 1 (inside thumb 0..4)
        let grab_offset = app.compute_scrollbar_grab_offset(
            ScrollbarTarget::Filters,
            6,
            top_y,
            track_height,
            total,
            visible,
        );
        assert_eq!(grab_offset, 1);

        // 2. Drag mouse down to row 10 (moved down by 4 rows)
        app.apply_scrollbar_drag_to_row(
            ScrollbarTarget::Filters,
            10,
            top_y,
            track_height,
            total,
            visible,
            grab_offset,
        );
        // available_travel = 20 - 4 = 16.
        // target_thumb_start = (10 - 5 - 1).min(16) = 4.
        // ratio = 4 / 16 = 0.25.
        // max_scroll = 50 - 10 = 40.
        // new_offset = round(0.25 * 40) = 10.
        assert_eq!(app.filter_scroll_offset, 10);
        // The selection should NOT jump to the bottom of the viewport; it clamps to visible range [10..20)
        assert_eq!(app.selected_filter_idx, 10);

        // 3. Drag further to bottom boundary: row 25
        app.apply_scrollbar_drag_to_row(
            ScrollbarTarget::Filters,
            25,
            top_y,
            track_height,
            total,
            visible,
            grab_offset,
        );
        assert_eq!(app.filter_scroll_offset, 40);
        assert_eq!(app.selected_filter_idx, 49);

        // 4. Test immediate mouse wheel view scrolling on Filters modal
        app.filter_scroll_offset = 5;
        app.selected_filter_idx = 7;
        app.scroll_filter_view_down(10);
        assert_eq!(app.filter_scroll_offset, 6);
        assert_eq!(app.selected_filter_idx, 8);

        app.scroll_filter_view_up(10);
        assert_eq!(app.filter_scroll_offset, 5);
        assert_eq!(app.selected_filter_idx, 7);
    }

    #[tokio::test]
    async fn test_settings_modal_height_adaptation_and_no_overflow() {
        let mut app = App::new();
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 3; // Rclone stats interval

        // 1. Écran de taille moyenne (100x30) : logo masqué pour laisser 25 lignes à la modale
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| crate::ui::render(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let rendered: String = (0..buffer.area.height)
            .map(|y| (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        // Le premier et le dernier élément (30s) doivent impérativement être visibles dans le rendu
        assert!(rendered.contains("1s"), "Le premier élément 1s doit être rendu");
        assert!(rendered.contains("30s"), "Le dernier élément 30s doit être présent sans dépassement");

        // 2. Grand écran (120x40) : logo affiché et modale complète
        let backend_large = TestBackend::new(120, 40);
        let mut terminal_large = Terminal::new(backend_large).unwrap();
        terminal_large.draw(|f| crate::ui::render(f, &mut app)).unwrap();
        let buffer_large = terminal_large.backend().buffer();
        let rendered_large: String = (0..buffer_large.area.height)
            .map(|y| (0..buffer_large.area.width).map(|x| buffer_large[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered_large.contains("30s"), "Le dernier élément 30s doit être présent sur grand écran");
    }

    #[tokio::test]
    async fn test_first_run_modal_flow() {
        let mut app = App::new();
        let mut first_run = FirstRunState::new(&app.config.remote);
        first_run.client_id.clear();
        first_run.client_secret.clear();
        first_run.step = FirstRunStep::RcloneCheck;
        first_run.rclone_status = RcloneInstallStatus::Installed("rclone v1.68.0".to_string());
        first_run.active_field = FirstRunField::ContinueButton;
        app.modal = Modal::FirstRun(Box::new(first_run));

        // 1. Press Enter to transition to Google Credentials step
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        app.handle_key(enter);

        match &app.modal {
            Modal::FirstRun(st) => {
                assert_eq!(st.step, FirstRunStep::GoogleCredentials);
                assert_eq!(st.active_field, FirstRunField::ClientIdInput);
            }
            _ => panic!("Expected Modal::FirstRun in GoogleCredentials step"),
        }

        // 2. Type into Client ID
        app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));

        // 3. Tab to Client Secret
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        match &app.modal {
            Modal::FirstRun(st) => {
                assert_eq!(st.client_id, "abc");
                assert_eq!(st.active_field, FirstRunField::ClientSecretInput);
            }
            _ => panic!("Expected ClientSecretInput"),
        }

        // 4. Type into Client Secret
        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

        // 5. Tab to Save button
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        match &app.modal {
            Modal::FirstRun(st) => {
                assert_eq!(st.client_secret, "xy");
                assert_eq!(st.active_field, FirstRunField::SaveCredentialsButton);
            }
            _ => panic!("Expected SaveCredentialsButton"),
        }

        // 6. Enter to Save & Complete
        app.handle_key(enter);
        assert_eq!(app.modal, Modal::None);
        assert_eq!(app.config.first_run_completed, Some(true));
    }

    #[tokio::test]
    async fn test_first_run_buttons_arrow_navigation() {
        let mut app = App::new();
        let mut first_run = FirstRunState::new(&app.config.remote);
        first_run.step = FirstRunStep::GoogleCredentials;
        first_run.active_field = FirstRunField::SaveCredentialsButton;
        app.modal = Modal::FirstRun(Box::new(first_run));

        // On line 3, press Right -> SkipCredentialsButton
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        match &app.modal {
            Modal::FirstRun(st) => assert_eq!(st.active_field, FirstRunField::SkipCredentialsButton),
            _ => panic!("Expected Modal::FirstRun"),
        }

        // Press Right -> ToggleHelpButton
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        match &app.modal {
            Modal::FirstRun(st) => assert_eq!(st.active_field, FirstRunField::ToggleHelpButton),
            _ => panic!("Expected Modal::FirstRun"),
        }

        // Press Right -> wraps to SaveCredentialsButton
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        match &app.modal {
            Modal::FirstRun(st) => assert_eq!(st.active_field, FirstRunField::SaveCredentialsButton),
            _ => panic!("Expected Modal::FirstRun"),
        }

        // Press Left -> wraps to ToggleHelpButton
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        match &app.modal {
            Modal::FirstRun(st) => assert_eq!(st.active_field, FirstRunField::ToggleHelpButton),
            _ => panic!("Expected Modal::FirstRun"),
        }

        // Press Left -> SkipCredentialsButton
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        match &app.modal {
            Modal::FirstRun(st) => assert_eq!(st.active_field, FirstRunField::SkipCredentialsButton),
            _ => panic!("Expected Modal::FirstRun"),
        }

        // Press Up from buttons -> moves to ClientSecretInput (line 2)
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        match &app.modal {
            Modal::FirstRun(st) => assert_eq!(st.active_field, FirstRunField::ClientSecretInput),
            _ => panic!("Expected Modal::FirstRun"),
        }

        // Press Down from line 2 -> moves to SaveCredentialsButton (line 3)
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        match &app.modal {
            Modal::FirstRun(st) => assert_eq!(st.active_field, FirstRunField::SaveCredentialsButton),
            _ => panic!("Expected Modal::FirstRun"),
        }
    }

    #[tokio::test]
    async fn test_settings_google_client_secret_render_no_panic() {
        let mut app = App::new();
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 9; // GoogleClientSecret

        // Set credentials so that secret is configured ("••••••••••••")
        let _ = config::write_rclone_credentials(&app.config.remote, "test-client-id", "test-client-secret-12345");

        let backend = ratatui::backend::TestBackend::new(100, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        // Must render without any panic
        terminal.draw(|f| {
            crate::ui::render(f, &mut app);
        }).unwrap();

        // Clean up
        let _ = config::write_rclone_credentials(&app.config.remote, "", "");
    }

    #[tokio::test]
    async fn test_toggle_boxes_and_focus_adjustment() {
        let mut app = App::new();
        app.config.show_cadrans = true;
        app.config.show_metrics = true;
        app.config.show_history = true;
        app.config.show_logs = true;
        app.config.show_recent = true;
        let _ = crate::config::save_config(&app.config);

        assert!(app.is_box_visible(1));
        assert!(app.is_box_visible(2));
        assert!(app.is_box_visible(3));
        assert!(app.is_box_visible(4));
        assert!(app.is_box_visible(5));
        assert!(app.any_box_visible());

        // Toggle box 1 off (disks & cloud)
        app.toggle_box(1);
        assert!(!app.is_box_visible(1));
        assert!(app.any_box_visible());

        // Toggle box 2 off (metrics)
        app.toggle_box(2);
        assert!(!app.is_box_visible(2));
        assert!(app.any_box_visible());

        // Set focus to History (box 3), then toggle it off -> focus should shift to next visible panel (Logs)
        app.focused_panel = FocusedPanel::History;
        app.toggle_box(3);
        assert!(!app.is_box_visible(3));
        assert_eq!(app.focused_panel, FocusedPanel::Logs);

        // Toggle remaining boxes off
        app.toggle_box(4);
        app.toggle_box(5);
        assert!(!app.any_box_visible());

        // Navigation when all are hidden
        app.focused_panel = FocusedPanel::Logs;
        app.next_visible_panel();
        assert_eq!(app.focused_panel, FocusedPanel::Logs);
        app.prev_visible_panel();
        assert_eq!(app.focused_panel, FocusedPanel::Logs);

        // Turn boxes 3 and 5 back on
        app.toggle_box(3);
        app.toggle_box(5);
        assert!(app.any_box_visible());
        app.focused_panel = FocusedPanel::History;
        app.next_visible_panel();
        assert_eq!(app.focused_panel, FocusedPanel::RecentFiles);
        app.next_visible_panel();
        assert_eq!(app.focused_panel, FocusedPanel::History);
        app.prev_visible_panel();
        assert_eq!(app.focused_panel, FocusedPanel::RecentFiles);

        // Clean up: restore all boxes to visible
        app.config.show_cadrans = true;
        app.config.show_metrics = true;
        app.config.show_history = true;
        app.config.show_logs = true;
        app.config.show_recent = true;
        let _ = crate::config::save_config(&app.config);
    }

    #[tokio::test]
    async fn test_empty_dashboard_rendering_no_panic() {
        let mut app = App::new();
        // Hide all 5 boxes
        app.config.show_cadrans = false;
        app.config.show_metrics = false;
        app.config.show_history = false;
        app.config.show_logs = false;
        app.config.show_recent = false;
        assert!(!app.any_box_visible());

        let backend = ratatui::backend::TestBackend::new(120, 35);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        terminal.draw(|f| {
            crate::ui::render(f, &mut app);
        }).unwrap();

        // Hitboxes should contain ToggleBox(1..=5) from the empty dashboard
        let toggle_actions: Vec<_> = app.hit_mgr.dashboard.iter().filter_map(|hb| {
            if let HitAction::ToggleBox(n) = hb.action {
                Some(n)
            } else {
                None
            }
        }).collect();

        assert!(toggle_actions.contains(&1));
        assert!(toggle_actions.contains(&2));
        assert!(toggle_actions.contains(&3));
        assert!(toggle_actions.contains(&4));
        assert!(toggle_actions.contains(&5));
    }

    #[tokio::test]
    async fn test_partial_dashboard_layouts() {
        let mut app = App::new();
        app.service_info.state = ServiceState::Idle;
        app.live.is_syncing = false;
        let area = Rect::new(0, 0, 100, 40);

        // Case 1: Only history visible
        app.config.show_cadrans = false;
        app.config.show_metrics = false;
        app.config.show_history = true;
        app.config.show_logs = false;
        app.config.show_recent = false;
        let layout = crate::ui::dashboard::compute_dashboard_layout(area, &app);
        assert_eq!(layout.cadrans_area.height, 0);
        assert_eq!(layout.history_area.height, 40);
        assert_eq!(layout.logs_area.height, 0);
        assert_eq!(layout.recent_area.height, 0);

        // Case 2: Only bottom row (logs + recent) visible
        app.config.show_cadrans = false;
        app.config.show_metrics = false;
        app.config.show_history = false;
        app.config.show_logs = true;
        app.config.show_recent = true;
        let layout2 = crate::ui::dashboard::compute_dashboard_layout(area, &app);
        assert_eq!(layout2.cadrans_area.height, 0);
        assert_eq!(layout2.history_area.height, 0);
        assert!(layout2.logs_area.height > 0);
        assert!(layout2.recent_area.height > 0);
        assert_eq!(layout2.logs_area.height + layout2.recent_area.height, 40);
    }

    #[tokio::test]
    async fn test_log_filter_persistence() {
        let mut app = App::new();
        // Set log filter to Problems
        app.set_log_filter(LogFilter::Problems);
        assert_eq!(app.log_filter, LogFilter::Problems);
        assert_eq!(app.config.log_filter, LogFilter::Problems);

        // Load config from disk and verify it is restored
        let loaded = crate::config::load_config();
        assert_eq!(loaded.log_filter, LogFilter::Problems);

        // Change to Files
        app.set_log_filter(LogFilter::Files);
        assert_eq!(app.log_filter, LogFilter::Files);
        let loaded2 = crate::config::load_config();
        assert_eq!(loaded2.log_filter, LogFilter::Files);

        // Clean up: reset to All and remove test config file
        app.set_log_filter(LogFilter::All);
        let _ = std::fs::remove_file(crate::config::config_file());
    }

    #[tokio::test]
    async fn test_timer_cycle_remaining_and_progress() {
        let mut app = App::new();

        // 1. Timer cycle seconds parsing
        app.config.timer_interval = "10min".to_string();
        assert_eq!(app.timer_cycle_seconds(), 600);
        app.config.timer_interval = "15min".to_string();
        assert_eq!(app.timer_cycle_seconds(), 900);
        app.config.timer_interval = "30min".to_string();
        assert_eq!(app.timer_cycle_seconds(), 1800);
        app.config.timer_interval = "1h".to_string();
        assert_eq!(app.timer_cycle_seconds(), 3600);
        app.config.timer_interval = "2h".to_string();
        assert_eq!(app.timer_cycle_seconds(), 7200);
        app.config.timer_interval = "4h".to_string();
        assert_eq!(app.timer_cycle_seconds(), 14400);

        // 2. Timer remaining seconds parsing
        app.config.timer_interval = "10min".to_string();
        app.service_info.timer_left = "4m 12s".to_string();
        assert_eq!(app.timer_remaining_seconds(), Some(252));

        app.service_info.timer_left = "12s left".to_string();
        assert_eq!(app.timer_remaining_seconds(), Some(12));

        app.service_info.timer_left = "9min left".to_string();
        assert_eq!(app.timer_remaining_seconds(), Some(540));

        app.service_info.timer_left = "imminent".to_string();
        assert_eq!(app.timer_remaining_seconds(), Some(0));

        app.service_info.timer_left = "after sync (10min)".to_string();
        assert_eq!(app.timer_remaining_seconds(), Some(600));

        app.service_info.timer_left = "Disabled".to_string();
        assert_eq!(app.timer_remaining_seconds(), None);

        app.service_info.timer_left = "--".to_string();
        assert_eq!(app.timer_remaining_seconds(), None);

        // 3. Timer progress ratio
        app.service_info.timer_left = "4m 12s".to_string(); // 252s rem / 600s cycle = 0.42 remaining -> 0.58 progress
        let prog = app.timer_progress().unwrap();
        assert!((prog - 0.58).abs() < 0.01);

        app.service_info.timer_left = "imminent".to_string();
        assert_eq!(app.timer_progress(), Some(1.0));

        app.service_info.timer_left = "Disabled".to_string();
        assert_eq!(app.timer_progress(), None);
    }

    #[tokio::test]
    async fn test_pulse_area_layout_geometry() {
        let mut app = App::new();
        app.config.show_cadrans = true;
        app.config.show_metrics = true;
        app.config.show_history = true;
        app.config.show_logs = true;
        app.config.show_recent = true;
        app.service_info.state = crate::systemd::ServiceState::Idle;
        app.live.is_syncing = false;

        // Normal dashboard height: pulse_area height should be 1
        let area = Rect { x: 0, y: 0, width: 120, height: 40 };
        let layout = crate::ui::dashboard::compute_dashboard_layout(area, &app);
        assert_eq!(layout.pulse_area.height, 1);
        assert_eq!(layout.pulse_area.y, layout.cadrans_area.y + layout.cadrans_area.height);

        // When cadrans are hidden: pulse_area height should be 0
        app.config.show_cadrans = false;
        app.config.show_metrics = false;
        let layout2 = crate::ui::dashboard::compute_dashboard_layout(area, &app);
        assert_eq!(layout2.pulse_area.height, 0);

        // When terminal is tiny (< 14 lines): pulse_area should collapse to 0
        app.config.show_cadrans = true;
        let tiny_area = Rect { x: 0, y: 0, width: 120, height: 12 };
        let layout3 = crate::ui::dashboard::compute_dashboard_layout(tiny_area, &app);
        assert_eq!(layout3.pulse_area.height, 0);
    }

    #[tokio::test]
    async fn test_render_pulse_line_no_panic() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        use crate::ui::dashboard::render_pulse_line;

        let mut app = App::new();
        let theme = app.current_theme.palette();
        let backend = TestBackend::new(100, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        // 1. Idle mode with countdown (Blocks)
        app.config.graph_style = crate::config::GraphStyleChoice::Blocks;
        app.service_info.timer_left = "3m 45s".to_string();
        terminal.draw(|f| {
            render_pulse_line(f, &app, &theme, Rect { x: 0, y: 0, width: 100, height: 1 });
        }).unwrap();
        let buffer_blocks: String = terminal.backend().buffer().content().iter().map(|c| c.symbol()).collect();
        assert!(buffer_blocks.contains('█') || buffer_blocks.contains('·'));

        // 1b. Idle mode with countdown (Braille)
        app.config.graph_style = crate::config::GraphStyleChoice::Braille;
        terminal.draw(|f| {
            render_pulse_line(f, &app, &theme, Rect { x: 0, y: 0, width: 100, height: 1 });
        }).unwrap();
        let buffer_braille: String = terminal.backend().buffer().content().iter().map(|c| c.symbol()).collect();
        assert!(buffer_braille.contains('⣿') || buffer_braille.contains('·'));

        // 2. Idle mode with disabled timer
        app.service_info.timer_left = "Disabled".to_string();
        terminal.draw(|f| {
            render_pulse_line(f, &app, &theme, Rect { x: 0, y: 0, width: 100, height: 1 });
        }).unwrap();

        // 3. Syncing mode - indeterminate (scan/diff)
        app.live.is_syncing = true;
        app.live.transfer.files_done = 0;
        app.live.transfer.bytes_done = String::new();
        terminal.draw(|f| {
            render_pulse_line(f, &app, &theme, Rect { x: 0, y: 0, width: 100, height: 1 });
        }).unwrap();

        // 4. Syncing mode - active transfer with percentage
        app.live.transfer.bytes_done = "50 MB".to_string();
        app.live.transfer.bytes_total = "100 MB".to_string();
        app.live.transfer.files_done = 2;
        app.live.transfer.speed = "12 MB/s".to_string();
        terminal.draw(|f| {
            render_pulse_line(f, &app, &theme, Rect { x: 0, y: 0, width: 100, height: 1 });
        }).unwrap();

        // 5. Very compact area
        terminal.draw(|f| {
            render_pulse_line(f, &app, &theme, Rect { x: 0, y: 0, width: 25, height: 1 });
        }).unwrap();

        // 6. Test various widths across idle and sync to verify no panic or overflow
        for w in [15, 25, 35, 45, 55, 65, 80, 100, 120, 160] {
            let b = TestBackend::new(w, 1);
            let mut t = Terminal::new(b).unwrap();
            // Idle
            app.live.is_syncing = false;
            app.service_info.timer_left = "4m 12s".to_string();
            t.draw(|f| {
                render_pulse_line(f, &app, &theme, Rect { x: 0, y: 0, width: w, height: 1 });
            }).unwrap();

            // Sync indeterminate
            app.live.is_syncing = true;
            app.live.transfer.files_done = 0;
            app.live.transfer.bytes_done = String::new();
            t.draw(|f| {
                render_pulse_line(f, &app, &theme, Rect { x: 0, y: 0, width: w, height: 1 });
            }).unwrap();

            // Sync with progress
            app.live.transfer.files_done = 3;
            app.live.transfer.bytes_done = "40 MB".to_string();
            t.draw(|f| {
                render_pulse_line(f, &app, &theme, Rect { x: 0, y: 0, width: w, height: 1 });
            }).unwrap();
        }
    }

    #[tokio::test]
    async fn test_sync_phase_progression_and_path_modified() {
        use crate::monitor::streamer::StreamerState;

        let mut app = App::new();
        let mut state = StreamerState::default();

        // 1. Initial sync state when starting: must start at phase 0 (Listings), NOT phase 3 (Applying)
        assert_eq!(state.phase_index, 0);
        assert!(!state.path1_modified);
        assert!(!state.path2_modified);

        // 2. Path modified detection
        state.path1_modified = true;
        app.live = state.clone();
        assert!(app.live.path1_modified);
        assert!(!app.live.path2_modified);

        // 3. Render active sync section with path modified: must not crash
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let backend = TestBackend::new(120, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = app.current_theme.palette();

        app.live.is_syncing = true;
        terminal.draw(|f| {
            crate::ui::dashboard::sync::render_active_sync_section(f, &app, &theme, Rect { x: 0, y: 0, width: 120, height: 10 });
        }).unwrap();

        // 4. Render active sync section in Phase 3 (Applying) with an active file
        app.live.phase_index = 3;
        app.live.active_files.insert(
            "Debout - Leo Succulent.mp3".to_string(),
            crate::monitor::parser::ActiveFile {
                name: "aaaa/réveil/Debout - Leo Succulent.mp3".to_string(),
                pct: 29,
                speed: "60.104 KiB/s".to_string(),
                last_seen: std::time::Instant::now(),
            },
        );
        terminal.draw(|f| {
            crate::ui::dashboard::sync::render_active_sync_section(f, &app, &theme, Rect { x: 0, y: 0, width: 120, height: 10 });
        }).unwrap();
    }

    #[tokio::test]
    async fn test_dry_run_reopening_and_enhanced_modal_rendering() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        use crate::ui::popups::render_popups;
        use crate::ui::theme::ThemeChoice;

        let mut app = App::new();
        let theme = ThemeChoice::TokyoNight.palette();
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        // 1. Initially, pressing 'd' opens DryRun directly
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::DryRun);

        // 2. Start dry-run: pressing Enter inside DryRun starts dry_run
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::DryRun);
        assert!(app.dry_run_running);

        // 3. User closes modal to return to dashboard (Esc or q)
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);
        assert!(app.dry_run_running);

        // 4. On dashboard, pressing 'd' must REOPEN DryRun modal!
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::DryRun);

        // 5. Clicking DryRun button also reopens DryRun modal
        app.modal = Modal::None;
        app.execute_hit_action(HitAction::ButtonDryRun, false, 0);
        assert_eq!(app.modal, Modal::DryRun);

        // 6. Pressing 'r' while dry-run is running should not launch another run
        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::DryRun);

        // 7. Render modal while running (empty / initial logs)
        let mut hitboxes = Vec::new();
        terminal.draw(|f| {
            render_popups(f, &app, &theme, &mut hitboxes);
        }).unwrap();

        // 8. Populate logs with realistic dry-run output (Path1 and Path2 changes)
        app.dry_run_running = false;
        app.dry_run_logs = vec![
            "2026/09/20 18:00:00 INFO  : - Path2    File is new - document.pdf".to_string(),
            "2026/09/20 18:00:00 INFO  : - Path2    File was deleted - old.txt".to_string(),
            "2026/09/20 18:00:00 INFO  : - Path1    File is new - remote.png".to_string(),
            "2026/09/20 18:00:00 INFO  : - Path1    File changed - config.json".to_string(),
            "2026/09/20 18:00:00 INFO  : Checks:                150 / 150, 100%".to_string(),
            "2026/09/20 18:00:00 INFO  : Transferred:   1.234 MiB / 1.234 MiB, 100%".to_string(),
            "2026/09/20 18:00:00 INFO  : Elapsed time:        2.5s".to_string(),
            "2026/09/20 18:00:00 INFO  : Bisync successful".to_string(),
        ];

        // 9. Render modal with populated summary and logs
        hitboxes.clear();
        terminal.draw(|f| {
            render_popups(f, &app, &theme, &mut hitboxes);
        }).unwrap();

        // 10. Verify scroll interaction
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.dry_run_scroll, 1);
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.dry_run_scroll, 0);

        // 11. When finished, pressing 'd' from dashboard opens DryRun modal directly
        app.modal = Modal::None;
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::DryRun);
    }

    #[test]
    fn test_no_dead_code_or_unused_imports() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");

        // 1. Verify that no #[allow(dead_code)] or #[allow(unused...)] exists in any source file
        let src_dir = std::path::Path::new(manifest_dir).join("src");
        fn collect_rs_files(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        collect_rs_files(&path, files);
                    } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                        files.push(path);
                    }
                }
            }
        }

        let mut rs_files = Vec::new();
        collect_rs_files(&src_dir, &mut rs_files);
        assert!(!rs_files.is_empty(), "Expected to find .rs files in src/");

        let mut allow_violations = Vec::new();
        for file_path in rs_files {
            let content = std::fs::read_to_string(&file_path).unwrap_or_default();
            for (line_no, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                    continue;
                }
                if trimmed.starts_with("#[allow(") && (trimmed.contains("dead_code") || trimmed.contains("unused")) {
                    allow_violations.push(format!("{}:{}: {}", file_path.display(), line_no + 1, trimmed));
                }
            }
        }

        assert!(
            allow_violations.is_empty(),
            "Found #[allow(dead_code)] or #[allow(unused...)] attributes in source files:\n{}",
            allow_violations.join("\n")
        );

        // 2. Verify with cargo check --message-format=json --all-targets that there are zero dead_code or unused warnings
        let output = std::process::Command::new("cargo")
            .current_dir(manifest_dir)
            .args(["check", "--message-format=json", "--all-targets"])
            .output()
            .expect("Failed to execute cargo check");

        assert!(output.status.success(), "cargo check --all-targets failed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut compiler_violations = Vec::new();

        for line in stdout.lines() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                if val["reason"] == "compiler-message" {
                    let code = val["message"]["code"]["code"].as_str().unwrap_or("");
                    let msg = val["message"]["message"].as_str().unwrap_or("");
                    let rendered = val["message"]["rendered"].as_str().unwrap_or("");

                    if code == "dead_code"
                        || code.starts_with("unused")
                        || msg.contains("dead_code")
                        || msg.contains("unused import")
                        || msg.contains("unused variable")
                    {
                        compiler_violations.push(rendered.to_string());
                    }
                }
            }
        }

        assert!(
            compiler_violations.is_empty(),
            "Found dead code or unused imports/variables:\n{}",
            compiler_violations.join("\n")
        );

        // 3. Verify with cargo clippy --all-targets -- -D warnings that there are zero linter warnings
        let clippy_output = std::process::Command::new("cargo")
            .current_dir(manifest_dir)
            .args(["clippy", "--all-targets", "--", "-D", "warnings"])
            .output()
            .expect("Failed to execute cargo clippy");

        assert!(
            clippy_output.status.success(),
            "cargo clippy failed:\n{}",
            String::from_utf8_lossy(&clippy_output.stderr)
        );
    }
