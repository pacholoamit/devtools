use crate::{Command, Watcher};
use async_stream::try_stream;
use bytes::BytesMut;
use devtools_wire_format as wire;
use devtools_wire_format::instrument;
use devtools_wire_format::instrument::instrument_server::InstrumentServer;
use devtools_wire_format::instrument::{instrument_server, InstrumentRequest};
use devtools_wire_format::meta::metadata_server::MetadataServer;
use devtools_wire_format::meta::{metadata_server, AppMetadata, AppMetadataRequest};
use devtools_wire_format::sources::sources_server::SourcesServer;
use devtools_wire_format::sources::{Chunk, Entry, EntryRequest, FileType};
use devtools_wire_format::tauri::tauri_server::TauriServer;
use devtools_wire_format::tauri::{
    tauri_server, Config, ConfigRequest, Metrics, MetricsRequest, Versions, VersionsRequest,
};
use futures::{FutureExt, Stream, TryStreamExt};
use std::net::SocketAddr;
use std::path::{Component, PathBuf};
use tauri::http::header::HeaderValue;
use tauri::{AppHandle, Runtime};
use tokio::sync::mpsc;
use tonic::codegen::http::Method;
use tonic::codegen::tokio_stream::wrappers::ReceiverStream;
use tonic::codegen::BoxStream;
use tonic::{Request, Response, Status};
use tonic_health::pb::health_server::HealthServer;
use tonic_health::server::HealthReporter;
use tonic_health::ServingStatus;
use tower_http::cors::{AllowHeaders, CorsLayer};

/// Default maximum capacity for the channel of events sent from a
/// [`Server`] to each subscribed client.
///
/// When this capacity is exhausted, the client is assumed to be inactive,
/// and may be disconnected.
const DEFAULT_CLIENT_BUFFER_CAPACITY: usize = 1024 * 4;

/// The `gRPC` server that exposes the instrumenting API
/// This is made up of 3 services:
/// - [`InstrumentService`]: Instrumentation related functionality, such as logs, spans etc.
/// - [`TauriService`]: Tauri-specific functionality, such as config, assets, metrics etc.
/// - [`HealthServer`]: `gRPC` health service for monitoring the health of the instrumenting API itself.
pub(crate) struct Server<R: Runtime> {
    instrument: InstrumentService,
    tauri: TauriService<R>,
    sources: SourcesService<R>,
    meta: MetaService<R>,
    health: HealthServer<tonic_health::server::HealthService>,
}

struct InstrumentService {
    tx: mpsc::Sender<Command>,
    health_reporter: HealthReporter,
}

struct TauriService<R: Runtime> {
    app_handle: AppHandle<R>,
}

struct SourcesService<R: Runtime> {
    app_handle: AppHandle<R>,
}

struct MetaService<R: Runtime> {
    app_handle: AppHandle<R>,
}

impl<R: Runtime> Server<R> {
    pub fn new(cmd_tx: mpsc::Sender<Command>, app_handle: AppHandle<R>) -> Self {
        let (mut health_reporter, health_service) = tonic_health::server::health_reporter();

        health_reporter
            .set_serving::<InstrumentServer<InstrumentService>>()
            .now_or_never()
            .unwrap();
        health_reporter
            .set_serving::<TauriServer<TauriService<R>>>()
            .now_or_never()
            .unwrap();

        Self {
            instrument: InstrumentService {
                tx: cmd_tx,
                health_reporter,
            },
            tauri: TauriService {
                app_handle: app_handle.clone(),
            }, // the TauriServer doesn't need a health_reporter. It can never fail.
            meta: MetaService {
                app_handle: app_handle.clone(),
            },
            sources: SourcesService { app_handle },
            health: unsafe { std::mem::transmute(health_service) },
        }
    }

    pub async fn run(self, addr: SocketAddr) -> crate::Result<()> {
        tracing::info!("Listening on {}", addr);

        let cors = CorsLayer::new()
            // allow `GET` and `POST` when accessing the resource
            .allow_methods([Method::GET, Method::POST])
            .allow_headers(AllowHeaders::any());

        let cors = if option_env!("__DEVTOOLS_LOCAL_DEVELOPMENT").is_some() {
            cors.allow_origin(tower_http::cors::Any)
        } else {
            cors.allow_origin(HeaderValue::from_str("https://devtools.crabnebula.dev").unwrap())
        };

        tonic::transport::Server::builder()
            .accept_http1(true)
            .layer(cors)
            .add_service(tonic_web::enable(InstrumentServer::new(self.instrument)))
            .add_service(tonic_web::enable(TauriServer::new(self.tauri)))
            .add_service(tonic_web::enable(SourcesServer::new(self.sources)))
            .add_service(tonic_web::enable(MetadataServer::new(self.meta)))
            .add_service(tonic_web::enable(self.health))
            .serve(addr)
            .await?;

        Ok(())
    }
}

