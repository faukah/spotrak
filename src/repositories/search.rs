use sqlx::PgPool;

use crate::{
    domain::catalog::{EntityRef, SearchResults},
    error::Result,
};

pub async fn search(pool: &PgPool, query: &str, limit: i64) -> Result<SearchResults> {
    let pattern = format!("%{query}%");
    let tracks = sqlx::query_as::<_, EntityRef>(
        r#"
        SELECT id, name
        FROM tracks
        WHERE name ILIKE $1
        ORDER BY similarity(name, $2) DESC, name ASC, id ASC
        LIMIT $3
        "#,
    )
    .bind(&pattern)
    .bind(query)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let artists = sqlx::query_as::<_, EntityRef>(
        r#"
        SELECT id, name
        FROM artists
        WHERE name ILIKE $1
        ORDER BY similarity(name, $2) DESC, name ASC, id ASC
        LIMIT $3
        "#,
    )
    .bind(&pattern)
    .bind(query)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let albums = sqlx::query_as::<_, EntityRef>(
        r#"
        SELECT id, name
        FROM albums
        WHERE name ILIKE $1
        ORDER BY similarity(name, $2) DESC, name ASC, id ASC
        LIMIT $3
        "#,
    )
    .bind(&pattern)
    .bind(query)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(SearchResults {
        tracks,
        artists,
        albums,
    })
}
