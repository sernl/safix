## MODIFIED Requirements

### Requirement: `keygen` belongs to the person and `adduser` to the operator

`keygen` SHALL mint an identity on the machine of the person who will hold it.
`adduser` SHALL scaffold a declaration for a person from a public recipient they supply, and SHALL mint nothing.

#### Scenario: Onboarding requires a public key only

- **WHEN** a person is onboarded
- **THEN** the only material they provide is a public recipient, or the public key it derives from
- **AND** the derivation is reproducible by anyone holding that public key alone

#### Scenario: `adduser` mints nothing

- **WHEN** `adduser` runs
- **THEN** it creates no key, no password material, and no secret value
- **AND** its help text states this

#### Scenario: Only the recipient's shape is checked

- **WHEN** a recipient is supplied
- **THEN** its shape is validated
- **AND** the help text states that whether anyone holds the private half is not knowable from the operator's machine

#### Scenario: A recipient requiring interaction is refused for the primary field

- **WHEN** a recipient that requires a physical interaction to decrypt is supplied as the primary recipient
- **THEN** it is refused
- **AND** the message directs it to the further-recipients field, because activation decrypts without interaction

#### Scenario: A newly declared person holds nothing

- **WHEN** `adduser` completes
- **THEN** the person's declarations are empty, so no audience includes them and no rule is emitted for them yet
- **AND** the output names the sequence that gives them their first secret

#### Scenario: --show prints the operator's own public recipient and mints nothing

- **WHEN** `keygen --show` runs
- **THEN** it prints the public recipient derived from the identity already minted on this machine
- **AND** it mints no identity, appends nothing to the keys file, and writes nothing
- **AND** when no identity exists yet on this machine, it refuses, naming plain `keygen` as the remedy

## ADDED Requirements

### Requirement: sync and audit act across every declared target unless narrowed by one

`sync` and `audit` SHALL each accept an optional target argument naming `clan` or `keepassxc`, acting on every declared relationship across both targets when none is given, and narrowing to the named target's own declared mappings when one is given.
Neither verb SHALL accept a mapping name unless a target has already been named, and neither verb SHALL offer an `all` target: acting on every target is spelled by omitting the target argument, not by naming one that means all of them.

#### Scenario: Bare sync or audit acts on every declared relationship

- **WHEN** `sync` or `audit` runs with no target and no mapping named
- **THEN** it acts on every mapping declared under both `clan` and `keepassxc`

#### Scenario: A target argument narrows to that target's own mappings

- **WHEN** `sync <target>` or `audit <target>` runs, naming `clan` or `keepassxc`
- **THEN** it acts only on mappings declared under that target
- **AND** mapping names may follow the target, narrowing further

#### Scenario: A mapping name with no target is refused rather than guessed

- **WHEN** a mapping name is given to `sync` or `audit` with no target named first
- **THEN** it is refused, naming the two targets a mapping name may follow
- **AND** the reason given is that a mapping id may be declared under both targets' namespaces, so guessing which one a bare name belongs to would be ambiguous exactly when it matters

#### Scenario: There is no all target

- **WHEN** `sync` or `audit`'s accepted target arguments are enumerated
- **THEN** `clan` and `keepassxc` are the only two
- **AND** running with no target is the one spelling for every relationship on every target

## REMOVED Requirements

### Requirement: Absent verbs are recorded rather than left mysterious

**Reason**: `import` and `export` no longer exist as verbs at all, so the "verb of the same name exists with different semantics" framing this requirement's second scenario used for them no longer applies — there is no verb of that name to compare semantics against.
The recorded-absences framing this requirement's first scenario already used for the never-built `upload` verb is the one both retired words now need too, alongside the reservation `import` alone carries.

**Migration**: See the ADDED requirement "Retired and reserved verbs are recorded rather than left mysterious" in this same delta.

## ADDED Requirements

### Requirement: Retired and reserved verbs are recorded rather than left mysterious

The command's help SHALL state which verbs from the tool this package replaces do not exist here, and why.
Where a word this package's own vocabulary once used is retired outright, the help SHALL say so; where one is reserved for a feature not yet built, the help SHALL say that instead, so a reservation is never mistaken for an oversight.

#### Scenario: The recorded absences

- **WHEN** the help text is read
- **THEN** it states that no upload verb exists because activation already delivers what it would
- **AND** it states that no export verb exists, naming clan's own `export` as the bulk plaintext dump safix's design refuses to build on either side of the boundary

#### Scenario: A reserved absence is told apart from a retired one

- **WHEN** the help for `import` is read
- **THEN** it states that `import` is reserved for a future, unbuilt feature — ingesting a value from an external plaintext source one entry at a time — rather than retired outright
- **AND** it states that this is distinct from `export`'s retirement, which is permanent
