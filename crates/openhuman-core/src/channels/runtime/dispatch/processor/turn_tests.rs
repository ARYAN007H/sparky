use super::*;

#[test]
fn voice_message_console_preview_never_includes_transcript_text() {
    let envelope = tinychannels_bus::ChannelInboundEnvelope {
        parts: vec![tinychannels_bus::ContentPart::VoiceNote {
            voice_note: tinychannels_bus::VoiceNote {
                transcript: Some(tinychannels_bus::VoiceTranscript {
                    text: "private dictated words".into(),
                    language: Some("en".into()),
                    state: tinychannels_bus::VoiceTranscriptState::Complete,
                    source: tinychannels_bus::VoiceTranscriptSource::HostStt,
                }),
                ..Default::default()
            },
        }],
        ..Default::default()
    };

    assert_eq!(
        inbound_console_preview(&envelope, "private dictated words"),
        "[voice message]"
    );
}

#[test]
fn ordinary_text_console_preview_remains_readable() {
    let envelope = tinychannels_bus::ChannelInboundEnvelope::default();

    assert_eq!(inbound_console_preview(&envelope, "hello"), "hello");
}
