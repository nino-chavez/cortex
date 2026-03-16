# Execution Profiles

**Version**: 1.0.0
**Last Updated**: 2026-02-13
**Audience**: Developers, Project Managers

## Overview

Execution profiles provide fine-grained control over how Specchain creates and implements specifications. Instead of a single "mode" setting, execution profiles use two orthogonal axes: **Strategy** (who does the work) and **Depth** (how thorough the work is).

This approach allows you to compose the exact workflow you need for each feature, from rapid prototyping to mission-critical releases.

## The Two Axes

### Axis 1: Strategy — Who Does the Work

Strategy determines the agent model used for implementation.

| Value | Description | Agent Model |
|-------|-------------|-------------|
| **`solo`** | Single agent handles ALL implementation | One agent works through all task groups sequentially. No specialist delegation, no domain verifiers. Tasks are grouped by feature slice (vertical through the stack). |
| **`squad`** | Multi-agent delegation to specialists | Multiple specialist agents are assigned to task groups based on domain expertise (database-engineer, api-engineer, ui-designer, testing-engineer). Tasks are grouped by domain layer. Each specialist's work is verified by a domain verifier. |

**When to use `solo`:**
- Small features (≤8 tasks)
- Single-domain work (e.g., API-only or database-only)
- Hotfixes and bug fixes
- Infrastructure/config changes
- You prefer a conductor-like workflow with a single context window

**When to use `squad`:**
- Medium to large features (>8 tasks)
- Multi-domain work (database + API + frontend)
- Features requiring specialist expertise
- When you want parallel execution across domains
- Projects with well-defined specialist roles

---

### Axis 2: Depth — How Thorough

Depth determines the rigor and verification levels applied throughout the workflow.

| Value | Spec Creation | Task Breakdown | Implementation | Verification |
|-------|--------------|----------------|----------------|--------------|
| **`lean`** | 3-5 questions, skip visual analysis | Skip testing-engineer group, minimal verification | Minimal checks: tests pass + tasks checked | Skip domain verifiers, minimal final verification |
| **`standard`** | 6-9 questions, full requirements gathering | Standard task groups, focused tests (2-8 per group) | Full pipeline with domain specialists | Domain verifiers + full final verification |
| **`thorough`** | 6-9 questions + architecture/testing questions | TDD red-green-refactor template per task | Phase checkpoints with user confirmation, fresh agents | Deep reusability audit, coverage reports, manual verification prompts |

**When to use `lean`:**
- Prototyping or proof-of-concept work
- Simple features with minimal risk
- Internal tools or developer utilities
- When speed is prioritized over rigor

**When to use `standard`:**
- Most production features
- Balanced speed and quality
- Well-understood requirements
- Standard release cadence

**When to use `thorough`:**
- Mission-critical features
- High-risk changes (auth, payments, data integrity)
- Major releases
- Features with complex integration requirements
- When you need maximum confidence before deployment

---

## Composition Matrix

The power of execution profiles comes from composing strategy and depth:

| | `lean` | `standard` | `thorough` |
|---|---|---|---|
| **`solo`** | Fastest path. One agent, minimal checks, 3-5 questions, skip verifiers. | Single-agent conductor. Full pipeline (6-9 questions, focused tests), one agent handles all domains. | Max single-agent rigor. TDD per task, phase boundary checkpoints, full verification with one agent. |
| **`squad`** | Fast specialists. Parallel delegation, 3-5 questions, skip testing group and verifiers. | **Default mode.** Balanced rigor with specialist delegation, domain verifiers, focused testing. | Full rigor. TDD per specialist, fresh agents per task group, all verification layers, coverage reports, manual gates. |

### Common Combinations

| Use Case | Profile | Why |
|----------|---------|-----|
| Hotfix / small bug | `--solo --lean` | Single agent, minimal ceremony, fastest path to resolution |
| Config change | `--solo --lean` | Infrastructure-only, no UI verification needed |
| Standard feature | *(default: `squad` + `standard`)* | Balanced approach for production features |
| Solo developer flow | `--solo --standard` | Full pipeline, single context, conductor-like workflow |
| Major feature | `--squad --thorough` | Full specialist rigor with TDD and phase checkpoints |
| Critical release (auth, payments) | `--squad --thorough` | Maximum verification including manual confirmation gates |

---

## Flag Propagation Hierarchy