impl InstrumentService {
    async fn set_status(&self, status: ServingStatus) {
        let mut r = self.health_reporter.clone();
        r.set_service_status("rs.devtools.instrument.Instrument", status)
            .await;
    }
}

#[tonic::async_trait]
impl instrument_server::Instrument for InstrumentService {
    type WatchUpdatesStream = BoxStream<instrument::Update>;

    async fn watch_updates(
        &self,
        req: Request<InstrumentRequest>,
    ) -> Result<Response<Self::WatchUpdatesStream>, Status> {
        if let Some(addr) = req.remote_addr() {
            tracing::debug!(client.addr = %addr, "starting a new watch");
        } else {
            tracing::debug!(client.addr = %"<unknown>", "starting a new watch");
        }

        // reserve capacity to message the aggregator
        let Ok(permit) = self.tx.reserve().await else {
            self.set_status(ServingStatus::NotServing).await;
            return Err(Status::internal(
                "cannot start new watch, aggregation task is not running",
            ));
        };

        // create output channel and send tx to the aggregator for tracking
        let (tx, rx) = mpsc::channel(DEFAULT_CLIENT_BUFFER_CAPACITY);

        permit.send(Command::Instrument(Watcher { tx }));

        tracing::debug!("watch started");

        let stream = ReceiverStream::new(rx).or_else(|err| async move {
            tracing::error!("Aggregator failed with error {err:?}");

            // TODO set the health service status to NotServing here

            Err(Status::internal("boom"))
        });

        Ok(Response::new(Box::pin(stream)))
    }
}

#[tonic::async_trait]
impl<R: Runtime> tauri_server::Tauri for TauriService<R> {
    async fn get_versions(
        &self,
        _req: Request<VersionsRequest>,
    ) -> Result<Response<Versions>, Status> {
        let versions = Versions {
            tauri: tauri::VERSION.to_string(),
            webview: tauri::webview_version().ok(),
        };

        Ok(Response::new(versions))
    }

    async fn get_config(&self, _req: Request<ConfigRequest>) -> Result<Response<Config>, Status> {
        let config: Config = (&*self.app_handle.config()).into();

        Ok(Response::new(config))
    }

    async fn get_metrics(
        &self,
        _req: Request<MetricsRequest>,
    ) -> Result<Response<Metrics>, Status> {
        Ok(Response::new(Metrics::default()))
    }
}

#[tonic::async_trait]
impl<R: Runtime> wire::sources::sources_server::Sources for SourcesService<R> {
    type ListEntriesStream = BoxStream<Entry>;

    async fn list_entries(
        &self,
        req: Request<EntryRequest>,
    ) -> Result<Response<Self::ListEntriesStream>, Status> {
        tracing::debug!("list entries");

        if self.app_handle.asset_resolver().iter().count() == 0 {
            let path = PathBuf::from(req.into_inner().path);

            // deny requests that contain special path components, like root dir, parent dir,
            // or weird windows ones. Only plain old regular, relative paths.
            if !path
                .components()
                .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
            {
                return Err(Status::not_found("file with the specified path not found"));
            }

            let mut cwd = std::env::current_dir()?;
            cwd.push(path);

            let stream = self.list_entries_from_dir(cwd).or_else(|err| async move {
                tracing::error!("List Entries failed with error {err:?}");
                // TODO set the health service status to NotServing here
                Err(Status::internal("boom"))
            });
            Ok(Response::new(Box::pin(stream)))
        } else {
            let inner = req.into_inner();
            let path = inner.path.trim_end_matches('.');
            let stream = self
                .list_entries_from_assets(path)
                .or_else(|err| async move {
                    tracing::error!("List Entries failed with error {err:?}");
                    // TODO set the health service status to NotServing here
                    Err(Status::internal("boom"))
                });
            Ok(Response::new(Box::pin(stream)))
        }
    }

    type GetEntryBytesStream = BoxStream<Chunk>;

