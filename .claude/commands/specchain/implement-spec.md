# Spec Implementation Process

Now that we have a spec and tasks list ready for implementation, we will proceed with implementation of this spec by following this multi-phase process:

PHASE 0: Load session state and context
PHASE 0.5: Resolve execution profile
PHASE 1: Plan the subagent assignments for each task group (squad only)
PHASE 2: Delegate implementation of each task group
PHASE 3: Delegate verifications to verifier subagents (squad + standard/thorough only)
PHASE 4: Produce the final verification report
PHASE 5: Update session state

Follow each of these phases and their individual workflows IN SEQUENCE:

## Command Flags

This command supports the following optional flags:

**Execution profile overrides:**
- `--solo` | `--squad` — Strategy override
- `--lean` | `--standard` | `--thorough` — Depth override

**Context management:**
- `--fresh-agent` — Force spawning a fresh agent for the next task group
- `--context-report` — Display context summary before proceeding with implementation

## Multi-Phase Process

### PHASE 0: Load session state and context

Read `specchain/STATE.md` to understand the current project state:

1. **Review Session Context**: Understand where work left off in previous sessions
2. **Check Active Blockers**: Identify any blockers that may affect the current spec
3. **Review Key Decisions**: Note any relevant decisions that apply to this implementation
4. **Review Patterns Established**: Use established patterns to maintain consistency

If the `--context-report` flag is provided, display a summary of:
- Current session context
- Active blockers relevant to this spec
- Recent key decisions
- Established patterns that apply

Store the current spec folder path for later STATE.md updates.

---

### PHASE 0.5: Resolve Execution Profile

1. **Read persisted profile**: Load `specchain/specs/[this-spec]/planning/execution-profile.yml`
2. **Apply command flag overrides**: If `--solo`, `--squad`, `--lean`, `--standard`, or `--thorough` flags were provided, they override the persisted values.
3. **If no profile exists and no flags**: Read defaults from `specchain/config.yml` under the `execution` section.

#### Auto-Suggest (when `auto_suggest: true` AND no explicit strategy/depth flags provided)

If `specchain/config.yml` has `execution.auto_suggest: true` and no explicit flags were passed:

1. **Analyze the spec**: Read `specchain/specs/[this-spec]/tasks.md` and evaluate:
   - Total number of tasks
   - Number of distinct domains touched (database, API, frontend, etc.)
   - Complexity indicators (number of task groups, cross-cutting concerns)

2. **Apply suggestion rules**:
   - Tasks <= 8 AND single domain -> suggest `solo`
   - Tasks > 20 -> suggest `thorough`
   - Spec touches >= 2 domains -> suggest `squad`
   - Infrastructure/config only tasks -> suggest `solo` + `lean`

3. **Present recommendation to user**:
   ```
   Spec Analysis: [N] tasks, [domains description].
   Recommended: [strategy] + [depth]
   Reason: [rationale based on rules above]

   Accept? Or override with flags (e.g., --squad --standard)
   ```

4. **Wait for user confirmation or override.** Never silently decide.

5. **Store resolved strategy + depth** for all subsequent phases.

If a flag override changed the profile, update `planning/execution-profile.yml` with the new values and set `set_by: implement-spec`.

---

### PHASE 1: Plan subagent assignments

**This phase is conditional on strategy:**

#### If strategy is `solo`: **SKIP Phase 1 entirely.** No agent assignment needed — the orchestrator handles all task groups directly.

#### If strategy is `squad`: Current behavior.

Read the following files:
- `specchain/specs/[this-spec]/tasks.md`
- `specchain/roles/implementers.yml`

Create `specchain/specs/[this-spec]/planning/task-assignments.yml` with this structure:

```yaml
task_assignments:
  - task_group: "Task Group 1: [Title from tasks.md]"
    assigned_subagent: "[implementer-id-from-implementers.yml]"

  - task_group: "Task Group 2: [Title from tasks.md]"
    assigned_subagent: "[implementer-id-from-implementers.yml]"

  # Continue for all task groups found in tasks.md
```

Ensure each assigned subagent exists in both of these locations:
- In implementers.yml there must be an implementer with this role ID.
- In `.claude/agents/specchain` there must be a file named by this implementer ID.

---

### PHASE 2: Delegate task group implementations

Loop through each task group in `specchain/specs/[this-spec]/tasks.md` and delegate its implementation.

#### Strategy: `solo`

Delegate ALL task groups to the orchestrator (self). Process task groups **sequentially**. For each task group, include:
- The full spec file: `specchain/specs/[this-spec]/spec.md`
- STATE.md context (blockers, decisions, patterns)
- The task group with all its sub-tasks

#### Strategy: `squad`

Current behavior — delegate each task group to its assigned specialist subagent from `task-assignments.yml`.

For each delegation, provide the subagent with:
- The task group (including the parent task and all sub-tasks)
- The spec file: `specchain/specs/[this-spec]/spec.md`
- **Relevant STATE.md context:**
  - Any active blockers related to this task group
  - Key decisions that may affect implementation
  - Established patterns to follow
- Instruct subagent to:
  1. Perform their implementation
  2. Check off the task and sub-task(s) in `specchain/specs/[this-spec]/tasks.md`
  3. Document their work in an implementation report named and numbered by this task name and placed in `specchain/specs/[this-spec]/implementation/`.
  4. Report any new blockers discovered during implementation
  5. Report any new patterns established during implementation

