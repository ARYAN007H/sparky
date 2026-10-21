//! One-shot `create_chat_model*` entry points, the readiness probe, the
//! default-temperature wrapper, and the unresolved-route error builder.

use super::*;
use crate::config::ops::resolve_primary_cloud_provider_string;
use crate::core::runtime::DispatchCtx;
use crate::inference::provider::factory::access_gates::verify_session_active;
use tinyagents_harness::host::{
    BuildOpts, ModelBuildGate, ModelBuildRequest, ModelFactory, ModelPurpose,
};
use tinyinference_llm::model::{ModelRequestDefaults, ProfileOverrideModel, ResolvedModelRoute};

/// OpenHuman's host policy and construction adapter for a harness turn.
///
/// The harness owns turn assembly; this adapter resolves OpenHuman provider
/// configuration and enforces product access policy for each requested model.
pub(crate) struct OpenHumanModelFactory<'a> {
    pub(crate) ctx: &'a DispatchCtx,
    pub(crate) config: &'a Config,
    pub(crate) primary_provider: Option<&'a str>,
    pub(crate) egress: &'a EgressLedger,
    pub(crate) thread_id: Option<&'a str>,
}

impl OpenHumanModelFactory<'_> {
    fn provider_for(&self, request: &ModelBuildRequest) -> String {
        match request.purpose {
            ModelPurpose::Primary | ModelPurpose::Summarization => self
                .primary_provider
                .map(str::to_owned)
                .unwrap_or_else(|| provider_for_role(&request.role, self.config)),
            ModelPurpose::WorkloadTier(_) => provider_for_role(&request.role, self.config),
        }
    }

    fn injected(&self, request: &ModelBuildRequest) -> Option<Arc<dyn ChatModel<()>>> {
        let local = self.ctx.host_overrides()?;
        let route = request.route.as_deref();
        match request.purpose {
            ModelPurpose::Primary => local.model_for(&request.role),
            ModelPurpose::Summarization => local
                .role_models
                .get("summarization")
                .cloned()
                .or_else(|| local.model_for(&request.role)),
            ModelPurpose::WorkloadTier(_) => route
                .and_then(|route| local.role_models.get(route))
                .or_else(|| local.role_models.get(&request.role))
                .cloned(),
        }
    }

    fn build_request(
        &self,
        request: &ModelBuildRequest,
    ) -> anyhow::Result<(Arc<dyn ChatModel<()>>, ResolvedModelRoute)> {
        let role = request.role.as_str();
        let pin = request.model_pin.as_deref().unwrap_or("");
        let provider = self.provider_for(request);
        if let Some(model) = self.injected(request) {
            let profile = model.profile().cloned().unwrap_or_default();
            let route = ResolvedModelRoute::new(
                profile.provider.as_deref().unwrap_or("injected"),
                profile.model.as_deref().unwrap_or(pin),
                request.route.as_deref().unwrap_or(role),
            );
            return Ok((model, route));
        }

        if resolves_to_managed_backend(role, self.config)
            || matches!(request.purpose, ModelPurpose::Primary | ModelPurpose::Summarization)
                && provider.trim() == PROVIDER_OPENHUMAN
        {
            let resolved = self.ctx.resolved();
            let extensions = resolved
                .as_deref()
                .map(crate::core::runtime::CoreContext::extensions);
            let (model, concrete) = super::managed_backend::make_openhuman_backend_model_for_thread_in(
                extensions,
                role,
                self.config,
                (!pin.is_empty()).then_some(pin),
                request.opts.native_tools,
                self.thread_id,
                self.egress,
            )?;
            let concrete = if concrete.trim().is_empty() { pin } else { &concrete };
            return Ok((
                model,
                ResolvedModelRoute::new(PROVIDER_OPENHUMAN, concrete, request.route.as_deref().unwrap_or(role)),
            ));
        }

        let (model, route_provider, resolved_model) = if let Some(result) =
            try_create_claude_agent_sdk_chat_model_from_string(role, &provider, self.config, self.egress)
        {
            let model = result?;
            let concrete = if pin.is_empty() { model.profile().and_then(|p| p.model.clone()).unwrap_or_default() } else { pin.to_owned() };
            (model, CLAUDE_AGENT_SDK_PROVIDER.to_owned(), concrete)
        } else if let Some(result) = try_create_claude_code_chat_model_from_string(
            role, &provider, self.config, (!pin.is_empty()).then_some(pin), self.egress,
        ) {
            let (model, _) = result?;
            (model, "claude-code".to_owned(), pin.to_owned())
        } else if let Some(result) = try_create_local_runtime_chat_model(role, self.config, self.egress) {
            let (model, resolved) = result?;
            let provider = model.profile().and_then(|p| p.provider.clone()).unwrap_or_else(|| "local".into());
            (model, provider, resolved)
        } else if let Some(result) = try_create_cloud_slug_chat_model_with_native_tools_for(
            role, self.config, request.opts.native_tools, self.egress,
            self.ctx.resolved().as_deref().map(crate::core::runtime::CoreContext::extensions),
        ) {
            let (model, resolved) = result?;
            let provider = provider.trim().split(':').next().unwrap_or(provider.trim()).to_owned();
            (model, provider, resolved)
        } else {
            let extensions = self.ctx.resolved().as_deref().map(crate::core::runtime::CoreContext::extensions);
            return Err(unresolved_chat_model_error_for(role, &provider, self.config, self.egress, extensions));
        };

        let route = ResolvedModelRoute::new(
            route_provider,
            resolved_model,
            request.route.as_deref().unwrap_or(role),
        );
        Ok((model, route))
    }
}

