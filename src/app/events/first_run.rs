use super::*;

impl App {
    pub(super) fn handle_key_first_run(&mut self, key: &KeyEvent) -> Action {
        if let Modal::FirstRun(ref mut state) = self.modal {
            enum FirstRunKeyAction {
                None,
                InstallRclone,
                SaveCredentials(String, String, String),
                SkipCredentials,
                AdvanceRemote,
                SkipAll,
            }

            let mut action_to_do = FirstRunKeyAction::None;

            match state.step {
                FirstRunStep::RcloneCheck => match key.code {
                    KeyCode::Enter => match state.rclone_status {
                        RcloneInstallStatus::Installed(_) => {
                            state.step = FirstRunStep::RemoteSetup;
                            state.active_field = FirstRunField::RemoteInput;
                            state.remote_cursor = state.remote_input.chars().count();
                        }
                        RcloneInstallStatus::NotInstalled | RcloneInstallStatus::Failed(_) => {
                            if state.active_field == FirstRunField::SkipRcloneButton {
                                state.step = FirstRunStep::RemoteSetup;
                                state.active_field = FirstRunField::RemoteInput;
                                state.remote_cursor = state.remote_input.chars().count();
                            } else {
                                action_to_do = FirstRunKeyAction::InstallRclone;
                            }
                        }
                        RcloneInstallStatus::Installing => {}
                    },
                    KeyCode::Tab | KeyCode::Right | KeyCode::Down => {
                        if !matches!(state.rclone_status, RcloneInstallStatus::Installed(_) | RcloneInstallStatus::Installing) {
                            state.active_field = match state.active_field {
                                FirstRunField::InstallRcloneButton => FirstRunField::SkipRcloneButton,
                                _ => FirstRunField::InstallRcloneButton,
                            };
                        }
                    }
                    KeyCode::BackTab | KeyCode::Left | KeyCode::Up => {
                        if !matches!(state.rclone_status, RcloneInstallStatus::Installed(_) | RcloneInstallStatus::Installing) {
                            state.active_field = match state.active_field {
                                FirstRunField::SkipRcloneButton => FirstRunField::InstallRcloneButton,
                                _ => FirstRunField::SkipRcloneButton,
                            };
                        }
                    }
                    KeyCode::Esc => {
                        state.step = FirstRunStep::RemoteSetup;
                        state.active_field = FirstRunField::RemoteInput;
                        state.remote_cursor = state.remote_input.chars().count();
                    }
                    _ => {}
                },
                FirstRunStep::RemoteSetup => match key.code {
                    KeyCode::Tab => {
                        state.active_field = match state.active_field {
                            FirstRunField::RemoteInput => FirstRunField::RemoteContinueButton,
                            FirstRunField::RemoteContinueButton => FirstRunField::RemoteSkipButton,
                            FirstRunField::RemoteSkipButton => FirstRunField::RemoteInput,
                            _ => FirstRunField::RemoteInput,
                        };
                    }
                    KeyCode::BackTab => {
                        state.active_field = match state.active_field {
                            FirstRunField::RemoteInput => FirstRunField::RemoteSkipButton,
                            FirstRunField::RemoteContinueButton => FirstRunField::RemoteInput,
                            FirstRunField::RemoteSkipButton => FirstRunField::RemoteContinueButton,
                            _ => FirstRunField::RemoteInput,
                        };
                    }
                    KeyCode::Down => {
                        state.active_field = match state.active_field {
                            FirstRunField::RemoteInput => FirstRunField::RemoteContinueButton,
                            _ => FirstRunField::RemoteInput,
                        };
                    }
                    KeyCode::Up => {
                        state.active_field = match state.active_field {
                            FirstRunField::RemoteContinueButton | FirstRunField::RemoteSkipButton => {
                                FirstRunField::RemoteInput
                            }
                            _ => FirstRunField::RemoteContinueButton,
                        };
                    }
                    KeyCode::Enter => match state.active_field {
                        FirstRunField::RemoteInput => {
                            state.active_field = FirstRunField::RemoteContinueButton;
                        }
                        FirstRunField::RemoteContinueButton => {
                            action_to_do = FirstRunKeyAction::AdvanceRemote;
                        }
                        FirstRunField::RemoteSkipButton => {
                            action_to_do = FirstRunKeyAction::SkipAll;
                        }
                        _ => {}
                    },
                    KeyCode::Esc => {
                        action_to_do = FirstRunKeyAction::SkipAll;
                    }
                    KeyCode::Left => {
                        if state.active_field == FirstRunField::RemoteInput && state.remote_cursor > 0 {
                            state.remote_cursor -= 1;
                        } else {
                            state.active_field = match state.active_field {
                                FirstRunField::RemoteContinueButton => FirstRunField::RemoteSkipButton,
                                FirstRunField::RemoteSkipButton => FirstRunField::RemoteContinueButton,
                                _ => state.active_field,
                            };
                        }
                    }
                    KeyCode::Right => {
                        if state.active_field == FirstRunField::RemoteInput
                            && state.remote_cursor < state.remote_input.chars().count()
                        {
                            state.remote_cursor += 1;
                        } else {
                            state.active_field = match state.active_field {
                                FirstRunField::RemoteContinueButton => FirstRunField::RemoteSkipButton,
                                FirstRunField::RemoteSkipButton => FirstRunField::RemoteContinueButton,
                                _ => state.active_field,
                            };
                        }
                    }
                    KeyCode::Home => {
                        if state.active_field == FirstRunField::RemoteInput {
                            state.remote_cursor = 0;
                        }
                    }
                    KeyCode::End => {
                        if state.active_field == FirstRunField::RemoteInput {
                            state.remote_cursor = state.remote_input.chars().count();
                        }
                    }
                    KeyCode::Backspace => {
                        if state.active_field == FirstRunField::RemoteInput {
                            backspace_at_cursor(&mut state.remote_input, &mut state.remote_cursor);
                        }
                    }
                    KeyCode::Delete => {
                        if state.active_field == FirstRunField::RemoteInput {
                            delete_at_cursor(&mut state.remote_input, state.remote_cursor);
                        }
                    }
                    KeyCode::Char(c) => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) && (c == 'v' || c == 'V') {
                            if state.active_field == FirstRunField::RemoteInput {
                                if let Some(clip) = crate::clipboard::paste_from_clipboard() {
                                    let clean = clip.trim().replace(['\r', '\n'], "");
                                    insert_str_at_cursor(&mut state.remote_input, &mut state.remote_cursor, &clean);
                                }
                            }
                        } else if !key.modifiers.contains(KeyModifiers::CONTROL)
                            && state.active_field == FirstRunField::RemoteInput
                        {
                            insert_char_at_cursor(&mut state.remote_input, &mut state.remote_cursor, c);
                        }
                    }
                    _ => {}
                },
                FirstRunStep::GoogleCredentials => match key.code {
                    KeyCode::Tab => {
                        state.active_field = match state.active_field {
                            FirstRunField::ClientIdInput => FirstRunField::ClientSecretInput,
                            FirstRunField::ClientSecretInput => FirstRunField::SaveCredentialsButton,
                            FirstRunField::SaveCredentialsButton => FirstRunField::SkipCredentialsButton,
                            FirstRunField::SkipCredentialsButton => FirstRunField::ToggleHelpButton,
                            FirstRunField::ToggleHelpButton => FirstRunField::ClientIdInput,
                            _ => FirstRunField::ClientIdInput,
                        };
                    }
                    KeyCode::BackTab => {
                        state.active_field = match state.active_field {
                            FirstRunField::ClientIdInput => FirstRunField::ToggleHelpButton,
                            FirstRunField::ClientSecretInput => FirstRunField::ClientIdInput,
                            FirstRunField::SaveCredentialsButton => FirstRunField::ClientSecretInput,
                            FirstRunField::SkipCredentialsButton => FirstRunField::SaveCredentialsButton,
                            FirstRunField::ToggleHelpButton => FirstRunField::SkipCredentialsButton,
                            _ => FirstRunField::ClientIdInput,
                        };
                    }
                    KeyCode::Down => {
                        state.active_field = match state.active_field {
                            FirstRunField::ClientIdInput => FirstRunField::ClientSecretInput,
                            FirstRunField::ClientSecretInput => FirstRunField::SaveCredentialsButton,
                            FirstRunField::SaveCredentialsButton
                            | FirstRunField::SkipCredentialsButton
                            | FirstRunField::ToggleHelpButton => FirstRunField::ClientIdInput,
                            _ => FirstRunField::ClientIdInput,
                        };
                    }
                    KeyCode::Up => {
                        state.active_field = match state.active_field {
                            FirstRunField::ClientIdInput => FirstRunField::SaveCredentialsButton,
                            FirstRunField::ClientSecretInput => FirstRunField::ClientIdInput,
                            FirstRunField::SaveCredentialsButton
                            | FirstRunField::SkipCredentialsButton
                            | FirstRunField::ToggleHelpButton => FirstRunField::ClientSecretInput,
                            _ => FirstRunField::ClientIdInput,
                        };
                    }
                    KeyCode::Enter => match state.active_field {
                        FirstRunField::ClientIdInput => {
                            state.active_field = FirstRunField::ClientSecretInput;
                        }
                        FirstRunField::ClientSecretInput => {
                            state.active_field = FirstRunField::SaveCredentialsButton;
                        }
                        FirstRunField::SaveCredentialsButton => {
                            action_to_do = FirstRunKeyAction::SaveCredentials(
                                state.remote_input.clone(),
                                state.client_id.clone(),
                                state.client_secret.clone(),
                            );
                        }
                        FirstRunField::SkipCredentialsButton => {
                            action_to_do = FirstRunKeyAction::SkipCredentials;
                        }
                        FirstRunField::ToggleHelpButton => {
                            state.show_help = !state.show_help;
                        }
                        _ => {}
                    },
                    KeyCode::Esc => {
                        action_to_do = FirstRunKeyAction::SkipCredentials;
                    }
                    KeyCode::Char('?') | KeyCode::Char('h')
                        if state.active_field != FirstRunField::ClientIdInput
                            && state.active_field != FirstRunField::ClientSecretInput =>
                    {
                        state.show_help = !state.show_help;
                    }
                    KeyCode::Left => match state.active_field {
                        FirstRunField::ClientIdInput => {
                            state.client_id_cursor = state.client_id_cursor.saturating_sub(1);
                        }
                        FirstRunField::ClientSecretInput => {
                            state.client_secret_cursor = state.client_secret_cursor.saturating_sub(1);
                        }
                        FirstRunField::SaveCredentialsButton => {
                            state.active_field = FirstRunField::ToggleHelpButton;
                        }
                        FirstRunField::SkipCredentialsButton => {
                            state.active_field = FirstRunField::SaveCredentialsButton;
                        }
                        FirstRunField::ToggleHelpButton => {
                            state.active_field = FirstRunField::SkipCredentialsButton;
                        }
                        _ => {}
                    },
                    KeyCode::Right => match state.active_field {
                        FirstRunField::ClientIdInput => {
                            if state.client_id_cursor < state.client_id.chars().count() {
                                state.client_id_cursor += 1;
                            }
                        }
                        FirstRunField::ClientSecretInput => {
                            if state.client_secret_cursor < state.client_secret.chars().count() {
                                state.client_secret_cursor += 1;
                            }
                        }
                        FirstRunField::SaveCredentialsButton => {
                            state.active_field = FirstRunField::SkipCredentialsButton;
                        }
                        FirstRunField::SkipCredentialsButton => {
                            state.active_field = FirstRunField::ToggleHelpButton;
                        }
                        FirstRunField::ToggleHelpButton => {
                            state.active_field = FirstRunField::SaveCredentialsButton;
                        }
                        _ => {}
                    },
                    KeyCode::Home => match state.active_field {
                        FirstRunField::ClientIdInput => state.client_id_cursor = 0,
                        FirstRunField::ClientSecretInput => state.client_secret_cursor = 0,
                        _ => {}
                    },
                    KeyCode::End => match state.active_field {
                        FirstRunField::ClientIdInput => state.client_id_cursor = state.client_id.chars().count(),
                        FirstRunField::ClientSecretInput => state.client_secret_cursor = state.client_secret.chars().count(),
                        _ => {}
                    },
                    KeyCode::Backspace => match state.active_field {
                        FirstRunField::ClientIdInput => {
                            backspace_at_cursor(&mut state.client_id, &mut state.client_id_cursor);
                        }
                        FirstRunField::ClientSecretInput => {
                            backspace_at_cursor(&mut state.client_secret, &mut state.client_secret_cursor);
                        }
                        _ => {}
                    },
                    KeyCode::Delete => match state.active_field {
                        FirstRunField::ClientIdInput => {
                            delete_at_cursor(&mut state.client_id, state.client_id_cursor);
                        }
                        FirstRunField::ClientSecretInput => {
                            delete_at_cursor(&mut state.client_secret, state.client_secret_cursor);
                        }
                        _ => {}
                    },
                    KeyCode::Char(c) => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) && (c == 'v' || c == 'V') {
                            if let Some(clip) = crate::clipboard::paste_from_clipboard() {
                                let clean = clip.trim().replace(['\r', '\n'], "");
                                match state.active_field {
                                    FirstRunField::ClientIdInput => {
                                        insert_str_at_cursor(&mut state.client_id, &mut state.client_id_cursor, &clean);
                                    }
                                    FirstRunField::ClientSecretInput => {
                                        insert_str_at_cursor(&mut state.client_secret, &mut state.client_secret_cursor, &clean);
                                    }
                                    _ => {}
                                }
                            }
                        } else if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            match state.active_field {
                                FirstRunField::ClientIdInput => {
                                    insert_char_at_cursor(&mut state.client_id, &mut state.client_id_cursor, c);
                                }
                                FirstRunField::ClientSecretInput => {
                                    insert_char_at_cursor(&mut state.client_secret, &mut state.client_secret_cursor, c);
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                },
            }

            match action_to_do {
                FirstRunKeyAction::InstallRclone => {
                    self.start_rclone_install();
                }
                FirstRunKeyAction::SaveCredentials(remote, id, sec) => {
                    self.save_first_run_credentials(&remote, &id, &sec);
                }
                FirstRunKeyAction::AdvanceRemote => {
                    self.advance_from_remote_continue();
                }
                FirstRunKeyAction::SkipCredentials => {
                    self.config.first_run_completed = Some(true);
                    let _ = config::save_config(&self.config);
                    self.modal = Modal::None;
                    self.hit_mgr.active_modal_area = None;
                    self.set_toast("✔ Setup skipped. You can configure credentials anytime in Settings.");
                }
                FirstRunKeyAction::SkipAll => {
                    self.config.first_run_completed = Some(true);
                    let _ = config::save_config(&self.config);
                    self.modal = Modal::None;
                    self.hit_mgr.active_modal_area = None;
                    self.set_toast("✔ Setup skipped. You can configure the remote anytime in Settings.");
                }
                FirstRunKeyAction::None => {}
            }
        }
        Action::None
    }
}
