# VIDA: PROJECT Comprehensive Analysis

Read and execute the VIDA project analysis command instructions:

@vida/commands/vida-project-analyze.md

Use @AGENTS.md for Memory Protocol context.

---

## Syntax

```bash
/project:vida-project-analyze [OPTIONS]

OPTIONS:
  (none)         Full analysis with comprehensive report
  --quick        Fast analysis, compact output
  --json         Machine-readable JSON output
  --no-update    Analysis only, don't modify PROJECT: record
  --health-only  Show health score breakdown only
  --trends       Focus on velocity and trends analysis
```

## Purpose

Deep analysis of PROJECT: state with health assessment, velocity tracking, and actionable recommendations. Analyzes project health from multiple dimensions and updates PROJECT: record in Memory + files.

## What This Does

1. **Collects State** — From State Engine (doc/state/state.yaml) OR direct artifact scanning
2. **Analyzes Health** — Computes 0-100 health score based on:
   - Discovery Loop status (20 pts)
   - Epic formation rate (30 pts)
   - Implementation progress (30 pts)
   - Blocker count (20 pts)
3. **Tracks Velocity** — Git commits, epic formation, task completion over 30 days
4. **Detects Trends** — Accelerating, stable, or decelerating
5. **Identifies Issues** — Stale items, blockers, bottlenecks
6. **Updates PROJECT** — Memory MCP + _vida/README.md frontmatter
7. **Recommends Next** — Prioritized actions based on current state

## When to Use

- **Manual**: Periodic health check (weekly/biweekly)
- **Auto**: After major milestones complete
- **Scheduled**: After 10 tasks completed, 7 days no activity
- **Debug**: When project feels stalled or unclear state

## Output Modes

### Default (Comprehensive Report)
```bash
/project:vida-project-analyze
```
Full report with all sections: overview, tech stack, progress, health breakdown, velocity, blockers, recommendations.

### Quick Mode (One-Line Summary)
```bash
/project:vida-project-analyze --quick
```
Output: `[PROJECT] mobile-odoo | Health: 76/100 Good ↗ | Epic: 1/17 (6%) | → /vida-implement T-03`

### JSON Mode (Programmatic)
```bash
/project:vida-project-analyze --json
```
Structured JSON output for CI/CD integration, metrics tracking, dashboards.

### Health Only (Score Breakdown)
```bash
/project:vida-project-analyze --health-only
```
Shows detailed scoring: Discovery (20), Epics (18), Implementation (18), Blockers (20) = Total 76/100.

### Trends (Velocity Analysis)
```bash
/project:vida-project-analyze --trends
```
Focus on 30-day velocity: commits, epics formed, milestones started, tasks completed. Trend detection.

## Health Score Interpretation

| Score   | Rating    | Action                    |
|---------|-----------|---------------------------|
| 90-100  | Excellent | Maintain momentum         |
| 70-89   | Good      | Address stale items       |
| 50-69   | Fair      | Review blockers, re-plan  |
| 30-49   | Poor      | Escalate to user          |
| 0-29    | Critical  | Major intervention needed |

## Portability

This command is **framework-agnostic**. Works with:
- ✅ State Engine (doc/state/state.yaml) — preferred
- ✅ Direct artifact scanning — fallback
- ✅ Memory MCP — optional (enhances persistence)
- ✅ Any VIDA-like structure with epics/milestones/tasks

No hardcoded paths — uses manifest.yaml configuration.

## Related Commands

- `/project:vida-status` — Current state snapshot (read-only, no analysis)
- `/project:vida-sync` — Regenerate State Engine
- `/project:vida-lead` — Autonomous orchestrator (uses PROJECT: analysis)

---

**After analysis, follow recommended Next Actions to continue work.**
