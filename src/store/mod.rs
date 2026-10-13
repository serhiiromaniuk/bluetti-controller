//! SQLite storage (ROADMAP.md §6): one task owns the write connection, readers open
//! the file read-only, and the schema changes only through numbered migrations.

mod schema;

pub use schema::{open_reader, open_writer};
