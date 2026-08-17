## Purpose

Delegation gets a record with two consenting sides and a stated boundary: managers scaffold, never mint, and never read by virtue of managing.

## ADDED Requirements

### Requirement: Delegation is recorded on both sides and confers no read

An organization SHALL be able to name its managers, and a person SHALL be able to declare `managedBy` naming an organization; evaluation SHALL refuse either side naming what the fleet does not declare.
Neither record SHALL place any key in any audience: managing confers scaffolding, never reading, and the option documentation SHALL state the boundary — the verbs bind the cooperative path, the tree remains the authorization, and key generation stays with the person.

#### Scenario: The records are declarations, not access

- **WHEN** alice is named a manager of acme and bob declares `managedBy` acme
- **THEN** no generated rule and no governed file changes
- **AND** bob's audience is exactly what it was

#### Scenario: Dangling references refuse

- **WHEN** `managers` names a person the fleet does not declare, or `managedBy` names an undeclared organization
- **THEN** evaluation refuses, naming the record and the missing declaration
