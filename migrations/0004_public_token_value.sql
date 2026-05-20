-- Store the raw public share token so the owner can copy the current link.
-- The token is still looked up by hash for public access checks.

ALTER TABLE public_tokens
  ADD COLUMN IF NOT EXISTS token_value TEXT;
