use super::channel_has_approval_surface;
use tinychannels_bus::{Channel, ChannelMessage, SendMessage};

struct ConfiguredChannel {
    approvals: bool,
}

#[async_trait::async_trait]
impl Channel for ConfiguredChannel {
    fn name(&self) -> &str {
        "configured"
    }

    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        Ok(())
    }

    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        Ok(())
    }

    fn supports_chat_approvals(&self) -> bool {
        self.approvals
    }
}

// Sub-issue 2 of #3098: this gate is what decides whether the dispatch
// loop sets an `ApprovalChatContext` (→ gate fires for `Prompt`-class
// tools) versus the legacy bypass (→ tool calls silently allowed).
// Pin the matrix: every chat channel is served by
// `ChannelApprovalSurfaceSubscriber`; channels with no conversational reply
// path must not be scoped, or every parked call would TTL-deny.

#[test]
fn configured_channel_capability_controls_approval_surface() {
    let supports_approval = ConfiguredChannel { approvals: true };
    let no_approval = ConfiguredChannel { approvals: false };
    assert!(channel_has_approval_surface(Some(&supports_approval)));
    assert!(!channel_has_approval_surface(Some(&no_approval)));
    assert!(!channel_has_approval_surface(None));
}
