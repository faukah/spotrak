-- Durable backend response cache and deduplicated Spotify metadata hydration queue.

CREATE TABLE IF NOT EXISTS response_cache (
  namespace TEXT NOT NULL,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  cache_key TEXT NOT NULL,
  payload JSONB NOT NULL,
  computed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at TIMESTAMPTZ,
  PRIMARY KEY (namespace, user_id, cache_key)
);

CREATE INDEX IF NOT EXISTS idx_response_cache_expires_at
  ON response_cache(expires_at)
  WHERE expires_at IS NOT NULL;

CREATE TABLE IF NOT EXISTS spotify_artist_hydration_queue (
  artist_id TEXT PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  attempts INTEGER NOT NULL DEFAULT 0,
  next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_error TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_spotify_artist_hydration_queue_due
  ON spotify_artist_hydration_queue(next_attempt_at, created_at);
