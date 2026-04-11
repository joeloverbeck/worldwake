# Writing the Updated Spec (Step 7)

After all findings are resolved and approved:

## Pre-Apply Verification

Run targeted checks to confirm each finding still holds (e.g., grep confirming symbol presence/absence, count validation). If a finding is invalidated, re-present the corrected finding before applying. Do not silently substitute different changes.

## Apply Changes

- Incorporate corrections from the user's plan approval or question responses.
- Preserve existing structure and voice. Change only what was agreed upon.
- When changes are numerous and spread throughout, a full Write is acceptable. Prefer Edit for <=3 localized changes; prefer full Write when changes span >50% of deliverables or when insertions cause cascading renumbering.
- If inserting new deliverables, renumber all subsequent deliverables and update any intra-spec cross-references to deliverable numbers.
- When removing deliverables, grep the spec for all references to the removed deliverable number (e.g., "D4") and update or remove them. Check: Behavioral Guarantees, Verification steps, FOUNDATIONS Alignment table, Section H, Summary, and any cross-deliverable references.
- **New deliverable vs. amendment**: When a finding introduces substantial new logic (new mechanism, new type, new event tag), consider a new numbered deliverable rather than expanding an existing one. Criteria: (a) distinct implementation site, (b) independently implementable and testable, (c) would make existing deliverable unwieldy if inlined.
- If new deliverables introduce actions, components, or system functions, update Section H for P30 compliance. Also update Section H's information-path and stored-state entries when reassessment changes the causal mechanism.
- If the user requests corrections after reviewing, apply them and re-present affected sections.

## Post-Apply Confirmation

Grep the updated spec for: (1) eliminated stale references (should return zero matches), and (2) corrected references (should return expected matches). Record results for Step 8.
