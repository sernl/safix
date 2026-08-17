## Purpose

The differential gate is retired. Its subject was a pair of runtimes and there will be one.

## REMOVED Requirements

### Requirement: Both runtimes are driven over one fixture fleet minted at test time

**Reason**: There is one runtime. A fleet driven through two entry points has one entry point left.

**Migration**: The fixture fleet's construction moves into the integration suite's harness, where it is the subject of `behavioural-suite` rather than of a comparison.

### Requirement: Standard output is compared without normalization

**Reason**: Comparison requires a comparand.

**Migration**: Each subcommand's standard output is asserted against a literal in the integration test that covers it, per the parity table in this change's design.

### Requirement: Standard error is compared without normalization, under a reporter that exists in the code

**Reason**: Comparison requires a comparand.

**Migration**: The refusal prose is already snapshot-tested in `crates/safix/src/snapshots/`, per reporter and per refusal, and the integration suite asserts which refusal a condition produces.

### Requirement: Reporter selection changes rendering and nothing else

**Reason**: This was a claim about the rust runtime rendered comparatively; it does not depend on a second runtime.

**Migration**: Retained. It is asserted by the existing paired plain and graphical snapshots, which cover the same refusal set under both reporters.

### Requirement: Exit codes are compared exactly

**Reason**: Comparison requires a comparand.

**Migration**: Each refusal's exit status is asserted against a literal in the integration suite, alongside the refusal code the error model already requires.

### Requirement: Repository effects are compared over a canonical projection computed by one program

**Reason**: The projection existed so two runtimes' effects could be diffed. One runtime's effect is asserted directly.

**Migration**: The integration tests assert the paths in the commit, the commit message, and the ciphertext lines that did and did not move — the same properties the projection canonicalized, stated as expectations.

### Requirement: Neither run leaves plaintext behind

**Reason**: The requirement is retained in substance and re-homed, not dropped. Only its two-runtime phrasing is removed.

**Migration**: Becomes the residue and pipe claims in `behavioural-suite`, asserted of the one runtime, and the syscall proof is retained under `safix-syscall-proof` with its linux-only condition unchanged.

### Requirement: Each channel's comparison is held by a severity drill

**Reason**: The drills are retained in substance; their phrasing as holders of a *comparison* is what is removed.

**Migration**: Becomes the drill requirement in `behavioural-suite`, mutating the one runtime per channel and failing unless each mutation is caught. This is the last thing retired rather than the first, because without it the successor suite is a set of assertions nobody has shown can fail.

### Requirement: Retirement is per subcommand, and nothing ships on a partial pass

**Reason**: The gate it governed is discharged. `packages.safix` is the rust binary and has been since the gate went green across every subcommand.

**Migration**: The ordering discipline it encoded survives as the parity obligation in `behavioural-suite`: a claim may not be deleted from one place before it exists in another, itemized per mode rather than per subcommand.
