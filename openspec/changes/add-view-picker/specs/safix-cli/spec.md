## ADDED Requirements

### Requirement: Browsing is a verb, and the choosing half is shared with editing

The command SHALL provide a `view [<user>] [<name>]` subcommand which, given a name, decrypts that one entry and writes it to the terminal, and, given no name, offers every entry the named user holds for selection before decrypting the chosen one.
`edit` SHALL accept the same nameless form, offering the same selection over the entries that user holds excluding every public placement, and SHALL determine the operator's editor before the selection opens.
Both verbs SHALL reach the selection through one shared implementation, so a candidate list, a match order, or a refusal cannot differ between them.

#### Scenario: `view` names an entry and gets it on the terminal

- **WHEN** an operator runs `view` with an entry's name
- **THEN** the value is decrypted and written to the terminal
- **AND** nothing is offered for selection, and no terminal is required beyond the one the value is written to

#### Scenario: `view` with no name offers what the user holds

- **WHEN** an operator runs `view` with no name
- **THEN** every entry that user holds is offered, showing the same facts `list` reports for it
- **AND** choosing one decrypts exactly that entry, as though it had been named

#### Scenario: `get` is the pipe and `view` is the terminal

- **WHEN** the two read verbs' help is read
- **THEN** each states that `get` writes the value to standard output and is what a pipeline calls
- **AND** each states that `view` writes it to the terminal and requires one only when it must offer a choice
- **AND** `get`'s own behaviour, output stream and exit code are unchanged by the existence of `view`

#### Scenario: A lone argument is a user when it names one

- **WHEN** `view` is given exactly one argument
- **THEN** it is the user when the declarations declare a user by that name, leaving the entry to be chosen
- **AND** it is otherwise the entry's name, under the default user
- **AND** an entry whose name is also a person's is reachable by naming both, and the help says so

#### Scenario: The selection is shared rather than duplicated

- **WHEN** the implementation of choosing is read
- **THEN** one function serves both verbs
- **AND** the difference between them is the candidate set alone: editing excludes public placements, because a public output is refused for editing and a refusal reachable by selection is one the choice should never have offered

### Requirement: Choosing without a terminal, over nothing, or by leaving are three distinct refusals

Each SHALL be its own refusal with its own code and its own prose, and none SHALL reuse the refusal enrollment raises for a missing terminal.

#### Scenario: No terminal to choose on

- **WHEN** a selection is required and no terminal can be opened, or its attributes cannot be read
- **THEN** the run is refused before anything is decrypted
- **AND** the refusal names both remedies: name the entry, or list what the user holds
- **AND** its prose is the picker's own, not the prose of the refusal a hardware enrollment raises for the same missing terminal

#### Scenario: Nothing to choose from

- **WHEN** a selection is required and the named user holds no entry
- **THEN** the run is refused naming that user
- **AND** the refusal is distinct from the terminal one, because holding nothing is a state of the declarations rather than of the session

#### Scenario: Leaving without choosing

- **WHEN** the operator leaves the selection without choosing one
- **THEN** the run ends non-zero having written no value and decrypted nothing it retained
- **AND** the terminal is left in the state it was found in
- **AND** the refusal is distinct from having nothing to choose from, because one is a choice and the other is a state

#### Scenario: A missing editor is refused before a choice is offered

- **WHEN** `edit` is run with no name and neither editor variable is set
- **THEN** the run is refused before any selection is offered and before anything is decrypted
- **AND** the reason is recorded: a refusal after a person has browsed is one that wasted their time and decrypted values for nothing

### Requirement: The preview shows one value at a time and leaves nothing behind

When a selection is offered, the runtime SHALL decrypt the highlighted entry and show a bounded rendering of its value, holding at most one decrypted value at any moment, and SHALL offer an option that suppresses the preview entirely.

#### Scenario: One value at a time

- **WHEN** the highlight moves from one entry to another
- **THEN** the previously previewed value is dropped and overwritten before the next is decrypted
- **AND** no decrypted value is retained across the move, so nothing accumulates over a long browse

#### Scenario: A decrypt happens after the operator stops moving

- **WHEN** the operator moves the highlight through several entries without pausing
- **THEN** the entries passed through are not decrypted
- **AND** the entry the operator came to rest on is

#### Scenario: The preview is bounded and sanitized

- **WHEN** a value longer or wider than the region, or containing control bytes, is previewed
- **THEN** the rendering is bounded to the region, control bytes are shown as visible placeholders, and the truncation is marked
- **AND** a value that is not valid text is described by its size rather than rendered as mojibake

#### Scenario: The preview never becomes a value the rest of the program can hold

