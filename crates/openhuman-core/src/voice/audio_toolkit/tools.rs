mod podcast;

pub use podcast::{
    AudioEmailPodcastTool, AudioGenerateAndEmailPodcastTool, AudioGeneratePodcastTool,
};

/// Build this domain's tools in their persisted catalogue order.
pub(crate) fn registered_tools(
    config: std::sync::Arc<crate::config::Config>,
    security: &std::sync::Arc<crate::security::SecurityPolicy>,
) -> Vec<Box<dyn tinytools::Tool>> {
    vec![
        // Audio-toolkit podcast tools — gated with the `voice` feature (they
        // live in the `audio_toolkit` domain, which is compiled out when voice
        // is disabled).
        Box::new(AudioGeneratePodcastTool::new(
            config.clone(),
            security.clone(),
        )),
        Box::new(AudioEmailPodcastTool::new(config.clone(), security.clone())),
        Box::new(AudioGenerateAndEmailPodcastTool::new(
            config.clone(),
            security.clone(),
        )),
    ]
}
