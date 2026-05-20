-- Greenfield PostgreSQL schema for the Rust/Axum Spotrak rewrite.

CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  username TEXT NOT NULL,
  spotify_id TEXT NOT NULL UNIQUE,
  admin BOOLEAN NOT NULL DEFAULT FALSE,
  access_token TEXT,
  refresh_token TEXT,
  token_expires_at TIMESTAMPTZ,
  last_spotify_poll_at TIMESTAMPTZ,
  first_listened_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE user_settings (
  user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  history_line BOOLEAN NOT NULL DEFAULT TRUE,
  preferred_stats_period TEXT NOT NULL DEFAULT 'day'
    CHECK (preferred_stats_period IN ('day', 'week', 'month', 'year')),
  nb_elements INTEGER NOT NULL DEFAULT 10 CHECK (nb_elements BETWEEN 5 AND 50),
  metric_used TEXT NOT NULL DEFAULT 'number'
    CHECK (metric_used IN ('number', 'duration')),
  dark_mode TEXT NOT NULL DEFAULT 'follow'
    CHECK (dark_mode IN ('follow', 'dark', 'light')),
  timezone TEXT,
  date_format TEXT NOT NULL DEFAULT 'yyyy-MM-dd',
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE global_preferences (
  id BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),
  allow_registrations BOOLEAN NOT NULL DEFAULT TRUE,
  allow_affinity BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO global_preferences (id) VALUES (TRUE)
ON CONFLICT (id) DO NOTHING;

CREATE TABLE sessions (
  token_hash TEXT PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_seen_at TIMESTAMPTZ
);

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);

CREATE TABLE oauth_states (
  state_hash TEXT PRIMARY KEY,
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_oauth_states_expires_at ON oauth_states(expires_at);

CREATE TABLE public_tokens (
  token_hash TEXT PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_public_tokens_user_id ON public_tokens(user_id);

CREATE TABLE artists (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  href TEXT,
  uri TEXT,
  type TEXT,
  popularity INTEGER,
  images JSONB NOT NULL DEFAULT '[]'::jsonb,
  genres JSONB NOT NULL DEFAULT '[]'::jsonb,
  raw JSONB,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_artists_name_trgm ON artists USING gin (name gin_trgm_ops);

CREATE TABLE albums (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  album_type TEXT,
  release_date TEXT,
  release_date_precision TEXT,
  release_year INTEGER,
  total_tracks INTEGER,
  href TEXT,
  uri TEXT,
  type TEXT,
  images JSONB NOT NULL DEFAULT '[]'::jsonb,
  raw JSONB,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_albums_name_trgm ON albums USING gin (name gin_trgm_ops);
CREATE INDEX idx_albums_release_year ON albums(release_year);

CREATE TABLE album_artists (
  album_id TEXT NOT NULL REFERENCES albums(id) ON DELETE CASCADE,
  artist_id TEXT NOT NULL REFERENCES artists(id) ON DELETE RESTRICT,
  position INTEGER NOT NULL,
  PRIMARY KEY (album_id, artist_id)
);

CREATE INDEX idx_album_artists_artist_id ON album_artists(artist_id);

CREATE TABLE tracks (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  album_id TEXT NOT NULL REFERENCES albums(id) ON DELETE RESTRICT,
  duration_ms INTEGER NOT NULL,
  explicit BOOLEAN NOT NULL DEFAULT FALSE,
  href TEXT,
  uri TEXT,
  type TEXT,
  popularity INTEGER,
  disc_number INTEGER,
  track_number INTEGER,
  images JSONB NOT NULL DEFAULT '[]'::jsonb,
  raw JSONB,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_tracks_album_id ON tracks(album_id);
CREATE INDEX idx_tracks_name_trgm ON tracks USING gin (name gin_trgm_ops);

CREATE TABLE track_artists (
  track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
  artist_id TEXT NOT NULL REFERENCES artists(id) ON DELETE RESTRICT,
  position INTEGER NOT NULL,
  PRIMARY KEY (track_id, artist_id)
);

CREATE INDEX idx_track_artists_artist_id ON track_artists(artist_id);

CREATE TABLE listening_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE RESTRICT,
  album_id TEXT NOT NULL REFERENCES albums(id) ON DELETE RESTRICT,
  primary_artist_id TEXT NOT NULL REFERENCES artists(id) ON DELETE RESTRICT,
  duration_ms INTEGER NOT NULL,
  played_at TIMESTAMPTZ NOT NULL,
  blacklisted_by TEXT CHECK (blacklisted_by IS NULL OR blacklisted_by = 'artist'),
  source TEXT NOT NULL DEFAULT 'poller'
    CHECK (source IN ('poller', 'privacy-import', 'full-privacy-import', 'seed')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (user_id, track_id, played_at)
);

CREATE INDEX idx_events_user_time
  ON listening_events(user_id, played_at DESC);

CREATE INDEX idx_events_user_track_time
  ON listening_events(user_id, track_id, played_at DESC);

CREATE INDEX idx_events_user_artist_time
  ON listening_events(user_id, primary_artist_id, played_at DESC);

CREATE INDEX idx_events_user_album_time
  ON listening_events(user_id, album_id, played_at DESC);

CREATE INDEX idx_events_user_unblacklisted_time
  ON listening_events(user_id, played_at DESC)
  WHERE blacklisted_by IS NULL;

CREATE TABLE user_blacklisted_artists (
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  artist_id TEXT NOT NULL REFERENCES artists(id) ON DELETE RESTRICT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, artist_id)
);

CREATE TABLE import_jobs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  import_type TEXT NOT NULL CHECK (import_type IN ('privacy', 'full-privacy')),
  status TEXT NOT NULL CHECK (status IN ('queued', 'progress', 'success', 'failure', 'cancelled')),
  total INTEGER NOT NULL DEFAULT 0,
  current INTEGER NOT NULL DEFAULT 0,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  error_message TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_import_jobs_user_id ON import_jobs(user_id);
CREATE INDEX idx_import_jobs_status ON import_jobs(status);

CREATE TABLE import_files (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  job_id UUID NOT NULL REFERENCES import_jobs(id) ON DELETE CASCADE,
  path TEXT NOT NULL,
  original_name TEXT NOT NULL,
  size_bytes BIGINT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
