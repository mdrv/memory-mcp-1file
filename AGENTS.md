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
| 🆕 Ad-hoc Task | Create TASK (ad_hoc) → SYNC | [AD_HOC_TASK](#-ad_hoc_task-user--external-tasks) |
| 🧪 Research | Create RESEARCH → Cycle → SYNC | [RESEARCH_PROTOCOL](#-research_protocol-investigation--architecture) |
| ✏️ Changed subtask | `update_memory` → SYNC | [SYNC_PROTOCOL](#-sync_protocol-status-synchronization) |
| ✅ Completed WP | `invalidate` → update EPIC → SYNC | [TASK_COMPLETE](#-task_complete-completing-work-package) |

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
| `RESEARCH:` | semantic | Investigation & Findings | 🔵 High |
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
Type: standard | ad_hoc  <-- NEW
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

### RESEARCH

```
RESEARCH: {Research Topic}
ID: {RES-date-topic}
Status: active | completed | paused
Goal: {What do we want to find out?}
Path: {path to doc/research/...md}
Updated: {ISO 8601 timestamp}

Open Questions:
- [ ] {Question 1}
- [ ] {Question 2}

Conclusions (Findings):
- {Key finding 1}
- {Key finding 2}

Approved Decisions:
- {Decision 1} (create DECISION record if important)
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

## 🔄 SYNC_PROTOCOL: Status Synchronization

<sync_protocol priority="CRITICAL">
MANDATORY to execute on EVERY status change or task completion.
Ensures consistency between Memory, Task Tools, and Reality.
</sync_protocol>

### ⚠️ TRIPLE OBIGATION Rule (for Standard Tasks)
For standard development flow, you must synchronize **ALL THREE** systems immediately.

| System | Action | Criticality |
|--------|--------|-------------|
| **1. Memory (MCP)** | `update_memory` / `store_memory` / `invalidate` | 🔴 **MANDATORY** |
| **2. Task Tool (IDE/CLI)** | Update status (subtask/task) in tool | 🔴 **MANDATORY** |
| **3. Documents** | Update markdown files (Task/Epic/Project) | 🔴 **MANDATORY** (Standard Flow)<br/>🟡 If applicable (Ad-hoc) |

<checklist id="sync_protocol">
- [ ] **Memory**: Updated Status, Current, or Blockers
- [ ] **Task Tool**: Checked items or updated status in IDE/CLI
- [ ] **Documents**: Updated relevant .md files (REQUIRED for Standard Tasks)
</checklist>

<constraints type="sync_protocol">
- FORBIDDEN to update only one system
- FORBIDDEN to delay synchronization (MUST be immediate)
- FORBIDDEN to proceed without updating Documents (for Standard Tasks)
</constraints>

---

## 🔄 TASK_UPDATE: When to Update Memory

<task_update>
Update TASK on EVERY significant state change.
DO NOT update on every tool call — that's too frequent.
</task_update>

| Trigger | Action |
|---------|--------|
| Completed subtask (T001 → T002) | `update_memory` → **EXECUTE SYNC_PROTOCOL** |
| Encountered blocker | `update_memory` (blocked) → **EXECUTE SYNC_PROTOCOL** |
| Made a decision | + `store_memory` DECISION |
| User says "stop/pause" | `update_memory` (paused) → **EXECUTE SYNC_PROTOCOL** |
| Created/modified files | Add to Context |
| Fully completed WP | `invalidate` + new TASK → **EXECUTE SYNC_PROTOCOL** |

<checklist id="task_update">
- [ ] Updating TASK when Current subtask changes
- [ ] Adding changed files to Context
- [ ] Creating DECISION for important decisions
- [ ] Updating Status on blockers
- [ ] **EXECUTE SYNC_PROTOCOL** (Memory + Task Tool)
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
- [ ] **EXECUTE SYNC_PROTOCOL** (Triple Sync)
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
   - Type: standard
   - Status: in_progress
   - Current: first subtask
   - Path: path to new WP file
   
5. EXECUTE SYNC_PROTOCOL (Update Task Tool + Docs)
```

<constraints type="task_complete">
- FORBIDDEN to move to new WP WITHOUT invalidating old TASK
- FORBIDDEN to forget updating EPIC Progress
- FORBIDDEN to use delete_memory — ONLY invalidate
- FORBIDDEN to skip SYNC_PROTOCOL
</constraints>

---

## ⚡ AD_HOC_TASK: User & External Tasks

<ad_hoc_task>
Protocol for tasks NOT defined in the standard Roadmap/Epic structure.
Includes: User requests, Bug fixes outside sprints, One-off maintenance.
</ad_hoc_task>

### Algorithm

```
┌─────────────────────────────────────────────────────────────┐
│                    AD_HOC_TASK                              │
├─────────────────────────────────────────────────────────────┤
│ 1. Creation:                                                │
│    store_memory("TASK: ...")                                │
│    - ID: {generated_id} (e.g. USER-20240101)                │
│    - Type: ad_hoc                                           │
│    - Status: in_progress                                    │
│    - Description: {user request}                            │
│                                                             │
│ 2. Sync Start:                                              │
│    → Add to Task Tool (IDE/CLI) under "Ad-hoc" or similar   │
│                                                             │
│ 3. Execution:                                               │
│    → Execute subtasks                                       │
│    → SYNC_PROTOCOL after EACH step/subtask                  │
│                                                             │
│ 4. Completion:                                              │
│    → invalidate(id="{task_id}", reason="Completed")         │
│    → Mark Done in Task Tool                                 │
│    → Notify User                                            │
└─────────────────────────────────────────────────────────────┘
```

<constraints type="ad_hoc_task">
- FORBIDDEN to execute "just a quick task" without recording in Memory
- FORBIDDEN to skip Task Tool entry for ad-hoc tasks
- **MANDATORY** to follow SYNC_PROTOCOL (Memory + Tool)
</constraints>

---

## 🧪 RESEARCH_PROTOCOL: Investigation & Architecture

<research_protocol>
Protocol for investigations, selecting libraries, and designing architecture.
Balances Memory limits by storing details in files and summaries in Memory.
</research_protocol>

### ⚖️ Memory vs File Strategy

| Type | Where to store | Content |
|------|----------------|---------|
| **Meta-data** | **Memory (MCP)** | Status, Goal, *Key* Open Questions, *Key* Findings. <br/> **Limit:** ~1000-2000 chars per record. |
| **Details** | **File (.md)** | Full benchmarks, long descriptions, code examples, logs. |

### Algorithm

```
┌─────────────────────────────────────────────────────────────┐
│                  RESEARCH_PROTOCOL                          │
├─────────────────────────────────────────────────────────────┤
│ 1. Initialization:                                          │
│    Create file: doc/research/{topic}.md                     │
│    store_memory("RESEARCH: ...")                            │
│    - Path: {path to file}                                   │
│    - Goal: {objective}                                      │
│    - Open Questions: {list of questions}                    │
│    → EXECUTE SYNC_PROTOCOL                                  │
│                                                             │
│ 2. Research Cycle (Iterative):                              │
│    → Investigate / Experiment                               │
│    → Write details to File (.md)                            │
│    → Update Memory ("RESEARCH: ...")                        │
│         - Remove answered questions from Open Questions     │
│         - Add answer to Conclusions                         │
│    → EXECUTE SYNC_PROTOCOL                                  │
│                                                             │
│ 3. Completion:                                              │
│    → Formulate final Decisions                              │
│    → store_memory("DECISION: ...") (for approved choices)   │
│    → invalidate(id="{research_id}", reason="Completed")     │
│    → Update PROJECT/EPIC with results                       │
│    → EXECUTE SYNC_PROTOCOL                                  │
└─────────────────────────────────────────────────────────────┘
```

<constraints type="research_protocol">
- FORBIDDEN to dump huge texts into Memory (use linked File)
- FORBIDDEN to conduct research without defining "Goal" and "Open Questions"
- **MANDATORY** to fix Approved Decisions as separate DECISION records upon completion
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
- [ ] **GIT COMMIT (MANDATORY):** Commit all changes for the completed feature
</checklist>

### Algorithm

```
1. invalidate(id="{epic_id}", reason="feature completed")

2. store_memory(content="PROJECT: ...") with:
   - Last Completed: {feature-id}
   - Current Epic: None | {next feature}
   
3. For EACH important decision of the feature:
   store_memory(content="DECISION: ...", memory_type="semantic")

4. GIT COMMIT (MANDATORY):
   git add -A
   git commit -m "feat({feature-id}): complete {feature description}"
   
   Commit message format:
   - feat({id}): for new features
   - fix({id}): for bug fix features
   - refactor({id}): for refactoring features
   
   Include in commit body (optional):
   - List of completed WPs
   - Key decisions made
```

<constraints type="epic_complete">
- FORBIDDEN to complete epic WITHOUT updating PROJECT
- FORBIDDEN to lose DECISION records
- FORBIDDEN to complete epic WITHOUT git commit of all changes
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


<!-- CLAVIX:START -->
# Clavix Instructions for Generic Agents

This guide is for agents that can only read documentation (no slash-command support). If your platform supports custom slash commands, use those instead.

---

## ⛔ CLAVIX MODE ENFORCEMENT

**CRITICAL: Know which mode you're in and STOP at the right point.**

**OPTIMIZATION workflows** (NO CODE ALLOWED):
- Improve mode - Prompt optimization only (auto-selects depth)
- Your role: Analyze, optimize, show improved prompt, **STOP**
- ❌ DO NOT implement the prompt's requirements
- ✅ After showing optimized prompt, tell user: "Run `/clavix:implement --latest` to implement"

**PLANNING workflows** (NO CODE ALLOWED):
- Conversational mode, requirement extraction, PRD generation
- Your role: Ask questions, create PRDs/prompts, extract requirements
- ❌ DO NOT implement features during these workflows

**IMPLEMENTATION workflows** (CODE ALLOWED):
- Only after user runs execute/implement commands
- Your role: Write code, execute tasks, implement features
- ✅ DO implement code during these workflows

**If unsure, ASK:** "Should I implement this now, or continue with planning?"

See `.clavix/instructions/core/clavix-mode.md` for complete mode documentation.

---

## 📁 Detailed Workflow Instructions

For complete step-by-step workflows, see `.clavix/instructions/`:

| Workflow | Instruction File | Purpose |
|----------|-----------------|---------|
| **Conversational Mode** | `workflows/start.md` | Natural requirements gathering through discussion |
| **Extract Requirements** | `workflows/summarize.md` | Analyze conversation → mini-PRD + optimized prompts |
| **Prompt Optimization** | `workflows/improve.md` | Intent detection + quality assessment + auto-depth selection |
| **PRD Generation** | `workflows/prd.md` | Socratic questions → full PRD + quick PRD |
| **Mode Boundaries** | `core/clavix-mode.md` | Planning vs implementation distinction |
| **File Operations** | `core/file-operations.md` | File creation patterns |
| **Verification** | `core/verification.md` | Post-implementation verification |

**Troubleshooting:**
- `troubleshooting/jumped-to-implementation.md` - If you started coding during planning
- `troubleshooting/skipped-file-creation.md` - If files weren't created
- `troubleshooting/mode-confusion.md` - When unclear about planning vs implementation

---

## 🔍 Workflow Detection Keywords

| Keywords in User Request | Recommended Workflow | File Reference |
|---------------------------|---------------------|----------------|
| "improve this prompt", "make it better", "optimize" | Improve mode → Auto-depth optimization | `workflows/improve.md` |
| "analyze thoroughly", "edge cases", "alternatives" | Improve mode (--comprehensive) | `workflows/improve.md` |
| "create a PRD", "product requirements" | PRD mode → Socratic questioning | `workflows/prd.md` |
| "let's discuss", "not sure what I want" | Conversational mode → Start gathering | `workflows/start.md` |
| "summarize our conversation" | Extract mode → Analyze thread | `workflows/summarize.md` |
| "refine", "update PRD", "change requirements", "modify prompt" | Refine mode → Update existing content | `workflows/refine.md` |
| "verify", "check my implementation" | Verify mode → Implementation verification | `core/verification.md` |

**When detected:** Reference the corresponding `.clavix/instructions/workflows/{workflow}.md` file.

---

## 📋 Clavix Commands (v5)

### Setup Commands (CLI)
| Command | Purpose |
|---------|---------|
| `clavix init` | Initialize Clavix in a project |
| `clavix update` | Update templates after package update |
| `clavix diagnose` | Check installation health |
| `clavix version` | Show version |

### Workflow Commands (Slash Commands)
All workflows are executed via slash commands that AI agents read and follow:

> **Command Format:** Commands shown with colon (`:`) format. Some tools use hyphen (`-`): Claude Code uses `/clavix:improve`, Cursor uses `/clavix-improve`. Your tool autocompletes the correct format.

| Slash Command | Purpose |
|---------------|---------|
| `/clavix:improve` | Optimize prompts (auto-selects depth) |
| `/clavix:prd` | Generate PRD through guided questions |
| `/clavix:plan` | Create task breakdown from PRD |
| `/clavix:implement` | Execute tasks or prompts (auto-detects source) |
| `/clavix:start` | Begin conversational session |
| `/clavix:summarize` | Extract requirements from conversation |
| `/clavix:refine` | Refine existing PRD or saved prompt |

### Agentic Utilities (Project Management)
These utilities provide structured workflows for project completion:

| Utility | Purpose |
|---------|---------|
| `/clavix:verify` | Check implementation against PRD requirements, run validation |
| `/clavix:archive` | Archive completed work to `.clavix/archive/` for reference |

**Quick start:**
```bash
npm install -g clavix
clavix init
```

**How it works:** Slash commands are markdown templates. When invoked, the agent reads the template and follows its instructions using native tools (Read, Write, Edit, Bash).

---

## 🔄 Standard Workflow

**Clavix follows this progression:**

```
PRD Creation → Task Planning → Implementation → Archive
```

**Detailed steps:**

1. **Planning Phase**
   - Run: `/clavix:prd` or `/clavix:start` → `/clavix:summarize`
   - Output: `.clavix/outputs/{project}/full-prd.md` + `quick-prd.md`
   - Mode: PLANNING

2. **Task Preparation**
   - Run: `/clavix:plan` transforms PRD into curated task list
   - Output: `.clavix/outputs/{project}/tasks.md`
   - Mode: PLANNING (Pre-Implementation)

3. **Implementation Phase**
   - Run: `/clavix:implement`
   - Agent executes tasks systematically
   - Mode: IMPLEMENTATION
   - Agent edits tasks.md directly to mark progress (`- [ ]` → `- [x]`)

4. **Completion**
   - Run: `/clavix:archive`
   - Archives completed work
   - Mode: Management

**Key principle:** Planning workflows create documents. Implementation workflows write code.

---

## 💡 Best Practices for Generic Agents

1. **Always reference instruction files** - Don't recreate workflow steps inline, point to `.clavix/instructions/workflows/`

2. **Respect mode boundaries** - Planning mode = no code, Implementation mode = write code

3. **Use checkpoints** - Follow the CHECKPOINT pattern from instruction files to track progress

4. **Create files explicitly** - Use Write tool for every file, verify with ls, never skip file creation

5. **Ask when unclear** - If mode is ambiguous, ask: "Should I implement or continue planning?"

6. **Track complexity** - Use conversational mode for complex requirements (15+ exchanges, 5+ features, 3+ topics)

7. **Label improvements** - When optimizing prompts, mark changes with [ADDED], [CLARIFIED], [STRUCTURED], [EXPANDED], [SCOPED]

---

## ⚠️ Common Mistakes

### ❌ Jumping to implementation during planning
**Wrong:** User discusses feature → agent generates code immediately

**Right:** User discusses feature → agent asks questions → creates PRD/prompt → asks if ready to implement

### ❌ Skipping file creation
**Wrong:** Display content in chat, don't write files

**Right:** Create directory → Write files → Verify existence → Display paths

### ❌ Recreating workflow instructions inline
**Wrong:** Copy entire fast mode workflow into response

**Right:** Reference `.clavix/instructions/workflows/improve.md` and follow its steps

### ❌ Not using instruction files
**Wrong:** Make up workflow steps or guess at process

**Right:** Read corresponding `.clavix/instructions/workflows/*.md` file and follow exactly

---

**Artifacts stored under `.clavix/`:**
- `.clavix/outputs/<project>/` - PRDs, tasks, prompts
- `.clavix/templates/` - Custom overrides

---

**For complete workflows:** Always reference `.clavix/instructions/workflows/{workflow}.md`

**For troubleshooting:** Check `.clavix/instructions/troubleshooting/`

**For mode clarification:** See `.clavix/instructions/core/clavix-mode.md`

<!-- CLAVIX:END -->
