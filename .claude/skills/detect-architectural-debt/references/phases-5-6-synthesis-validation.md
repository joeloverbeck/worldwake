# Phases 5-6: Synthesis and Validation

## Phase 5: SYNTHESIZE — Cross-Lens Reinforcement

This phase is the core value of the unified skill. Compare findings from both lenses:

| Lens A Finding | Lens B Finding | Result |
|---------------|---------------|--------|
| Cluster with signals | Fracture in overlapping modules | **Merged finding** — confidence elevated automatically |
| Cluster with signals | No fracture | **Contained scatter** — lower severity (Medium or Low) |
| No cluster | Fracture detected | **Boundary-level fracture** — severity by fracture type |
| Single signal from either lens | — | **Needs Investigation** |

For each validated finding (from either lens or merged), produce a candidate abstraction:

- **title**: Descriptive name (e.g., "Goal Dispatch Protocol")
- **lens_source**: Lens A / Lens B / Merged (both lenses)
- **kind**: One of: Protocol | Authority boundary | Bounded context | Projection owner | Capability ledger | Workflow coordinator | Translation boundary | Lifecycle carrier
- **scope**: Which crates/modules it spans
- **owned_truth**: What state or invariant this abstraction would own (the single most important field — if you can't name this, the candidate is not ready)
- **invariants**: What must always be true when this abstraction is correctly implemented
- **owner_boundary**: Which crate/module should own it
- **modules_affected**: Existing modules that would be absorbed, constrained, or simplified
- **scenario_families_explained**: Which scenario families from Phase 2 this candidate accounts for
- **expected_simplification**: What gets simpler — fewer writers, fewer repeated predicates, fewer cross-boundary transitions, fewer co-change edges, clearer ownership
- **severity**: Critical / High / Medium / Low (see Severity Ranking below)
- **confidence**: High / Medium / Low (evidence certainty)
- **counter_evidence**: What would falsify this hypothesis. **MANDATORY** — every candidate must have this field populated.

### Severity Ranking

| Level | Definition |
|-------|-----------|
| **Critical** | Multiple subsystems write the same truth with no single owner. Fixing a bug requires synchronized cross-boundary changes. |
| **High** | Lifecycle transitions scattered across subsystem boundaries, or protocol split so "what"/"when"/"whether" live in different modules. |
| **Medium** | Intra-subsystem scatter with strong structural signals. Contained but substantial. |
| **Low** | Single-subsystem scatter with moderate signals, or boundary-level fracture with limited blast radius. |

Ranking rules (in priority order):
1. Cross-lens reinforced > single-lens at same signal strength
2. More scenario families explained > fewer
3. Temporal coupling evidence present > absent
4. More affected modules > fewer (tiebreaker within same severity)

## Phase 6: VALIDATE — Survival Criteria + FOUNDATIONS Alignment

**Prerequisite**: Read `docs/FOUNDATIONS.md` in full before this phase (skip if already read in this session).

Apply two validation filters, in this order:

**Filter 1 — Survival criteria.** Drop any candidate that fails ANY of these:

1. It explains at least two tests or one whole scenario family
2. It reduces at least one real architectural cost (not just "cleaner")
3. It can name the owned truth
4. It can name the rightful owner boundary
5. It does not merely wrap existing code with a facade

**Filter 2 — FOUNDATIONS alignment.** For surviving candidates only, check against `docs/FOUNDATIONS.md`.

### Always-check principles (every finding):

| Principle | Check |
|-----------|-------|
| **P1** — Maximal Emergence Through Local Causality | Does the authority confusion prevent emergent composition? Would a first-class type enable new system interactions? |
| **P3** — Concrete State Over Abstract Scores | Is the concept represented as an abstract score or flag when it should be concrete state with identity? |
| **P7** — Locality of Motion, Interaction, and Communication | Does the scattering force modules to query non-local information to derive what should be locally available? |
| **P26** — Systems Interact Through State, Not Through Each Other | Are systems calling each other's functions instead of reading shared state? Does the scattered logic create hidden coupling? |
| **P27** — Derived Summaries Are Caches, Never Truth | Is derived state being recomputed from scratch instead of stored and maintained? |
| **P28** — No Backward Compatibility in Live Authority Paths | Are there shims, deprecated wrappers, or compatibility layers masking the need for a proper abstraction? |

### Auto-selected principles (2-3 additional, based on domain):

- **Combat / needs / metabolism** -> P8 (action cost/occupancy), P11 (feedback dampeners)
- **Belief / knowledge / perception** -> P14 (world state is not belief state), P15 (knowledge locality), P16 (ignorance is first-class)
- **Agent decision / goal / planning** -> P19 (agent symmetry), P20 (resource-bounded reasoning), P21 (revisable commitments)
- **Institutional / office / social** -> P23 (roles/offices as world state), P24 (ownership/custody/access), P25/P25A (social artifacts)
- **Production / trade / economy** -> P4 (persistent identity and explicit transfer), P5 (carriers of consequence)

For each relevant principle, note whether the candidate aligns, strains, or conflicts. Flag conflicts prominently — a candidate that violates FOUNDATIONS needs redesign before it becomes a spec.

**This ordering matters.** Recovery first, judgement second. Do not let FOUNDATIONS bias the fracture detection — detect what IS, then evaluate what SHOULD BE.
