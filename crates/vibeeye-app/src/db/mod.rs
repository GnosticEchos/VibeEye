//! SurrealDB embedded storage for crawled content.
//!
//! Feature-gated behind `surrealdb`. All operations use a single database
//! with a `group` field for logical crawl isolation.

pub mod client;
pub mod export;
pub mod import;
pub mod migrations;
pub mod models;
pub mod ops;
pub mod output;
pub mod schema;
pub mod util;

pub use client::DbClient;
pub use models::{LinkRecord, PageRecord, QueryResult};
pub use output::SurrealOutput;
