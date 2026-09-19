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
    async fn test_tick_rate_stepping() {
        let mut app = App::new();
        app.tick_rate_ms_live = 2000;

        // '-' speeds up (lower ms: 2000 -> 1500 -> 1000)
        let minus_event = KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE);
        app.handle_key(minus_event);
        assert_eq!(app.tick_rate_ms_live, 1500);

        app.handle_key(minus_event);
        assert_eq!(app.tick_rate_ms_live, 1000);

        // '+' slows down (higher ms)
        let plus_event = KeyEvent::new(KeyCode::Char('+'), KeyModifiers::NONE);
        app.handle_key(plus_event);
        assert_eq!(app.tick_rate_ms_live, 1500);

        app.handle_key(plus_event);
        assert_eq!(app.tick_rate_ms_live, 2000);
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
    async fn test_tick_rate_stepping_and_settings_harmony() {
        let mut app = App::new();
        app.tick_rate_ms_live = 1000;

        // Step up
        app.step_tick_rate(false); // ralentit
        assert_eq!(app.tick_rate_ms_live, 1500);

        // Step down
        app.step_tick_rate(true); // accélère
        assert_eq!(app.tick_rate_ms_live, 1000);

        // Cycle via settings
        app.settings_tab = 1;
        app.settings_selected_idx = 5; // Taux de rafraîchissement UI
        app.cycle_setting(true);
        assert_eq!(app.tick_rate_ms_live, 1500);
        assert_eq!(app.config.tick_rate_ms, Some(1500));
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

        // 2. Appuyer sur 'd' -> ouvre la confirmation de dry-run
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::ConfirmDryRun);

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
        assert_eq!(app.modal, Modal::ConfirmDryRun);
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
    async fn test_initial_unselected_and_btop_scroll() {
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
    async fn test_tick_rate_boundaries() {
        let mut app = App::new();
        app.tick_rate_ms_live = 100;
        assert!(!app.can_dec_tick_rate());
        assert!(app.can_inc_tick_rate());

        app.tick_rate_ms_live = 10000;
        assert!(app.can_dec_tick_rate());
        assert!(!app.can_inc_tick_rate());

        app.tick_rate_ms_live = 1000;
        assert!(app.can_dec_tick_rate());
        assert!(app.can_inc_tick_rate());
    }

    #[tokio::test]
    async fn test_settings_resync_modal_trigger() {
        let mut app = App::new();
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 5; // Resynchronisation complète

        // Appuyer sur Entrée doit ouvrir ConfirmResync
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::ConfirmResync);

        // Annuler avec Esc
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        // Rouvrir et tester avec Flèche Droite
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 5;
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

        // 6. Menu btop : 'q' quitte l'application
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
        app.settings_selected_idx = 3; // Local directory
        app.config.local_dir = "/home/user/drive".to_string();

        // Appui sur Entrée pour entrer en mode édition
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

        // Sortie avec Échap : ce qui est écrit dans le champ est sauvegardé (selon demande utilisateur)
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.is_editing_setting());
        assert_eq!(app.config.local_dir, "/home/user/drive/su");

        // Entrée en édition avec 'e', modification et validation avec Entrée
        app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(app.is_editing_setting());
        app.edit_state = EditState::Setting { tab: 0, index: 3, buffer: "/home/new/path".to_string() };
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!app.is_editing_setting());
        assert_eq!(app.config.local_dir, "/home/new/path");

        // Test sur le remote (option 4) : par exemple "GoogleDrive:"
        app.settings_tab = 0;
        app.settings_selected_idx = 4;
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
        assert_eq!(KeybindingRegistry::get_key_str(KeyAction::Validate), "Entrée");
        assert_eq!(KeybindingRegistry::get_key_str(KeyAction::CancelEdit), "Échap");

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

        // Number keys 1-3 jump directly to filter
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('3'),
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

        // Setting 6 opens full logs
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 6;
        let action = app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(action, Action::OpenFullLogs);
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
        app.settings_tab = 1;
        app.settings_selected_idx = 5; // UI loop frequency (22 choices)

        // 1. Écran de taille moyenne (100x30) : logo masqué pour laisser 25 lignes à la modale
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| crate::ui::render(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let rendered: String = (0..buffer.area.height)
            .map(|y| (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        // Le premier et le dernier élément (10000ms) doivent impérativement être visibles dans le rendu
        assert!(rendered.contains("100ms"), "Le premier élément 100ms doit être rendu");
        assert!(rendered.contains("10000ms"), "Le dernier élément 10000ms doit être présent sans dépassement");

        // 2. Grand écran (120x40) : logo affiché et modale complète
        let backend_large = TestBackend::new(120, 40);
        let mut terminal_large = Terminal::new(backend_large).unwrap();
        terminal_large.draw(|f| crate::ui::render(f, &mut app)).unwrap();
        let buffer_large = terminal_large.backend().buffer();
        let rendered_large: String = (0..buffer_large.area.height)
            .map(|y| (0..buffer_large.area.width).map(|x| buffer_large[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered_large.contains("10000ms"), "Le dernier élément 10000ms doit être présent sur grand écran");
    }


