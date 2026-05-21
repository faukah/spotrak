use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{
        catalog::{AlbumDetail, AlbumRef, ArtistDetail, EntityRef, TrackDetail},
        spotify::{
            SpotifyAlbum, SpotifyArtist, SpotifyRecentlyPlayedItem, SpotifySimpleArtist,
            SpotifyTrack,
        },
    },
    error::{AppError, Result},
};

#[derive(Debug, FromRow)]
struct TrackRow {
    id: String,
    name: String,
    duration_ms: i32,
    explicit: bool,
    href: Option<String>,
    uri: Option<String>,
    popularity: Option<i32>,
    disc_number: Option<i32>,
    track_number: Option<i32>,
    images: Value,
    album_id: String,
    album_name: String,
    album_images: Value,
}

#[derive(Debug, FromRow)]
struct ArtistRow {
    id: String,
    name: String,
    href: Option<String>,
    uri: Option<String>,
    popularity: Option<i32>,
    images: Value,
    genres: Value,
    blacklisted: bool,
}

#[derive(Debug, FromRow)]
struct AlbumRow {
    id: String,
    name: String,
    album_type: Option<String>,
    release_date: Option<String>,
    release_year: Option<i32>,
    total_tracks: Option<i32>,
    href: Option<String>,
    uri: Option<String>,
    images: Value,
}

#[derive(Debug, FromRow)]
struct SpotifySearchCacheRow {
    found: bool,
    raw: Option<Value>,
}

pub enum SpotifySearchCacheHit {
    Found(Box<SpotifyTrack>),
    NotFound,
}

pub async fn user_has_track(pool: &PgPool, user_id: Uuid, track_id: &str) -> Result<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM listening_events
          WHERE user_id = $1
            AND track_id = $2
            AND blacklisted_by IS NULL
        )
        "#,
    )
    .bind(user_id)
    .bind(track_id)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

pub async fn user_has_artist(pool: &PgPool, user_id: Uuid, artist_id: &str) -> Result<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM listening_events le
          WHERE le.user_id = $1
            AND le.blacklisted_by IS NULL
            AND (
              le.primary_artist_id = $2
              OR EXISTS (
                SELECT 1 FROM track_artists ta
                WHERE ta.track_id = le.track_id AND ta.artist_id = $2
              )
            )
        )
        "#,
    )
    .bind(user_id)
    .bind(artist_id)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

pub async fn user_has_album(pool: &PgPool, user_id: Uuid, album_id: &str) -> Result<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM listening_events
          WHERE user_id = $1
            AND album_id = $2
            AND blacklisted_by IS NULL
        )
        "#,
    )
    .bind(user_id)
    .bind(album_id)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

pub async fn spotify_track_from_cache(pool: &PgPool, id: &str) -> Result<Option<SpotifyTrack>> {
    let raw = sqlx::query_scalar::<_, Value>(
        r#"
        SELECT raw
        FROM tracks
        WHERE id = $1 AND raw IS NOT NULL
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(raw.and_then(|value| serde_json::from_value::<SpotifyTrack>(value).ok()))
}

pub async fn spotify_track_from_name_artist_cache(
    pool: &PgPool,
    track_name: &str,
    artist_name: &str,
) -> Result<Option<SpotifyTrack>> {
    let raw = sqlx::query_scalar::<_, Value>(
        r#"
        SELECT t.raw
        FROM tracks t
        JOIN track_artists ta ON ta.track_id = t.id
        JOIN artists a ON a.id = ta.artist_id
        WHERE lower(t.name) = lower($1)
          AND lower(a.name) = lower($2)
          AND t.raw IS NOT NULL
        ORDER BY ta.position ASC, t.updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(track_name)
    .bind(artist_name)
    .fetch_optional(pool)
    .await?;

    Ok(raw.and_then(|value| serde_json::from_value::<SpotifyTrack>(value).ok()))
}

pub async fn spotify_search_cache(
    pool: &PgPool,
    query_key: &str,
) -> Result<Option<SpotifySearchCacheHit>> {
    let row = sqlx::query_as::<_, SpotifySearchCacheRow>(
        r#"
        SELECT found, raw
        FROM spotify_search_cache
        WHERE query_key = $1
        "#,
    )
    .bind(query_key)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };
    if !row.found {
        return Ok(Some(SpotifySearchCacheHit::NotFound));
    }
    let Some(raw) = row.raw else {
        return Ok(None);
    };
    Ok(serde_json::from_value::<SpotifyTrack>(raw)
        .ok()
        .map(Box::new)
        .map(SpotifySearchCacheHit::Found))
}

pub async fn upsert_spotify_search_cache(
    pool: &PgPool,
    query_key: &str,
    query: &str,
    track: Option<&SpotifyTrack>,
) -> Result<()> {
    let raw = track.map(|track| serde_json::to_value(track).unwrap_or(Value::Null));
    sqlx::query(
        r#"
        INSERT INTO spotify_search_cache (query_key, query, track_id, raw, found, updated_at)
        VALUES ($1, $2, NULL, $3, $4, now())
        ON CONFLICT (query_key) DO UPDATE SET
          query = EXCLUDED.query,
          raw = EXCLUDED.raw,
          found = EXCLUDED.found,
          updated_at = now()
        "#,
    )
    .bind(query_key)
    .bind(query)
    .bind(raw)
    .bind(track.is_some())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn has_user_track_play_near(
    pool: &PgPool,
    user_id: Uuid,
    track_id: &str,
    played_at: DateTime<Utc>,
) -> Result<bool> {
    let duplicate = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM listening_events
          WHERE user_id = $1
            AND track_id = $2
            AND played_at BETWEEN $3::timestamptz - interval '30 seconds'
                          AND $3::timestamptz + interval '30 seconds'
        )
        "#,
    )
    .bind(user_id)
    .bind(track_id)
    .bind(played_at)
    .fetch_one(pool)
    .await?;
    Ok(duplicate)
}

