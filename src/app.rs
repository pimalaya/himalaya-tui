use std::path::PathBuf;

use anyhow::{bail, Result};
use edtui::{EditorMode, EditorState, Lines};
use mml::template::{self, TemplateCursor};
use pimalaya_toolbox::config::TomlConfig;

use crate::config::{Config, ImapConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Mailboxes,
    Envelopes,
    Message,
    Compose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomPanelMode {
    None,
    Message,
    Compose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposeAction {
    SaveToDrafts,
    Abandon,
}

impl ComposeAction {
    pub const ALL: [ComposeAction; 2] = [ComposeAction::SaveToDrafts, ComposeAction::Abandon];

    pub fn label(&self) -> &'static str {
        match self {
            ComposeAction::SaveToDrafts => "Save to Drafts",
            ComposeAction::Abandon => "Abandon",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Mailbox {
    pub name: String,
    pub delimiter: Option<char>,
    pub subscribed: bool,
}

#[derive(Debug, Clone)]
pub struct Envelope {
    pub uid: u32,
    pub date: String,
    pub from: String,
    pub subject: String,
    pub flags: Vec<String>,
}

pub struct App {
    pub running: bool,
    pub active_panel: Panel,
    pub mailboxes: Vec<Mailbox>,
    pub mailbox_index: usize,
    pub envelopes: Vec<Envelope>,
    pub envelope_index: usize,
    pub selected_mailbox: Option<String>,
    pub account_name: String,
    pub email: String,
    pub display_name: String,
    pub signature: String,
    pub imap_config: ImapConfig,
    pub status_message: Option<String>,

    // Message viewing
    pub bottom_panel_mode: BottomPanelMode,
    pub message_content: Option<String>,
    pub message_scroll: u16,

    // Message composition
    pub editor_state: EditorState,
    pub show_compose_dialog: bool,
    pub compose_dialog_index: usize,

    // Ctrl-C tracking for Ctrl-C Ctrl-C
    pub ctrl_c_pending: bool,
}

impl App {
    pub fn new(config_paths: &[PathBuf], account_name: Option<&str>) -> Result<Self> {
        let config = Config::from_paths_or_default(config_paths)?;
        let (name, account_config) = config.get_account(account_name)?;
        let Some(imap_config) = account_config.imap else {
            bail!("IMAP config is missing for this account")
        };

        let email = account_config.email.clone();
        let display_name = account_config
            .display_name
            .or(config.display_name)
            .unwrap_or_default();
        let signature = account_config
            .signature
            .or(config.signature)
            .unwrap_or_default();

        Ok(Self {
            running: true,
            active_panel: Panel::Mailboxes,
            mailboxes: Vec::new(),
            mailbox_index: 0,
            envelopes: Vec::new(),
            envelope_index: 0,
            selected_mailbox: None,
            account_name: name,
            email,
            display_name,
            signature,
            imap_config,
            status_message: Some("Loading mailboxes...".to_string()),
            bottom_panel_mode: BottomPanelMode::None,
            message_content: None,
            message_scroll: 0,
            editor_state: EditorState::new(Lines::from("")),
            show_compose_dialog: false,
            compose_dialog_index: 0,
            ctrl_c_pending: false,
        })
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Close the current "frame" - returns true if something was closed
    pub fn close_current(&mut self) -> bool {
        match self.active_panel {
            Panel::Message | Panel::Compose => {
                self.close_bottom_panel();
                true
            }
            Panel::Envelopes if self.bottom_panel_mode != BottomPanelMode::None => {
                self.close_bottom_panel();
                true
            }
            _ => false,
        }
    }

    pub fn toggle_panel(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::Mailboxes => Panel::Envelopes,
            Panel::Envelopes => {
                if self.bottom_panel_mode == BottomPanelMode::Message {
                    Panel::Message
                } else if self.bottom_panel_mode == BottomPanelMode::Compose {
                    Panel::Compose
                } else {
                    Panel::Mailboxes
                }
            }
            Panel::Message => Panel::Mailboxes,
            Panel::Compose => Panel::Mailboxes,
        };
    }

    pub fn next_item(&mut self) {
        match self.active_panel {
            Panel::Mailboxes => {
                if !self.mailboxes.is_empty() {
                    self.mailbox_index = (self.mailbox_index + 1) % self.mailboxes.len();
                }
            }
            Panel::Envelopes => {
                if !self.envelopes.is_empty() {
                    self.envelope_index = (self.envelope_index + 1) % self.envelopes.len();
                }
            }
            Panel::Message => {
                self.message_scroll = self.message_scroll.saturating_add(1);
            }
            Panel::Compose => {}
        }
    }

    pub fn previous_item(&mut self) {
        match self.active_panel {
            Panel::Mailboxes => {
                if !self.mailboxes.is_empty() {
                    self.mailbox_index = self
                        .mailbox_index
                        .checked_sub(1)
                        .unwrap_or(self.mailboxes.len() - 1);
                }
            }
            Panel::Envelopes => {
                if !self.envelopes.is_empty() {
                    self.envelope_index = self
                        .envelope_index
                        .checked_sub(1)
                        .unwrap_or(self.envelopes.len() - 1);
                }
            }
            Panel::Message => {
                self.message_scroll = self.message_scroll.saturating_sub(1);
            }
            Panel::Compose => {}
        }
    }

    pub fn select_mailbox(&mut self) {
        let mailbox_name = self
            .mailboxes
            .get(self.mailbox_index)
            .map(|m| m.name.clone());

        if let Some(name) = mailbox_name {
            self.selected_mailbox = Some(name.clone());
            self.envelope_index = 0;
            self.envelopes.clear();
            self.close_bottom_panel();
            self.status_message = Some(format!("Loading envelopes from {}...", name));
        }
    }

    pub fn set_mailboxes(&mut self, mailboxes: Vec<Mailbox>) {
        self.mailboxes = mailboxes;
        self.mailbox_index = 0;
        if !self.mailboxes.is_empty() {
            self.select_mailbox();
        }
        self.status_message = None;
    }

    pub fn set_envelopes(&mut self, envelopes: Vec<Envelope>) {
        self.envelopes = envelopes;
        self.envelope_index = 0;
        self.status_message = None;
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    pub fn show_message(&mut self, content: String) {
        self.message_content = Some(content);
        self.message_scroll = 0;
        self.bottom_panel_mode = BottomPanelMode::Message;
        self.active_panel = Panel::Message;
    }

    pub fn close_bottom_panel(&mut self) {
        self.bottom_panel_mode = BottomPanelMode::None;
        self.message_content = None;
        self.show_compose_dialog = false;
        self.ctrl_c_pending = false;
        if self.active_panel == Panel::Message || self.active_panel == Panel::Compose {
            self.active_panel = Panel::Envelopes;
        }
    }

    pub fn start_compose(&mut self) {
        let tpl = template::new::build(template::new::BuildNewTemplateArgs {
            from: self.email.clone(),
            from_name: self.display_name.clone(),
            signature: self.signature.clone(),
            ..Default::default()
        });

        match tpl {
            Ok(tpl) => self.open_editor_with_template(&tpl.content, &tpl.cursor),
            Err(err) => self.set_status(format!("Error building template: {err}")),
        }
    }

    pub fn start_reply(&mut self, raw_message: &[u8]) {
        let Some(msg) = mail_parser::MessageParser::default().parse(raw_message) else {
            self.set_status("Error: failed to parse message");
            return;
        };

        let tpl = template::reply::build(
            &msg,
            template::reply::BuildReplyTemplateArgs {
                from: self.email.clone(),
                from_name: self.display_name.clone(),
                signature: self.signature.clone(),
                ..Default::default()
            },
        );

        match tpl {
            Ok(tpl) => self.open_editor_with_template(&tpl.content, &tpl.cursor),
            Err(err) => self.set_status(format!("Error building reply template: {err}")),
        }
    }

    pub fn start_forward(&mut self, raw_message: &[u8]) {
        let Some(msg) = mail_parser::MessageParser::default().parse(raw_message) else {
            self.set_status("Error: failed to parse message");
            return;
        };

        let tpl = template::forward::build(
            &msg,
            template::forward::BuildForwardTemplateArgs {
                from: self.email.clone(),
                from_name: self.display_name.clone(),
                signature: self.signature.clone(),
                ..Default::default()
            },
        );

        match tpl {
            Ok(tpl) => self.open_editor_with_template(&tpl.content, &tpl.cursor),
            Err(err) => self.set_status(format!("Error building forward template: {err}")),
        }
    }

    fn open_editor_with_template(&mut self, content: &str, cursor: &TemplateCursor) {
        let mut state = EditorState::new(Lines::from(content));
        state.mode = EditorMode::Insert;
        state.cursor = edtui::Index2::new(cursor.row.saturating_sub(1), cursor.col);
        self.editor_state = state;
        self.bottom_panel_mode = BottomPanelMode::Compose;
        self.active_panel = Panel::Compose;
        self.show_compose_dialog = false;
        self.ctrl_c_pending = false;
    }

    pub fn finish_compose(&mut self) {
        self.show_compose_dialog = true;
        self.compose_dialog_index = 0;
    }

    pub fn get_compose_content(&self) -> String {
        self.editor_state.lines.to_string()
    }

    pub fn cancel_compose(&mut self) {
        self.show_compose_dialog = false;
        self.close_bottom_panel();
    }

    pub fn get_selected_envelope(&self) -> Option<&Envelope> {
        self.envelopes.get(self.envelope_index)
    }

    // Dialog navigation
    pub fn dialog_next(&mut self) {
        self.compose_dialog_index = (self.compose_dialog_index + 1) % ComposeAction::ALL.len();
    }

    pub fn dialog_previous(&mut self) {
        self.compose_dialog_index = self
            .compose_dialog_index
            .checked_sub(1)
            .unwrap_or(ComposeAction::ALL.len() - 1);
    }

    pub fn get_selected_compose_action(&self) -> ComposeAction {
        ComposeAction::ALL[self.compose_dialog_index]
    }
}
