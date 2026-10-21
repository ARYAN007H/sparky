use super::*;
use crate::config::schema::ChannelVoiceOverride;
use tinychannels_bus::{ChannelRef, ConversationRef, VoiceNote};

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
    record_inbound_envelope(&envelope("c-voice", "m1", true), &tenant, "m1");
    record_inbound_envelope(&envelope("c-text", "m2", false), &tenant, "m2");
    let cfg = ChannelVoiceConfig::default();
    let voice = resolve_policy(&cfg, &scope("telegram", "c-voice", "m1"), &tenant);
    assert_eq!(voice.mode, HostVoiceReplyMode::Mirror);
    assert!(voice.inbound_has_voice);
    let text = resolve_policy(&cfg, &scope("telegram", "c-text", "m2"), &tenant);
    assert!(!text.inbound_has_voice);
}

#[test]
fn voice_facts_are_correlated_with_the_runner_request_id() {
    let tenant = crate::core::runtime::Tenant::default();
    let inbound = envelope("voice-conversation", "provider-message-id", true);
    record_inbound_envelope(
        &inbound,
        &tenant,
        "telegram:provider-message-id:runner-uuid",
    );

    let policy = resolve_policy(
        &ChannelVoiceConfig::default(),
        &scope(
            "telegram",
            "voice-conversation",
            "telegram:provider-message-id:runner-uuid",
        ),
        &tenant,
    );

    assert!(
        policy.inbound_has_voice,
        "reply policy is queried with the runner request ID, not the provider message ID"
    );
    assert!(
        !resolve_policy(
            &ChannelVoiceConfig::default(),
            &scope("telegram", "voice-conversation", "provider-message-id"),
            &tenant,
        )
        .inbound_has_voice,
        "a provider message ID is not an alias for the runner request ID"
    );
    assert!(
        !resolve_policy(
            &ChannelVoiceConfig::default(),
            &scope(
                "telegram",
                "other-conversation",
                "telegram:provider-message-id:runner-uuid"
            ),
            &tenant,
        )
        .inbound_has_voice,
        "voice facts remain bound to the full conversation scope"
    );
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
    record_inbound_envelope(&voice, &tenant, "shared-turn");
    record_inbound_envelope(&text, &tenant, "shared-turn");

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
    assert!(
        alice.inbound_has_voice,
        "Alice's voice turn was overwritten by Bob's text turn"
    );
    assert!(!bob.inbound_has_voice);
}

#[test]
fn voice_facts_require_matching_configured_account_and_full_conversation() {
    let tenant = crate::core::runtime::Tenant {
        profile: Some("voice-scope-profile".into()),
        agent: None,
    };
    let mut voice = envelope("shared", "same-turn", true);
    voice.conversation.thread_id = Some("thread-a".into());
    voice.conversation.topic_id = Some("topic-a".into());
    record_inbound_envelope(&voice, &tenant, "same-turn");

    let config = ChannelVoiceConfig::default();
    let matching = HostVoiceScope {
        conversation: voice.conversation.clone(),
        ..scope("telegram", "shared", "same-turn")
    };
    assert!(resolve_policy(&config, &matching, &tenant).inbound_has_voice);

    let mut other_thread = matching.clone();
    other_thread.conversation.thread_id = Some("thread-b".into());
    assert!(!resolve_policy(&config, &other_thread, &tenant).inbound_has_voice);

    let mut other_topic = matching.clone();
    other_topic.conversation.topic_id = Some("topic-b".into());
    assert!(!resolve_policy(&config, &other_topic, &tenant).inbound_has_voice);

    let mut mismatched_account = matching.clone();
    mismatched_account.configured_instance = "telegram:other".into();
    assert!(!resolve_policy(&config, &mismatched_account, &tenant).inbound_has_voice);

    let mut mismatched_channel = matching;
    mismatched_channel.channel.id = "discord".into();
    assert!(!resolve_policy(&config, &mismatched_channel, &tenant).inbound_has_voice);
}

#[test]
fn same_channel_account_and_turn_ids_do_not_cross_voice_facts_between_tenants() {
    let alice = crate::core::runtime::Tenant {
        profile: Some("profile-alice".into()),
        agent: None,
    };
    let bob = crate::core::runtime::Tenant {
        profile: Some("profile-bob".into()),
        agent: None,
    };
    let envelope_voice = envelope("same-conversation", "same-turn", true);
    let envelope_text = envelope("same-conversation", "same-turn", false);

    record_inbound_envelope(&envelope_voice, &alice, "same-turn");
    record_inbound_envelope(&envelope_text, &bob, "same-turn");

    assert!(
        resolve_policy(
            &ChannelVoiceConfig::default(),
            &scope("telegram", "same-conversation", "same-turn"),
            &alice,
        )
        .inbound_has_voice
    );
    assert!(
        !resolve_policy(
            &ChannelVoiceConfig::default(),
            &scope("telegram", "same-conversation", "same-turn"),
            &bob,
        )
        .inbound_has_voice
    );
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
        resolve_policy(
            &cfg,
            &scope("telegram", "c", "t"),
            &crate::core::runtime::Tenant::default()
        )
        .mode,
        HostVoiceReplyMode::Always
    );
    cfg.threads.insert("c".into(), VoiceReplyMode::Never);
    assert_eq!(
        resolve_policy(
            &cfg,
            &scope("telegram", "c", "t"),
            &crate::core::runtime::Tenant::default()
        )
        .mode,
        HostVoiceReplyMode::Never
    );
    assert_eq!(
        resolve_policy(
            &cfg,
            &scope("slack", "other", "t"),
            &crate::core::runtime::Tenant::default()
        )
        .mode,
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
            turn_key(
                &tenant,
                &ChannelRef::default(),
                &conversation,
                &index.to_string(),
            ),
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
