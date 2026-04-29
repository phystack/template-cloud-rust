// Cloud App entrypoint (Rust).
//
// Receives operator-configured settings, persists them in-process, and emits
// analytics events back to PhyHub. Same contract as the TypeScript template:
// the platform owns settings, your service owns runtime. Persist on every
// settings update so a PhyHub outage doesn't lose your config.

use anyhow::{Context, Result};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

const DEFAULT_PHYHUB_URL: &str = "http://localhost:14400";
const SETTINGS_POLL_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
struct CloudAppConfig {
    app_id: String,
    app_secret: String,
    phyhub_url: String,
}

impl CloudAppConfig {
    fn from_env() -> Result<Self> {
        let app_id = std::env::var("APP_ID").context("APP_ID required")?;
        let app_secret = std::env::var("APP_SECRET").context("APP_SECRET required")?;
        let phyhub_url = std::env::var("PHYHUB_URL")
            .unwrap_or_else(|_| DEFAULT_PHYHUB_URL.to_string())
            .trim_end_matches('/')
            .to_string();
        Ok(Self {
            app_id,
            app_secret,
            phyhub_url,
        })
    }

    fn auth_header(&self) -> String {
        format!("CloudAppSecret {}:{}", self.app_id, self.app_secret)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct AppSettings {
    #[serde(default)]
    event_names: Vec<String>,
    #[serde(default = "default_frequency_ms")]
    frequency_ms: u64,
}

fn default_frequency_ms() -> u64 {
    5_000
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            event_names: vec!["ping".to_string()],
            frequency_ms: default_frequency_ms(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct SettingsResponse {
    settings: AppSettings,
    version: i64,
}

#[derive(Debug, Serialize)]
struct EventBody<'a> {
    #[serde(rename = "appId")]
    app_id: &'a str,
    #[serde(rename = "eventName")]
    event_name: &'a str,
    payload: serde_json::Value,
    timestamp: String,
}

struct CloudApp {
    config: CloudAppConfig,
    client: Client,
    settings: Arc<RwLock<AppSettings>>,
    version: Arc<RwLock<i64>>,
}

impl CloudApp {
    fn new(config: CloudAppConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("reqwest client"),
            settings: Arc::new(RwLock::new(AppSettings::default())),
            version: Arc::new(RwLock::new(-1)),
        }
    }

    /// Fetch the current settings; returns true when the version moved forward.
    async fn fetch_settings(&self) -> Result<bool> {
        let url = format!(
            "{}/api/v1/cloud-app/settings?appId={}",
            self.config.phyhub_url, self.config.app_id,
        );
        let response = self
            .client
            .get(&url)
            .header("Authorization", self.config.auth_header())
            .send()
            .await?
            .error_for_status()?
            .json::<SettingsResponse>()
            .await?;
        let mut current_version = self.version.write().await;
        if response.version > *current_version {
            *self.settings.write().await = response.settings;
            *current_version = response.version;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn emit_event(&self, event_name: &str) -> Result<()> {
        let url = format!("{}/api/v1/cloud-app/events", self.config.phyhub_url);
        let body = EventBody {
            app_id: &self.config.app_id,
            event_name,
            payload: serde_json::json!({ "source": "cloud-app-template" }),
            timestamp: Utc::now().to_rfc3339(),
        };
        self.client
            .post(&url)
            .header("Authorization", self.config.auth_header())
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = CloudAppConfig::from_env()?;
    let app = Arc::new(CloudApp::new(config));

    // Initial settings load — fail fast if PhyHub is unreachable.
    app.fetch_settings()
        .await
        .context("initial settings fetch failed")?;
    info!("connected; initial settings loaded");

    // Settings poller.
    {
        let app = app.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(SETTINGS_POLL_INTERVAL).await;
                match app.fetch_settings().await {
                    Ok(true) => info!("settings updated"),
                    Ok(false) => {}
                    Err(err) => warn!(?err, "settings poll failed"),
                }
            }
        });
    }

    // Event ticker — round-robin through configured event names at the configured cadence.
    let mut next_event_index: usize = 0;
    loop {
        let (frequency_ms, event_names) = {
            let settings = app.settings.read().await;
            (settings.frequency_ms, settings.event_names.clone())
        };
        if event_names.is_empty() {
            tokio::time::sleep(Duration::from_millis(frequency_ms)).await;
            continue;
        }
        let event_name = event_names[next_event_index % event_names.len()].clone();
        next_event_index = next_event_index.wrapping_add(1);
        if let Err(err) = app.emit_event(&event_name).await {
            error!(?err, ?event_name, "emitEvent failed");
        }
        tokio::time::sleep(Duration::from_millis(frequency_ms)).await;
    }
}
