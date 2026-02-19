//! Validated SurrealDB Thing ID type for SQL injection prevention.
//!
//! This module provides a type-safe wrapper around SurrealDB Thing IDs
//! that validates input at construction time, preventing SQL injection.

use anyhow::{ensure, Result};
use std::fmt;

/// A validated SurrealDB Thing ID (table:id format).
///
/// This type ensures that both the table name and ID contain only
/// safe characters, preventing SQL injection attacks.
///
/// # Examples
/// ```ignore
/// use memory_mcp::types::ThingId;
///
/// let thing = ThingId::new("entities", "abc123")?;
/// assert_eq!(thing.as_str(), "entities:abc123");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ThingId(String);

impl ThingId {
    /// Creates a new validated ThingId.
    ///
    /// # Arguments
    /// * `table` - The SurrealDB table name (alphanumeric + underscore)
    /// * `id` - The record ID (alphanumeric + underscore + hyphen)
    ///
    /// # Errors
    /// Returns an error if the table or id contain invalid characters.
    pub fn new(table: &str, id: &str) -> Result<Self> {
        ensure!(!table.is_empty(), "Table name cannot be empty");
        ensure!(!id.is_empty(), "ID cannot be empty");
        ensure!(
            Self::is_valid_table_name(table),
            "Invalid table name '{}': must contain only alphanumeric characters and underscores",
            table
        );
        ensure!(
            Self::is_valid_id(id),
            "Invalid ID '{}': must contain only alphanumeric characters, underscores, and hyphens",
            id
        );

        Ok(Self(format!("{}:{}", table, id)))
    }

    /// Creates a ThingId from an existing Thing-format string.
    ///
    /// Validates that the string is in "table:id" format with valid characters.
    pub fn parse(thing_str: &str) -> Result<Self> {
        let parts: Vec<&str> = thing_str.splitn(2, ':').collect();
        ensure!(
            parts.len() == 2,
            "Invalid Thing format '{}': expected 'table:id'",
            thing_str
        );
        Self::new(parts[0], parts[1])
    }

    /// Returns the full Thing ID string (table:id format).
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns just the table name portion.
    pub fn table(&self) -> &str {
        self.0.split(':').next().unwrap_or("")
    }

    /// Returns just the ID portion.
    pub fn id(&self) -> &str {
        self.0.split(':').nth(1).unwrap_or("")
    }

    /// Validates a table name.
    /// Must start with a letter or underscore, followed by alphanumeric or underscore.
    fn is_valid_table_name(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }
        let mut chars = s.chars();
        let first = chars.next().unwrap();
        if !first.is_ascii_alphabetic() && first != '_' {
            return false;
        }
        chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    /// Validates an ID.
    /// Can contain alphanumeric, underscore, and hyphen.
    fn is_valid_id(s: &str) -> bool {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    }

    /// Convert to native SurrealDB Thing for query binding.
    ///
    /// This is the primary method for creating type-safe bindings
    /// that work correctly with SurrealDB's Record Link type matching.
    ///
    /// # Example
    /// ```ignore
    /// let thing_id = ThingId::new("entities", "abc123")?;
    /// let thing = thing_id.to_thing();
    /// db.query("SELECT * FROM relations WHERE `in` = $id")
    ///     .bind(("id", thing))
    ///     .await?;
    /// ```
    pub fn to_thing(&self) -> super::Thing {
        super::RecordId::new(self.table().to_string(), self.id().to_string())
    }
}

/// Batch conversion helper for IN queries.
///
/// Converts a slice of string IDs to a Vec of SurrealDB Things
/// for use with `WHERE x IN $ids` queries.
///
/// # Arguments
/// * `table` - The SurrealDB table name
/// * `ids` - Slice of ID strings
///
/// # Example
/// ```ignore
/// let things = things_from_ids("entities", &["a", "b", "c"])?;
/// db.query("SELECT * FROM relations WHERE `in` IN $ids")
///     .bind(("ids", things))
///     .await?;
/// ```
pub fn things_from_ids(table: &str, ids: &[String]) -> Result<Vec<super::Thing>> {
    ids.iter()
        .map(|id| ThingId::new(table, id).map(|t| t.to_thing()))
        .collect()
}

impl fmt::Display for ThingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ThingId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_thing_id() {
        let thing = ThingId::new("entities", "abc123").unwrap();
        assert_eq!(thing.as_str(), "entities:abc123");
        assert_eq!(thing.table(), "entities");
        assert_eq!(thing.id(), "abc123");
    }

    #[test]
    fn test_valid_with_underscore_and_hyphen() {
        let thing = ThingId::new("code_symbols", "abc-123_def").unwrap();
        assert_eq!(thing.as_str(), "code_symbols:abc-123_def");
    }

    #[test]
    fn test_parse_valid() {
        let thing = ThingId::parse("relations:xyz789").unwrap();
        assert_eq!(thing.table(), "relations");
        assert_eq!(thing.id(), "xyz789");
    }

    #[test]
    fn test_invalid_table_sql_injection() {
        // Attempt SQL injection in table name
        let result = ThingId::new("entities; DROP TABLE--", "id");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_id_sql_injection() {
        // Attempt SQL injection in ID
        let result = ThingId::new("entities", "id'; DELETE FROM entities--");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_empty_table() {
        let result = ThingId::new("", "id");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_empty_id() {
        let result = ThingId::new("table", "");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_special_chars() {
        assert!(ThingId::new("table", "id\"test").is_err());
        assert!(ThingId::new("table", "id'test").is_err());
        assert!(ThingId::new("table", "id;test").is_err());
        assert!(ThingId::new("table", "id/test").is_err());
        assert!(ThingId::new("table", "id\\test").is_err());
    }

    #[test]
    fn test_parse_invalid_format() {
        assert!(ThingId::parse("no_colon").is_err());
        assert!(ThingId::parse("").is_err());
    }

    #[test]
    fn test_display() {
        let thing = ThingId::new("memories", "test123").unwrap();
        assert_eq!(format!("{}", thing), "memories:test123");
    }
}
