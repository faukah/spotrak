ALTER TABLE listening_events
  ADD COLUMN import_job_id UUID REFERENCES import_jobs(id) ON DELETE CASCADE;

CREATE INDEX idx_events_import_job_id ON listening_events(import_job_id);
