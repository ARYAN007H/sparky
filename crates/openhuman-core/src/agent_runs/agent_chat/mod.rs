//! Full agent chat turns and their shared turn guards.

#[cfg(test)]
#[path = "session_turn_tests.rs"]
mod tests;

pub(crate) mod guards;
mod session_turn;

pub(crate) use session_turn::{
    agent_chat, agent_chat_for, agent_chat_reply_for, AgentChatEnv, AgentChatReply,
    AgentChatTarget,
};
