//! Championship Manager 01/02 database importer for cLLm.
//!
//! The player supplies their own `CM3_Data` folder; this crate never ships CM data.

pub mod convert;
pub mod dat;

pub use convert::{ImportOptions, ImportSummary, build_world, finish_world};
pub use dat::Database;
