mod app;
mod auth;
mod config;
mod domain;
mod dto;
mod error;
mod repositories;
mod routes;
mod services;
mod state;

use std::{net::SocketAddr, path::PathBuf};

use clap::{Parser, Subcommand};
use secrecy::ExposeSecret;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, fmt};
use utoipa::OpenApi;

use crate::{config::Config, error::Result, state::AppState};

#[derive(Debug, Parser)]
#[command(name = "spotrak", version, about = "Greenfield Spotrak backend")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the HTTP API server.
    Serve,
    /// Run database migrations and exit.
    Migrate,
    /// Check database connectivity and exit.
    CheckDb,
    /// Seed deterministic development data.
    SeedDev,
    /// Poll Spotify recently played once for all users with refresh tokens.
    PollOnce,
    /// Write the OpenAPI document to stdout or a file.
    Openapi {
        /// Output path. Writes to stdout when omitted.
        output: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Command::Serve);
    if let Command::Openapi { output } = command {
        write_openapi(output).await?;
        return Ok(());
    }

    let config = Config::from_env()?;
    init_tracing(&config.log_level);
    tracing::debug!(?config, "loaded configuration");

    match command {
        Command::Serve => serve(config).await?,
        Command::Migrate => {
            let state = AppState::connect(config).await?;
            run_migrations(&state).await?;
        }
        Command::CheckDb => {
            check_db(&config).await?;
        }
        Command::SeedDev => {
            let state = AppState::connect(config).await?;
            run_migrations(&state).await?;
            seed_dev(&state).await?;
        }
        Command::PollOnce => {
            let state = AppState::connect(config).await?;
            run_migrations(&state).await?;
            let summary = crate::services::poller::poll_once(&state).await?;
            tracing::info!(?summary, "Spotify poll completed");
        }
        Command::Openapi { output } => {
            write_openapi(output).await?;
        }
    }

    Ok(())
}

async fn write_openapi(output: Option<PathBuf>) -> Result<()> {
    let json = serde_json::to_string_pretty(&crate::dto::openapi::ApiDoc::openapi())
        .map_err(|err| crate::error::AppError::internal(err.to_string()))?;
    if let Some(path) = output {
        tokio::fs::write(path, json)
            .await
            .map_err(|err| crate::error::AppError::internal(err.to_string()))?;
    } else {
        println!("{json}");
    }
    Ok(())
}

async fn serve(config: Config) -> Result<()> {
    let port = config.port;
    let state = AppState::connect(config).await?;
    tracing::info!(
        spotify_api_delay_ms = state.config.spotify_api_delay.as_millis(),
        "Spotify rate limiter configured"
    );
    run_migrations(&state).await?;
    spawn_cleanup_jobs(state.clone());
    spawn_spotify_poller(state.clone());
    spawn_import_worker(state.clone());
    spawn_artist_hydration_worker(state.clone());

    let router = app::build(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|err| crate::error::AppError::internal(err.to_string()))?;
    tracing::info!(%addr, "serving API");
    axum::serve(listener, router)
        .await
        .map_err(|err| crate::error::AppError::internal(err.to_string()))?;
    Ok(())
}

async fn run_migrations(state: &AppState) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(&state.db)
        .await
        .map_err(|err| crate::error::AppError::internal(err.to_string()))?;
    Ok(())
}

async fn check_db(config: &Config) -> Result<()> {
    let pool = sqlx::PgPool::connect(config.database_url.expose_secret()).await?;
    let value = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await?;
    tracing::info!(value, "database connectivity ok");
    Ok(())
}

fn init_tracing(default_level: &str) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));
    fmt().with_env_filter(filter).init();
}

fn spawn_import_worker(state: AppState) {
    tracing::info!("starting import worker");
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let worker_state = state.clone();
            match tokio::spawn(async move {
                crate::services::imports::process_queued_once(&worker_state).await
            })
            .await
            {
                Ok(Ok(processed)) if processed > 0 => {
                    state.metrics.inc_import_jobs_processed(processed as u64);
                    tracing::debug!(processed, "processed import jobs")
                }
                Ok(Ok(_)) => {}
                Ok(Err(err)) => tracing::warn!(?err, "import worker failed"),
                Err(err) => tracing::warn!(?err, "import worker panicked"),
            }
        }
    });
}

fn spawn_artist_hydration_worker(state: AppState) {
    tracing::info!("starting Spotify artist metadata hydration worker");
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            let worker_state = state.clone();
            match tokio::spawn(async move {
                crate::services::artist_hydration::process_queued_once(&worker_state).await
            })
            .await
            {
                Ok(Ok(hydrated)) if hydrated > 0 => {
                    tracing::debug!(hydrated, "hydrated Spotify artist metadata")
                }
                Ok(Ok(_)) => {}
                Ok(Err(err)) => tracing::warn!(?err, "artist metadata hydration worker failed"),
                Err(err) => tracing::warn!(?err, "artist metadata hydration worker panicked"),
            }
        }
    });
}

fn spawn_spotify_poller(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(120));
        loop {
            interval.tick().await;
            let worker_state = state.clone();
            match tokio::spawn(
                async move { crate::services::poller::poll_once(&worker_state).await },
            )
            .await
            {
                Ok(Ok(summary)) => tracing::debug!(?summary, "Spotify poll completed"),
                Ok(Err(err)) => tracing::warn!(?err, "Spotify poll failed"),
                Err(err) => tracing::warn!(?err, "Spotify poller panicked"),
            }
        }
    });
}

