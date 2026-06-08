//! runtime.rs — Rhelma v5.2 Enterprise Agent Runtime
//!
//! Responsibilities:
//!   - Build & own ObservabilityAgent
//!   - Start heartbeat loop
//!   - Start reflex signal loop (obs.signal topics allow-list)
//!   - Wire AI command source → CommandExecutor
//!   - Wire AI decision source → apply_ai_decision
//!   - Provide cooperative shutdown & best-effort task draining

#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use rhelma_event::EventBus;

use tracing::info;

use crate::agent::config::ObservabilityAgentConfig;
use crate::agent::ObservabilityAgent;
use crate::commands::CommandExecutor;
use crate::error::AgentError;
use crate::io::{HeartbeatClient, KafkaSignalSource};

/// External command source abstraction.
pub trait ExternalCommandSource<B>: Send + Sync
where
    B: EventBus + Send + Sync + 'static,
{
    /// Start the command source loop and wire it to the provided executor.
    fn start(
        self: Arc<Self>,
        executor: Arc<CommandExecutor<B>>,
        shutdown: CancellationToken,
    ) -> JoinHandle<()>;
}

/// External decision source abstraction.
pub trait DecisionSource<B>: Send + Sync
where
    B: EventBus + Send + Sync + 'static,
{
    /// Start the decision source loop and wire it to the provided agent.
    fn start(
        self: Arc<Self>,
        agent: Arc<ObservabilityAgent<B>>,
        shutdown: CancellationToken,
    ) -> JoinHandle<()>;
}

/// Main runtime orchestrator for the Observability-Agent.
pub struct AgentRuntime<B: EventBus + Send + Sync + 'static> {
    cfg: Arc<ObservabilityAgentConfig>,
    bus: Arc<B>,
    agent: Arc<ObservabilityAgent<B>>,

    /// Reflex signal source (obs.signal topics allow-list).
    signal_source: Option<Arc<KafkaSignalSource<B>>>,

    /// Cooperative shutdown for all spawned loops.
    shutdown: CancellationToken,

    /// Join handles for best-effort draining on shutdown.
    handles: Mutex<Vec<JoinHandle<()>>>,
}

