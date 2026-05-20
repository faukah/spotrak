# Deployment and backups

## Runtime layout

- Backend: root Rust crate (`Cargo.toml`, `src/`, `migrations/`)
- Frontend: `web/` Astro/Svelte app
- Import cache: `imports/` at the repo root by default; this is local runtime data and is gitignored.

## Required environment

Set these for the backend process:

```bash
export DATABASE_URL='postgresql://spotrak:password@127.0.0.1:5432/spotrak'
export SPOTIFY_PUBLIC='spotify_client_id'
export SPOTIFY_SECRET='spotify_client_secret'
export API_ENDPOINT='http://your-host:8080'
export CLIENT_ENDPOINT='http://your-host:4322'
export CORS='http://your-host:4322'
```

Set this for the frontend build/runtime:

```bash
export PUBLIC_API_ENDPOINT='http://your-host:8080'
```

Spotify redirect URI must exactly match:

```text
$API_ENDPOINT/api/v1/auth/spotify/callback
```

## Start

Backend:

```bash
cargo run --release -- serve
```

Frontend:

```bash
deno task web:build
HOST=0.0.0.0 PORT=4322 deno task web:preview
```

## Backups

Dump PostgreSQL:

```bash
pg_dump "$DATABASE_URL" > spotrak_backup.sql
```

Restore into an empty database:

```bash
psql "$DATABASE_URL" < spotrak_backup.sql
```