impl ModelBuildGate for OpenHumanModelFactory<'_> {
    fn check(&self, request: &ModelBuildRequest) -> anyhow::Result<()> {
        // Explicit host model injection is authoritative and has no provider
        // construction, credential, or egress side effect to gate.
        if self.injected(request).is_some() {
            return Ok(());
        }
        let provider = self.provider_for(request);
        enforce_local_only_inference(&request.role, &provider)?;
        crate::inference::provider::factory::access_gates::verify_provider_session(
            self.config,
            &provider,
        )?;
        Ok(())
    }
}

impl ModelFactory<()> for OpenHumanModelFactory<'_> {
    fn build(
        &self,
        request: &ModelBuildRequest,
    ) -> anyhow::Result<(Arc<dyn ChatModel<()>>, ResolvedModelRoute)> {
        let (model, route) = self.build_request(request)?;
        let opts = ModelRequestDefaults {
            model: (!route.model.is_empty()).then(|| route.model.clone()),
            temperature: request.opts.temperature,
        };
        let model = if let Some(profile) = model.profile().cloned() {
            Arc::new(ProfileOverrideModel::new(model, profile).with_defaults(opts))
                as Arc<dyn ChatModel<()>>
        } else {
            model
        };
        Ok((model, route))
    }
}

/// Build an `Arc<dyn ChatModel>` for the given workload role, under `ctx`.
///
/// The crate [`ChatModel`] is the model interface for the harness and one-shot
/// inference callers. Production and tests both inject this native interface;
/// `temperature` is applied as the request default while an explicit per-call
/// value still wins.
pub fn create_chat_model_in(
    ctx: &DispatchCtx,
    role: &str,
    config: &Config,
    temperature: f64,
) -> anyhow::Result<Arc<dyn ChatModel<()>>> {
    Ok(create_chat_model_with_model_id_in(ctx, role, config, temperature)?.0)
}

/// Like [`create_chat_model_in`], but also returns the resolved model id.
///
/// One-shot callers that persist or log the concrete model (e.g. the memory
/// summarise audit) need the id the role resolved to; the plain
/// [`create_chat_model_in`] drops it. The host model overrides of the agent
/// `ctx` runs for win over the configured route.
pub fn create_chat_model_with_model_id_in(
    ctx: &DispatchCtx,
    role: &str,
    config: &Config,
    temperature: f64,
) -> anyhow::Result<(Arc<dyn ChatModel<()>>, String)> {
    let (model, model_id) = create_chat_model_with_model_id_inner(ctx, role, config)?;
    Ok((with_default_temperature(model, temperature), model_id))
}

