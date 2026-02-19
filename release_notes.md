# v0.3.0 - SurrealDB v3.0.0 Migration

This release marks a major upgrade to **SurrealDB v3.0.0**, bringing strict schema validation and improved type safety.

## 🚨 Breaking Changes
- **Database Schema**: All tables are now `SCHEMAFULL` for stricter validation.
- **Type Handling**: Strict enforcement of `RecordId` types. `Thing` struct usage updated to match SDK v3 requirements.
- **Query Syntax**: Deprecated `IS NOT NULL` syntax replaced with `IS NOT NONE`.

## ✨ Features & Improvements
- **SurrealDB v3 Support**: Full compatibility with `surrealdb` crate v3.0.0 and `surrealdb-types` v3.0.0.
- **Enhanced Schema**:
  - `memories` table now tracks `superseded_by` and `valid_until`.
  - `symbol_relation` table now includes source locations (`file_path`, `line_number`) and timestamps.
- **Robustness**:
  - `reset_db` operation is now atomic per-table, preventing failures when tables are missing.
  - Fixed N+1 query issues in relation fetching.
  - Implemented manual `Value::Object` deserialization to bypass SDK limitations with `RecordId`.

## 🛠️ Fixes
- Fixed serialisation issues where `RecordId` fields in `Relation` and `SymbolRelation` were not parsing correctly.
- Addressed 70+ compilation errors related to the SDK upgrade.
- Verified 73/73 unit tests passing.

---
**Upgrade Guide**:
Existing databases from v0.2.x are **NOT compatible** due to schema changes. Please re-index your data or start with a fresh database.
