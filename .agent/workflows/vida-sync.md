# VIDA: State Synchronization

Read and execute the VIDA sync command instructions:

@vida/commands/vida-sync.md

Use @AGENTS.md for Memory Protocol context.

---

## Syntax

```bash
/project:vida-sync [OPTIONS]

OPTIONS:
  (none)        Smart sync - regenerate only if stale
  --force       Force full regeneration regardless of cache
  --check       Validate only, report issues, no file changes
  --diff        Show what would change without applying
  --view=NAME   Regenerate specific view only (lead|status|memory)
  --fields=X,Y  Regenerate only specific fields (partial sync)
```

## Purpose

Regenerate project state from source artifacts using derivation rules defined in `manifest.yaml`. Produces cached projections for fast reads by other VIDA commands.

## When to Use

- Manual: When state seems out of sync
- Auto: Triggered by `/vida-lead` when state is stale
- CI/CD: With `--check` for validation
