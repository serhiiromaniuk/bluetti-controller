//! The BlueZ adapter: the only module that imports `bluer` (RULES.md
//! architecture rules). It turns BlueZ objects into our own [`Advert`] and,
//! from step 9, a [`Gatt`](super::Gatt) session, so core code never sees a
//! `bluer` type.
//!
//! [`Advert`]: super::Advert

pub mod scan;
