//! SQLite storage (ROADMAP.md §6): one task owns the write connection, readers open
//! the file read-only, and the schema changes only through numbered migrations.

mod rollup;
mod schema;
#[cfg(test)]
pub(crate) mod temp_db;
mod write;
mod writer;

pub use rollup::{compact, retention, rollup};
pub use schema::{open_reader, open_writer};
pub use write::{
    Failure, Health, Sample, Stage, close_outage, insert_failure, insert_sample, insert_snapshot,
    open_outage, station_health, sync_stations,
};
pub use writer::{Record, Station, Writer};
