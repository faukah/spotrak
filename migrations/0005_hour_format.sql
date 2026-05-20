ALTER TABLE user_settings
  ADD COLUMN hour_format TEXT NOT NULL DEFAULT '24'
    CHECK (hour_format IN ('12', '24'));
