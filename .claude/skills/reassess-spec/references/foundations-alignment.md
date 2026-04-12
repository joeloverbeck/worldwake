# FOUNDATIONS.md Alignment Check (Step 4)

## 4.0 Internal Contradictions

Before checking FOUNDATIONS, scan for contradictions between the spec's Design Goals, Non-Goals, FOUNDATIONS Alignment table, and Deliverables. If the spec includes a Stored State vs. Derived Read-Model table, verify consistency with FND-27 and FND-3.

## 4.1 Alignment Table Verification

If the spec has a FOUNDATIONS Alignment table, verify each entry. Check that principle numbers match names in `docs/FOUNDATIONS.md` — misnumbered principles are common. Flag mismatches as Issues.

## 4.2 Missing Principles

Identify Foundation principles the spec should address but doesn't. Pay particular attention to:
- **P1** (Maximal Emergence) — authored sequences or magic triggers?
- **P7** (Locality) — agents querying global state?
- **P14** (World State != Belief State) — agents reading authoritative state directly?
- **P26** (Systems Interact Through State) — cross-system direct calls?
- **P28** (No Backward Compatibility) — compatibility shims or deferred migration?
- **P30** (Causal Hooks Declaration) — count items from source each time (list may evolve). Full 18-item checklist for new system specs; bugfix/lifecycle/architecture-fix specs need only the relevant subset (typically: information-path, positive-feedback, stored state).

## 4.3 Record Alignment Issues

Record each issue with specific Foundation number and conflict.

## 4.4 Authoritative-to-AI Impact Rule

If the spec modifies action preconditions, `validate_*` functions, affordance generation (`enumerate_*_payloads`), `can_exercise_control`, goal satisfaction (`is_satisfied`), or candidate emission functions (`emit_*_candidates`), verify all 7 CLAUDE.md checklist points: `get_affordances`, `generate_candidates`, `search_plan`, `BestEffort` action start, `handle_plan_failure`, payload revalidation (`with_payload_override_validator`), and golden test pass.
