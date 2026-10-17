use super::*;

/// Compatibility adapter for callers that need a synchronous Run Now result.
/// TinyFlows owns admission, retries, delivery and persistence.
pub async fn execute_job_now(
    ctx: &crate::core::runtime::DispatchCtx,
    config: &Config,
    job: &CronJob,
    run_id: &str,
) -> (bool, String) {
    match run_job_now(
        ctx,
        config,
        job.clone(),
        run_id.to_string(),
        true,
        ManualDelivery::Requested,
    )
    .await
    {
        Ok(RunJobReply::Completed {
            success, attempt, ..
        }) => (success, attempt.output),
        Ok(RunJobReply::Refused { reason }) => (false, format!("cron job refused: {reason:?}")),
        Ok(RunJobReply::Accepted { .. }) => (
            false,
            "cron module accepted a synchronous run without completing it".to_string(),
        ),
        Err(error) => {
            tracing::debug!(job_id = %job.id, %run_id, %error, "[cron:scheduler] RunJobNow failed");
            (false, error.to_string())
        }
    }
}

/// Admit and optionally wait for a manual run through the module's durable
/// occurrence path. Detached runs keep their callback object served until the
/// module sends the terminal projection.
pub(crate) async fn run_job_now(
    ctx: &crate::core::runtime::DispatchCtx,
    config: &Config,
    job: CronJob,
    run_id: String,
    wait: bool,
    delivery: ManualDelivery,
) -> Result<RunJobReply> {
    let security = Arc::new(SecurityPolicy::from_config(
        &config.autonomy,
        &config.workspace_dir,
        &config.action_dir,
    ));
    let binding = Binding::new(ctx, config, security);
    let host = bus_host::register(binding.clone()).await.map_err(|e| anyhow!(e))?;
    let scope = binding
        .within(async { DocScope::current(config) })
        .await?;
    let storage = StoreConfig {
        backend: StoreBackend::Documents {
            callback: crate::cron::documents_port::address(&scope).await?,
        },
        max_run_history: config.cron.max_run_history,
        max_tasks: config.scheduler.max_tasks,
    };
    let request = RunJobNow {
        scope: CLAIM_SCOPE.to_string(),
        callback: host.address(),
        config: scheduler_config(config),
        storage,
        job,
        run_id: run_id.clone(),
        delivery,
        wait,
    };
    if !wait {
        bus_host::retain_manual_host(&run_id, host);
    }
    let reply = async {
        let proxy = module::run_proxy(config).await.map_err(|e| anyhow!(e))?;
        proxy
            .call(members::RUN_JOB_NOW, (request,))
            .await
            .map_err(|error| anyhow!(error.wire_message()))
    }
    .await;
    match reply {
        Ok(reply) => {
            if !wait && !matches!(reply, RunJobReply::Accepted { .. }) {
                bus_host::release_manual_host(&run_id);
            }
            Ok(reply)
        }
        Err(error) => {
            if !wait {
                bus_host::release_manual_host(&run_id);
            }
            Err(error)
        }
    }
}

/// Single poll cycle of the scheduler loop, extracted so tests can drive
/// it without owning `tokio::time::interval`.
///
/// Emits a `scheduler` health signal in two cases:
/// - Poll itself failed (DB read) → `healthy: false` with the DB error.
/// - Poll succeeded, queue empty or not → `healthy: true` (#3312
///   recovery signal). Without this, a single transient job failure
///   that flipped the component to `error` by a dispatched job would stay
///   there indefinitely while the queue was idle — the health endpoint would
///   keep returning 503 and Docker would mark the container `unhealthy` for
///   hours until a manual restart.
/// - Per-job results (published by each job's delivery) continue to flip the
///   component back to `healthy: false` on a failure; the next tick that
///   survives the DB read will re-flip it to `healthy: true`.
pub(crate) async fn tick_once(
    config: &Config,
    last_emitted_health: &mut Option<bool>,
    dispatcher: &mut JobDispatcher,
) {
    tracing::debug!("[cron:scheduler] tick poll begin");
    let dispatched = match dispatcher.tick(config).await {
        Ok(count) => count,
        Err(e) => {
            tracing::warn!("[cron:scheduler] tick poll failed: {e}");
            // Transition-only emission: repeat failures stay quiet so
            // subscribers don't see an event-storm during a long outage.
            if *last_emitted_health != Some(false) {
                BUS.publish(DomainEvent::HealthChanged {
                    component: "scheduler".to_string(),
                    healthy: false,
                    message: Some(e.to_string()),
                });
                *last_emitted_health = Some(false);
            }
            return;
        }
    };

    // Transition-only emission for the recovery signal: the very first
    // successful tick after boot (or after a failure) fires; later successful
    // ticks are no-ops on the wire.
    if *last_emitted_health != Some(true) {
        tracing::debug!(
            "[cron:scheduler] tick poll ok dispatched={dispatched} (recovery signal: healthy=true)"
        );
        BUS.publish(DomainEvent::HealthChanged {
            component: "scheduler".to_string(),
            healthy: true,
            message: None,
        });
        *last_emitted_health = Some(true);
    } else {
        tracing::trace!(
            "[cron:scheduler] tick poll ok dispatched={dispatched} (steady state, no event)"
        );
    }

    if dispatched > 0 {
        // A dispatched job publishes `healthy: false` on failure directly on
        // the bus, without touching this tracker. Reset so the next
        // successful tick is again a transition and re-emits `healthy: true`.
        *last_emitted_health = None;
    }
}

