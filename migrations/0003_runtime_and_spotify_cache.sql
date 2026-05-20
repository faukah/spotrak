-- Runtime coordination and durable Spotify lookup cache.

CREATE TABLE IF NOT EXISTS app_runtime_state (
  key TEXT PRIMARY KEY,
  value_ts TIMESTAMPTZ,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS spotify_search_cache (
  query_key TEXT PRIMARY KEY,
  query TEXT NOT NULL,
  track_id TEXT REFERENCES tracks(id) ON DELETE SET NULL,
  raw JSONB,
  found BOOLEAN NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_spotify_search_cache_track_id ON spotify_search_cache(track_id);