pub async fn track(pool: &PgPool, id: &str) -> Result<TrackDetail> {
    let row = sqlx::query_as::<_, TrackRow>(
        r#"
        SELECT t.id, t.name, t.duration_ms, t.explicit, t.href, t.uri, t.popularity,
               t.disc_number, t.track_number, t.images,
               a.id AS album_id, a.name AS album_name, a.images AS album_images
        FROM tracks t
        JOIN albums a ON a.id = t.album_id
        WHERE t.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let artists = track_artists(pool, id).await?;
    Ok(TrackDetail {
        id: row.id,
        name: row.name,
        duration_ms: row.duration_ms,
        explicit: row.explicit,
        href: row.href,
        uri: row.uri,
        popularity: row.popularity,
        disc_number: row.disc_number,
        track_number: row.track_number,
        images: row.images,
        album: AlbumRef {
            id: row.album_id,
            name: row.album_name,
            images: row.album_images,
        },
        artists,
    })
}

pub async fn artist(pool: &PgPool, user_id: Uuid, id: &str) -> Result<ArtistDetail> {
    let row = sqlx::query_as::<_, ArtistRow>(
        r#"
        SELECT a.id, a.name, a.href, a.uri, a.popularity,
               COALESCE(
                 NULLIF(a.images, '[]'::jsonb),
                 CASE WHEN cover.url IS NOT NULL THEN jsonb_build_array(jsonb_build_object('url', cover.url, 'source', 'album-fallback')) ELSE '[]'::jsonb END
               ) AS images,
               a.genres,
               EXISTS (
                 SELECT 1 FROM user_blacklisted_artists uba
                 WHERE uba.artist_id = a.id AND uba.user_id = $2
               ) AS blacklisted
        FROM artists a
        LEFT JOIN LATERAL (
          SELECT COALESCE(t.images->0->>'url', al.images->0->>'url') AS url
          FROM listening_events le
          JOIN tracks t ON t.id = le.track_id
          JOIN albums al ON al.id = le.album_id
          WHERE le.user_id = $2 AND le.primary_artist_id = a.id
          ORDER BY le.played_at DESC
          LIMIT 1
        ) cover ON TRUE
        WHERE a.id = $1
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(ArtistDetail {
        id: row.id,
        name: row.name,
        href: row.href,
        uri: row.uri,
        popularity: row.popularity,
        images: row.images,
        genres: row.genres,
        blacklisted: row.blacklisted,
    })
}

pub async fn album(pool: &PgPool, id: &str) -> Result<AlbumDetail> {
    let row = sqlx::query_as::<_, AlbumRow>(
        r#"
        SELECT id, name, album_type, release_date, release_year, total_tracks, href, uri, images
        FROM albums
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let artists = album_artists(pool, id).await?;
    Ok(AlbumDetail {
        id: row.id,
        name: row.name,
        album_type: row.album_type,
        release_date: row.release_date,
        release_year: row.release_year,
        total_tracks: row.total_tracks,
        href: row.href,
        uri: row.uri,
        images: row.images,
        artists,
    })
}

pub async fn track_artists(pool: &PgPool, track_id: &str) -> Result<Vec<EntityRef>> {
    let artists = sqlx::query_as::<_, EntityRef>(
        r#"
        SELECT a.id, a.name
        FROM track_artists ta
        JOIN artists a ON a.id = ta.artist_id
        WHERE ta.track_id = $1
        ORDER BY ta.position ASC, a.name ASC, a.id ASC
        "#,
    )
    .bind(track_id)
    .fetch_all(pool)
    .await?;
    Ok(artists)
}

pub async fn album_artists(pool: &PgPool, album_id: &str) -> Result<Vec<EntityRef>> {
    let artists = sqlx::query_as::<_, EntityRef>(
        r#"
        SELECT a.id, a.name
        FROM album_artists aa
        JOIN artists a ON a.id = aa.artist_id
        WHERE aa.album_id = $1
        ORDER BY aa.position ASC, a.name ASC, a.id ASC
        "#,
    )
    .bind(album_id)
    .fetch_all(pool)
    .await?;
    Ok(artists)
}

pub async fn blacklist_artist(pool: &PgPool, user_id: Uuid, artist_id: &str) -> Result<()> {
    let mut tx = pool.begin().await?;
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM artists WHERE id = $1)")
            .bind(artist_id)
            .fetch_one(&mut *tx)
            .await?;
    if !exists {
        return Err(AppError::NotFound);
    }

    sqlx::query(
        r#"
        INSERT INTO user_blacklisted_artists (user_id, artist_id)
        VALUES ($1, $2)
        ON CONFLICT (user_id, artist_id) DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(artist_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE listening_events
        SET blacklisted_by = 'artist'
        WHERE user_id = $1 AND primary_artist_id = $2
        "#,
    )
    .bind(user_id)
    .bind(artist_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn unblacklist_artist(pool: &PgPool, user_id: Uuid, artist_id: &str) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM user_blacklisted_artists WHERE user_id = $1 AND artist_id = $2")
        .bind(user_id)
        .bind(artist_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        UPDATE listening_events
        SET blacklisted_by = NULL
        WHERE user_id = $1 AND primary_artist_id = $2 AND blacklisted_by = 'artist'
        "#,
    )
    .bind(user_id)
    .bind(artist_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn upsert_recently_played_event(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    event: &SpotifyRecentlyPlayedItem,
    source: &str,
    import_job_id: Option<Uuid>,
) -> Result<bool> {
    if event.track.is_local {
        return Ok(false);
    }

    let Some(track_id) = event.track.id.as_deref() else {
        return Ok(false);
    };
    let Some(album_id) = event.track.album.id.as_deref() else {
        return Ok(false);
    };
    let Some(primary_artist) = event
        .track
        .artists
        .iter()
        .find(|artist| artist.id.is_some())
    else {
        return Ok(false);
    };
    let Some(primary_artist_id) = primary_artist.id.as_deref() else {
        return Ok(false);
    };

    let duplicate = has_fuzzy_duplicate(tx, user_id, track_id, event.played_at).await?;
    if duplicate {
        return Ok(false);
    }

    for artist in event
        .track
        .album
        .artists
        .iter()
        .chain(event.track.artists.iter())
    {
        upsert_artist(tx, artist).await?;
    }
    upsert_album(tx, &event.track.album).await?;
    for (position, artist) in event.track.album.artists.iter().enumerate() {
        if let Some(artist_id) = artist.id.as_deref() {
            sqlx::query(
                r#"
                INSERT INTO album_artists (album_id, artist_id, position)
                VALUES ($1, $2, $3)
                ON CONFLICT (album_id, artist_id) DO UPDATE SET position = EXCLUDED.position
                "#,
            )
            .bind(album_id)
            .bind(artist_id)
            .bind(position as i32)
            .execute(&mut **tx)
            .await?;
        }
    }

    upsert_track(tx, &event.track).await?;
    for (position, artist) in event.track.artists.iter().enumerate() {
        if let Some(artist_id) = artist.id.as_deref() {
            sqlx::query(
                r#"
                INSERT INTO track_artists (track_id, artist_id, position)
                VALUES ($1, $2, $3)
                ON CONFLICT (track_id, artist_id) DO UPDATE SET position = EXCLUDED.position
                "#,
            )
            .bind(track_id)
            .bind(artist_id)
            .bind(position as i32)
            .execute(&mut **tx)
            .await?;
        }
    }

    let blacklisted = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM user_blacklisted_artists
          WHERE user_id = $1 AND artist_id = $2
        )
        "#,
    )
    .bind(user_id)
    .bind(primary_artist_id)
    .fetch_one(&mut **tx)
    .await?;
    let blacklisted_by = blacklisted.then_some("artist");

    let result = sqlx::query(
        r#"
        INSERT INTO listening_events (
          user_id, track_id, album_id, primary_artist_id, duration_ms, played_at, blacklisted_by, source, import_job_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(track_id)
    .bind(album_id)
    .bind(primary_artist_id)
    .bind(event.track.duration_ms)
    .bind(event.played_at)
    .bind(blacklisted_by)
    .bind(source)
    .bind(import_job_id)
    .execute(&mut **tx)
    .await?;

    Ok(result.rows_affected() == 1)
}

async fn has_fuzzy_duplicate(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    track_id: &str,
    played_at: DateTime<Utc>,
) -> Result<bool> {
    let duplicate = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM listening_events
          WHERE user_id = $1
            AND track_id = $2
            AND played_at BETWEEN $3::timestamptz - interval '30 seconds'
                          AND $3::timestamptz + interval '30 seconds'
        )
        "#,
    )
    .bind(user_id)
    .bind(track_id)
    .bind(played_at)
    .fetch_one(&mut **tx)
    .await?;
    Ok(duplicate)
}

pub async fn artists_missing_images(pool: &PgPool, ids: &[String]) -> Result<Vec<String>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT input.id
        FROM unnest($1::text[]) AS input(id)
        LEFT JOIN artists a ON a.id = input.id
        WHERE a.id IS NULL OR jsonb_array_length(a.images) = 0
        "#,
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn upsert_artist_metadata(pool: &PgPool, artists: &[SpotifyArtist]) -> Result<()> {
    if artists.is_empty() {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    for artist in artists {
        upsert_full_artist(&mut tx, artist).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn upsert_artist(
    tx: &mut Transaction<'_, Postgres>,
    artist: &SpotifySimpleArtist,
) -> Result<()> {
    let Some(id) = artist.id.as_deref() else {
        return Ok(());
    };
    let raw = serde_json::to_value(artist).unwrap_or(Value::Null);
    sqlx::query(
        r#"
        INSERT INTO artists (id, name, href, uri, type, raw)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (id) DO UPDATE SET
          name = EXCLUDED.name,
          href = COALESCE(EXCLUDED.href, artists.href),
          uri = COALESCE(EXCLUDED.uri, artists.uri),
          type = COALESCE(EXCLUDED.type, artists.type),
          raw = COALESCE(EXCLUDED.raw, artists.raw),
          updated_at = now()
        "#,
    )
    .bind(id)
    .bind(&artist.name)
    .bind(&artist.href)
    .bind(&artist.uri)
    .bind(&artist.item_type)
    .bind(raw)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn upsert_full_artist(
    tx: &mut Transaction<'_, Postgres>,
    artist: &SpotifyArtist,
) -> Result<()> {
    let images = serde_json::to_value(&artist.images).unwrap_or_else(|_| json!([]));
    let genres = serde_json::to_value(&artist.genres).unwrap_or_else(|_| json!([]));
    let raw = serde_json::to_value(artist).unwrap_or(Value::Null);
    sqlx::query(
        r#"
        INSERT INTO artists (id, name, href, uri, type, popularity, images, genres, raw)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (id) DO UPDATE SET
          name = EXCLUDED.name,
          href = COALESCE(EXCLUDED.href, artists.href),
          uri = COALESCE(EXCLUDED.uri, artists.uri),
          type = COALESCE(EXCLUDED.type, artists.type),
          popularity = COALESCE(EXCLUDED.popularity, artists.popularity),
          images = CASE
            WHEN jsonb_array_length(EXCLUDED.images) > 0 THEN EXCLUDED.images
            ELSE artists.images
          END,
          genres = CASE
            WHEN jsonb_array_length(EXCLUDED.genres) > 0 THEN EXCLUDED.genres
            ELSE artists.genres
          END,
          raw = COALESCE(EXCLUDED.raw, artists.raw),
          updated_at = now()
        "#,
    )
    .bind(&artist.id)
    .bind(&artist.name)
    .bind(&artist.href)
    .bind(&artist.uri)
    .bind(&artist.item_type)
    .bind(artist.popularity)
    .bind(images)
    .bind(genres)
    .bind(raw)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn upsert_album(tx: &mut Transaction<'_, Postgres>, album: &SpotifyAlbum) -> Result<()> {
    let Some(id) = album.id.as_deref() else {
        return Ok(());
    };
    let release_year = album
        .release_date
        .as_deref()
        .and_then(|value| value.get(0..4))
        .and_then(|value| value.parse::<i32>().ok());
    let images = serde_json::to_value(&album.images).unwrap_or_else(|_| json!([]));
    let raw = serde_json::to_value(album).unwrap_or(Value::Null);

    sqlx::query(
        r#"
        INSERT INTO albums (
          id, name, album_type, release_date, release_date_precision, release_year,
          total_tracks, href, uri, type, images, raw
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (id) DO UPDATE SET
          name = EXCLUDED.name,
          album_type = COALESCE(EXCLUDED.album_type, albums.album_type),
          release_date = COALESCE(EXCLUDED.release_date, albums.release_date),
          release_date_precision = COALESCE(EXCLUDED.release_date_precision, albums.release_date_precision),
          release_year = COALESCE(EXCLUDED.release_year, albums.release_year),
          total_tracks = COALESCE(EXCLUDED.total_tracks, albums.total_tracks),
          href = COALESCE(EXCLUDED.href, albums.href),
          uri = COALESCE(EXCLUDED.uri, albums.uri),
          type = COALESCE(EXCLUDED.type, albums.type),
          images = CASE
            WHEN jsonb_array_length(EXCLUDED.images) > 0 THEN EXCLUDED.images
            ELSE albums.images
          END,
          raw = COALESCE(EXCLUDED.raw, albums.raw),
          updated_at = now()
        "#,
    )
    .bind(id)
    .bind(&album.name)
    .bind(&album.album_type)
    .bind(&album.release_date)
    .bind(&album.release_date_precision)
    .bind(release_year)
    .bind(album.total_tracks)
    .bind(&album.href)
    .bind(&album.uri)
    .bind(&album.item_type)
    .bind(images)
    .bind(raw)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn upsert_track(tx: &mut Transaction<'_, Postgres>, track: &SpotifyTrack) -> Result<()> {
    let Some(id) = track.id.as_deref() else {
        return Ok(());
    };
    let Some(album_id) = track.album.id.as_deref() else {
        return Ok(());
    };
    let images = serde_json::to_value(&track.album.images).unwrap_or_else(|_| json!([]));
    let raw = serde_json::to_value(track).unwrap_or(Value::Null);

    sqlx::query(
        r#"
        INSERT INTO tracks (
          id, name, album_id, duration_ms, explicit, href, uri, type, popularity,
          disc_number, track_number, images, raw
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ON CONFLICT (id) DO UPDATE SET
          name = EXCLUDED.name,
          album_id = EXCLUDED.album_id,
          duration_ms = EXCLUDED.duration_ms,
          explicit = EXCLUDED.explicit,
          href = COALESCE(EXCLUDED.href, tracks.href),
          uri = COALESCE(EXCLUDED.uri, tracks.uri),
          type = COALESCE(EXCLUDED.type, tracks.type),
          popularity = COALESCE(EXCLUDED.popularity, tracks.popularity),
          disc_number = COALESCE(EXCLUDED.disc_number, tracks.disc_number),
          track_number = COALESCE(EXCLUDED.track_number, tracks.track_number),
          images = CASE
            WHEN jsonb_array_length(EXCLUDED.images) > 0 THEN EXCLUDED.images
            ELSE tracks.images
          END,
          raw = COALESCE(EXCLUDED.raw, tracks.raw),
          updated_at = now()
        "#,
    )
    .bind(id)
    .bind(&track.name)
    .bind(album_id)
    .bind(track.duration_ms)
    .bind(track.explicit)
    .bind(&track.href)
    .bind(&track.uri)
    .bind(&track.item_type)
    .bind(track.popularity)
    .bind(track.disc_number)
    .bind(track.track_number)
    .bind(images)
    .bind(raw)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
