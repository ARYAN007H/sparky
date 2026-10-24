use super::*;
use crate::config::schema::ChannelVoiceOverride;
use tinychannels_bus::{ChannelRef, ConversationRef};

fn scope(channel: &str, conversation: &str, turn: &str) -> HostVoiceScope {
    HostVoiceScope {
        configured_instance: channel.into(),
        channel: ChannelRef {
            id: tinychannels_bus::provider_id(channel).into(),
            account_id: Some(channel.into()),
        },
        conversation: ConversationRef {
            id: conversation.into(),
            ..ConversationRef::default()
        },
        turn_id: turn.into(),
    }
}

fn policy_request(
    scope: HostVoiceScope,
    voice: bool,
    explicit: bool,
) -> HostVoiceReplyPolicyRequest {
    HostVoiceReplyPolicyRequest {
        scope,
        inbound_has_voice: voice,
        explicitly_requested: explicit,
    }
}

#[test]
fn policy_mode_ignores_module_supplied_inbound_facts() {
    let cfg = ChannelVoiceConfig::default();
    let voice = resolve_policy(
        &cfg,
        &policy_request(scope("telegram", "c", "t"), true, true),
    );
    let text = resolve_policy(
        &cfg,
        &policy_request(scope("telegram", "c", "t"), false, false),
    );
    assert_eq!(voice.mode, HostVoiceReplyMode::Mirror);
    assert_eq!(
        voice, text,
        "the module applies the facts; the host owns the mode"
    );
}

#[test]
fn policy_mode_follows_thread_then_channel_overrides() {
    let mut cfg = ChannelVoiceConfig::default();
    cfg.channels.insert(
        "telegram".into(),
        ChannelVoiceOverride {
            reply_mode: Some(VoiceReplyMode::Always),
            ..Default::default()
        },
    );
    assert_eq!(
        resolve_policy(
            &cfg,
            &policy_request(scope("telegram", "c", "t"), false, false)
        )
        .mode,
        HostVoiceReplyMode::Always
    );
    cfg.threads.insert("c".into(), VoiceReplyMode::Never);
    assert_eq!(
        resolve_policy(
            &cfg,
            &policy_request(scope("telegram", "c", "t"), false, false)
        )
        .mode,
        HostVoiceReplyMode::Never
    );
    assert_eq!(
        resolve_policy(
            &cfg,
            &policy_request(scope("slack", "other", "t"), false, false)
        )
        .mode,
        HostVoiceReplyMode::Mirror
    );
}

#[test]
fn outbound_length_respects_channel_cap() {
    let mut cfg = ChannelVoiceConfig::default();
    cfg.channels.insert(
        "telegram".into(),
        ChannelVoiceOverride {
            max_outbound_secs: Some(2),
            ..Default::default()
        },
    );
    assert!(check_outbound_length(&cfg, "telegram", &"a".repeat(30)).is_ok());
    assert!(check_outbound_length(&cfg, "telegram", &"a".repeat(31)).is_err());
    assert!(check_outbound_length(&cfg, "slack", &"a".repeat(31)).is_ok());
}

#[tokio::test]
async fn incomplete_scope_is_rejected_before_any_voice_work() {
    let host = ChannelVoiceHost {
        config: Arc::new(Config::default()),
    };
    let bad = HostSpeechSynthesisRequest {
        scope: scope("telegram", "", "t"),
        text: "hello".into(),
        voice: None,
        format: None,
    };
    assert!(host.synthesize_speech(bad).await.is_err());
    let bad = HostVoiceTranscriptionRequest {
        scope: HostVoiceScope::default(),
        ..Default::default()
    };
    assert!(host.transcribe_voice(bad).await.is_err());
}

#[tokio::test]
async fn overlong_reply_is_rejected_without_calling_the_backend() {
    let mut config = Config::default();
    config.channel_voice.max_outbound_secs = 1;
    let host = ChannelVoiceHost {
        config: Arc::new(config),
    };
    let err = host
        .synthesize_speech(HostSpeechSynthesisRequest {
            scope: scope("telegram", "c", "t"),
            text: "x".repeat(100),
            voice: None,
            format: None,
        })
        .await
        .unwrap_err();
    assert!(err.to_string().contains("maximum spoken duration"));
}
