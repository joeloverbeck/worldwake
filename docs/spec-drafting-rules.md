# Spec Drafting Rules (FND-01 Section H)

Every future system spec (E09+) MUST include the following analysis sections:

1. **Information-path analysis**: How does each piece of information reach the agents who act on it? Trace the path from source event through perception, witnesses, reports, and belief updates. If information arrives at an agent without a traceable multi-hop path, the design violates Principle 7 (Locality).
2. **Positive-feedback analysis**: Identify every amplifying loop (A increases B, B increases A) in the system. If no loops exist, state so explicitly.
3. **Concrete dampeners**: For each positive-feedback loop, specify the physical world mechanism that limits amplification. Numerical clamps (e.g., `min(value, cap)`) are NOT acceptable dampeners — the dampener must be a physical world process (Principle 8).
4. **Stored state vs. derived read-model list**: Explicitly enumerate what is authoritative stored state (components, relations) and what is a transient derived computation. No derived value may be stored as authoritative state (Principle 3).
5. **Planner-formalism analysis**: For any planner-facing feature, state whether the behavior is plain GOAP/affordance search, HTN method decomposition over existing affordances, both with fallback, or method-required. HTN method registration is justified only by a reusable pursuit pattern that materially constrains search: multi-stage lawful decomposition, information gathering before action, role- or motive-specific strategy, repeated planner budget exhaustion, utility thrash between equivalent branches, or method-specific failure attribution. A method-required goal must name the explicit schema contract and explain why flat GOAP fallback would be semantically invalid.

See `specs/FND-01-phase1-foundations-alignment.md` Section H and `docs/FOUNDATIONS.md` Principles 3, 7, 8, and 20 for rationale.

## Belief-View Accessor Source-Class Rule

Any spec that adds or changes a planner- or player-visible belief-view accessor
must declare the accessor's source class before implementation:

1. **Self**: facts about the observing actor.
2. **Same-tick local physical observation**: directly perceivable physical facts
   about entities at the actor's effective place.
3. **Direct possession**: observable facts about entities directly possessed by
   the actor.
4. **Belief-backed**: remote, delayed, social, relational, or inferred facts that
   require a belief, memory, testimony, report, record, or other explicit
   evidence carrier.
5. **Public topology**: intentionally public place-graph facts that do not imply
   remote entity, occupant, or content visibility.

The spec must also state the stale or unknown behavior for each accessor. If no
lawful source exists, the accessor must return `None`, empty, or `false` rather
than read current authoritative world state on behalf of the agent.

Social and relational facts, including ownership, rights, control,
jurisdiction, seller/controller identity, testimony, source credibility, and
institutional claims, are belief-gated even when the subject is co-located.
Co-location alone exposes only directly perceivable physical facts under
FND-14A.

## HTN Method Drafting Checklist

Any spec that adds or materially changes an HTN method must include a checklist
covering:

1. **Reusable pursuit pattern**: name the repeated domain pursuit pattern the
   method encodes, and why it belongs in method decomposition rather than in a
   one-off scenario, goal special case, or tactical operator.
2. **Why flat GOAP is insufficient**: explain the concrete search-control need
   that plain GOAP/affordance search does not satisfy by itself, such as
   multi-stage lawful decomposition, information gathering before action,
   role- or motive-specific strategy, repeated planner budget exhaustion,
   utility thrash between equivalent branches, or method-specific failure
   attribution.
3. **Fallback policy**: state whether flat-GOAP fallback remains allowed,
   forbidden, or allowed only after a traced method failure. A method-required
   goal is invalid unless the schema contract proves that flat fallback would
   satisfy the wrong semantic condition.
4. **Information reads**: list every belief, memory, record, observation, goal
   evidence field, motive source, and profile value the method selector or stage
   builder reads. Each read must be belief-backed or a lawful same-tick local
   observation under `docs/FOUNDATIONS.md`.
5. **Enforced declarations only**: any field or precondition expressing required
   artifacts, claims, records, roles, failure modes, locations, or capabilities
   must have a live selector, planner, validation, trace, or runtime consumer
   when it is declared. Do not add schema fields that merely document intended
   semantics.
6. **Proof surface**: name the focused and golden tests that prove method
   selection, method rejection with failing precondition, fallback behavior, and
   method trace contents. If rejection or fallback is impossible for the method,
   explain which enforced schema contract makes it impossible.

## Agent Profile Scenario Contract

Every spec that adds a new ECS component registered on `EntityKind::Agent` that
affects agent behavior must:

1. Classify the component as **universal** (every agent needs it to function as
   a reasoning, perceiving, socially-participating agent) or **role-specific**
   (only relevant for agents in specific roles).
2. Add the component to `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs`.
   If the component contains `EntityId` references, create a `*Def` wrapper type
   with string names following the `MerchandiseProfileDef` / `PatrolRouteDef`
   pattern.
3. Add the `set_component_*` call in `spawn_agent()` in
   `crates/worldwake-cli/src/scenario/mod.rs`:
   - Universal: `unwrap_or_default()` and always applied.
   - Role-specific: conditional `if let Some(...)` and applied only if present in
     RON.
4. Universal profiles must have a `Default` impl.
5. Runtime access to universal profiles on known agents uses `expect()`, not
   silent fallback.

Components that are purely runtime-generated state such as `ActiveGoal`,
`IntentionFrame`, and `WoundList` are exempt because they emerge from
simulation, not configuration.

Any new ECS component that affects agent behavior must be exercisable through the
scenario system. If a component changes what an agent can do, perceive, decide,
or communicate, a scenario author must be able to configure it. Silent absence
of behavioral components is a bug, not a feature.
