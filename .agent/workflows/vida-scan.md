---
description: VIDA scan - proactive error detection and routing
---

Read and execute the scan instructions from _vida/commands/vida-scan.md

## Quick Reference

- `/vida-scan` - Full proactive scan with report
- `/vida-scan --quick` - Analyze only
- `/vida-scan fix N` - Route issue #N to fix
- `/vida-scan autofix` - Auto-fix LOW issues

## Flow

```
DETECT → CLASSIFY → REPORT → ROUTE
```

| Severity | Routing |
|----------|---------|
| CRITICAL | /vida-bug-fix --meta |
| HIGH | /vida-bug-fix |
| MEDIUM | /vida-think |
| LOW | Auto-fix or skip |
