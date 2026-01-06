# Memory MCP Server Constitution

## Core Principles

### I. Test-Driven Reliability
**Test-first approach is mandatory.**
- Pure logic must have unit tests.
- Storage and Embedding layers must have integration tests.
- All 84+ tests must pass before final delivery.
- No "commenting out" failing tests to make the build pass.

### II. Single Responsibility & Modularity
**Code must be modular and strictly organized.**
- Follow the defined folder structure: `types/`, `storage/`, `embedding/`, `graph/`, `server/`.
- No monolithic files; split concerns into focused modules.
- Dependencies should be injected via Traits (e.g., `StorageBackend`) to allow mocking.

### III. Robust Error Handling
**Errors must be typed and handled gracefully.**
- Use `thiserror` for library/domain types to define explicit error variants.
- Use `anyhow` for application-level error context and bubbling.
- No `unwrap()` or `expect()` in production code paths; handle all `Result`s.
- Tool errors must return structured `CallToolResult.error`, not crash the server.

### IV. Security & Privacy First
**Data privacy is paramount.**
- **Token Protection**: Embeddings must `#[serde(skip_serializing)]` and NEVER be returned in API responses.
- **Log Hygiene**: Logs must go to `stderr` (stdout is reserved for MCP protocol). No sensitive data in logs.
- **Isolation**: Project data is strictly isolated; external network calls are limited to model downloads only.

## Technical Constraints

- **Language**: Rust 2021 edition.
- **Dependencies**: Use `cargo add`; do not edit `Cargo.toml` manually.
- **Binary Size**: Keep under 30MB (excluding models).
- **Startup Time**: Server must respond to `tools/list` within 1 second (background model loading).

## Governance

This constitution supersedes implicit defaults. Changes to these principles require a formal update to this file via the `/spec-kitty.constitution` command.

**Version**: 1.0.0 | **Ratified**: 2026-01-06