#### Depth modifier: `thorough` (applies to both strategies)

Prepend the following **TDD instruction block** to each task group delegation:

> "For each sub-task in this task group, follow the TDD Red-Green-Refactor cycle:
> 1. **RED** — Write a failing test that defines the expected behavior
> 2. **GREEN** — Write the minimum code necessary to make the test pass
> 3. **REFACTOR** — Clean up the code while keeping all tests green
>
> After completing each sub-task, verify tests pass before proceeding to the next."

**Phase checkpoints (thorough only):** After completing each task group, pause and present a verification prompt to the user:
```
Task Group [N]: [Title] — Complete.
[Brief summary of what was implemented]

Verify and confirm before proceeding to Task Group [N+1]? (y/continue/override)
```
Wait for user confirmation before proceeding to the next task group.

#### Context Management

After completing each task group, track the count of completed task groups in this session.

**`lean` and `standard` depths:**
When the configured `warn_after_task_groups` threshold is reached (default: 3), display:
```
Context Advisory: [N] task groups completed in this session.
Consider using fresh agents for remaining tasks.
Use --fresh-agent flag to spawn a fresh agent for the next task group.
```

**`thorough` + `squad`:** Force a **fresh agent for every task group** (not just after threshold).

**`thorough` + `solo`:** Force a fresh agent at **phase boundaries only** (not per group — preserves solo identity). Phase boundaries are between major implementation milestones (e.g., after all data model tasks, before API tasks).

If `--fresh-agent` flag is active OR if fresh agent is required by depth rules:
- Spawn a fresh agent for the next task group
- Provide the fresh agent with:
  - STATE.md content (session context, decisions, blockers)
  - Current task group from tasks.md
  - Path to spec.md
  - List of completed task groups (names only, not implementation details)

---

### PHASE 3: Delegate verifications to verifier subagents

**This phase is conditional on strategy and depth:**

#### If strategy is `solo` (any depth): **SKIP Phase 3 entirely.** No domain verifiers for single-agent work.

#### If strategy is `squad` + depth is `lean`: **SKIP Phase 3 entirely.**

#### If strategy is `squad` + depth is `standard` or `thorough`: Run verifiers (current behavior).

1. Collect the list of subagent IDs that were delegated to in Phase 2.

2. Read `implementers.yml` and find those subagent IDs. Collect the verifier role IDs specified in their `verified_by` field.

3. If there are verifier roles, ensure those verifiers are defined in `specchain/roles/verifiers.yml`.

4. If there are verifier roles, delegate to each verifier subagent:
   - Collect all task groups that fall under the purview of this verifier (i.e. these tasks' implementers' verified_by specifies this verifier).
   - Provide to the verifier:
     1. Details of those task groups (parent task and sub-tasks) to the verifier for verification.
     2. The spec file: `specchain/specs/[this-spec]/spec.md` for context.
   - Instruct the verifier:
     1. Read and analyze these tasks and where they fit in the context of this spec.
     2. Run tests to verify implementation of these tasks.
     3. Verify whether `specchain/specs/[this-spec]/tasks.md` has been updated to reflect these tasks' completeness.
     4. Document your verification report and place this document in: `specchain/specs/[this-spec]/verification/`

---

### PHASE 4: Produce the final verification report

**The scope of verification depends on depth:**

Use the **implementation-verifier** subagent to do its implementation verification and produce its final verification report.

Provide to the subagent the following:
- The path to this spec: `specchain/specs/[this-spec]`
- **The resolved depth** (lean, standard, or thorough)

Instruct the subagent to do the following:
  1. Run all of its final verifications according to its built-in workflow (depth-aware)
  2. Produce the final verification report in `specchain/specs/[this-spec]/verification/final-verification.md`.

---

### PHASE 5: Update session state

Update `specchain/STATE.md` to reflect the completed implementation session:

1. **Update Last Updated timestamp**: Set to current date/time

2. **Update Active Spec**: Set to the spec folder that was just implemented

3. **Update Session Context**: Write 2-3 sentences summarizing:
   - What spec was implemented
   - Overall outcome (success, partial, issues)
   - What should be done next

4. **Add Key Decisions**: If any significant decisions were made during implementation, add them to the Key Decisions table with:
   - Date
   - Decision made
   - Rationale
   - Spec/Context reference

5. **Update Blockers**:
   - Move any resolved blockers from Active to Resolved Blockers table
   - Add any new blockers discovered during implementation to Active Blockers

6. **Add Execution Profile entry**: Add a row to the **Execution Profiles** table:
   | [Spec name] | [strategy] | [depth] | [today's date] |

7. **Add Patterns Established**: If any new patterns were discovered or established during implementation, add them to the Patterns Established section

8. **Add Session Log Entry**: Add a new entry with:
   - Current date and session number
   - Summary of what was done (task groups completed, verifications run)
   - Execution profile used (strategy + depth)
   - Next steps for future sessions

**Pruning**: If any section exceeds the configured limits in `config.yml`:
- `max_decisions`: Keep only the most recent N decisions
- `max_resolved_blockers`: Keep only the most recent N resolved blockers
- `max_session_logs`: Keep only the most recent N session log entries