    async fn get_entry_bytes(
        &self,
        req: Request<EntryRequest>,
    ) -> Result<Response<Self::GetEntryBytesStream>, Status> {
        let entry_path = req.into_inner().path;
        let asset_path = entry_path.trim_start_matches('.');

        if let Some(asset) = self
            .app_handle
            .asset_resolver()
            .iter()
            .find(|(path, _bytes)| **path == asset_path)
            // decompress the asset
            .and_then(|(path, _bytes)| self.app_handle.asset_resolver().get((*path).to_string()))
        {
            let chunks = asset
                .bytes
                .chunks(512)
                .map(|b| {
                    Ok(Chunk {
                        bytes: bytes::Bytes::copy_from_slice(b),
                    })
                })
                .collect::<Vec<_>>();
            let stream = futures::stream::iter(chunks);
            Ok(Response::new(Box::pin(stream)))
        } else {
            let entry_path = PathBuf::from(entry_path);
            // deny requests that contain special path components, like root dir, parent dir,
            // or weird windows ones. Only plain old regular, relative paths.
            if !entry_path
                .components()
                .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
            {
                return Err(Status::not_found("file with the specified path not found"));
            }

            let mut path = std::env::current_dir()?;
            path.push(entry_path);

            let stream = try_stream! {
                use tokio::io::AsyncReadExt;
                let mut file = tokio::fs::File::open(path).await?;
                let mut buf = BytesMut::with_capacity(512);

                while let Ok(n) = file.read_buf(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    yield Chunk { bytes: buf.split().freeze() };
                }
            };

            Ok(Response::new(Box::pin(stream)))
        }
    }
}

impl<R: Runtime> SourcesService<R> {
    fn list_entries_from_assets(&self, root: &str) -> impl Stream<Item = crate::Result<Entry>> {
        let resolver = self.app_handle.asset_resolver();

        let mut entries: Vec<Entry> = Vec::new();
        for (asset_path, _bytes) in self.app_handle.asset_resolver().iter() {
            // strip `/` prefix
            let path: String = asset_path.chars().skip(1).collect();

            let mut entry_path = path;
            let mut entry_type = FileType::FILE;

            if root.is_empty() {
                if let Some((dir, _path)) = entry_path.split_once('/') {
                    entry_path = dir.to_string();
                    entry_type = FileType::DIR;
                }
            } else if let Some(p) = entry_path.strip_prefix(&format!("{root}/")) {
                if let Some((dir, _path)) = p.split_once('/') {
                    entry_path = dir.to_string();
                    entry_type = FileType::DIR;
                } else {
                    entry_path = p.to_string();
                }
            } else {
                // asset does not belong to root
                continue;
            }

            if !entries.iter().any(|e| e.path == entry_path) {
                entries.push(Entry {
                    path: entry_path,
                    // we use resolver.get since it increases the size sometimes (e.g. injecting CSP on HTML files)
                    size: resolver.get((*asset_path).to_string()).unwrap().bytes.len() as u64,
                    file_type: (FileType::ASSET | entry_type).bits(),
                });
            }
        }

        futures::stream::iter(entries.into_iter().map(Ok))
    }

    fn list_entries_from_dir(&self, root: PathBuf) -> impl Stream<Item = crate::Result<Entry>> {
        let app_handle = self.app_handle.clone();

        try_stream! {
            let mut entries = tokio::fs::read_dir(&root).await?;

            while let Some(entry) = entries.next_entry().await? {
                let raw_file_type = entry.file_type().await?;
                let mut file_type = FileType::empty();
                if raw_file_type.is_dir() {
                    file_type |= FileType::DIR;
                }
                if raw_file_type.is_file() {
                    file_type |= FileType::FILE;
                }
                if raw_file_type.is_symlink() {
                    file_type |= FileType::SYMLINK;
                }

                let path = entry.path();
                let path = path.strip_prefix(&root)?;

                let path = path.to_string_lossy().to_string();

                let is_asset = app_handle.asset_resolver().iter().any(|(p, _)| p.ends_with(&path));
                if is_asset {
                    file_type |= FileType::ASSET;
                }

                yield Entry {
                    path,
                    size: entry.metadata().await?.len(),
                    file_type: file_type.bits(),
                };
            }
        }
    }
}

