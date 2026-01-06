# 🧠 Memory Protocol (Memory MCP)

<critical>
This protocol is MANDATORY. Violation = loss of context between sessions.
Goal: any agent can continue another agent's work without losing context.
</critical>

---

## ⚡ Quick Reference

<quick_reference>

| Situation | Action | Section |
|-----------|--------|---------|
| 🚀 Session start | `search_text` → show TASK → AUTO_CONTINUE | [SESSION_START](#-session_start-session-startup-algorithm) |
| 🔍 Found TASK | Show to user → wait 30 sec | [AUTO_CONTINUE](#-auto_continue-confirmation-protocol-with-timer) |
| ✏️ Changed subtask | `update_memory` with new Current | [TASK_UPDATE](#-task_update-when-to-update-memory) |
| ✅ Completed WP | `invalidate` → update EPIC → new TASK | [TASK_COMPLETE](#-task_complete-completing-work-package) |

</quick_reference>

<critical_reminder>
🔴 MOST COMMON MISTAKE: Continuing work WITHOUT showing task state to user.
User message BEFORE showing TASK — is NOT a confirmation!
</critical_reminder>

---

## 📋 Mandatory Prefix System

<prefixes>

**EVERY memory entry MUST start with a prefix.**

| Prefix | memory_type | Purpose | Priority |
|--------|-------------|---------|----------|
| `PROJECT:` | semantic | Overall project state | 🟢 Low |
| `EPIC:` | procedural | WP group, feature progress | 🟡 Medium |
| `TASK:` | episodic | Active Work Package | 🔴 **Highest** |
| `DECISION:` | semantic | Architectural decision with reason | 🟢 Low |
| `CONTEXT:` | semantic | Technical context (stack, architecture) | 🟢 Low |
| `USER:` | semantic | User preferences | 🟢 Low |

</prefixes>

<constraints type="prefixes">
- FORBIDDEN to store entries WITHOUT prefix
- FORBIDDEN to use other prefixes
- FORBIDDEN to store TASK/EPIC without `Updated:` field
</constraints>

---

## 📐 Record Structures

### TASK (Work Package) — most important for recovery

```
TASK: {WP-id}-{short-description}
ID: {WP-id}
Status: in_progress | blocked | completed | paused
Lane: planned | in_progress | review | done
Feature: {feature-id}
Path: {path to WP file, e.g. kitty-specs/.../tasks/WP01-xxx.md}
Updated: {ISO 8601 timestamp}

Command: {recovery command, e.g. /spec-kitty.implement WP01}
Agent: {executing agent, e.g. spec-kitty}

Subtasks:
- [x] T001: {description} - {result}
- [ ] T002: {description}
- [ ] T003: {description}

AC (Acceptance Criteria):
- [x] {criterion 1}
- [ ] {criterion 2}

Current: {current subtask, e.g. T002}
CurrentFile: {file being worked on}
Blockers: {None | blocker description}

Context:
- {important information for continuation}
- {changes that were made}
```

<important>
**Command** and **Agent** — REQUIRED fields for automatic recovery after compaction.
</important>

### EPIC (Feature/WP group)

```
EPIC: {feature-id}
ID: {feature-id}
Status: active | paused | completed
Path: {path to kitty-specs/{feature-id}/}
Updated: {ISO 8601 timestamp}

Work Packages: {total} total
Progress: {completed}/{total} completed
Current WP: {WP-id} ({name})

Dependency Chain:
{WP01 → WP02 → ...}

Next: {what to do after current WP}
```

### PROJECT

```
PROJECT: {project name}
ID: {project-id}
Status: active | paused | completed
Path: {project root}
Branch: {git branch}
Updated: {ISO 8601 timestamp}

Tech Stack: {key technologies}
Current Epic: {feature-id} | None
Last Completed: {last completed epic}
Next Steps: {what to do next}
```

### DECISION

```
DECISION: {short decision description}
ID: {DEC-xxx}
Feature: {feature-id}
Updated: {ISO 8601 timestamp}

REASON: {why this decision was made}
ALTERNATIVES_REJECTED:
- {alternative 1}: {why rejected}
- {alternative 2}: {why rejected}
IMPLICATIONS: {consequences of the decision}
```

---

## 🚀 SESSION_START: Session Startup Algorithm

<session_start priority="BLOCKING">
EXECUTE IMMEDIATELY on first user message.
No other actions BEFORE completing this protocol.
</session_start>

<checklist id="session_start">
- [ ] `search_text("Status: in_progress", limit=5)`
- [ ] `search_text("TASK:", limit=5)`
- [ ] `search_text("EPIC:", limit=3)`
- [ ] `search_text("PROJECT:", limit=3)`
- [ ] Determined scenario (active/paused/new)
- [ ] Executed AUTO_CONTINUE if found TASK
</checklist>

### Algorithm

```
┌─────────────────────────────────────────────────────────────┐
│                    SESSION_START                            │
├─────────────────────────────────────────────────────────────┤
│ STEP 1: Search for active tasks (BM25 — exact match)        │
│                                                             │
│   search_text(query="Status: in_progress", limit=5)         │
│   search_text(query="TASK:", limit=5)                       │
│   search_text(query="EPIC:", limit=3)                       │
│   search_text(query="PROJECT:", limit=3)                    │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│ STEP 2: Decision tree                                       │
│                                                             │
│   IF found TASK with Status=in_progress:                    │
│      → Show task state to user                              │
│      → Execute AUTO_CONTINUE protocol (see below)           │
│      → Wait for confirmation OR 30 sec timer                │
│                                                             │
│   ELSE IF found TASK with Status=paused/blocked:            │
│      → Show context and Blockers                            │
│      → Ask: "Continue {TASK}?"                              │
│                                                             │
│   ELSE IF found EPIC with Status=active:                    │
│      → Show Progress and Current WP                         │
│      → Ask: "Start {next WP}?"                              │
│                                                             │
│   ELSE IF found PROJECT:                                    │
│      → Show project state                                   │
│      → Ask: "What are we working on?"                       │
│                                                             │
│   ELSE:                                                     │
│      → COLD START — ask user for context                    │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│ STEP 3: After recovery                                      │
│                                                             │
│   - DO NOT update memory (only on state change)             │
│   - Load file from Path for full context                    │
│   - Check git status to understand changes                  │
└─────────────────────────────────────────────────────────────┘
```

<error_handling id="session_start">

| Error | Fallback |
|-------|----------|
| `search_text` → 0 results | Execute `get_valid(limit=10)`, search by content |
| Memory MCP unavailable | Ask user for context directly |
| TASK.Path file doesn't exist | Show TASK from memory, ask for current path |

</error_handling>

<constraints type="session_start">
- FORBIDDEN to start work WITHOUT searching memory
- FORBIDDEN to continue work WITHOUT executing AUTO_CONTINUE protocol
- FORBIDDEN to ignore found active records
</constraints>

---

## ⏳ AUTO_CONTINUE: Confirmation Protocol with Timer

<auto_continue priority="BLOCKING">
MANDATORY when finding an active task.
Show state → Wait for confirmation OR 30 sec timer.
</auto_continue>

### ⚠️ CRITICAL: What is NOT a confirmation

<critical_rule>
User message BEFORE showing task state — is NOT a confirmation!
User cannot confirm what they haven't seen yet.
</critical_rule>

| Scenario | Example | Is this confirmation? |
|----------|---------|----------------------|
| User wrote something → you found TASK | "Continue" before search | ❌ **NO** — they haven't seen the task |
| You showed TASK → user responded | "Yes/go ahead" after showing | ✅ **YES** |
| You showed TASK → 30 sec timer | Silence | ✅ **YES** (auto-continue) |

<checklist id="auto_continue">
- [ ] Showed task state to user (table)
- [ ] Asked "Continue this task?"
- [ ] Started timer `sleep 30`
- [ ] Received confirmation OR timer triggered
- [ ] ONLY AFTER this continued work
</checklist>

### Algorithm

```
┌─────────────────────────────────────────────────────────────┐
│                    AUTO_CONTINUE                            │
├─────────────────────────────────────────────────────────────┤
│ 1. Show user the found task:                                │
│                                                             │
│    ╔══════════════════════════════════════════════════════╗ │
│    ║ 🔍 Found unfinished task in memory:                  ║ │
│    ║                                                      ║ │
│    ║ TASK: {WP-id} - {name}                               ║ │
│    ║ Status: {status}                                     ║ │
│    ║ Current: {current subtask}                           ║ │
│    ║ Progress: {N}/{total} subtasks                       ║ │
│    ║ Command: {continuation command}                      ║ │
│    ║                                                      ║ │
│    ║ Continue this task?                                  ║ │
│    ║ (auto-continue in 30 sec)                            ║ │
│    ╚══════════════════════════════════════════════════════╝ │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│ 2. SIMULTANEOUSLY start timer:                              │
│                                                             │
│    bash: sleep 30 && echo "AUTO_CONTINUE_TRIGGER"           │
│    timeout: 35000ms                                         │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│ 3. Handle result:                                           │
│                                                             │
│    IF user responded BEFORE timer:                          │
│       → "yes/continue/go ahead" → continue                  │
│       → "no/stop/other" → ask what to do                    │
│       → new task → switch to it                             │
│                                                             │
│    ELSE IF timer triggered (no response):                   │
│       → Automatically continue task                         │
│       → Notify: "⏳ Continuing automatically..."            │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│ 4. Launch recovery command:                                 │
│                                                             │
│    IF TASK has Command field (e.g. /spec-kitty.implement):  │
│       → Execute slashcommand (see below)                    │
│                                                             │
│    ELSE:                                                    │
│       → Continue work manually using Context                │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 🔧 What is Command (slashcommand)

<slashcommand>
**Command** — is NOT a bash command, but a reference to an .md file with agent instructions.

**Format**: `/{prefix}.{action} {arguments}`
- Example: `/spec-kitty.implement WP01`
</slashcommand>

**Execution algorithm:**

1. **Parse the command:**
   - `/spec-kitty.implement WP01` → command=`spec-kitty.implement`, args=`WP01`
   
2. **Find instruction file in IDE/CLI directory:**
   
   | IDE/CLI | Path |
   |---------|------|
   | OpenCode | `.opencode/command/{command}.md` |
   | Cursor | `.cursor/command/{command}.md` |
   | Claude Code | `.claude/command/{command}.md` |
   | Windsurf | `.windsurf/command/{command}.md` |
   
3. **Read the ENTIRE file and execute instructions:**
   - `$ARGUMENTS` → substitute args (e.g. `WP01`)
   - File contains FULL workflow with all steps
   - Execute step by step

<warning>
`.opencode/command/spec-kitty.implement.md` = **276 lines** of full workflow
`.kittify/.../implement.md` = **12 lines** just bash command

If you only read the short file — you're missing 90% of instructions!
</warning>

### Output Format (MANDATORY)

<output_format>
Start your response EXACTLY like this:

```
🔍 **Found unfinished task in memory:**

┌────────────────────────────────────────────┐
│ TASK: WP01-poc-validation                  │
│ Status: in_progress                        │
│ Current: T002 (rmcp PoC)                   │
│ Progress: 1/3 subtasks done                │
│ Command: /spec-kitty.implement WP01        │
│                                            │
│ Subtasks:                                  │
│   [x] T001: Candle PoC - DONE              │
│   [ ] T002: rmcp PoC ← current             │
│   [ ] T003: SurrealDB PoC                  │
└────────────────────────────────────────────┘

**Continue this task?**
_(auto-continue in 30 seconds if no response)_
```
</output_format>

<constraints type="auto_continue">
- FORBIDDEN to continue WITHOUT showing information to user
- FORBIDDEN to wait longer than 30 seconds
- FORBIDDEN to ignore user response if it arrived
</constraints>

---

## 🔄 TASK_UPDATE: When to Update Memory

<task_update>
Update TASK on EVERY significant state change.
DO NOT update on every tool call — that's too frequent.
</task_update>

| Trigger | Action |
|---------|--------|
| Completed subtask (T001 → T002) | `update_memory` with new Current |
| Encountered blocker | `update_memory` with Blockers, Status=blocked |
| Made a decision | + `store_memory` DECISION |
| User says "stop/pause" | `update_memory` Status=paused |
| Created/modified files | Add to Context |
| Fully completed WP | `invalidate` + new TASK for next WP |

<checklist id="task_update">
- [ ] Updating TASK when Current subtask changes
- [ ] Adding changed files to Context
- [ ] Creating DECISION for important decisions
- [ ] Updating Status on blockers
- [ ] **TRIPLE SYNC:** Sync status to Project Documents (e.g. TASK.md)
- [ ] **TRIPLE SYNC:** Update active Task Management Tool (CLI/IDE) status
</checklist>

<constraints type="task_update">
- FORBIDDEN to update on every tool call (too frequent)
- FORBIDDEN to NOT update on subtask change (too rare)
- FORBIDDEN to leave Status=in_progress when blocked
- FORBIDDEN to have conflicting status between Memory, Task Tools, and Documents
</constraints>

---

## ✅ TASK_COMPLETE: Completing Work Package

<task_complete>
EXECUTE BEFORE moving to next WP.
Step order is important!
</task_complete>

<checklist id="task_complete">
- [ ] `invalidate(id="{task_memory_id}", reason="WP completed")`
- [ ] `update_memory(id="{epic_id}")` with Progress: {N+1}/{total}
- [ ] `store_memory("DECISION: ...")` for important decisions
- [ ] `store_memory("TASK: ...")` for new WP
- [ ] **TRIPLE SYNC:** Update active Task Management Tool (CLI/IDE) status
- [ ] **TRIPLE SYNC:** Mark as Completed in all relevant Project Documents
</checklist>

### Algorithm

```
1. invalidate(
     id="{task_memory_id}",
     reason="WP completed successfully"
   )

2. update_memory(id="{epic_id}") with:
   - Progress: {N+1}/{total}
   - Current WP: {next WP}
   
3. If there were important decisions:
   store_memory(content="DECISION: ...", memory_type="semantic")

4. store_memory for new TASK:
   - Status: in_progress
   - Current: first subtask
   - Path: path to new WP file
```

<constraints type="task_complete">
- FORBIDDEN to move to new WP WITHOUT invalidating old TASK
- FORBIDDEN to forget updating EPIC Progress
- FORBIDDEN to use delete_memory — ONLY invalidate
</constraints>

---

## 🏁 EPIC_COMPLETE: Completing Feature

<epic_complete>
EXECUTE when closing all WPs of a feature.
</epic_complete>

<checklist id="epic_complete">
- [ ] `invalidate(id="{epic_id}", reason="feature completed")`
- [ ] `store_memory("PROJECT: ...")` with Last Completed
- [ ] `store_memory("DECISION: ...")` for each important decision
- [ ] **TRIPLE SYNC:** Update active Task Management Tool (CLI/IDE) status
- [ ] **TRIPLE SYNC:** Mark Epic as Done in active Task Management Tools (CLI/IDE)
</checklist>

### Algorithm

```
1. invalidate(id="{epic_id}", reason="feature completed")

2. store_memory(content="PROJECT: ...") with:
   - Last Completed: {feature-id}
   - Current Epic: None | {next feature}
   
3. For EACH important decision of the feature:
   store_memory(content="DECISION: ...", memory_type="semantic")
```

<constraints type="epic_complete">
- FORBIDDEN to complete epic WITHOUT updating PROJECT
- FORBIDDEN to lose DECISION records
</constraints>

---

## 🔍 Search Method Selection

| Situation | Method | Why |
|-----------|--------|-----|
| **Session start** | `search_text` | BM25 accurately finds prefixes |
| Search by ID | `get_memory` | Direct retrieval |
| Search decisions | `search_text("DECISION:")` | Exact prefix match |
| Semantic search | `search` or `recall` | When exact words unknown |
| Change history | `get_valid_at` | State at point in time |
| All current | `get_valid` | Filters by valid_until |

<important>
`recall` uses hybrid search (vector + BM25 + PPR), 
but for prefixes `search_text` is more reliable.
</important>

---

## 📊 Knowledge Graph (optional)

<knowledge_graph>
Use for complex projects with dependencies.
</knowledge_graph>

```
# Creating hierarchy
create_entity(name="Feature:001-memory-mcp", entity_type="feature")
create_entity(name="WP:WP01", entity_type="work_package")
create_entity(name="Task:T001", entity_type="task")

# Relations
create_relation(from="WP:WP01", to="Feature:001", relation_type="belongs_to")
create_relation(from="Task:T001", to="WP:WP01", relation_type="part_of")
create_relation(from="WP:WP02", to="WP:WP01", relation_type="depends_on")

# Navigation
get_related(entity_id="WP:WP01", depth=2, direction="both")
```

---

## ⚠️ Critical Rules

### MUST (REQUIRED)

<must_do>
- ✅ Call `search_text` at the start of EVERY session
- ✅ Show task state to user BEFORE continuing (AUTO_CONTINUE)
- ✅ Every entry starts with prefix (PROJECT:/EPIC:/TASK:/DECISION:)
- ✅ Every TASK/EPIC has `Updated:` field with ISO timestamp
- ✅ TASK has fields: Status, Current, Path, Command, Agent
- ✅ Use `invalidate` instead of `delete_memory`
- ✅ Update TASK on subtask change
- ✅ Update EPIC on WP completion
- ✅ Store DECISION with REASON
</must_do>

### MUST NOT (FORBIDDEN)

<must_not>
- ❌ Store entries without prefix
- ❌ Start work without searching memory
- ❌ Continue work WITHOUT showing task state to user
- ❌ Consider user message BEFORE showing task as confirmation
- ❌ Move to new WP without invalidating old TASK
- ❌ Use `delete_memory` (only invalidate)
- ❌ Ignore found active TASK records
- ❌ Store duplicates — use `update_memory`
</must_not>

---

## 📋 Rules Summary

| Rule | Description |
|------|-------------|
| **External repositories** | Only in `_tmp/` directory |
| **Package installation** | Use `cargo add`, don't edit `Cargo.toml` manually |
| **Communication language** | Ukrainian only |
| **Memory: start** | REQUIRED `search_text` + show to user |
| **Memory: completion** | REQUIRED `invalidate` + `store_memory` |
| **Memory: deletion** | FORBIDDEN `delete_memory`, only `invalidate` |

---

*Last updated: 2026-01-06*
