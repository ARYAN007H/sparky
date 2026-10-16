use super::*;
use openhuman_embed::__host::voice_registry::{
    unregister_stt_provider, unregister_tts_provider,
};

#[test]
fn registration_covers_every_historical_cloud_slug() {
    register();
    let config = Config::default();
    for slug in STT_SLUGS {
        let factory = lookup_stt(slug);
        assert_eq!(factory("ignored", &config).unwrap().name(), "cloud", "{slug}");
    }
    for slug in TTS_SLUGS {
        let factory = lookup_tts(slug);
        assert_eq!(factory(None, &config).unwrap().name(), "cloud", "{slug}");
    }
    for slug in STT_SLUGS {
        unregister_stt_provider(slug);
    }
    for slug in TTS_SLUGS {
        unregister_tts_provider(slug);
    }
}

#[test]
fn cloud_tts_without_a_voice_carries_none_so_the_backend_default_applies() {
    // #5355: a Piper voice id must never reach the hosted proxy as `voice_id`.
    let provider = CloudTtsProvider::new(None);
    assert_eq!(provider.voice, None);
    let provider = CloudTtsProvider::new(Some("JBFqnCBsd6RMkjVDRZzb".into()));
    assert_eq!(provider.voice.as_deref(), Some("JBFqnCBsd6RMkjVDRZzb"));
}
