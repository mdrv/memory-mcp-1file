# /vida-think — Mandatory Thinking Processor

Read and execute the thinking processor instructions:

@_vida/commands/vida-think.md

Use @_vida/algorithms/algorithm-selector.md for algorithm selection.

---

## Quick Reference

**Usage:** `/project:vida-think {user_request}`

**Algorithm Selection:**
- Score ≤15 → STC (silent step-critique)
- Score 16-25 → PR-CoT (4 perspectives)
- Score 26-35 → MAR (3 rounds × 4 agents)
- Score 36-45 → 5-SOL (2 rounds × 5 options)
- Score >45 → META (ensemble)

**Overrides:**
- Security/Auth → META
- Database → META
- DEC-XXX → MAR
- Multiple errors → 5-SOL
- "Choose between X, Y, Z" → 5-SOL

**Internal Mode:** All algorithms execute silently. User sees conclusion only.
