//! OpenHuman's user-facing copy for generic TinyAgents session operations.
//!
//! Rendering, classification and validation live in TinyAgents. This module
//! supplies the product wording at those seams.

/// OpenHuman copy for bounded summaries of a turn stopped by a breaker.
pub(crate) const OPENHUMAN_STOP_COPY: tinyagents_harness::turn_stop::StopCopy =
    tinyagents_harness::turn_stop::StopCopy {
        opening_template: "I stopped this turn early because {reason}.\n",
        what_happened_heading: "\n**What happened**\n",
        success_template: "- `{name}` succeeded{count}",
        failure_template: "- `{name}` failed{count}{error}",
        hidden_calls_template: "- …and {count} more tool call(s)",
        proceed_prompt: "\nTell me how you'd like to proceed, or ask me to try again.",
    };

/// OpenHuman's prompt and fallback wording for generic TinyAgents close paths.
pub(crate) const OPENHUMAN_CLOSE_COPY: tinyagents_harness::close::CloseCopy =
    tinyagents_harness::close::ENGLISH_CLOSE_COPY;
