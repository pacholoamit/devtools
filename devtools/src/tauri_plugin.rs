use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, RunEvent, Runtime};
use tokio::sync::{broadcast, watch};
use tauri_devtools_shared::{LogEntry, Metrics, SpanEntry};
use crate::broadcaster::Broadcaster;
use crate::server::Server;

/// URL of the web-based devtool
/// The server host is added automatically eg: `127.0.0.1:56609`.
const DEVTOOL_URL: &str = "https://crabnebula.dev/debug/#";

pub struct TauriPlugin {
    enabled: bool,
    init: Option<(Broadcaster, broadcast::Sender<Vec<LogEntry>>, broadcast::Sender<Vec<SpanEntry>>)>,
    metrics: Arc<Mutex<Metrics>>,
    shutdown_tx: watch::Sender<()>
}

impl TauriPlugin {
    pub(crate) fn new(enabled: bool, broadcaster: Broadcaster, logs_tx: broadcast::Sender<Vec<LogEntry>>,
               spans_tx: broadcast::Sender<Vec<SpanEntry>>, shutdown_tx: watch::Sender<()>) -> Self {
        Self {
            enabled,
            init: Some((broadcaster, logs_tx, spans_tx)),
            metrics: Arc::new(Mutex::new(Metrics { initialized_at: api::now(), ready_at: 0 })),
            shutdown_tx,
        }
    }
}

impl<R: Runtime> tauri::plugin::Plugin<R> for TauriPlugin {
    fn name(&self) -> &'static str {
        "devtools"
    }

    fn initialize(&mut self, app_handle: &AppHandle<R>, _: serde_json::Value) -> tauri::plugin::Result<()> {
        if !self.enabled {
            return Ok(())
        }

        let (broadcaster, logs_tx, spans_tx) = self.init.take().unwrap();

        let server = Server::new(logs_tx, spans_tx, app_handle.clone(), self.metrics.clone());
        spawn_handler_thread(broadcaster, server);

        Ok(())
    }

    fn on_event(&mut self, _: &AppHandle<R>, event: &RunEvent) {
        if !self.enabled {
            return
        }

        match event {
            RunEvent::Ready => {
                if let Ok(mut metrics) = self.metrics.lock() {
                    metrics.ready_at = api::now();
                }
                tracing::debug!("Application is ready");
            }
            RunEvent::Exit => {
                // Shutdown signal for the `Broadcaster`, this will make sure all queued items
                // are sent to all event subscribers.
                if let Err(e) = self.shutdown_tx.send(()) {
                    tracing::error!("{e}");
                }
            }
            RunEvent::WindowEvent { label, .. } => {
                tracing::debug!("Window {} received an event", label);
            }
            RunEvent::ExitRequested { .. } => {
                tracing::debug!("Exit requested");
            }
            RunEvent::Resumed => {
                tracing::debug!("Event loop is being resumed");
            }
            _ => {}
        }
    }
}

fn spawn_handler_thread<R: Runtime>(broadcaster: Broadcaster, server: Server<R>) {
    thread::spawn(move || {
        let _subscriber_guard =
            tracing::subscriber::set_default(tracing_core::subscriber::NoSubscriber::default());

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async move {
            let broadcaster = tokio::spawn(broadcaster.run());

            let (server_addr, server_handle) = server.run().await.unwrap();

            println!("--------- Tauri Plugin Devtools ---------\n");
            println!("Listening at:\n  ws://{server_addr}\n",);
            println!("Inspect in browser:\n  {DEVTOOL_URL}{server_addr}");
            println!("\n--------- Tauri Plugin Devtools ---------");

            server_handle.stopped().await;
            broadcaster.abort();
        })
    });
}