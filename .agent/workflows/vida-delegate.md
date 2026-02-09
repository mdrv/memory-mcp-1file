# VIDA: Task Delegation

Read and execute the VIDA delegate command instructions:

@vida/commands/vida-delegate.md

Use @AGENTS.md for Memory Protocol context.

---

## Syntax

```bash
/project:vida-delegate <task-id> <agent-name>
```

**Arguments:**
- `<task-id>` — Task ID (e.g., `T-01`, `T-02`)
- `<agent-name>` — Agent name from DEV-AGENTS-MATRIX.md

## Purpose

Delegate a task to a specialized sub-agent with proper 7-section payload.

## Agent Selection

| Task Type | Agent | Speed |
|-----------|-------|-------|
| Architecture, multi-file | `dev-senior-minimax` | ~30s |
| Deep research, docs | `dev-senior-glm` | ~60s |
| Typing, refactoring | `dev-middle-gemini-pro` | ~40s |
| Complex debugging | `dev-middle-gemini-reasoning` | ~60s |
| Quick search, simple fix | `dev-junior-gemini-flash` | ~20s |
