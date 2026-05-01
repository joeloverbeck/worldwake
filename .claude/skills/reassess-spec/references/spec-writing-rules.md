# Writing the Updated Spec (Step 7)

After all findings are resolved and approved:

## Pre-Apply Verification

Run targeted checks to confirm each finding still holds (e.g., grep confirming symbol presence/absence, count validation). Classify any mismatch between check and finding into one of two tiers:

- **Recommendation-changing mismatch**: the check invalidates the finding's *recommendation* — the approved fix no longer applies, the target text/symbol has moved, or a different fix is now warranted. Re-present the corrected finding to the user and wait for confirmation before applying any edit **for that finding**. Do not silently substitute different changes. If the correction is a pure retraction (no substitute fix is warranted — the finding itself is withdrawn), note the retraction transparently and proceed with the remaining approved findings; fresh re-approval is only required when a different fix is being substituted in place of the retracted one.
- **Evidence-refining mismatch**: the check refines the finding's *supporting evidence* but leaves the recommendation unchanged (e.g., a symbol the finding claimed was absent turns out to exist at a different location, and the recommendation already targets the actual location used by the consumer). Note the refinement inline in the Result column of the pre-apply table and proceed. The user sees the refinement in the emitted table, so this is not silent modification.

When in doubt, treat the mismatch as recommendation-changing and re-present — it is cheaper to ask than to apply the wrong fix.

## Apply Changes

- Incorporate corrections from the user's plan approval or question responses.
- Preserve existing structure and voice. Change only what was agreed upon.
- When changes are numerous and spread throughout, a full Write is acceptable. Prefer Edit for <=3 localized changes; prefer full Write when changes span >50% of deliverables or when insertions cause cascading renumbering.
- If inserting new deliverables, renumber all subsequent deliverables and update any intra-spec cross-references to deliverable numbers.
- When removing deliverables, grep the spec for all references to the removed deliverable number (e.g., "D4") and update or remove them. Check: Behavioral Guarantees, Verification steps, FOUNDATIONS Alignment table, Section H, Summary, and any cross-deliverable references.
- When materially modifying a deliverable's mechanism, name, or surface area (without changing its number), grep the spec for the deliverable's old key concepts (function names, trait names, variant names that the modification eliminates) AND scan these sections for restatements that need updating: Summary, Dependencies, FOUNDATIONS Alignment, Cross-System Interactions, Outcome, and Decomposition Hint. Cross-section restatements drift silently because the deliverable's number is unchanged — only its content shifted.
- **New deliverable vs. amendment**: When a finding introduces substantial new logic (new mechanism, new type, new event tag), consider a new numbered deliverable rather than expanding an existing one. Criteria: (a) distinct implementation site, (b) independently implementable and testable, (c) would make existing deliverable unwieldy if inlined.
- If new deliverables introduce actions, components, or system functions, update Section H for P30 compliance. Also update Section H's information-path and stored-state entries when reassessment changes the causal mechanism.
- **Late-discovered findings**: If writing reveals minor factual errors not covered by the plan (e.g., incorrect crate names in prose sections, typos in cross-references), fix them and note in Step 8 as "Also fixed:" items. If the new finding would constitute a HIGH or CRITICAL issue, re-present to the user before applying.
- If the user requests corrections after reviewing, apply them and re-present affected sections.

## Post-Apply Confirmation

Grep the updated spec for: (1) eliminated stale references (should return zero matches *except* where the spec deliberately documents the absence — e.g., a Non-Goals entry or FND-28 row explaining "X is not introduced because Y" — in which case verify the residual matches are intentional deferral statements rather than missed eliminations), (2) corrected references (should return expected matches), (3) file path references in newly added deliverables (should resolve to existing files), (4) if the spec contains a Decomposition Hint section, grep it for the deliverable surfaces (function names, trait names, key types) named in each ticket bundle and confirm they match the current D-section text — mismatches mean the ticket boundaries describe stale concepts and will mislead `/spec-to-tickets` downstream, and (5) when deliverables were inserted, removed, or renumbered, grep the spec for the deliverable-key pattern (e.g., `\bD[0-9]+\b` for D-numbered, `\bP[0-9]+\b` for phase-numbered) and verify each cross-reference points to the intended deliverable in the renumbered scheme — full Writes commonly leave one or two stale `(see D5)`-style references behind that the textual rewrite missed. Record results for Step 8.