- **WHEN** the rendering path is read
- **THEN** the decrypted value reaches the terminal writer directly, and no ordinary string or byte slice of it is produced
- **AND** the secret type's existing prohibitions on debugging, displaying and serializing a value are unchanged

#### Scenario: The preview is not written where a redirected stream would catch it

- **WHEN** a run's standard output or standard error is redirected
- **THEN** nothing the selection drew appears in either
- **AND** the chosen value, on the reading verb, appears on the terminal after the drawing region has been left, so what remains on screen is the value asked for and nothing used to find it

#### Scenario: The preview can be suppressed

- **WHEN** the suppressing option is given to either verb
- **THEN** entries are offered with the facts `list` reports and no value is decrypted until one is chosen
- **AND** the option's documentation states what it is for: a shared screen, a recording, or a session whose scrollback the operator does not control

#### Scenario: A failed preview is not a failed selection

- **WHEN** decrypting the highlighted entry fails
- **THEN** the preview region says so and the selection remains usable
- **AND** the run is not refused, because failing to show one value says nothing about the operator's ability to choose another

## MODIFIED Requirements

### Requirement: One command covers the lifecycle, by name and never by file

The package SHALL provide a single command named `safix` with the subcommands `set`, `edit`, `get`, `view`, `list`, `generate`, `check`, `fix`, `audit`, `sync`, `keygen`, `adduser`, `enroll`, `group`, and `upload`.
Every subcommand that addresses a secret SHALL address it by name, and SHALL NOT require the operator to name a file.
Where a subcommand offers the operator a choice among the entries a user holds, that choice SHALL be among names, and choosing SHALL be equivalent to having named the chosen entry.

#### Scenario: Addressing a secret

- **WHEN** an operator sets, reads, or generates a value
- **THEN** they name the secret
- **AND** the file and the key within it are resolved from the declarations

#### Scenario: Choosing is a way of naming

- **WHEN** an operator chooses an entry from those offered rather than typing its name
- **THEN** the run proceeds exactly as though the chosen name had been given as an argument
- **AND** no file is named at any point

#### Scenario: The subcommand set is closed

- **WHEN** an unrecognised subcommand is given
- **THEN** the command fails naming the subcommands it accepts
- **AND** the sentence is derived from the table the command dispatches on, so it can neither omit a subcommand that exists nor name one that does not

#### Scenario: Runtime dependencies are pinned into the command

- **WHEN** the command runs
- **THEN** the tools it invokes come from its own closure
- **AND** none of them is inherited from the caller's environment

### Requirement: A global entry file replaces the flake for evaluation

The command SHALL accept a global `--entry <file>` option and a `SAFIX_ENTRY` environment variable naming a plain nix file.
When either is set, every nix evaluation SHALL target that file — `nix eval --file <file> <attribute>` — instead of `<root>#<attribute>`, using the same attribute spellings the flake path uses.
`--entry` SHALL take precedence over `SAFIX_ENTRY` when both are given.
Fifteen of the sixteen subcommands SHALL behave identically under `--entry` as against a flake; `generate` is the documented exception.

#### Scenario: The attribute spellings are unchanged

- **WHEN** any of the twelve attributes the runtime evaluates is read under `--entry`
- **THEN** the string naming it is identical to the one read against a flake
- **AND** only how the target is built — `--file <entry> <attribute>` rather than `<root>#<attribute>` — differs

#### Scenario: Nested payload shapes evaluate identically under either path

- **WHEN** an entry file's declarations include at least one generator, one bridge mapping, and one keepassxc mapping
- **THEN** the JSON `--entry` evaluation emits for `generatorPlan`, `bridge`, and `keepassxc` deserializes without an unknown-field error against the same `#[serde(deny_unknown_fields)]` structs the flake path deserializes against
- **AND** the same fixture, evaluated against an equivalent flake, produces byte-identical JSON apart from any store path a generator's tooling reference embeds

#### Scenario: A verb unaffected by --entry works exactly as it does against a flake

- **WHEN** `list`, `get`, `view`, `set`, `check`, `fix`, `keygen`, `adduser`, `enroll`, `group`, `import`, `export`, `audit`, `sync`, `upload`, or `edit` is run under `--entry`
- **THEN** its behaviour, output, and exit code match the same invocation against an equivalent flake
- **AND** neither git operations nor workspace root discovery change, because `--entry` governs only how declarations are evaluated

#### Scenario: The workspace root is still found by git

- **WHEN** `--entry` or `SAFIX_ENTRY` is set
- **THEN** the repository root a run stages and commits into is still the one git reports for the current directory
- **AND** the entry file need not be inside that repository, though every declaration it makes about paths under `root` still resolves relative to whatever `root` its own expression names
