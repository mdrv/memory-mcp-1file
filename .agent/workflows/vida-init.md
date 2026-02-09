# VIDA: Initialize Framework

Read and execute the VIDA init command instructions:

@vida/commands/vida-init.md

Use @AGENTS.md for Memory Protocol context.

---

## Syntax

```bash
/project:vida-init [OPTIONS]

OPTIONS:
  (none)           Interactive setup
  --minimal        Create only essential files
  --full           Create full structure with examples
  --from=PATH      Copy config from another VIDA project
```

## Purpose

Initialize VIDA State Engine in a new project. Creates required directory structure, copies templates, and runs initial sync.

## What It Creates

- `_vida/` — VIDA Framework root with config, manifest, commands
- `doc/planning/` — Planning artifacts (epics, milestones)
- `doc/research/` — Research documents
- `doc/specs/` — Technical specifications
