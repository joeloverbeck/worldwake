# Spec Drafting Rules (FND-01 Section H)

Every future system spec (E09+) MUST include the following analysis sections:

1. **Information-path analysis**: How does each piece of information reach the agents who act on it? Trace the path from source event through perception, witnesses, reports, and belief updates. If information arrives at an agent without a traceable multi-hop path, the design violates Principle 7 (Locality).
2. **Positive-feedback analysis**: Identify every amplifying loop (A increases B, B increases A) in the system. If no loops exist, state so explicitly.
3. **Concrete dampeners**: For each positive-feedback loop, specify the physical world mechanism that limits amplification. Numerical clamps (e.g., `min(value, cap)`) are NOT acceptable dampeners — the dampener must be a physical world process (Principle 8).
4. **Stored state vs. derived read-model list**: Explicitly enumerate what is authoritative stored state (components, relations) and what is a transient derived computation. No derived value may be stored as authoritative state (Principle 3).

See `specs/FND-01-phase1-foundations-alignment.md` Section H and `docs/FOUNDATIONS.md` Principles 3, 7, 8 for rationale.

## 5. Agent Profile Scenario Contract

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
