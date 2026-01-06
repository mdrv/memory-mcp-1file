---
work_package_id: WP08
title: "Temporal Tools"
phase: "Phase 7"
priority: P2
subtasks: ["T052", "T053", "T054"]
lane: planned
dependencies: ["WP05"]
history:
  - date: 2026-01-06
    action: created
    by: spec-kitty.tasks
---

# WP08: Temporal Tools

## Objective

Implement bi-temporal query support with 3 temporal tools: get_valid, get_valid_at, invalidate.

## Context

Temporal features enable fact versioning - agents can query what was known at a specific point in time and invalidate outdated facts without losing history.

**Can run in parallel with WP07 and WP09** - only depends on WP05.

## Subtasks

### T052: Implement tool: get_valid

```rust
    /// Get all currently valid memories. Returns memories where valid_until is not set or is in the future.
    #[tool(description = "Get all currently valid memories. Returns memories where valid_until is not set or is in the future.")]
    async fn get_valid(
        &self,
        /// Optional user ID for multi-tenant isolation
        user_id: Option<String>,
        /// Maximum number of memories to return (default: 20, max: 100)
        limit: Option<usize>,
    ) -> Result<CallToolResult, McpError> {
        let limit = limit.unwrap_or(20).min(100);
        
        let memories = self.state.storage
            .get_valid(user_id.as_deref(), limit)
            .await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        Ok(CallToolResult::success(serde_json::json!({
            "results": memories,
            "count": memories.len()
        })))
    }
```

**Storage Query** (in SurrealStorage):
```sql
SELECT * FROM memories 
WHERE (valid_until IS NULL OR valid_until > time::now())
  AND ($user_id IS NULL OR user_id = $user_id)
ORDER BY ingestion_time DESC
LIMIT $limit
```

---

### T053: Implement tool: get_valid_at

```rust
    /// Get memories that were valid at a specific point in time. Timestamp in ISO 8601 format.
    #[tool(description = "Get memories that were valid at a specific point in time. Timestamp in ISO 8601 format.")]
    async fn get_valid_at(
        &self,
        /// Timestamp in ISO 8601 format (e.g., "2024-01-15T10:30:00Z")
        timestamp: String,
        /// Optional user ID for multi-tenant isolation
        user_id: Option<String>,
        /// Maximum number of memories to return (default: 20, max: 100)
        limit: Option<usize>,
    ) -> Result<CallToolResult, McpError> {
        let limit = limit.unwrap_or(20).min(100);
        
        // Parse timestamp
        let ts: chrono::DateTime<chrono::Utc> = timestamp.parse()
            .map_err(|_| McpError::invalid_params("Invalid timestamp format. Use ISO 8601 (e.g., 2024-01-15T10:30:00Z)"))?;
        
        let memories = self.state.storage
            .get_valid_at(ts, user_id.as_deref(), limit)
            .await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        Ok(CallToolResult::success(serde_json::json!({
            "results": memories,
            "count": memories.len(),
            "timestamp": timestamp
        })))
    }
```

**Storage Query**:
```sql
SELECT * FROM memories 
WHERE valid_from <= $timestamp 
  AND (valid_until IS NULL OR valid_until > $timestamp)
  AND ($user_id IS NULL OR user_id = $user_id)
ORDER BY ingestion_time DESC
LIMIT $limit
```

---

### T054: Implement tool: invalidate

```rust
    /// Invalidate (soft-delete) a memory. Sets valid_until to now and optionally links to replacement.
    #[tool(description = "Invalidate (soft-delete) a memory. Sets valid_until to now and optionally links to replacement.")]
    async fn invalidate(
        &self,
        /// The memory ID to invalidate
        id: String,
        /// Optional reason for invalidation
        reason: Option<String>,
        /// Optional ID of memory that supersedes this one
        superseded_by: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        // First check memory exists
        match self.state.storage.get_memory(&id).await {
            Ok(None) => return Ok(CallToolResult::error(format!("Memory not found: {}", id))),
            Err(e) => return Err(McpError::internal(e.to_string())),
            Ok(Some(_)) => {}
        }
        
        let success = self.state.storage
            .invalidate(&id, reason.as_deref(), superseded_by.as_deref())
            .await
            .map_err(|e| McpError::internal(e.to_string()))?;
        
        Ok(CallToolResult::success(serde_json::json!({
            "invalidated": success
        })))
    }
```

**Storage Query**:
```sql
UPDATE memories SET 
  valid_until = time::now(),
  invalidation_reason = $reason
WHERE id = $id
RETURN AFTER
```

Note: `superseded_by` is stored in metadata or a separate field if needed for traceability.

---

## Definition of Done

1. get_valid excludes memories with valid_until in the past
2. get_valid_at returns memories valid at exact timestamp
3. invalidate sets valid_until to now (soft-delete)
4. Invalidated memories remain queryable via get_valid_at (historical)
5. All temporal filters work correctly with timezone handling

## Edge Cases

| Case | Expected Behavior |
|------|-------------------|
| Memory already invalidated | invalidate succeeds (idempotent) |
| Timestamp in future | get_valid_at returns currently valid (future timestamp treated as now) |
| No valid memories | Return empty array, not error |
| Invalid timestamp format | Return tool error with format hint |

## Risks

| Risk | Mitigation |
|------|------------|
| Timezone confusion | Always use UTC, document in tool description |
| Precision loss | Store as SurrealDB datetime (nanosecond precision) |

## Reviewer Guidance

- Verify temporal filters in storage queries
- Check ISO 8601 parsing handles various formats
- Confirm invalidated memories excluded from search results
- Test with edge cases (future dates, same-second operations)
