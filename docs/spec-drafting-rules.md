# Spec Drafting Rules (FND-01 Section H)

Every future system spec (E09+) MUST include the following analysis sections:

1. **Information-path analysis**: How does each piece of information reach the agents who act on it? Trace the path from source event through perception, witnesses, reports, and belief updates. If information arrives at an agent without a traceable multi-hop path, the design violates Principle 7 (Locality).
2. **Positive-feedback analysis**: Identify every amplifying loop (A increases B, B increases A) in the system. If no loops exist, state so explicitly.
3. **Concrete dampeners**: For each positive-feedback loop, specify the physical world mechanism that limits amplification. Numerical clamps (e.g., `min(value, cap)`) are NOT acceptable dampeners — the dampener must be a physical world process (Principle 8).
4. **Stored state vs. derived read-model list**: Explicitly enumerate what is authoritative stored state (components, relations) and what is a transient derived computation. No derived value may be stored as authoritative state (Principle 3).

See `specs/FND-01-phase1-foundations-alignment.md` Section H and `docs/FOUNDATIONS.md` Principles 3, 7, 8 for rationale.