Execution profiles are resolved using a three-tier hierarchy (highest priority first):

### 1. Command Flags (Highest Priority)

Flags passed directly to commands override all other settings.

```bash
# Override for this invocation only
/implement-spec --solo --lean
/new-spec --squad --thorough [description]
/create-spec --standard
```

**Available flags:**
- Strategy: `--solo` | `--squad`
- Depth: `--lean` | `--standard` | `--thorough`

### 2. Per-Spec Profile (Medium Priority)

Each spec folder contains `planning/execution-profile.yml`, created when you run `/new-spec`.

```yaml
# specchain/specs/2026-02-13-user-auth/planning/execution-profile.yml
strategy: squad      # solo | squad
depth: thorough      # lean | standard | thorough
set_by: new-spec     # which command last set this profile
```

This profile is:
- **Created** by `/new-spec` (with flags or project defaults)
- **Loaded** by `/create-spec` and `/implement-spec`
- **Updated** if you provide flag overrides to `/create-spec` or `/implement-spec`

This allows each spec to maintain its own execution profile across conversation boundaries.

### 3. Project Config (Lowest Priority)

Project-wide defaults in `specchain/config.yml` under the `execution` section.

```yaml
# specchain/config.yml
execution:
  strategy: squad       # Default for all new specs
  depth: standard       # Default for all new specs
  auto_suggest: true    # Enable auto-suggestion
```

These are the fallback values when no flags are provided and no per-spec profile exists.

---

## Auto-Suggest Behavior

When `execution.auto_suggest: true` in `specchain/config.yml`, the system can analyze a spec and recommend an execution profile.

### How It Works

1. **Triggered when**: You run `/implement-spec` WITHOUT explicit `--solo/--squad` or `--lean/--standard/--thorough` flags
2. **Analysis**: Reads `tasks.md` and evaluates:
   - Total number of tasks
   - Number of distinct domains (database, API, frontend, testing)
   - Complexity indicators (task groups, cross-cutting concerns)
3. **Recommendation rules**:
   - Tasks ≤ 8 AND single domain → suggest `solo`
   - Tasks > 20 → suggest `thorough`
   - Spec touches ≥ 2 domains → suggest `squad`
   - Infrastructure/config only → suggest `solo` + `lean`
4. **Presentation**: Shows recommendation with rationale and waits for user confirmation:

```
Spec Analysis: 6 tasks, single domain (API), no frontend components.
Recommended: solo + lean
Reason: Small scope, single domain, no UI verification needed.

Accept? Or override with flags (e.g., --squad --standard)
```

5. **User response**: You can:
   - Accept the suggestion (enter `y` or `yes`)
   - Provide flag overrides (e.g., `--squad --standard`)
   - Skip auto-suggest for future specs by setting `auto_suggest: false` in config.yml

**Important**: The system NEVER applies suggestions silently. It always prompts for confirmation.

---

## Behavioral Differences by Profile

### Spec Creation (new-spec → create-spec)

| Phase | `lean` | `standard` | `thorough` |
|-------|--------|------------|------------|
| **Questions** | 3-5 focused functional questions | 6-9 questions + visual asset request + reusability check | 6-9 questions + architecture integration + critical test scenarios |
| **Visual Analysis** | Skipped | Mandatory visual check via bash | Mandatory visual check via bash |
| **Reusability** | Skipped | Asked for existing similar features | Asked + deep codebase audit (verifier only) |
| **Verification** | Skipped (no spec-verifier) | Full spec-verifier with standard checks | Full spec-verifier + deep reusability audit |

### Task Breakdown (tasks-list-creator)

| Aspect | `solo` | `squad` |
|--------|--------|---------|
| **Grouping** | By feature slice (vertical through stack) | By domain layer (database, API, frontend, testing) |
| **Assignment** | All groups assigned to `solo` | Groups assigned to specialist implementers |
| **Testing** | Inline tests in each group (2-8 per group) | Separate testing-engineer group (adds up to 10 tests) |

| Aspect | `lean` | `standard` | `thorough` |
|--------|--------|------------|------------|
| **Test Count** | 2-4 focused tests per group | 2-8 focused tests per group | TDD red-green-refactor template per group |
| **Testing Group** | Skipped (squad mode only) | Included (adds up to 10 gap-filling tests) | Included with coverage requirements |
| **Verification Steps** | Minimal (test pass + smoke check) | Full verification commands per group | Detailed verification + coverage metrics |

