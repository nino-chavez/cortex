# Create Spec Process

You are creating a comprehensive specification for a new feature along with a tasks breakdown.  This process will follow 5 main phases, each with their own workflows:

Process overview (details to follow)

PHASE 0. Load execution profile
PHASE 1. Write the spec document
PHASE 2. Create the tasks list
PHASE 3. Verify the spec & tasks list (conditional on depth)
PHASE 4. Display the results to user

Follow each of these phases and their individual workflows IN SEQUENCE:

## Command Flags

This command supports the following optional flags that **override** the persisted execution profile:

**Strategy override:**
- `--solo` | `--squad`

**Depth override:**
- `--lean` | `--standard` | `--thorough`

## Process:

### PHASE 0: Load Execution Profile

1. Read `specchain/specs/[current-spec]/planning/execution-profile.yml` to load the persisted strategy and depth.
2. If command flags were provided (e.g., `--thorough`, `--solo`), apply them as overrides to the loaded profile.
3. If the execution-profile.yml doesn't exist, read defaults from `specchain/config.yml` under the `execution` section.
4. Store the resolved **strategy** and **depth** for use in subsequent phases.

**Important**: If a flag override changed the profile, update `planning/execution-profile.yml` with the new values and set `set_by: create-spec`.

### PHASE 1: Delegate to Spec Writer

Use the **spec-writer** subagent to create the specification document for this spec:

Provide the spec-writer with:
- The spec folder path (find the current one or the most recent in `specchain/specs/*/`)
- The requirements from `planning/requirements.md`
- Any visual assets in `planning/visuals/`

The spec-writer will create `spec.md` inside the spec folder.

Wait until the spec-writer has created `spec.md` before proceeding with PHASE 2 (delegating to task-list-creator).

### PHASE 2: Delegate to Tasks List Creator

Once `spec.md` has been created, use the **tasks-list-creator** subagent to break down the spec into an actionable tasks list with strategic grouping and ordering.

Provide the tasks-list-creator:
- The spec folder path (find the current one or the most recent in `specchain/specs/*/`)
- The `spec.md` file that was just created.
- The original requirement from `planning/requirements.md`
- Any visual assets in `planning/visuals/`

The tasks-list-creator will read `planning/execution-profile.yml` itself to determine strategy/depth for task grouping.

The tasks-list-creator will create `tasks.md` inside the spec folder.

### PHASE 3: Verify Specifications (Depth-Conditional)

**This phase is conditional on the resolved depth:**

#### If depth is `lean`: **SKIP Phase 3 entirely.** Proceed directly to Phase 4.

#### If depth is `standard`: Run verification as normal (current behavior).

#### If depth is `thorough`: Run verification with enhanced reusability audit.

**For `standard` and `thorough`:**

Use the **spec-verifier** subagent to verify accuracy:

Provide the spec-verifier with:
- ALL of the questions that were asked to the user during requirements gathering (from earlier in this conversation)
- ALL of the user's raw responses to those questions
- The spec folder path
- **The resolved depth** — instruct the verifier:
  - `standard`: Run standard verification workflow.
  - `thorough`: Run standard verification + perform deep reusability audit (Step 5 in spec-verifier workflow).

The spec-verifier will run its verifications and produce a report in `verification/spec-verification.md`

### PHASE 4: Display Results

DISPLAY to the user:
- The spec creation summary from spec-writer
- The tasks list creation summary from tasks-list-creator
- **Execution profile**: strategy=[strategy], depth=[depth]
- If Phase 3 was run: The verification summary from spec-verifier
- If Phase 3 was skipped (lean): Note that verification was skipped per lean depth

If verification found issues, highlight them for the user's attention.

## Expected Output

After completion, you should have:

```
specchain/specs/[date-spec-name]/
├── planning/
│   ├── initialization.md
│   ├── requirements.md
│   ├── execution-profile.yml
│   └── visuals/
├── implementation/
├── verification/
│   └── spec-verification.md (only for standard/thorough)
├── spec.md
└── tasks.md
```
