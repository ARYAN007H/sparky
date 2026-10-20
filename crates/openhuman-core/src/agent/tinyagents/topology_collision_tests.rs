#[test]
fn all_topologies_names_do_not_collide_with_production_tools() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let config = std::sync::Arc::new(crate::config::Config::default());
    let security = std::sync::Arc::new(crate::security::SecurityPolicy::default());
    let audit = std::sync::Arc::new(
        crate::security::AuditLogger::new(
            crate::config::AuditConfig {
                enabled: false,
                log_path: "audit.log".into(),
                max_size_mb: 10,
            },
            tmp.path().to_path_buf(),
        )
        .expect("create audit logger"),
    );
    let browser = crate::config::BrowserConfig::default();
    let http = crate::config::HttpRequestConfig::default();
    let agents = std::collections::HashMap::new();

    let tools = crate::tools::ops::all_tools(
        config.clone(),
        &security,
        audit,
        &browser,
        &http,
        tmp.path(),
        &agents,
        &config,
    );
    let tool_names: std::collections::HashSet<_> = tools.iter().map(|t| t.name()).collect();

    let reports = crate::agent::tinyagents::tool_registration::host_graph_topologies();
    for report in &reports {
        assert!(
            !tool_names.contains(report.name),
            "graph name '{}' collides with registered tool of the same name",
            report.name
        );
    }
}
