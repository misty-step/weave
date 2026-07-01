# Define the versioned contracts between the pieces

Priority: P1 · Status: pending · Estimate: L

## Goal
Every seam in the loop (powder→BB events, BB→cerberus dispatch, cerberus→crucible packets, canary→BB triage, crucible→threshold specs) is a versioned, schema-checked contract with a consumer test on each side — no coincidental JSON.

## Oracle
- [ ] Each seam has a schema with `schema_version`, owned in the producer repo, consumed via pin.
- [ ] A breaking field rename on any side fails a CI contract test, not a runtime.

## Notes
Groom sweep evidence: only canary→BB carries schema_version today; "126" phantom-ref incident shows tracking drift. BB must resume releases so consumers can pin.