impl<B> AgentRuntime<B>
where
    B: EventBus + Send + Sync + 'static,
{
    /// Build a runtime from config + EventBus.
    pub fn new(cfg: ObservabilityAgentConfig, bus: Arc<B>) -> Result<Self, AgentError> {
        cfg.validate()?;

        let cfg = Arc::new(cfg);
        let agent = Arc::new(ObservabilityAgent::new(cfg.clone(), bus.clone()));

        Ok(Self {
            cfg,
            bus,
            agent,
            signal_source: None,
            shutdown: CancellationToken::new(),
            handles: Mutex::new(Vec::new()),
        })
    }

    /// Expose agent handle.
    pub fn agent(&self) -> Arc<ObservabilityAgent<B>> {
        self.agent.clone()
    }

    /// Expose shutdown token (for wiring sources).
    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown.clone()
    }

    /// Attach reflex signal source.
    ///
    /// NOTE:
    /// `KafkaSignalSource::with_shutdown` consumes `self`, so we bind shutdown here
    /// (not inside `run`) and store it as an `Arc`.
    pub fn attach_signal_source(&mut self, src: KafkaSignalSource<B>) {
        let src = src.with_shutdown(self.shutdown.clone());
        self.signal_source = Some(Arc::new(src));
    }

    /// Request cooperative shutdown.
    pub fn shutdown(&self) {
        self.shutdown.cancel();
    }

    /// Spawn an optional admin HTTP server (health + metrics).
    ///
    /// This server is **opt-in** and starts only when `OBS_AGENT_ADMIN_ADDR`
    /// is set to a valid socket address (e.g. `127.0.0.1:9090`).
    ///
    /// Endpoints:
    /// - `GET /healthz` (also `/readyz`, `/livez`)
    /// - `GET /metrics` (Prometheus text format)
    fn spawn_admin_server_if_enabled(&self) -> Option<JoinHandle<()>> {
        use std::net::SocketAddr;
        use std::str::FromStr;

        let addr = match std::env::var("OBS_AGENT_ADMIN_ADDR") {
            Ok(v) => {
                let v = v.trim();
                if v.is_empty() || v.eq_ignore_ascii_case("none") || v == "0" {
                    return None;
                }
                match SocketAddr::from_str(v) {
                    Ok(a) => a,
                    Err(e) => {
                        tracing::warn!(error = %e, value = %v, "invalid OBS_AGENT_ADMIN_ADDR; admin server disabled");
                        return None;
                    }
                }
            }
            Err(_) => return None, // opt-in
        };

        let agent = self.agent.clone();
        let shutdown = self.shutdown.clone();

        Some(tokio::spawn(async move {
            use hyper::service::{make_service_fn, service_fn};
            use hyper::{Body, Method, Request, Response, Server, StatusCode};
            use serde_json::json;

            async fn handle_req<B: EventBus + Send + Sync + 'static>(
                req: Request<Body>,
                agent: Arc<ObservabilityAgent<B>>,
            ) -> Result<Response<Body>, hyper::Error> {
                let path = req.uri().path();
                match (req.method(), path) {
                    (&Method::GET, "/metrics") => {
                        let body = crate::io::internal_metrics::export_prometheus();
                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header("content-type", "text/plain; version=0.0.4; charset=utf-8")
                            .body(Body::from(body))
                            .unwrap())
                    }
                    (&Method::GET, "/healthz" | "/readyz" | "/livez") => {
                        let now = chrono::Utc::now();
                        let decision = agent.active_ai_decision().map(|d| {
                            json!({
                                "incident_id": d.incident_id,
                                "received_at": d.received_at,
                                "expires_at": d.expires_at,
                            })
                        });
                        let payload = json!({
                            "service": agent.cfg.service_name,
                            "environment": agent.cfg.environment,
                            "region": agent.cfg.region,
                            "version": agent.cfg.service_version,
                            "degraded": agent.degraded.load(std::sync::atomic::Ordering::Relaxed),
                            "sampling": agent.sampling.load(std::sync::atomic::Ordering::Relaxed),
                            "active_decision": decision,
                            "time_utc": now,
                        });

                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header("content-type", "application/json; charset=utf-8")
                            .body(Body::from(payload.to_string()))
                            .unwrap())
                    }
                    _ => Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .header("content-type", "text/plain; charset=utf-8")
                        .body(Body::from("not found"))
                        .unwrap()),
                }
            }

            let make_svc = make_service_fn(move |_conn| {
                let agent = agent.clone();
                async move {
                    Ok::<_, hyper::Error>(service_fn(move |req| handle_req(req, agent.clone())))
                }
            });

            let server = Server::bind(&addr).serve(make_svc);

            info!(%addr, "[agent] admin server started");
            let graceful = server.with_graceful_shutdown(async move {
                shutdown.cancelled().await;
            });

            if let Err(e) = graceful.await {
                tracing::warn!(error = %e, "admin server exited with error");
            } else {
                info!("[agent] admin server stopped");
            }
        }))
    }

    /// Start full agent runtime and block until shutdown is requested.
    pub async fn run(
        &self,
        command_source: Option<Arc<dyn ExternalCommandSource<B>>>,
        decision_source: Option<Arc<dyn DecisionSource<B>>>,
    ) -> Result<(), AgentError> {
        // 0) Optional admin server (/healthz, /metrics)
        if let Some(h) = self.spawn_admin_server_if_enabled() {
            self.handles.lock().await.push(h);
        }

        // 1) Heartbeat loop.
        {
            // IMPORTANT: spawn_loop_with_shutdown returns JoinHandle<()>
            let h = HeartbeatClient::new(self.cfg.clone(), self.bus.clone())
                .spawn_loop_with_shutdown(self.shutdown.clone());

            self.handles.lock().await.push(h);
        }

        // 2) Reflex: obs.signal topics allow-list loop.
        if let Some(src) = &self.signal_source {
            // src is already Arc<KafkaSignalSource<B>> and already has shutdown bound.
            let h = src.clone().start();
            self.handles.lock().await.push(h);
        }

        // 3) ai.command.execute listener.
        if self.cfg.command_enabled {
            if let Some(src) = command_source {
                let executor = Arc::new(CommandExecutor::new(
                    self.bus.clone(),
                    self.cfg.residency_mode.clone(),
                    self.cfg.service_name.clone(),
                    self.cfg.region.clone(),
                ));

                let h = src.start(executor, self.shutdown.clone());
                self.handles.lock().await.push(h);
            }
        }

        // 4) ai.incident.decision listener.
        if self.cfg.decision_enabled {
            if let Some(src) = decision_source {
                let h = src.start(self.agent.clone(), self.shutdown.clone());
                self.handles.lock().await.push(h);
            }
        }

        // 5) Block until shutdown (Ctrl+C triggers cooperative shutdown).
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Ctrl+C received; shutting down…");
                self.shutdown.cancel();
            }
            _ = self.shutdown.cancelled() => {}
        }

        // Drain tasks (best-effort).
        self.drain_tasks_best_effort().await;

        Ok(())
    }

    async fn drain_tasks_best_effort(&self) {
        let mut handles = self.handles.lock().await;
        let mut local = Vec::new();
        std::mem::swap(&mut *handles, &mut local);
        drop(handles);

        for mut h in local {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(750)) => {
                    h.abort();
                }
                _ = &mut h => {}
            }
        }
    }
}
