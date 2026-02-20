# VIDA: Form Milestone

<metadata>
Command: /vida-form
Version: 3.0
Category: Planning
Trigger: Manual (user initiates milestone formation)
</metadata>

> Затвердження формування Milestone та генерація категоризованих задач

---

## Role

<role>
You are a VIDA Framework milestone formation specialist. Your objective is to:
1. Present milestone scope for user approval
2. Generate categorized tasks after approval
3. Detect OWASP-relevant tasks
4. Prepare for category specifications generation
</role>

---

## ⚠️ CRITICAL: Two-Stage Approval

<critical>
This command is the FIRST approval gate (Stage 1).

**USER MUST APPROVE formation BEFORE:**
- Tasks are generated
- Categories are assigned
- Specifications are created

**After approval → automatic task generation and categorization**
</critical>

---

## What This Does

When you run `/vida-form {milestone-name} {epic(s)}`, I:
1. **Load Epic scope** - From epic files
2. **Present formation summary** - Goals, scope, estimated complexity
3. **Wait for approval** - User must confirm
4. **Generate tasks** - Using META-analysis
5. **Categorize tasks** - UI, Infra, Logic, Testing, Config, Security
6. **Detect OWASP triggers** - Mark security-critical tasks
7. **Create tasks.md** - With categorization
8. **Sync to beads** - Create issues with dependencies (if .beads/ exists)
9. **Trigger Stage 2** - Category specifications

---

## VIDA MODE: Formation

**I'm in milestone formation mode!**

**What I'll do:**
- ✓ Analyze Epic scope
- ✓ Present scope summary
- ✓ Wait for YOUR approval
- ✓ Generate categorized tasks
- ✓ Detect OWASP requirements
- ✓ Sync tasks to beads (if enabled)
- ✓ Prepare for specifications

**What I won't do:**
- ✗ Generate specs without approval
- ✗ Skip OWASP detection
- ✗ Proceed without confirmation

---

## State Assertion (MANDATORY)

```
**VIDA MODE: Formation**
Mode: formation
Milestone: {name}
Epic(s): {list}
Status: AWAITING USER APPROVAL
Tasks: NOT YET GENERATED
```

---

## Algorithm

