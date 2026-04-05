# Implementation Order & Dependency Graph

## Completed Work

Phases 1–5 (E01–E22, FND-01, FND-02, S01–S49) completed.  
Phase 6 has started, and [S50-rights-lattice.md](../archive/specs/S50-rights-lattice.md), [S51-artifact-issuance-goals.md](../archive/specs/S51-artifact-issuance-goals.md), [S52-evidence-aftermath.md](../archive/specs/S52-evidence-aftermath.md), [S53-cognitive-execution-split.md](../archive/specs/S53-cognitive-execution-split.md), S50's remaining golden closeout [S57-golden-gaps-S50.md](../archive/specs/S57-golden-gaps-S50.md), and S51's remaining golden closeout [S58-golden-gaps-S51.md](../archive/specs/S58-golden-gaps-S51.md) are now completed and archived.
See `archive/` for detailed completion records.

---

## Phase 6: Architectural Substrates II

Derived from external ChatGPT architecture assessment (`brainstorming/ai-architecture-assessment.md`) validated against the actual codebase and `docs/FOUNDATIONS.md`. These specs address confirmed architectural gaps in rights/jurisdiction modeling, AI-driven artifact issuance, evidence materialization, cognitive/engine profile separation, claim-based belief provenance, causally grounded blocker invalidation, and context-modulated perception.

### Dependency Graph

```text
S54 (independent — entity belief claims)
S56 (independent — perception exposure)

S54 ──→ S55 (causal blocker invalidation benefits from richer belief claims)
```

Completed in this phase so far:
- **S50**: Rights Lattice — completed and archived
- **S51**: Social Artifact Issuance Goals — completed and archived
- **S52**: Evidence Artifacts and Aftermath Materialization — completed and archived
- **S53**: Cognitive Profile vs Execution Budget — completed and archived
- **S57**: Rights lattice golden gap closeout — completed and archived
- **S58**: Artifact issuance golden gap closeout — completed and archived

The remaining active specs except S55 are independent and can run in parallel. S55 benefits from S54's richer belief substrate but can also work with the current belief layer — the dependency is soft.

### Active Execution Steps

**Wave 1** (parallel, no deps):
- **S54**: Entity Belief Claims — Introduce claim-based substrate beneath `BelievedEntityState`. Multiple conflicting claims per entity coexist with provenance chains. Working-memory summary derived from highest-confidence claims.
- **S56**: Context-Modulated Perception Exposure — Replace static `observation_fidelity` with context-modulated perception accounting for fatigue, action occupancy, and place concealment.

**Wave 2** (after S54):
- **S55**: Causally Grounded Blocker Invalidation — Replace TTL-only blocker expiry with condition-aware invalidation. Blockers clear when evidence of changed conditions arrives (restock, price change, new path). TTL preserved as fallback.

### Phase 6 Gate

- [x] Rights queries distinguish at least: physical possession, ownership, faction authority, office authority, jurisdictional authority
- [ ] AI agents autonomously post at least one bounty or notice in a soak test (T30-equivalent)
- [x] Actions leave perceivable evidence at scene locations; evidence decays over time
- [x] `ReasoningProfile` fully replaced by `CognitiveProfile` + `ExecutionBudget` with no behavioral regression
- [ ] Entity beliefs stored as claims with provenance; `BelievedEntityState` derived from claims
- [ ] At least 3 `BlockingFact` variants clear on evidence rather than TTL alone
- [ ] Observation fidelity modulated by fatigue and place concealment in perception system
- [ ] All existing golden tests pass (no regressions)
- [ ] Conservation invariants hold across all new systems
- [ ] Deterministic replay verified (save/load round-trip with new state)