### Implementation (implement-spec)

| Phase | `solo` | `squad` |
|-------|--------|---------|
| **Agent Assignment** | No assignment phase, orchestrator handles all groups | Phase 1 creates task-assignments.yml with specialist assignments |
| **Delegation** | Orchestrator processes each group directly | Each group delegated to assigned specialist from implementers.yml |
| **Domain Verifiers** | Skipped (no verifiers for solo) | Domain verifiers run after implementation groups (unless lean) |

| Aspect | `lean` | `standard` | `thorough` |
|--------|--------|------------|------------|
| **TDD** | Not enforced | Not enforced | Red-Green-Refactor cycle per task |
| **Phase Checkpoints** | None | None | User confirmation after each task group |
| **Fresh Agents** | Advisory after 3 groups | Advisory after 3 groups | Forced per group (squad) or per phase (solo) |
| **Domain Verifiers** | Skipped | Run after implementation (squad only) | Run with enhanced verification |
| **Final Verification** | Feature tests only | Full test suite | Full suite + coverage report + manual verification plan |

---

## Migration Guide

If you're upgrading from the old `workflow.default_mode` configuration, here's how to migrate:

### Old Configuration (Deprecated)

```yaml
# OLD (no longer used)
workflow:
  default_mode: selective  # direct | selective | thorough
  fresh_agent_for_modes:
    - thorough
```

### New Configuration

```yaml
# NEW
execution:
  strategy: squad          # solo | squad
  depth: standard          # lean | standard | thorough
  auto_suggest: true       # true | false

context_management:
  fresh_agent_for_depths:  # Renamed from fresh_agent_for_modes
    - thorough
```

### Mapping Old Modes to New Profiles

| Old `workflow.default_mode` | New `execution` Profile |
|-----------------------------|-------------------------|
| `direct` | `strategy: solo` + `depth: lean` |
| `selective` | `strategy: squad` + `depth: standard` |
| `thorough` | `strategy: squad` + `depth: thorough` |

**Steps to migrate:**

1. **Update config.yml**:
   - Remove the `workflow` section entirely
   - Add the `execution` section with `strategy`, `depth`, and `auto_suggest`
   - Rename `fresh_agent_for_modes` to `fresh_agent_for_depths` under `context_management`

2. **Update any documentation** that references the old mode names

3. **Re-run existing specs** (optional): If you have existing specs without `planning/execution-profile.yml`, they will use the new project defaults from `execution`

**Backward Compatibility**: The old `workflow` section is completely ignored. There is no automatic migration or fallback behavior.

---

## Examples

### Example 1: Hotfix Workflow

```bash
# Fast, single-agent, minimal verification
/new-spec --solo --lean "Fix null pointer in user profile endpoint"
/create-spec
/implement-spec
```

**Result**: 3-5 questions, one agent handles all work, feature tests only, no verifiers.

---

### Example 2: Standard Feature (Default)

```bash
# No flags needed - uses project defaults
/new-spec "Add comment system to blog posts"
/create-spec
/implement-spec
```

**Result**: 6-9 questions, specialist delegation (database → API → UI → testing), domain verifiers, full final verification.

---

### Example 3: Critical Feature with Maximum Rigor

```bash
# Maximum verification for auth system
/new-spec --squad --thorough "Two-factor authentication for admin users"
/create-spec --thorough
/implement-spec --thorough
```

**Result**: 6-9+ questions (including architecture and testing scenarios), specialist delegation, TDD per task, phase checkpoints, fresh agents per group, coverage reports, manual verification prompts.

---

### Example 4: Solo Developer Flow

```bash
# Full pipeline, single agent context
/new-spec --solo --standard "Implement CSV export for reports"
/create-spec
/implement-spec
```

**Result**: 6-9 questions, one agent handles database → API → UI tasks, focused tests, full verification.

---

### Example 5: Override Per-Spec Profile at Implementation

```bash
# Spec was created with default (squad + standard)
# But you want to implement it solo for this session
/implement-spec --solo
```

**Result**: Loads the spec's profile, applies the `--solo` override, updates `planning/execution-profile.yml` with `set_by: implement-spec`.

---

## Related Documentation