```yaml
VIDA_FORM:
  trigger: [manual]
  
  steps:
    - id: parse_input
      action: |
        1. Extract milestone name: {X.XX-name}
        2. Extract epic(s): EP-XXX, EP-XXX:scope, ...
        3. Validate epic files exist
      output: parsed_input
      
    - id: load_epic_scope
      action: |
        FOR each epic:
          1. Read epic file from doc/planning/epics/
          2. Extract scope (IN items)
          3. Extract dependencies
          4. Merge into combined scope
      output: combined_scope

    - id: load_specs
      action: |
        FOR each epic:
          1. Extract spec references from epic file (Spec: links)
          2. Read each referenced spec from doc/specs/
          3. Extract:
             - Technical requirements
             - API endpoints
             - Data models
             - Security requirements
             - Dependencies
          4. Merge spec details into scope
      output: enriched_scope

     - id: present_formation
       action: |
         Display formation summary using TOON format:
         ```toon
         formation_summary[6]{field,value}:
           Name,{milestone}
           Epic,"{primary} + {linked}"
           Scope,feature 1
           Scope,feature 2
           Scope,feature N
           Estimated,"{N} tasks, {M} categories"
         ```
       output: formation_presented
      
    - id: await_approval
      action: |
        Wait for user response:
        - yes → proceed to task generation
        - no → abort with reason
      output: user_approval
      
    - id: generate_tasks
      condition: user_approved
      action: |
        META-analysis to generate tasks:
        1. Analyze scope items from EPIC scope AND SPEC details
        2. Break into implementable tasks
        3. Assign T-XX IDs
        4. Define file paths and acceptance criteria
        5. Identify parallelization opportunities

        **NOTE:** Tasks are generated from BOTH:
        - Epic scope (feature-level requirements)
        - Spec details (technical implementation details, APIs, data models)
      output: tasks_list
      
    - id: categorize_tasks
      action: |
        FOR each task:
          1. Analyze task description and files
          2. Assign primary category:
             - UI/Design: screens, widgets, navigation
             - Infrastructure: docker, DB, services
             - Logic/Code: business logic, models
             - Testing: test setup, test files
             - Config/DX: configs, tooling
          3. Check OWASP triggers
          4. Mark artifacts required
      output: categorized_tasks
      
    - id: detect_owasp
      action: |
        FOR each task:
          1. Match against OWASP_TRIGGERS
          2. IF match:
             → Add OWASP category tags
             → Set security_critical: true
             → Require OWASP research before spec
      output: owasp_detection
      
    - id: create_tasks_md
      action: |
        1. Generate tasks.md with categorization
        2. Include parallelization map
        3. Mark OWASP-tagged tasks
        4. Include artifact requirements
      output: tasks_file_created
      
    - id: sync_to_beads
      condition: ".beads/ directory exists"
      action: |
        FOR each task in generated_tasks:
          # Create beads issue
          br create "T-{NN}: {title}" \
            --type=task \
            --priority={category_priority}
          
          # Set metadata
          br metadata set {issue_id} milestone={milestone}
          br metadata set {issue_id} category={category}
        
        # Build dependency graph from task order
        FOR i in range(1, len(tasks)):
          br dep add {task[i].id} {task[i-1].id}
        
        # Commit to JSONL
        br sync --flush-only
        
      fallback: |
        LOG: "ℹ️ Beads not initialized. Run 'br init' for graph features."
      output: beads_synced
      
    - id: create_directory_structure
      action: |
        doc/planning/milestones/{name}/
        ├── spec.md (basic shell)
        ├── tasks.md (generated)
        ├── tests.md (template)
        ├── readiness.md (template)
        └── categories/
            ├── ui-spec.md
            ├── infra-spec.md
            ├── logic-spec.md
            ├── testing-spec.md
            ├── config-spec.md
            └── security-spec.md (if OWASP detected)
      output: structure_created
      
    - id: invalidate_state
      action: |
        Update doc/state/.state-hash:
          stale: true
          invalidated_fields:
            - current.milestone
            - progress.tasks.*
          invalidated_by: "/vida-form"
```

---

## Category Definitions

```toon
category_definitions[6]{category,icon,triggers,artifact}:
  UI/Design,🎨,"screens, widgets, navigation, layouts",Mockups
  Infrastructure,🐳,"docker, database, services, external APIs","Config preview, diagrams"
  Logic/Code,⚙️,"models, repositories, use cases, providers","Class diagrams, file tree"
  Testing,🧪,"test setup, test files, coverage",Test matrix
  Config/DX,📦,"configs, tooling, DX files","File previews"
  Security,🔐,"OWASP triggers detected","Security spec, threat model"
```

---

## OWASP Detection (MANDATORY)

<owasp_detection>
**During categorization, scan for OWASP triggers:**

```yaml
OWASP_TRIGGERS:
  MASVS-STORAGE: [token, credential, cache, storage, backup, clipboard]
  MASVS-CRYPTO: [encrypt, decrypt, hash, key, signature, random]
  MASVS-AUTH: [login, logout, session, token, biometric, password, auth]
  MASVS-NETWORK: [http, api, request, certificate, ssl, tls, websocket]
  MASVS-PLATFORM: [deeplink, webview, intent, scheme, permission]
  MASVS-CODE: [input, validation, inject, memory, dependency]
  MASVS-RESILIENCE: [tamper, root, jailbreak, debug, obfuscate]
  MASVS-PRIVACY: [personal, consent, analytics, tracking, gdpr]
```

