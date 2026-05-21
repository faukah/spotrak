-- Security and queue hardening follow-ups.

ALTER TABLE oauth_states
  ADD COLUMN IF NOT EXISTS next_path TEXT;

ALTER TABLE spotify_artist_hydration_queue
  DROP CONSTRAINT IF EXISTS spotify_artist_hydration_queue_pkey;

ALTER TABLE spotify_artist_hydration_queue
  ADD CONSTRAINT spotify_artist_hydration_queue_pkey PRIMARY KEY (artist_id, user_id);
