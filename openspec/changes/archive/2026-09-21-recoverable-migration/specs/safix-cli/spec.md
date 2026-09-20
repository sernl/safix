# Spec Delta

## ADDED Requirements

### Requirement: `migrate` recovers its own interruptions

`safix migrate <plan.json>` SHALL resume an interrupted run of the same plan, and `safix migrate --abandon <plan.json>` SHALL discard one. The help for `migrate` SHALL state that an interrupted run resumes on rerun, that `--abandon` removes what the journal records, and that a journal from a different plan is refused. It SHALL NOT advise inspecting partial outputs by hand.

#### Scenario: The help states the recovery contract
- **WHEN** the help for `migrate` is read
- **THEN** it names the journal, the rerun-to-resume behaviour and `--abandon`
- **AND** it states that sources are retained on every path, including abandonment

#### Scenario: Abandon takes exactly one plan
- **WHEN** `--abandon` is given without a plan path or with more than one
- **THEN** the verb refuses as a usage error naming the expected form