pub(super) fn create_chat_model_with_model_id_inner(
    ctx: &DispatchCtx,
    role: &str,
    config: &Config,
) -> anyhow::Result<(Arc<dyn ChatModel<()>>, String)> {
    if let Some(model) = ctx.host_overrides().and_then(|local| local.model_for(role)) {
        return Ok((
            model,
            config
                .default_model
                .clone()
                .unwrap_or_else(|| "host-model".into()),
        ));
    }
    #[cfg(any(test, feature = "e2e-test-support", feature = "rss-bench"))]
    if let Some(model) = test_provider_override::current() {
        return Ok((model, "mock-model".to_string()));
    }
    // Managed OpenHuman backend → crate-native host `ChatModel`
    // (issue #4727 Motion B) instead of a
    // adapted provider. A native test-model override must still win, so only
    // take this path when no
    // override is installed. The public wrapper supplies the construction-time
    // default while preserving an explicit per-call `ModelRequest` temperature.
    let test_override_active = {
        #[cfg(any(test, feature = "e2e-test-support", feature = "rss-bench"))]
        {
            test_provider_override::current().is_some()
        }
        #[cfg(not(any(test, feature = "e2e-test-support", feature = "rss-bench")))]
        {
            false
        }
    };
    let resolved_context = ctx.resolved();
    let extensions = resolved_context
        .as_deref()
        .map(crate::core::runtime::CoreContext::extensions);
    if !test_override_active {
        if resolves_to_managed_backend(role, config) {
            return make_openhuman_backend_model_in(extensions, role, config);
        }
        if let Some(result) =
            try_create_claude_agent_sdk_chat_model(role, config, &EgressLedger::none())
        {
            return result;
        }
        if let Some(result) =
            try_create_claude_code_chat_model(role, config, None, &EgressLedger::none())
        {
            return result;
        }
        // Local OpenAI-compatible runtimes (Ollama / LM Studio / MLX / OMLX /
        // local-openai) → crate-native `ChatModel` (issue #4727 Motion B) instead
        // of a crate-adapted host provider. Cloud/BYOK/bespoke providers
        // return `None` here and fall through to the `Provider` path below.
        if let Some(result) =
            try_create_local_runtime_chat_model(role, config, &EgressLedger::none())
        {
            return result;
        }
        // Wire-equivalent BYOK cloud slugs (Anthropic / None / plain-Bearer, no
        // codex-oauth or `/v1/responses` fallback) → crate-native `ChatModel`
        // (issue #4727 Phase 3, conservative subset). `openai`/codex, custom
        // proxy slugs, and the managed entry return `None` and fall through.
        if let Some(result) = try_create_cloud_slug_chat_model_with_native_tools_for(
            role,
            config,
            true,
            &EgressLedger::none(),
            extensions,
        ) {
            return result;
        }
    }
    Err(unresolved_chat_model_error_for(
        role,
        &provider_for_role(role, config),
        config,
        &EgressLedger::none(),
        extensions,
    ))
}

/// Whether `role` resolves to the managed OpenHuman backend (vs BYOK / local /
/// claude-code). Uses the same empty/`cloud`/`openhuman` normalization as
/// [`create_chat_model_from_string`] so every managed role shares one path.
pub(crate) fn resolves_to_managed_backend(role: &str, config: &Config) -> bool {
    let mut resolved = provider_for_role(role, config);
    let trimmed = resolved.trim();
    if trimmed.is_empty() || trimmed == "cloud" {
        resolved = resolve_primary_cloud_provider_string(config);
    }
    resolved.trim() == PROVIDER_OPENHUMAN
}

