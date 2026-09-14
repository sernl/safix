## MODIFIED Requirements

### Requirement: Editing is its own verb

The command SHALL provide an `edit` verb addressing an entry by name or by choosing one from those the user holds, and SHALL NOT provide editor input as an option on the verb that writes a value from a stream.
When no name is given, the verb SHALL offer the entries that user holds excluding every public placement, and SHALL determine the operator's editor before the choice is offered.

#### Scenario: The verb addresses an entry by name

- **WHEN** an operator edits a value
- **THEN** they name the entry
- **AND** they do not name a file

#### Scenario: The verb addresses an entry by choosing one

- **WHEN** an operator edits with no name given
- **THEN** the entries that user holds are offered and choosing one proceeds exactly as naming it would
- **AND** they do not name a file in this form either

#### Scenario: What is offered is what could be edited

- **WHEN** the entries offered for editing are enumerated
- **THEN** no public placement is among them
- **AND** the reason is recorded: a public output is refused for editing because the generator declaring it is what mints it, and offering a choice that would then be refused is a worse interface than not offering it

#### Scenario: The editor is settled before the choice is offered

- **WHEN** the nameless form is run and neither editor variable is set
- **THEN** the run is refused before any entry is offered and before anything is decrypted
- **AND** the ordering is the same one the named form already holds — the editor is known before anything is decrypted or staged — so no operator browses, previews and chooses only to learn there was never an editor

#### Scenario: The stream-writing verb is unchanged

- **WHEN** the stream-writing verb's options are enumerated
- **THEN** none of them invokes an editor
- **AND** its requirement that the value arrives on a stream continues to hold without exception

#### Scenario: The reason for a verb rather than an option is recorded

- **WHEN** the decision is documented
- **THEN** it states that the two verbs have different custody profiles — one reads the existing value and hands it to a program the runtime does not control, and the other does neither
- **AND** it states that an option would make custody a function of a flag