fn spawn_cleanup_jobs(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 60));
        loop {
            interval.tick().await;
            if let Err(err) = crate::repositories::sessions::cleanup_expired(&state.db).await {
                tracing::warn!(?err, "failed to clean expired sessions");
            }
            if let Err(err) = crate::repositories::oauth_states::cleanup_expired(&state.db).await {
                tracing::warn!(?err, "failed to clean expired oauth states");
            }
            if let Err(err) = crate::repositories::response_cache::cleanup_expired(&state.db).await
            {
                tracing::warn!(?err, "failed to clean expired response cache");
            }
        }
    });
}

async fn seed_dev(state: &AppState) -> Result<()> {
    let mut tx = state.db.begin().await?;
    let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"
        INSERT INTO users (username, spotify_id, admin, first_listened_at)
        VALUES ('Seed User', 'spotify_seed_user', TRUE, '2024-03-10T06:30:00Z')
        ON CONFLICT (spotify_id) DO UPDATE SET username = EXCLUDED.username
        RETURNING id
        "#,
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO user_settings (user_id, timezone) VALUES ($1, 'America/New_York') ON CONFLICT (user_id) DO NOTHING")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO artists (id, name, genres) VALUES
          ('artist_seed_alpha', 'Seed Alpha', '["indie"]'),
          ('artist_seed_beta', 'Seed Beta', '["electronic"]'),
          ('artist_seed_guest', 'Seed Guest', '["pop"]')
        ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO albums (id, name, album_type, release_date, release_date_precision, release_year, total_tracks) VALUES
          ('album_seed_dawn', 'Seed Dawn', 'album', '2023-01-01', 'day', 2023, 2),
          ('album_seed_night', 'Seed Night', 'album', '2024-01-01', 'day', 2024, 3)
        ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO album_artists (album_id, artist_id, position) VALUES
          ('album_seed_dawn', 'artist_seed_alpha', 0),
          ('album_seed_night', 'artist_seed_beta', 0)
        ON CONFLICT (album_id, artist_id) DO UPDATE SET position = EXCLUDED.position
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO tracks (id, name, album_id, duration_ms, explicit, track_number) VALUES
          ('track_seed_sunrise', 'Seed Sunrise', 'album_seed_dawn', 180000, FALSE, 1),
          ('track_seed_noon', 'Seed Noon', 'album_seed_dawn', 210000, FALSE, 2),
          ('track_seed_midnight', 'Seed Midnight', 'album_seed_night', 240000, FALSE, 1),
          ('track_seed_duet', 'Seed Duet', 'album_seed_night', 200000, FALSE, 2),
          ('track_seed_guest', 'Seed Guest Solo', 'album_seed_night', 190000, FALSE, 3)
        ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, duration_ms = EXCLUDED.duration_ms
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO track_artists (track_id, artist_id, position) VALUES
          ('track_seed_sunrise', 'artist_seed_alpha', 0),
          ('track_seed_noon', 'artist_seed_alpha', 0),
          ('track_seed_midnight', 'artist_seed_beta', 0),
          ('track_seed_duet', 'artist_seed_beta', 0),
          ('track_seed_duet', 'artist_seed_guest', 1),
          ('track_seed_guest', 'artist_seed_guest', 0)
        ON CONFLICT (track_id, artist_id) DO UPDATE SET position = EXCLUDED.position
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO user_blacklisted_artists (user_id, artist_id)
        VALUES ($1, 'artist_seed_guest')
        ON CONFLICT (user_id, artist_id) DO NOTHING
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    for (track_id, album_id, artist_id, duration_ms, played_at) in [
        (
            "track_seed_sunrise",
            "album_seed_dawn",
            "artist_seed_alpha",
            180000,
            "2024-02-29T23:58:00Z",
        ),
        (
            "track_seed_noon",
            "album_seed_dawn",
            "artist_seed_alpha",
            210000,
            "2024-03-01T00:02:00Z",
        ),
        (
            "track_seed_midnight",
            "album_seed_night",
            "artist_seed_beta",
            240000,
            "2024-03-10T06:55:00Z",
        ),
        (
            "track_seed_duet",
            "album_seed_night",
            "artist_seed_beta",
            200000,
            "2024-03-10T07:05:00Z",
        ),
        (
            "track_seed_sunrise",
            "album_seed_dawn",
            "artist_seed_alpha",
            180000,
            "2024-10-31T23:59:00Z",
        ),
        (
            "track_seed_midnight",
            "album_seed_night",
            "artist_seed_beta",
            240000,
            "2025-01-01T00:01:00Z",
        ),
        (
            "track_seed_guest",
            "album_seed_night",
            "artist_seed_guest",
            190000,
            "2025-01-01T00:08:00Z",
        ),
        (
            "track_seed_sunrise",
            "album_seed_dawn",
            "artist_seed_alpha",
            180000,
            "2025-01-01T00:12:00Z",
        ),
        (
            "track_seed_noon",
            "album_seed_dawn",
            "artist_seed_alpha",
            210000,
            "2025-01-01T00:17:00Z",
        ),
        (
            "track_seed_midnight",
            "album_seed_night",
            "artist_seed_beta",
            240000,
            "2025-01-01T01:00:00Z",
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO listening_events (user_id, track_id, album_id, primary_artist_id, duration_ms, played_at, blacklisted_by, source)
            VALUES ($1, $2, $3, $4, $5, $6::timestamptz, CASE WHEN $4 = 'artist_seed_guest' THEN 'artist' ELSE NULL END, 'seed')
            ON CONFLICT (user_id, track_id, played_at) DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(track_id)
        .bind(album_id)
        .bind(artist_id)
        .bind(duration_ms)
        .bind(played_at)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO import_jobs (user_id, import_type, status, total, current, metadata)
        VALUES ($1, 'privacy', 'success', 6, 6, '{"seed": true}'::jsonb)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    tracing::info!(%user_id, "seeded development data");
    Ok(())
}