/// Probe whether `role` can actually complete an inference call right now
/// (issue B45 — the flows provider-connectivity author gate).
///
/// Two-stage check, mirroring the two ways a `role` can be un-runnable:
///
/// 1. **Construction** — [`create_chat_model_with_model_id_inner`] must
///    succeed. This is the existing Layer 1 check (BYOK-incomplete config,
///    unknown provider slug, local-only privacy-mode block, …) reused
///    verbatim so this probe never re-implements it.
/// 2. **Managed-backend readiness** — when `role` resolves to the managed
///    OpenHuman backend, the registered provider's readiness probe makes one
///    cheap real completion attempt to catch the "account has no provider API
///    key configured" class of failure that construction alone cannot see
///    (construction only builds the client; it never calls the backend).
///    BYOK/local models have no such hidden failure mode — their construction
///    step already validates what it can, so they return `Ok(())` here
///    unconditionally.
///
/// Respects the [`test_provider_override`] test seam: when a mock model is
/// installed, construction returns it immediately and this function returns
/// `Ok(())` without ever touching the network or resolving `role` again —
/// `resolves_to_managed_backend` is a pure config read that would otherwise
/// still call this "managed" in a test with a bare default `Config`.
pub async fn probe_inference_readiness(
    ctx: &DispatchCtx,
    role: &str,
    config: &Config,
) -> Result<(), String> {
    #[cfg(any(test, feature = "e2e-test-support", feature = "rss-bench"))]
    if test_provider_override::current().is_some() {
        log::debug!(
            "[flows][inference-probe] role={role} test model override active — skipping probe"
        );
        return Ok(());
    }

    log::debug!("[flows][inference-probe] role={role} verifying model construction");
    if let Err(e) = create_chat_model_with_model_id_inner(ctx, role, config) {
        log::debug!("[flows][inference-probe] role={role} construction failed: {e}");
        return Err(e.to_string());
    }

    if !resolves_to_managed_backend(role, config) {
        log::debug!(
            "[flows][inference-probe] role={role} resolves to a non-managed provider — \
             construction succeeded, nothing further to probe"
        );
        return Ok(());
    }

    log::debug!(
        "[flows][inference-probe] role={role} resolves to the managed OpenHuman backend — \
         probing readiness"
    );
    let resolved_context = ctx.resolved();
    let extensions = resolved_context
        .as_deref()
        .map(crate::core::runtime::CoreContext::extensions);
    let Some(entry) = crate::inference::registry::chat_model_for(extensions, PROVIDER_OPENHUMAN)
    else {
        log::debug!("[flows][inference-probe] role={role} managed provider was unregistered");
        return Ok(());
    };
    let Some(probe) = entry.probe else {
        // The registered provider has no readiness probe: construction above
        // succeeded, and there is nothing further to ask it.
        log::debug!("[flows][inference-probe] role={role} managed provider registered no probe");
        return Ok(());
    };
    let model_id = entry
        .models
        .as_ref()
        .and_then(|models| (models.resolve_model)(role, config, None))
        .or_else(|| {
            config
                .default_model
                .as_deref()
                .map(str::trim)
                .filter(|model| !model.is_empty())
                .map(str::to_string)
        })
        .ok_or_else(|| format!("provider {PROVIDER_OPENHUMAN} has no model for role {role}"))?;
    let result = probe(role.to_string(), model_id.clone(), config.clone()).await;
    log::debug!(
        "[flows][inference-probe] role={role} model={model_id} probe result: {}",
        if result.is_ok() { "ready" } else { "not ready" }
    );
    result
}

/// Build an `Arc<dyn ChatModel>` from an explicit provider string and config.
///
/// The explicit-string counterpart of [`create_chat_model`].
pub fn create_chat_model_from_string(
    role: &str,
    provider: &str,
    config: &Config,
    temperature: f64,
) -> anyhow::Result<Arc<dyn ChatModel<()>>> {
    create_chat_model_from_string_with_model_id(role, provider, config, temperature)
        .map(|(model, _)| model)
}

/// Build a crate [`ChatModel`] from an explicit provider string and return the
/// concrete model id selected by that provider.
///
/// Managed, local-runtime, configured cloud-slug, Claude SDK/Code, and Codex
/// strings all construct native `ChatModel` implementations directly.
pub fn create_chat_model_from_string_with_model_id(
    role: &str,
    provider: &str,
    config: &Config,
    temperature: f64,
) -> anyhow::Result<(Arc<dyn ChatModel<()>>, String)> {
    let (model, model_id) =
        create_chat_model_from_string_with_model_id_inner(role, provider, config)?;
    Ok((with_default_temperature(model, temperature), model_id))
}

pub(super) fn create_chat_model_from_string_with_model_id_inner(
    role: &str,
    provider: &str,
    config: &Config,
) -> anyhow::Result<(Arc<dyn ChatModel<()>>, String)> {
    #[cfg(any(test, feature = "e2e-test-support", feature = "rss-bench"))]
    if let Some(model) = test_provider_override::current() {
        return Ok((model, "mock-model".to_string()));
    }
    let test_override_active = {
        #[cfg(any(test, feature = "e2e-test-support", feature = "rss-bench"))]
        {
            test_provider_override::current().is_some()
        }
        #[cfg(not(any(test, feature = "e2e-test-support", feature = "rss-bench")))]
        {
            false
        }
    };
    if !test_override_active {
        let mut resolved = provider.trim().to_string();
        if resolved.is_empty() || resolved == "cloud" {
            resolved = resolve_primary_cloud_provider_string(config);
        }
        if resolved == PROVIDER_OPENHUMAN {
            return make_openhuman_backend_model(role, config);
        }
        if let Some(result) = try_create_claude_agent_sdk_chat_model_from_string(
            role,
            &resolved,
            config,
            &EgressLedger::none(),
        ) {
            return result;
        }
        if let Some(result) = try_create_claude_code_chat_model_from_string(
            role,
            &resolved,
            config,
            None,
            &EgressLedger::none(),
        ) {
            return result;
        }
        if let Some(result) = try_create_local_runtime_chat_model_from_string(
            role,
            &resolved,
            config,
            &EgressLedger::none(),
        ) {
            return result;
        }
        if let Some(result) = try_create_cloud_slug_chat_model_from_string(
            role,
            &resolved,
            config,
            &EgressLedger::none(),
        ) {
            return result;
        }
    }
    Err(unresolved_chat_model_error(
        role,
        provider,
        config,
        &EgressLedger::none(),
    ))
}

