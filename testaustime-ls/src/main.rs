use std::sync::Arc;

use arc_swap::ArcSwap;
use chrono::{DateTime, Local, TimeDelta};
use testaustime_shared::TestaustimeSettings;
use tokio::sync::Mutex;
use tower_lsp::{Client, LanguageServer, LspService, Server, jsonrpc::Result, lsp_types::*};

mod api;
use api::{APIClient, ActivityUpdate};

macro_rules! debug_log {
    ($self:expr, $($arg:tt)*) => {
        if $self.settings.load().debug_logs.unwrap_or(false) {
            $self.client
                .log_message(
                    MessageType::INFO,
                    format!("DEBUG: {}", format!($($arg)*))
                )
                .await;
        }
    };
}

#[derive(Default, Debug)]
struct Event {
    is_write: bool,
    language: Option<String>,
}

struct TestaustimeLanguageServer {
    settings: ArcSwap<TestaustimeSettings>,
    client: Client,
    api_client: Mutex<Option<APIClient>>,
    last_heartbeat: Mutex<DateTime<Local>>,
    workspace_name: ArcSwap<Option<String>>,
    last_language: ArcSwap<String>,
}

impl TestaustimeLanguageServer {
    async fn send(&self, event: Event) {
        const INTERVAL: TimeDelta = TimeDelta::seconds(30);

        let mut last_heartbeat = self.last_heartbeat.lock().await;
        let now = Local::now();

        if now - *last_heartbeat < INTERVAL && !event.is_write {
            return;
        }

        *last_heartbeat = now;

        let api_client = self.api_client.lock().await;
        let Some(ref client) = *api_client else {
            self.client
                .log_message(MessageType::ERROR, "API client not initialized")
                .await;
            return;
        };

        let project_name = match self.workspace_name.load().as_ref().as_ref() {
            Some(name) => name.clone(),
            None => {
                self.client
                    .log_message(MessageType::ERROR, "Workspace name not set")
                    .await;

                return;
            }
        };

        let language = if let Some(lang) = event.language {
            self.last_language.swap(Arc::new(lang.clone()));
            lang
        } else {
            self.last_language.load().as_ref().clone()
        };

        let activity = ActivityUpdate::new(
            project_name,
            language,
            hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string()),
        );

        debug_log!(self, "Heartbeat data: {:?}", activity);

        match client.heartbeat(activity).await {
            Ok(_) => {
                self.client
                    .log_message(MessageType::LOG, "Heartbeat sent successfully")
                    .await;
            }
            Err(e) => {
                self.client
                    .log_message(MessageType::ERROR, format!("Heartbeat failed: {}", e))
                    .await;
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for TestaustimeLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(initialization_options) = params.initialization_options {
            let settings = TestaustimeSettings::from_json(&initialization_options);

            let workspace_name = params
                .workspace_folders
                .as_ref()
                .and_then(|folders| folders.first())
                .and_then(|folder| {
                    if !folder.name.is_empty() {
                        Some(folder.name.clone())
                    } else {
                        // extract from folder URI if name is empty
                        folder
                            .uri
                            .path()
                            .split('/')
                            .filter(|s| !s.is_empty())
                            .last()
                            .map(|s| s.to_string())
                    }
                });

            debug_log!(self, "Workspace folders: {:?}", params.workspace_folders);

            self.workspace_name.swap(Arc::new(workspace_name));

            if let Some(ref api_key) = settings.api_key {
                let client = APIClient::new(api_key.clone(), settings.api_base_url.clone());
                match client.validate_api_key(api_key).await {
                    Ok(me) => {
                        self.client
                            .log_message(
                                MessageType::INFO,
                                format!("Testaustime authenticated as: {}", me.username),
                            )
                            .await;
                        *self.api_client.lock().await = Some(client);
                    }
                    Err(e) => {
                        self.client
                            .log_message(MessageType::ERROR, format!("Invalid API key: {}", e))
                            .await;
                    }
                }
            } else {
                self.client
                    .log_message(MessageType::WARNING, "No API key provided")
                    .await;
            }

            self.settings.swap(Arc::from(settings));
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "testaustime-ls".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Testaustime language server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        let api_client = self.api_client.lock().await;
        if let Some(ref client) = *api_client {
            let _ = client.flush().await;
        } else {
            self.client
                .log_message(
                    MessageType::ERROR,
                    "API client not initialized during shutdown",
                )
                .await;
        }
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let event = Event {
            is_write: false,
            language: Some(params.text_document.language_id.clone()),
        };

        self.send(event).await;
    }

    async fn did_change(&self, _params: DidChangeTextDocumentParams) {
        let event = Event {
            is_write: false,
            language: None,
        };

        self.send(event).await;
    }

    async fn did_save(&self, _params: DidSaveTextDocumentParams) {
        let event = Event {
            is_write: true,
            language: None,
        };

        self.send(event).await;
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| {
        Arc::new(TestaustimeLanguageServer {
            client,
            settings: ArcSwap::from_pointee(TestaustimeSettings::default()),
            last_heartbeat: Mutex::new(Local::now() - TimeDelta::seconds(31)),
            api_client: Mutex::new(None),
            workspace_name: ArcSwap::from_pointee(None),
            last_language: ArcSwap::from_pointee("Unknown".to_string()),
        })
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
