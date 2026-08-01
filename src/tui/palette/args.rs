//! Argument mode for the palette: once the input's head token exactly
//! names an available command with candidates, the rest of the input
//! filters those candidates instead of the registry.

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};

use crate::tui::{
    command::{COMMANDS, Candidate, CandidateSource},
    model::Model,
};

pub struct ArgumentQuery {
    pub source: CandidateSource,
    /// Canonical command token, re-emitted on completion.
    pub head: &'static str,
    pub fragment: String,
}

/// Argument-mode parse of the palette input: `Some` only when the
/// head token exactly matches an id or alias of an available command
/// that takes an argument.
pub fn argument_query(model: &Model) -> Option<ArgumentQuery> {
    let input = model.palette.as_ref()?.input.value();
    let (head, fragment) = input.split_once(char::is_whitespace)?;
    let command = COMMANDS
        .iter()
        .find(|c| c.tokens().any(|token| token == head))
        .filter(|c| (c.is_available)(model))?;
    Some(ArgumentQuery {
        source: command.arg?,
        head: command.id,
        fragment: fragment.trim_start().to_string(),
    })
}

/// The command's candidates ranked against the fragment; an empty
/// fragment keeps the provider's order.
pub fn candidates(model: &Model, query: &ArgumentQuery) -> Vec<Candidate> {
    let all = (query.source)(model);
    if query.fragment.is_empty() {
        return all;
    }
    let pattern = Pattern::parse(&query.fragment, CaseMatching::Ignore, Normalization::Smart);
    super::MATCHER.with_borrow_mut(|matcher| {
        pattern
            .match_list(all, matcher)
            .into_iter()
            .map(|(candidate, _)| candidate)
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;
    use crate::{
        email::{envelope::Envelope, mailbox::Mailbox},
        tui::{
            model::{Dialog, FlagAction, Message},
            palette::{PaletteMessage, update},
        },
    };

    fn mailbox(name: &str) -> Mailbox {
        Mailbox {
            id: name.into(),
            name: name.into(),
            total: None,
            unread: None,
        }
    }

    /// Envelope commands available; transfer targets INBOX, Archive, Sent.
    fn transfer_ready_model() -> Model {
        let mut model = Model {
            mailboxes: ["INBOX", "Archive", "Sent"].map(mailbox).to_vec(),
            ..Model::default()
        };
        model.envelopes.push(Envelope::stub());
        model
    }

    fn listed_candidates(model: &Model) -> Vec<String> {
        let query = argument_query(model).expect("input should be in argument mode");
        candidates(model, &query)
            .into_iter()
            .map(|candidate| candidate.label)
            .collect()
    }

    fn open_palette(model: &mut Model) {
        update(model, PaletteMessage::Open);
    }

    fn type_filter(model: &mut Model, text: &str) {
        for c in text.chars() {
            update(
                model,
                PaletteMessage::Input(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)),
            );
        }
    }

    #[test]
    fn tab_on_argument_command_appends_a_space() {
        let mut model = transfer_ready_model();
        open_palette(&mut model);
        type_filter(&mut model, "cop");

        update(&mut model, PaletteMessage::Complete);
        let palette = model.palette.as_ref().unwrap();
        assert_eq!(palette.input.value(), "copy ");
        assert_eq!(palette.selected, 0);
        assert_eq!(listed_candidates(&model), ["INBOX", "Archive", "Sent"]);
    }

    #[test]
    fn tab_completes_argument_and_confirm_dispatches_it() {
        let mut model = transfer_ready_model();
        open_palette(&mut model);
        type_filter(&mut model, "copy ar");
        assert_eq!(listed_candidates(&model), ["Archive"]);

        update(&mut model, PaletteMessage::Complete);
        let palette = model.palette.as_ref().unwrap();
        assert_eq!(palette.input.value(), "copy Archive");

        let confirmed = update(&mut model, PaletteMessage::Confirm);
        assert!(
            matches!(confirmed, Some(Message::CopySelectedTo(ref name)) if name == "Archive"),
            "expected CopySelectedTo(Archive), got {confirmed:?}"
        );
        assert!(model.palette.is_none());
    }

    #[test]
    fn argument_selection_navigates_candidates() {
        let mut model = transfer_ready_model();
        open_palette(&mut model);
        type_filter(&mut model, "move ");

        update(&mut model, PaletteMessage::Next);
        let confirmed = update(&mut model, PaletteMessage::Confirm);
        assert!(
            matches!(confirmed, Some(Message::MoveSelectedTo(ref name)) if name == "Archive"),
            "expected MoveSelectedTo(Archive), got {confirmed:?}"
        );
    }

    #[test]
    fn flag_argument_dispatches_the_action() {
        let mut model = transfer_ready_model();
        open_palette(&mut model);
        type_filter(&mut model, "add-flag se");

        assert_eq!(listed_candidates(&model)[0], "Seen");
        let confirmed = update(&mut model, PaletteMessage::Confirm);
        assert!(matches!(
            confirmed,
            Some(Message::FlagSelected {
                add: true,
                action: FlagAction::Seen
            })
        ));
    }

    #[test]
    fn unavailable_command_never_enters_argument_mode() {
        // No envelope is selected, so `copy` is greyed out.
        let mut model = Model {
            mailboxes: ["INBOX"].map(mailbox).to_vec(),
            ..Model::default()
        };
        open_palette(&mut model);
        type_filter(&mut model, "copy INBOX");

        assert!(argument_query(&model).is_none());
        let confirmed = update(&mut model, PaletteMessage::Confirm);
        assert!(confirmed.is_none());
        assert!(model.palette.is_some());
    }

    #[test]
    fn switch_account_argument_dispatches_the_switch() {
        let mut model = Model {
            config_source: Some(Vec::new()),
            account_names: vec!["personal".into(), "work".into()],
            ..Model::default()
        };
        open_palette(&mut model);
        type_filter(&mut model, "switch-account wo");

        assert_eq!(listed_candidates(&model), ["work"]);
        let confirmed = update(&mut model, PaletteMessage::Confirm);
        assert!(
            matches!(confirmed, Some(Message::SwitchAccount(ref name)) if name == "work"),
            "expected SwitchAccount(work), got {confirmed:?}"
        );
    }

    #[test]
    fn switch_account_without_config_source_stays_in_command_mode() {
        let mut model = Model {
            account_names: vec!["work".into()],
            ..Model::default()
        };
        open_palette(&mut model);
        type_filter(&mut model, "switch-account work");

        assert!(argument_query(&model).is_none());
    }

    #[test]
    fn confirm_on_bare_argument_command_opens_the_dialog() {
        let mut model = transfer_ready_model();
        open_palette(&mut model);
        type_filter(&mut model, "copy");

        let confirmed = update(&mut model, PaletteMessage::Confirm);
        assert!(matches!(
            confirmed,
            Some(Message::OpenDialog(Dialog::CopyTo))
        ));
    }
}