pub(super) fn with_default_temperature(
    model: Arc<dyn ChatModel<()>>,
    temperature: f64,
) -> Arc<dyn ChatModel<()>> {
    let profile = model.profile().cloned().unwrap_or_default();
    Arc::new(
        ProfileOverrideModel::new(model, profile).with_defaults(ModelRequestDefaults {
            model: None,
            temperature: Some(temperature),
        }),
    )
}

/// Reproduce the legacy provider factory's access gates and diagnostics for a
/// provider string that none of the crate-native model constructors accepted.
///
/// Successful production routes never reach this function. Keeping error
/// resolution separate means `create_chat_model*` no longer constructs a
/// legacy `Provider` merely to discover that a route is invalid.
pub(super) fn unresolved_chat_model_error(
    role: &str,
    provider: &str,
    config: &Config,
    egress: &EgressLedger,
) -> anyhow::Error {
    let extensions = crate::core::runtime::CoreContext::current_extensions();
    unresolved_chat_model_error_for(role, provider, config, egress, extensions.as_ref())
}

pub(super) fn unresolved_chat_model_error_for(
    role: &str,
    provider: &str,
    config: &Config,
    egress: &EgressLedger,
    extensions: Option<&crate::core::slots::Extensions>,
) -> anyhow::Error {
    let p = provider.trim();

    if let Err(error) = enforce_local_only_inference(role, p) {
        return error;
    }

    if p == BYOK_INCOMPLETE_SENTINEL {
        let inference_url = config
            .inference_url
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("<unset>");
        return anyhow::anyhow!(
            "[chat-factory] BYOK_INCOMPLETE: inference_url is set to a custom/direct endpoint \
             ({inference_url}) but no matching cloud_providers entry was found for role '{role}'. \
             To complete BYOK setup add a cloud_providers entry whose endpoint matches \
             {inference_url} (or use a workload-specific route). \
             To use the OpenHuman managed backend instead, clear inference_url from config."
        );
    }

    if p.is_empty() || p == "cloud" {
        return unresolved_chat_model_error_for(
            role,
            &resolve_primary_cloud_provider_string(config),
            config,
            egress,
            extensions,
        );
    }

    #[cfg(not(test))]
    if let Err(error) = verify_session_active(config) {
        return error;
    }

    // Preserve the legacy chokepoint's disclosure ordering for invalid custom
    // routes: after both gates pass, the attempted external destination is
    // visible even when configuration validation then fails.
    emit_inference_egress(role, p, egress);

    if let Some((slug, model_with_temperature)) = p.split_once(':') {
        if slug.trim().is_empty() {
            return anyhow::anyhow!(
                "[chat-factory] provider string '{}' for role '{}' has an empty slug",
                p,
                role
            );
        }
        let (model, _) = split_model_and_temperature(model_with_temperature);
        return match resolve_cloud_slug_for(role, slug.trim(), &model, config, extensions) {
            Err(error) => error,
            Ok(_) => anyhow::anyhow!(
                "[chat-factory] configured provider '{}' for role '{}' did not produce a crate-native chat model",
                p,
                role
            ),
        };
    }

    anyhow::anyhow!(
        "[chat-factory] unrecognised provider string '{}' for role '{}'. \
         Valid forms: openhuman, ollama:<model>, lmstudio:<model>, mlx:<model>, omlx:<model>, \
         local-openai:<model>, claude_agent_sdk, claude_agent_sdk:<model>, <slug>:<model>. \
         Configured slugs: [{}]",
        p,
        role,
        config
            .cloud_providers
            .iter()
            .map(|entry| entry.slug.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

// ── Internal helpers ──────────────────────────────────────────────────────────
