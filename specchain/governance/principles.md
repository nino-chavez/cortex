# Governance Principles

Distilled from the Aegis Constitutional AI Governance Framework. These principles guide how AI-assisted code should be planned, written, and verified.

---

## 1. Scope Minimization

Prefer the smallest viable change. Rank approaches by blast radius:

1. **MVP-Fix** — Targeted change to a single file or function
2. **Surgical Refactor** — Scoped change across 2-3 related files
3. **Systemic Change** — Architectural modification across many files

Always justify why a higher-impact tier is necessary before adopting it.

## 2. Behavioral Contracts

Assert observable behavior, not implementation details.

- **Good:** "User is redirected to /dashboard after login"
- **Bad:** "Code calls `router.push('/dashboard')`"

Specs and verification should describe *what happens*, not *how it happens*. This keeps tests resilient to refactoring.

## 3. Traceability

AI-generated code should be identifiable and its intent documented.

- Commit messages should indicate AI assistance when applicable
- Non-obvious decisions should have inline rationale (`// why:` comments)
- Spec references should link back to the originating requirement
- Session logs in STATE.md provide an audit trail of what was done and why

## 4. Boundary Validation

Validate at system boundaries. Trust internal code.

- **Validate:** User input, external API responses, environment variables, file I/O
- **Trust:** Internal function calls, typed interfaces, framework guarantees

Do not add defensive checks for conditions that internal code already prevents. Over-validation adds noise and hides real boundary issues.

## 5. Graceful Degradation

Systems should fail safely and recover where possible.

- No stack traces or internal paths in user-facing errors
- Provide fallback behavior for non-critical failures
- Log errors with enough context to diagnose without exposing secrets
- Distinguish between retryable and terminal failures

## 6. Observability

Key operations should produce traceable events.

- Log at operation boundaries (start, success, failure) not inside loops
- Include correlation IDs for operations that span multiple services
- Prefer structured logging over freeform strings
- Make it possible to reconstruct what happened from logs alone

## 7. Conversion Architecture

User-facing interfaces follow the Clarity -> Trust -> Action pipeline.

Every screen must answer three questions in order:
1. **What is this?** — Visual hierarchy and typography establish immediate understanding
2. **Why should I trust it?** — Proof elements, real imagery, and transparent information reduce hesitation
3. **What do I do next?** — A purposeful, high-contrast CTA provides clear direction

Design decisions are evaluated against this pipeline, not aesthetic preference. Refer to `standards/frontend/ux-conversion.md` for specific rules.

---

## Applying These Principles

These principles inform SpecChain workflows at every stage:

| Stage | Principles Applied |
|-------|-------------------|
| `/new-spec` | Scope minimization (right-size the spec), Conversion Architecture (ask about conversion goals) |
| `/create-spec` | Behavioral contracts (verification criteria), Conversion Architecture (specify trust signals, CTAs, hierarchy) |
| `/implement-spec` | Traceability (session logs), boundary validation |
| Verification | Behavioral contracts, observability, Conversion Architecture (verify UX conversion compliance) |
| Standards | All — standards encode these principles as rules |