/// One scope's scheduler: a manual-tick scheduler in the module, polled by the
/// host, dispatching under its context. Stopped when dropped.
pub(crate) struct JobDispatcher {
    binding: Binding,
    started: Option<Started>,
}

/// A scheduler running in the module, and the host object it calls back.
struct Started {
    handle: String,
    host: HostRegistration,
}

impl JobDispatcher {
    /// A dispatcher for the context `ctx` carries. Nothing is started until the
    /// first tick.
    pub(crate) fn new(
        ctx: &crate::core::runtime::DispatchCtx,
        config: &Config,
        security: &Arc<SecurityPolicy>,
    ) -> Self {
        Self {
            binding: Binding::new(ctx, config, Arc::clone(security)),
            started: None,
        }
    }

    /// Restrict the dispatcher to agent jobs (the hosted-profile sweep).
    #[must_use]
    pub(crate) fn agent_only(mut self) -> Self {
        self.binding = self.binding.clone().agent_only();
        self
    }

    /// The dispatch context the scheduler's callbacks act under.
    #[cfg(test)]
    pub(crate) fn dispatch_ctx(&self) -> crate::core::runtime::DispatchCtx {
        self.binding.dispatch_ctx()
    }

    async fn start(&self, config: &Config) -> Result<Started> {
        let host = bus_host::register(self.binding.clone())
            .await
            .map_err(|e| anyhow!(e))?;
        // The documents of the scope this dispatcher acts in: resolved under
        // its context, because the call below is made from a task that may
        // not carry it.
        let scope = self
            .binding
            .within(async { DocScope::current(config) })
            .await?;
        let storage = StoreConfig {
            backend: StoreBackend::Documents {
                callback: crate::cron::documents_port::address(&scope).await?,
            },
            max_run_history: config.cron.max_run_history,
            max_tasks: config.scheduler.max_tasks,
        };
        let request = StartScheduler {
            scope: scope.scheduler_scope(),
            callback: host.address(),
            config: scheduler_config(config),
            manual_ticks: true,
            storage: Some(storage),
        };
        let proxy = module::proxy(config).await.map_err(|e| anyhow!(e))?;
        let handle: String = proxy
            .call(members::START_SCHEDULER, (request,))
            .await
            .map_err(|error| anyhow!(error.wire_message()))?;
        tracing::debug!("[cron:scheduler] scheduler started in the tinyflows module");
        Ok(Started { handle, host })
    }

    async fn call<R: serde::de::DeserializeOwned>(
        &mut self,
        config: &Config,
        member: &'static str,
    ) -> Result<R> {
        if self.started.is_none() {
            self.started = Some(self.start(config).await?);
        }
        let handle = self
            .started
            .as_ref()
            .map(|started| started.handle.clone())
            .ok_or_else(|| anyhow!("scheduler not started"))?;
        let proxy = module::run_proxy(config).await.map_err(|e| anyhow!(e))?;
        proxy
            .call(member, (handle,))
            .await
            .map_err(|error| anyhow!(error.wire_message()))
    }

    /// Read the due jobs and dispatch them; the count dispatched. Errors when
    /// the store cannot be read.
    pub(crate) async fn tick(&mut self, config: &Config) -> Result<usize> {
        self.binding.set_config(config);
        let dispatched: usize = self.call(config, members::TICK_SCHEDULER).await?;
        tracing::trace!(dispatched, "[cron:scheduler] TickScheduler");
        Ok(dispatched)
    }

    /// Wait for every dispatched job to finish.
    pub(crate) async fn drain(&mut self) {
        let config = self.binding.config();
        if self.started.is_none() {
            return;
        }
        if let Err(error) = self.call::<()>(&config, members::DRAIN_SCHEDULER).await {
            tracing::warn!(%error, "[cron:dispatch] a dispatched job failed");
        }
    }
}

impl Drop for JobDispatcher {
    fn drop(&mut self) {
        let Some(started) = self.started.take() else {
            return;
        };
        let config = self.binding.config();
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        // Stop the module's scheduler (releasing its claims) before the
        // callback object it would call goes away.
        runtime.spawn(async move {
            match module::proxy(&config).await {
                Ok(proxy) => {
                    let stopped = proxy
                        .call::<()>(members::STOP_SCHEDULER, (started.handle.clone(),))
                        .await;
                    if let Err(error) = stopped {
                        tracing::debug!(error = %error.wire_message(), "[cron:scheduler] StopScheduler failed");
                    }
                }
                Err(error) => tracing::debug!(%error, "[cron:scheduler] StopScheduler skipped"),
            }
            drop(started.host);
        });
    }
}
