# New Spec Process

You are initiating a new spec for a new feature.  This process will follow 3 main phases, each with their own workflow steps:

Process overview (details to follow)

PHASE 1. Initialize spec
PHASE 2. Research requirements for this spec
PHASE 3. Inform the user that the spec has been initialized

Follow each of these phases and their individual workflows IN SEQUENCE:

## Command Flags

This command supports the following optional flags:

**Strategy override (mutually exclusive):**
- `--solo` — Single agent handles all implementation
- `--squad` — Multi-agent delegation to specialists (default)

**Depth override (mutually exclusive):**
- `--lean` — Minimal: 3-5 questions, skip visual analysis, skip verifiers
- `--standard` — Full pipeline (default)
- `--thorough` — Max rigor: extra questions on architecture and testing expectations

If no flags are provided, defaults are read from `specchain/config.yml` under the `execution` section.

## Multi-Phase Process:

### PHASE 1: Initialize Spec

Use the **spec-initializer** subagent to initialize a new spec.

IF the user has provided a description, provide that to the spec-initializer.

**Pass resolved execution flags** to the spec-initializer so it can write `planning/execution-profile.yml`:
- If `--solo` or `--squad` was specified, pass the strategy override
- If `--lean`, `--standard`, or `--thorough` was specified, pass the depth override
- If no flags were provided, tell the spec-initializer to use project config defaults

The spec-initializer will provide the path to the dated spec folder (YYYY-MM-DD-spec-name) they've created, along with the resolved execution profile (strategy + depth).

### PHASE 2: Research Requirements

After spec-initializer completes, immediately use the **spec-researcher** subagent:

Provide the spec-researcher with:
- The spec folder path from spec-initializer
- **The resolved depth** from the execution profile, with these instructions:
  - `lean`: "Ask 3-5 core questions. Skip visual asset request. Skip reusability question. Focus on functional requirements only."
  - `standard`: Follow standard behavior (6-9 questions with visual request and reusability check).
  - `thorough`: Follow standard behavior + "Add 1-2 additional questions about architectural integration with existing codebase and critical test scenarios that must be covered."

The spec-researcher will give you several separate responses that you MUST show to the user. These include:
1. Numbered clarifying questions along with a request for visual assets (show these to user, wait for user's response)
2. Follow-up questions if needed (based on user's answers and provided visuals)

**IMPORTANT**:
- Display these questions to the user and wait for their response
- The spec-researcher may ask you to relay follow-up questions that you must present to user

### PHASE 3: Inform the user

After all steps complete, inform the user:

"Spec initialized successfully!

Spec folder created: `[spec-path]`
Requirements gathered
Execution profile: **[strategy]** + **[depth]**
Visual assets: [Found X files / No files provided]

Run `/create-spec` to generate the detailed specification and task breakdown."
