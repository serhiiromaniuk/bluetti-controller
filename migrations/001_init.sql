-- migrations/001_init.sql
CREATE TABLE station (
  id           INTEGER PRIMARY KEY,
  name         TEXT NOT NULL UNIQUE,      -- from config, e.g. "router"
  model        TEXT NOT NULL,             -- EB3A | AC180 | Elite200V2
  capacity_wh  REAL NOT NULL,
  grid_sensor  INTEGER NOT NULL DEFAULT 0 -- counts towards the house outage state
);

-- one row per successful poll; core metrics only
CREATE TABLE sample (
  station_id INTEGER NOT NULL REFERENCES station(id),
  ts         INTEGER NOT NULL,            -- unix seconds, UTC
  soc        REAL,                        -- %
  ac_in_w    REAL, dc_in_w  REAL,
  ac_out_w   REAL, dc_out_w REAL,
  ac_in_v    REAL,
  encrypted  INTEGER NOT NULL,
  PRIMARY KEY (station_id, ts)
) WITHOUT ROWID;

-- every other decoded field, verified or not (extra telemetry)
CREATE TABLE sample_extra (
  station_id INTEGER NOT NULL,
  ts         INTEGER NOT NULL,
  metric     TEXT NOT NULL,               -- Metric::key()
  value      REAL NOT NULL,
  PRIMARY KEY (station_id, ts, metric)
) WITHOUT ROWID;

CREATE TABLE poll_failure (
  station_id INTEGER NOT NULL,
  ts         INTEGER NOT NULL,
  stage      TEXT NOT NULL,               -- discover|connect|handshake|read|decode
  error      TEXT NOT NULL,
  PRIMARY KEY (station_id, ts)
) WITHOUT ROWID;

-- hourly raw register blocks, for decoding new fields later (90-day retention)
CREATE TABLE register_snapshot (
  station_id INTEGER NOT NULL,
  ts         INTEGER NOT NULL,
  start_reg  INTEGER NOT NULL,
  data       BLOB NOT NULL,               -- big-endian u16 values
  PRIMARY KEY (station_id, ts, start_reg)
) WITHOUT ROWID;

-- grid presence per station and for the house; ended_at NULL = ongoing
CREATE TABLE grid_event (
  id          INTEGER PRIMARY KEY,
  scope       TEXT NOT NULL,              -- 'house' or a station name
  kind        TEXT NOT NULL,              -- 'outage'
  started_at  INTEGER NOT NULL,
  ended_at    INTEGER,
  precision_s INTEGER NOT NULL            -- = poll interval at detection
);

-- 5-minute rollups, kept forever
CREATE TABLE rollup_5m (
  station_id    INTEGER NOT NULL,
  ts            INTEGER NOT NULL,         -- bucket start
  soc_avg REAL, soc_min REAL,
  ac_in_w_avg REAL, dc_in_w_avg REAL, ac_out_w_avg REAL, dc_out_w_avg REAL,
  energy_in_wh  REAL, energy_out_wh REAL,
  n             INTEGER NOT NULL,
  PRIMARY KEY (station_id, ts)
) WITHOUT ROWID;