- [README.md](../../README.md) — Quick overview and setup
- [config.yml](../config.yml) — Project configuration reference
- [STATE.md](../STATE.md) — Session memory and execution tracking
- [/new-spec command](../../.claude/commands/specchain/new-spec.md) — Initialize specs with execution profiles
- [/create-spec command](../../.claude/commands/specchain/create-spec.md) — Generate spec and tasks
- [/implement-spec command](../../.claude/commands/specchain/implement-spec.md) — Execute implementation with profiles

---

## Best Practices

### Start with Defaults, Override as Needed

Set your most common workflow as the project default in `config.yml`, then use flags only when you need to deviate.

```yaml
# config.yml - most features use squad + standard
execution:
  strategy: squad
  depth: standard
  auto_suggest: true
```

```bash
# Use defaults for standard work
/new-spec "Add user preferences page"

# Override for hotfixes
/new-spec --solo --lean "Fix broken link in footer"
```

### Use Auto-Suggest for Unfamiliar Specs

When unsure which profile to use, let auto-suggest analyze and recommend:

```bash
# Let the system analyze and suggest
/implement-spec
# > Spec Analysis: 12 tasks, 3 domains (database, API, frontend)
# > Recommended: squad + standard
# > Accept? (y/override with flags)
```

### Match Depth to Risk

- **`lean`**: Low-risk internal tools, prototypes
- **`standard`**: Most production features
- **`thorough`**: Auth, payments, data integrity, major releases

### Persist Profiles Across Conversations

The per-spec profile in `planning/execution-profile.yml` ensures consistency when working on a spec across multiple sessions.

```bash
# Session 1 - Initialize with thorough
/new-spec --squad --thorough "Payment processing system"
/create-spec

# Session 2 (days later) - Automatically loads squad + thorough
/implement-spec
```

### Use Solo for Rapid Iteration

Solo mode keeps all context in one agent, which can be faster for small features and provides a more conversational flow.

---

## Troubleshooting

### Auto-Suggest Not Working

**Problem**: `/implement-spec` doesn't show a suggestion prompt.

**Causes**:
- `execution.auto_suggest: false` in config.yml
- You provided explicit flags (e.g., `/implement-spec --solo`)
- The spec already has a persisted profile and you didn't provide override flags

**Solution**: Enable auto-suggest in config.yml or omit flags to trigger analysis.

---

### Wrong Profile Applied

**Problem**: Spec is using a different profile than expected.

**Check the hierarchy**:
1. Did you pass command flags? They override everything.
2. Does `planning/execution-profile.yml` exist? It overrides project config.
3. What's in `specchain/config.yml` under `execution`?

**Solution**: Either update the per-spec profile file or use flags to override.

---

### Verification Skipped When Not Expected

**Problem**: Domain verifiers or spec-verifier didn't run.

**Causes**:
- `depth: lean` skips all verifiers
- `strategy: solo` skips domain verifiers (no squad delegation)

**Solution**: Use `--squad --standard` or `--thorough` for full verification.

---

### Too Many Tests Being Written

**Problem**: Implementers are writing comprehensive test suites.

**Expected behavior**:
- **Standard/Thorough**: Each implementation group writes 2-8 focused tests, testing-engineer adds up to 10 gap-filling tests (total ~16-34 tests per feature)
- **Lean**: 2-4 focused tests per group, no testing-engineer group

**Check**: Review `tasks.md` to ensure test limits are specified in each task group.

---

## Advanced Topics

### Custom Fresh Agent Rules

You can customize when fresh agents are spawned by editing `context_management.fresh_agent_for_depths`:

```yaml
context_management:
  warn_after_task_groups: 3   # Show advisory after N groups
  fresh_agent_for_depths:     # Auto-spawn fresh agents for these depths
    - thorough
```

For `thorough` + `squad`, a fresh agent is spawned per task group. For `thorough` + `solo`, fresh agents are spawned at phase boundaries only (to preserve solo identity).

---

### Combining with Context Reports

Use `--context-report` flag to see session state before implementation:

```bash
/implement-spec --context-report
```

This displays:
- Current session context
- Active blockers
- Recent key decisions
- Established patterns

Useful when resuming work after a break or switching between specs.

---

### Per-Spec Profile Introspection

To see what profile a spec is using:

```bash
# Read the per-spec profile
cat specchain/specs/2026-02-13-my-feature/planning/execution-profile.yml
```

To change a spec's profile mid-flight:

```bash
# Override with flags (updates the file)
/implement-spec --solo --lean
```

The `set_by` field tracks which command last updated the profile.