#[tonic::async_trait]
impl<R: Runtime> metadata_server::Metadata for MetaService<R> {
    async fn get_app_metadata(
        &self,
        _req: Request<AppMetadataRequest>,
    ) -> Result<Response<AppMetadata>, Status> {
        let info = self.app_handle.package_info();

        let meta = AppMetadata {
            name: info.name.clone(),
            version: info.version.to_string(),
            authors: info.authors.to_string(),
            description: info.description.to_string(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            debug_assertions: cfg!(debug_assertions),
        };

        Ok(Response::new(meta))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use devtools_wire_format::instrument::instrument_server::Instrument;
    use devtools_wire_format::sources::sources_server::Sources;
    use devtools_wire_format::tauri::tauri_server::Tauri;
    use futures::StreamExt;

    #[tokio::test]
    async fn tauri_get_config() {
        let tauri = TauriService {
            app_handle: tauri::test::mock_app().handle(),
        };

        let cfg = tauri
            .get_config(Request::new(ConfigRequest {}))
            .await
            .unwrap();

        assert_eq!(
            cfg.into_inner(),
            devtools_wire_format::tauri::Config::from(&*tauri.app_handle.config())
        );
    }

    #[tokio::test]
    async fn subscription() {
        let (health_reporter, _) = tonic_health::server::health_reporter();
        let (cmd_tx, mut cmd_rx) = mpsc::channel(1);
        let srv = InstrumentService {
            tx: cmd_tx,
            health_reporter,
        };

        let _stream = srv
            .watch_updates(Request::new(InstrumentRequest {
                log_filter: None,
                span_filter: None,
            }))
            .await
            .unwrap();

        let cmd = cmd_rx.recv().await.unwrap();

        assert!(matches!(cmd, Command::Instrument(_)));
    }

    #[tokio::test]
    async fn sources_list_entries() {
        let app_handle = tauri::test::mock_app().handle();
        let srv = SourcesService { app_handle };

        let stream = srv
            .list_entries(Request::new(EntryRequest {
                path: ".".to_string(),
            }))
            .await
            .unwrap();

        // this will list this crates directory, so should produce a `Cargo.toml` and `src` entry
        let entries: Vec<_> = stream.into_inner().collect().await;
        assert_eq!(entries.len(), 4);
    }

    #[tokio::test]
    async fn sources_list_entries_root() {
        let app_handle = tauri::test::mock_app().handle();
        let srv = SourcesService { app_handle };

        let res = srv
            .list_entries(Request::new(EntryRequest {
                path: "/".to_string(),
            }))
            .await;

        assert!(res.is_err(), "requesting the root path should fail");

        let res = srv
            .list_entries(Request::new(EntryRequest {
                path: "/foo/bar/this".to_string(),
            }))
            .await;

        assert!(res.is_err(), "requesting the root path should fail")
    }

    #[tokio::test]
    async fn sources_list_entries_parent() {
        let app_handle = tauri::test::mock_app().handle();
        let srv = SourcesService { app_handle };

        let res = srv
            .list_entries(Request::new(EntryRequest {
                path: "../".to_string(),
            }))
            .await;

        assert!(res.is_err(), "requesting an absolute path should fail");

        let res = srv
            .list_entries(Request::new(EntryRequest {
                path: "foo/bar/../this".to_string(),
            }))
            .await;

        assert!(res.is_err(), "requesting an absolute path should fail");

        let res = srv
            .list_entries(Request::new(EntryRequest {
                path: "..".to_string(),
            }))
            .await;

        assert!(res.is_err(), "requesting an absolute path should fail")
    }

    #[tokio::test]
    async fn sources_get_bytes() {
        let app_handle = tauri::test::mock_app().handle();
        let srv = SourcesService { app_handle };

        let stream = srv
            .get_entry_bytes(Request::new(EntryRequest {
                path: "./Cargo.toml".to_string(),
            }))
            .await
            .unwrap();

        // this will list this crates directory, so should produce a `Cargo.toml` and `src` entry
        let chunks: Vec<_> = stream.into_inner().collect().await;

        let mut buf = Vec::new();

        for chunk in chunks {
            buf.extend_from_slice(&chunk.unwrap().bytes);
        }

        // we don't want to hard code the exact size of Cargo.toml, that would be flaky
        // but it should definitely be larger than zero
        assert!(buf.len() > 0);
    }

    #[tokio::test]
    async fn sources_get_bytes_root() {
        let app_handle = tauri::test::mock_app().handle();
        let srv = SourcesService { app_handle };

        let res = srv
            .get_entry_bytes(Request::new(EntryRequest {
                path: "/".to_string(),
            }))
            .await;

        assert!(res.is_err(), "requesting the root path should fail");

        let res = srv
            .get_entry_bytes(Request::new(EntryRequest {
                path: "/foo/bar/this".to_string(),
            }))
            .await;

        assert!(res.is_err(), "requesting the root path should fail")
    }

    #[tokio::test]
    async fn sources_get_bytes_parent() {
        let app_handle = tauri::test::mock_app().handle();
        let srv = SourcesService { app_handle };

        let res = srv
            .get_entry_bytes(Request::new(EntryRequest {
                path: "../".to_string(),
            }))
            .await;

        assert!(res.is_err(), "requesting an absolute path should fail");

        let res = srv
            .get_entry_bytes(Request::new(EntryRequest {
                path: "foo/bar/../this".to_string(),
            }))
            .await;

        assert!(res.is_err(), "requesting an absolute path should fail");

        let res = srv
            .get_entry_bytes(Request::new(EntryRequest {
                path: "..".to_string(),
            }))
            .await;

        assert!(res.is_err(), "requesting an absolute path should fail")
    }
}
