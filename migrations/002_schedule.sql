-- migrations/002_schedule.sql — R3 slice 1 (ROADMAP.md §6, ADR-010)
-- one row per station output, edited in the web UI through the single writer
CREATE TABLE schedule (
  station_id INTEGER NOT NULL REFERENCES station(id),
  output     TEXT NOT NULL,               -- 'ac' | 'dc' (Output::key())
  on_min     INTEGER,                     -- minutes after local midnight, 0..1439
  off_min    INTEGER,                     -- same; at least one is set
  last_due   INTEGER,                     -- unix s of the due slot last applied; NULL = never
  PRIMARY KEY (station_id, output),
  CHECK (on_min IS NOT NULL OR off_min IS NOT NULL)
);
