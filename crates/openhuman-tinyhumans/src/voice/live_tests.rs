use super::*;
use openhuman_embed::__host::voice_registry::unregister_live_provider;

#[test]
fn parses_wrapped_and_bare_tickets() {
    let wrapped = json!({"data": {"wsUrl": "wss://x/ws?ticket=t", "sessionId": "s", "model": "m"}});
    assert_eq!(
        parse_ticket(&wrapped).unwrap(),
        GeminiTicket {
            ws_url: "wss://x/ws?ticket=t".into(),
            session_id: "s".into(),
            model: "m".into()
        }
    );
    let bare = json!({"wsUrl": "wss://y"});
    assert_eq!(parse_ticket(&bare).unwrap().session_id, "");
    assert_eq!(
        parse_ticket(&json!({"data": {}})).unwrap_err().code,
        "backend"
    );
}

#[test]
fn gemini_hosted_config_follows_the_settings() {
    let mut config = Config::default();
    config.voice_live.gemini.voice = Some("Kore".into());
    config.voice_live.gemini.model = Some("gemini-x".into());
    let mut live = LiveConfig::new();
    configure_gemini(&config, "prompt", &mut live);
    assert_eq!(live.system_instruction.as_deref(), Some("prompt"));
    assert_eq!(live.voice.as_deref(), Some("Kore"));
    assert_eq!(live.model.as_deref(), Some("gemini-x"));
    assert!(live
        .provider_options
        .get("context_window_compression")
        .is_some());
}

#[test]
fn elevenlabs_hosted_config_owns_its_prompt_and_ignores_blank_voices() {
    let mut config = Config::default();
    config.voice_live.elevenlabs.voice_id = Some("  ".into());
    let mut live = LiveConfig::new();
    configure_elevenlabs(&config, "prompt", &mut live);
    assert_eq!(live.system_instruction, None, "the hosted agent owns its prompt");
    assert_eq!(live.voice, None, "blank voice ids are ignored");
    config.voice_live.elevenlabs.voice_id = Some("v1".into());
    configure_elevenlabs(&config, "prompt", &mut live);
    assert_eq!(live.voice.as_deref(), Some("v1"));
}

#[test]
fn registration_lists_both_hosted_providers_as_hosted() {
    register();
    let config = Config::default();
    let infos = openhuman_embed::__host::voice_registry::provider_infos(&config);
    for id in [LIVE_PROVIDER_GEMINI_HOSTED, LIVE_PROVIDER_ELEVENLABS_HOSTED] {
        let info = infos.iter().find(|i| i.id == id).expect(id);
        assert_eq!(info.kind, LiveProviderKind::Hosted, "{id}");
        assert_eq!(info.key_slug, None, "{id}");
    }
    let gemini = infos
        .iter()
        .find(|i| i.id == LIVE_PROVIDER_GEMINI_HOSTED)
        .unwrap();
    assert!(gemini.voices.contains(&"Puck".to_string()));
    unregister_live_provider(LIVE_PROVIDER_GEMINI_HOSTED);
    unregister_live_provider(LIVE_PROVIDER_ELEVENLABS_HOSTED);
}
