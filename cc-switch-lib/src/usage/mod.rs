//! Usage tracking module
//!
//! Parses usage information from API responses and records to database.

mod parser;

pub use parser::UsageParser;

use crate::database::UsageRecord;

/// Trait for extracting usage from responses
pub trait UsageExtractor: Send + Sync {
    /// Parse usage from a response body
    fn extract(&self, body: &[u8]) -> Option<UsageRecord>;
}
