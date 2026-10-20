use super::*;
use crate::config::schema::ChannelVoiceOverride;
use tinychannels_bus::{ChannelRef, ConversationRef, VoiceNote};

fn scope(channel: &str, conversation: &str, turn: &str) -> HostVoiceScope {
    HostVoiceScope {
        configured_instance: channel.into(),
        channel: ChannelRef {
            id: channel.split(':').next().unwrap_or(channel).into(),
            account_id: Some(channel.into()),
        },
        conversation: ConversationRef {
            id: conversation.into(),
            ..ConversationRef::default()
        },
        turn_id: turn.into(),
    }
}

fn envelope(conversation: &str, message: &str, voice: bool) -> ChannelInboundEnvelope {
    let mut envelope = ChannelInboundEnvelope {
        message_id: message.into(),
        channel: ChannelRef {
            id: "telegram".into(),
            account_id: Some("telegram".into()),
        },
        conversation: ConversationRef {
            id: conversation.into(),
            ..ConversationRef::default()
        },
        ..ChannelInboundEnvelope::default()
    };
    envelope.parts.push(if voice {
        ContentPart::VoiceNote {
            voice_note: VoiceNote::default(),
        }
    } else {
        ContentPart::Text {
            text: "hi".into(),
            markdown_dialect: None,
        }
    });
    envelope
}

#[test]
fn mirror_policy_reports_inbound_voice_from_recorded_envelope() {
    let tenant = crate::core::runtime::Tenant::default();
    record_inbound_envelope(&envelope("c-voice", "m1", true), &tenant);
    record_inbound_envelope(&envelope("c-text", "m2", false), &tenant);
    let cfg = ChannelVoiceConfig::default();
    let voice = resolve_policy(&cfg, &scope("telegram", "c-voice", "m1"), &tenant);
    assert_eq!(voice.mode, HostVoiceReplyMode::Mirror);
    assert!(voice.inbound_has_voice);
    let text = resolve_policy(&cfg, &scope("telegram", "c-text", "m2"), &tenant);
    assert!(!text.inbound_has_voice);
}

#[test]
fn same_conversation_and_turn_ids_do_not_cross_voice_facts_between_accounts() {
    let mut voice = envelope("shared-conversation", "shared-turn", true);
    voice.channel = ChannelRef {
        id: "telegram".into(),
        account_id: Some("telegram:alice".into()),
    };
    let mut text = envelope("shared-conversation", "shared-turn", false);
    text.channel = ChannelRef {
        id: "telegram".into(),
        account_id: Some("telegram:bob".into()),
    };

    let tenant = crate::core::runtime::Tenant::default();
    record_inbound_envelope(&voice, &tenant);
    record_inbound_envelope(&text, &tenant);

    let alice = resolve_policy(
        &ChannelVoiceConfig::default(),
        &scope("telegram:alice", "shared-conversation", "shared-turn"),
        &tenant,
    );
    let bob = resolve_policy(
        &ChannelVoiceConfig::default(),
        &scope("telegram:bob", "shared-conversation", "shared-turn"),
        &tenant,
    );
    assert!(alice.inbound_has_voice, "Alice's voice turn was overwritten by Bob's text turn");
    assert!(!bob.inbound_has_voice);
}

#[test]
fn unknown_turn_defaults_to_no_voice() {
    let policy = resolve_policy(
        &ChannelVoiceConfig::default(),
        &scope("t", "nope", "nope"),
        &crate::core::runtime::Tenant::default(),
    );
    assert!(!policy.inbound_has_voice);
    assert!(!policy.explicitly_requested);
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
        resolve_policy(&cfg, &scope("telegram", "c", "t"), &crate::core::runtime::Tenant::default()).mode,
        HostVoiceReplyMode::Always
    );
    cfg.threads.insert("c".into(), VoiceReplyMode::Never);
    assert_eq!(
        resolve_policy(&cfg, &scope("telegram", "c", "t"), &crate::core::runtime::Tenant::default()).mode,
        HostVoiceReplyMode::Never
    );
    assert_eq!(
        resolve_policy(&cfg, &scope("slack", "other", "t"), &crate::core::runtime::Tenant::default()).mode,
        HostVoiceReplyMode::Mirror
    );
}

#[test]
fn registry_is_bounded() {
    let tenant = crate::core::runtime::Tenant::default();
    for index in 0..(MAX_TRACKED_TURNS + 10) {
        let conversation = ConversationRef {
            id: "bounded".into(),
            ..ConversationRef::default()
        };
        record_turn(
            turn_key(&tenant, &ChannelRef::default(), &conversation, &index.to_string()),
            VoiceTurnFacts::default(),
        );
    }
    assert!(TURNS.lock().len() <= MAX_TRACKED_TURNS);
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
        scope: scope("", "c", "t"),
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
