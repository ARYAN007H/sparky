use super::*;

impl CoreContext {
    pub fn current() -> Option<Arc<CoreContext>> {
        CURRENT_CONTEXT
            .try_with(|ctx| ctx.clone())
            .ok()
            .or_else(|| DEFAULT_CONTEXT.get().cloned())
    }

    /// The context scoped onto the current task, with no fallback to the
    /// process default. SaaS uses it: there, a task that lost its scope must
    /// fail rather than act as the operator.
    pub fn scoped() -> Option<Arc<CoreContext>> {
        CURRENT_CONTEXT.try_with(|ctx| ctx.clone()).ok()
    }

    /// The process default context (first built), independent of any active
    /// scope. Used by the dispatch chokepoint to establish the ambient scope.
    pub fn default_context() -> Option<Arc<CoreContext>> {
        DEFAULT_CONTEXT.get().cloned()
    }

    /// A minimal context for a caller that needs to carry per-turn authority
    /// but runs where no core was booted (a unit test, a library caller that
    /// never built a runtime). It has no workspace, no embedder config and the
    /// full domain set, which is what every reader already assumes when no
    /// context is scoped at all, so scoping it changes no answer except the
    /// one it exists to carry (the turn origin).
    pub(crate) fn bare() -> Arc<CoreContext> {
        Arc::new(CoreContext {
            host_kind: HostKind::Cli,
            workspace_binding: RwLock::new(Arc::new(RwLock::new(WorkspaceBinding {
                workspace_dir: None,
            }))),
            domains: crate::core::runtime::DomainSet::full(),
            tool_groups: Default::default(),
            components: Default::default(),
            embedder_config: None,
            user_skill_roots: true,
            backend_transport: None,
            turn_origin: None,
            channel_thread: None,
            session_agent: None,
            profile: None,
            agent: Default::default(),
        })
    }

    /// The turn origin bound to this context, if its entry point supplied one.
    pub(crate) fn turn_origin(&self) -> Option<&crate::agent::turn_origin::AgentTurnOrigin> {
        self.turn_origin.as_ref()
    }

    /// Typed origin bound to this immutable per-turn context, if supplied by
    /// its entrypoint. Missing authority retains library/CLI behavior.
    pub fn current_turn_origin() -> Option<crate::agent::turn_origin::AgentTurnOrigin> {
        Self::current().and_then(|ctx| ctx.turn_origin.clone())
    }

    /// Run a turn under a derived immutable context carrying its explicit
    /// entrypoint authority. The shared parent and other concurrent turns are
    /// untouched.
    pub async fn scope_with_turn_origin<F: Future>(
        ctx: Arc<CoreContext>,
        origin: Option<crate::agent::turn_origin::AgentTurnOrigin>,
        fut: F,
    ) -> F::Output {
        CURRENT_CONTEXT
            .scope(ctx.derive_channel_turn(None, origin), fut)
            .await
    }

    /// The relay thread of the current task, without an operator fallback.
    pub(crate) fn current_channel_thread() -> Option<String> {
        Self::scoped().and_then(|ctx| ctx.channel_thread.clone())
    }

    /// Bind channel facts to this agent context, preserving its tenant,
    /// tools, workspace and capability ceiling. No shared context is mutated.
    pub(crate) fn derive_channel_turn(
        &self,
        thread: Option<String>,
        origin: Option<crate::agent::turn_origin::AgentTurnOrigin>,
    ) -> Arc<CoreContext> {
        let ctx = self;
        Arc::new(CoreContext {
            host_kind: ctx.host_kind,
            workspace_binding: RwLock::new(
                ctx.workspace_binding
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone(),
            ),
            domains: ctx.domains,
            tool_groups: ctx.tool_groups.clone(),
            components: ctx.components,
            embedder_config: ctx.embedder_config.clone(),
            user_skill_roots: ctx.user_skill_roots,
            backend_transport: ctx.backend_transport.clone(),
            turn_origin: origin.or_else(|| ctx.turn_origin.clone()),
            channel_thread: thread.or_else(|| ctx.channel_thread.clone()),
            session_agent: ctx.session_agent.clone(),
            profile: ctx.profile.clone(),
            agent: ctx.agent.clone(),
        })
    }
}
