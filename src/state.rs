use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use reqwest::Client;
use secrecy::ExposeSecret;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{config::Config, error::Result};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: PgPool,
    pub http: Client,
    pub spotify_limiter: Arc<Mutex<Instant>>,
    pub currently_playing_locks: Arc<Mutex<HashMap<Uuid, Arc<Mutex<()>>>>>,
    pub metrics: Arc<AppMetrics>,
}

#[derive(Default)]
pub struct AppMetrics {
    spotify_requests_total: AtomicU64,
    spotify_failures_total: AtomicU64,
    import_jobs_processed_total: AtomicU64,
}

impl AppMetrics {
    pub fn inc_spotify_requests(&self) {
        self.spotify_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_spotify_failures(&self) {
        self.spotify_failures_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_import_jobs_processed(&self, count: u64) {
        self.import_jobs_processed_total
            .fetch_add(count, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            spotify_requests_total: self.spotify_requests_total.load(Ordering::Relaxed),
            spotify_failures_total: self.spotify_failures_total.load(Ordering::Relaxed),
            import_jobs_processed_total: self.import_jobs_processed_total.load(Ordering::Relaxed),
        }
    }
}

pub struct MetricsSnapshot {
    pub spotify_requests_total: u64,
    pub spotify_failures_total: u64,
    pub import_jobs_processed_total: u64,
}

impl AppState {
    pub async fn connect(config: Config) -> Result<Self> {
        let db = PgPoolOptions::new()
            .max_connections(10)
            .connect(config.database_url.expose_secret())
            .await?;
        let http = Client::builder()
            .user_agent(format!("spotrak/{}", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            config: Arc::new(config),
            db,
            http,
            spotify_limiter: Arc::new(Mutex::new(Instant::now())),
            currently_playing_locks: Arc::new(Mutex::new(HashMap::new())),
            metrics: Arc::new(AppMetrics::default()),
        })
    }
}
