//! The thread as the user saw it, in the contract's vocabulary.

use tinymemes_bus::{WireRole, WireTurn};

use crate::threads::store::ConversationMessage;

/// Whether a finished reply is worth remixing at all.
pub(super) fn should_remix(reply: &str, placeholder: &str) -> bool {
    reply != placeholder && !reply.trim().is_empty()
}

/// The thread as the user saw it, oldest first: delivered replies (remixed or
/// not) and the user's messages. `current_user_message` is appended unless it
/// is already the last stored turn (the store may have persisted it first).
pub(super) fn history_turns(
    messages: &[ConversationMessage],
    current_user_message: &str,
    is_remixed: impl Fn(&str) -> bool,
) -> Vec<WireTurn> {
    let mut turns: Vec<WireTurn> = messages
        .iter()
        .filter(|m| !m.content.trim().is_empty())
        .filter_map(|m| match m.sender.as_str() {
            "user" => Some(WireTurn {
                role: WireRole::User,
                text: m.content.clone(),
                remixed: false,
            }),
            "agent" | "assistant" => Some(WireTurn {
                role: WireRole::Assistant,
                text: m.content.clone(),
                remixed: is_remixed(&m.id),
            }),
            _ => None,
        })
        .collect();
    let current = current_user_message.trim();
    let already_there = turns
        .last()
        .is_some_and(|t| t.role == WireRole::User && t.text.trim() == current);
    if !current.is_empty() && !already_there {
        turns.push(WireTurn {
            role: WireRole::User,
            text: current.to_owned(),
            remixed: false,
        });
    }
    turns
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