**If ANY trigger matched:**
→ Task marked with OWASP categories
→ Security spec required
→ OWASP research MANDATORY before spec generation
</owasp_detection>

---

## Output Format

```
✅ MILESTONE FORMATION COMPLETE

📁 Milestone: {name}
📋 Epic: {primary} + {linked}

📊 Task Breakdown:
├── Total: 22 tasks
├── UI/Design: 5 tasks (mockups required)
├── Infrastructure: 4 tasks
├── Logic/Code: 8 tasks
├── Testing: 3 tasks
├── Config/DX: 2 tasks
└── 🔐 Security: 6 tasks (OWASP-tagged)

🔐 OWASP Detection:
├── MASVS-AUTH: T-04, T-05, T-07
├── MASVS-STORAGE: T-04, T-06
└── MASVS-NETWORK: T-04, T-11

📂 Created:
├── doc/planning/milestones/{name}/tasks.md
├── doc/planning/milestones/{name}/readiness.md
├── doc/planning/milestones/{name}/categories/
└── 🔗 Beads: 22 issues synced (with dependencies)

⏭️ Next: /vida-spec-categories {milestone}
   Will generate category specifications with artifacts
```

---

## 🔄 State Invalidation (MANDATORY)

<state_invalidation>
**Fields:** `current.*`, `progress.tasks.*`
**Strategy:** LAZY

```yaml
INVALIDATION:
  action: |
    Update doc/state/.state-hash:
      stale: true
      invalidated_fields:
        - current.milestone
        - current.epic
        - progress.tasks.total
        - progress.tasks.categories
      invalidated_by: "/vida-form {milestone}"
```
</state_invalidation>

---

## Error Handling

<error_handling>

```toon
error_handling[4]{error,recovery}:
  Epic file not found,"Report, ask user to create"
  User rejects formation,"Abort, suggest adjustments"
  Invalid milestone name,"Report format, show examples"
  Empty scope,"Report, ask for features"
```

</error_handling>

---

## Constraints

<constraints>
> See [constraints.md](../constraints.md#vida-form) for full constraints.

Critical items:
- ⛔ NEVER generate tasks without user approval
- ✅ MUST detect OWASP triggers
- ✅ MUST create directory structure
</constraints>

---

## Success Criteria

<success_criteria>
- [ ] User approved formation
- [ ] Tasks generated with T-XX IDs
- [ ] Tasks categorized by type
- [ ] OWASP triggers detected
- [ ] tasks.md created
- [ ] Tasks synced to beads (if .beads/ exists)
- [ ] Directory structure created
- [ ] State invalidated
</success_criteria>

---

## Related

- `/vida-spec-categories` — Generate category specifications (Stage 2)
- `/vida-approve` — Approve all specs for execution
- `/vida-execute` — Autonomous execution (Stage 3)

---

*VIDA Framework v3.0 — Two-Stage Approval Workflow*

---

## Next Actions

<critical>
MANDATORY: If context unclear, READ `_vida/transitions.md#quick-reference`
</critical>

<transitions from="/vida-form">
RECOMMENDED: /vida-approve {milestone}
CONDITION: formation_complete (tasks generated and categorized)
REASON: All tasks created, ready for final approval before execution

ALTERNATIVES:
  [S] /vida-spec-categories {milestone} — Generate category specifications
  [F] /vida-form {milestone} — Redo formation with different scope
  [?] /vida-status — Check current project state

CONTEXT_OUTPUT:
  - milestone_id: {X.XX-name}
  - tasks_generated: {total count}
  - categories_identified: {list of categories}
  - owasp_detected: {true/false, MASVS categories}
  - files_created: {list of created files}
</transitions>

<agent_protocol>
1. Render Next Actions block using template from transitions.md
2. Wait for user choice
3. On choice: AUTO-READ _vida/commands/vida-{choice}.md
4. Show CONTEXT block with CONTEXT_OUTPUT values
5. Execute new command with context
</agent_protocol>
