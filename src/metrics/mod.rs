//! Derived metrics (ROADMAP.md §7): pure functions and state machines over
//! samples. No I/O and no async; the store's writer task drives them.

pub mod capacity;
pub mod energy;
pub mod grid;
pub mod house;
pub mod localtime;
pub mod runtime;
