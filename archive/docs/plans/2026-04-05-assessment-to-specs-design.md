# Design: `assessment-to-specs` Skill

## Context

After implementing a phase, we generate an AI architecture report (`/ai-architecture-report`), feed it to an external LLM (ChatGPT Pro) alongside FOUNDATIONS.md, and receive an assessment document with proposed changes. This assessment needs to be triaged against the actual codebase, turned into specs, and organized into an implementation order for the next phase. This has been a manual process. This skill formalizes it.

The input is generic — any structured assessment document from an external LLM (architecture reviews, feature proposals, game design docs).

## Skill Metadata

```yaml
name: assessment-to-specs
description: "Triages an external LLM assessment against the codebase and FOUNDATIONS.md, writes draft specs for accepted proposals, and creates a fresh IMPLEMENTATION-ORDER.md."
user-invocable: true
arguments:
  - name: assessment_path
    description: "Path to the assessment document"
    required: true
```

Invocation: `/assessment-to-specs <path>`

## Process (3 Phases)

### Phase 1 — Triage (with user gate)

Mandatory reads: assessment doc, `docs/FOUNDATIONS.md`, `docs/spec-drafting-rules.md`, current `specs/IMPLEMENTATION-ORDER.md`.

For each proposal: extract claim and FOUNDATIONS references, grep codebase to verify assumptions, classify as Accept/Reject/Scope-Down with rationale.

Present triage report. Wait for user approval before proceeding.

### Phase 2 — Spec Writing

Auto-detect next S-number. For each approved proposal, write a draft spec to `specs/S{next}-{name}.md` following project conventions and `docs/spec-drafting-rules.md`. Draft quality — expects `/reassess-spec` pass before ticket decomposition. Use Explore agents in parallel.

### Phase 3 — Implementation Order

Write fresh `specs/IMPLEMENTATION-ORDER.md` with: one-line completed-phases reference, new phase name, dependency graph, wave groupings, phase gate criteria. Do NOT archive old IMPLEMENTATION-ORDER.md.

## Guardrails

- FOUNDATIONS alignment mandatory
- Codebase truth over external claims
- YAGNI — reject proposals without downstream consequences
- No backward compatibility layers
- Draft quality specs (reassess-spec before tickets)
- Worktree awareness
- No commit
- Spec-drafting-rules compliance (Permille, profiles, Section H)
