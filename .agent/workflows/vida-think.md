---
description: Mandatory Thinking Processor - routes user requests through appropriate thinking algorithm (STC/PR-CoT/MAR/5-SOL/META)
---

Read and execute the thinking processor instructions from _vida/commands/vida-think.md

Context: Read _vida/algorithms/algorithm-selector.md for algorithm selection logic.

## Quick Reference

- `/vida-think {request}` - Process request through appropriate algorithm

## Algorithm Selection

| Score | Algorithm | Purpose |
|-------|-----------|---------|
| ≤15 | STC | Step-by-step critique |
| 16-25 | PR-CoT | 4 perspectives validation |
| 26-35 | MAR | 3 rounds × 4 agents |
| 36-45 | 5-SOL | 2 rounds × 5 options |
| >45 | META | Ensemble (all 3 parallel) |

## Overrides

- Security/Auth decision → META
- Database schema → META  
- DEC-XXX creation → MAR
- Multiple errors → 5-SOL
- "Choose between X, Y, Z" → 5-SOL

## Internal Mode

ALL algorithms execute SILENTLY — user sees conclusion only.
