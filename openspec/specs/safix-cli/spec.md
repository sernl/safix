# safix-cli Specification

## Purpose

The secret lifecycle command: by-name value operations, explicit migration and identity artifacts, byte-preserving transport, recipient-policy boundaries and stable refusal codes.

## Requirements

### Requirement: One command covers the lifecycle, by name and never by file

The safix command SHALL retain its existing lifecycle verbs and add migrate and identity custody operations. Ordinary secret operations SHALL continue to address declaration names. Installation, explicit migration plans and identity backup/restore SHALL address their input artifacts rather than inventing declaration names for them.

#### Scenario: The explicit migration boundary
- **WHEN** an operator invokes migration with a versioned plan
- **THEN** that plan names the source, destination, recipient and deployment contracts to verify
- **AND** ordinary set and get operations continue resolving files from names

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
- **AND** that list includes every supported verb, including identity and migrate

#### Scenario: Runtime dependencies are pinned into the command

- **WHEN** the command runs
- **THEN** its default tool paths come from its own closure
- **AND** explicit upstream identity and GnuPG executable settings retain their documented precedence

#### Scenario: Installation names its manifest rather than a secret

- **WHEN** the help for `install` is read
- **THEN** it states that the manifest path is its argument because a manifest is a build product, not a declaration, and nothing in the declarations names it
- **AND** it states that the verb resolves no secret by name, so the by-name rule has nothing to apply to

### Requirement: Values move through pipes wherever a pipe remains possible

Secret and private-key payloads SHALL cross cryptographic subprocess boundaries through pipes, never command arguments or environment variables. Plaintext files SHALL be confined to protected generator/editor staging, private runtime installation state and explicitly selected owner-protected identity custody locations outside repositories and the Nix store.

#### Scenario: The stream-writing and reading verbs are unchanged

- **WHEN** a value is written from standard input or read to standard output
- **THEN** it travels a pipe end to end

#### Scenario: The encrypting backend is still driven by pipes

- **WHEN** the encrypting backend is invoked for any operation
- **THEN** the value reaches it on a pipe
- **AND** no invocation names a value in its arguments or environment

#### Scenario: The exception is bounded and named

- **WHEN** the exception to this requirement is read
- **THEN** it names generator and editor staging, runtime installation and protected local identity custody
- **AND** migration staging and published recovery artifacts contain ciphertext, not plaintext

#### Scenario: The change from the earlier absolute is stated

- **WHEN** this requirement is compared against the one it replaces
- **THEN** the difference is stated rather than presented as a clarification
- **AND** the reason is recorded: generators and editors need seekable files, installed services consume runtime paths, and cryptographic tools retain identities in protected local custody

### Requirement: The content half cannot alter the policy

Ordinary value-writing commands SHALL use only the declared audience and SHALL refuse inspectable SOPS recipient drift before mutation. They SHALL NOT add recipients to declarations. Raw age writes SHALL use the declared audience without claiming to verify the inaccessible previous recipient roster; explicit migration MAY name a different target audience and SHALL independently verify it.

#### Scenario: Setting a value grants nothing

- **WHEN** a value is written
- **THEN** the recipients used are those the file's own metadata or the committed policy already declares
- **AND** no run adds a recipient to the declared audience

#### Scenario: Policy changes go through one subcommand

- **WHEN** the recipient policy must change
- **THEN** the change comes from the declarations, applied by the reconciling subcommand
- **AND** that subcommand regenerates the policy before re-wrapping, never the other way round

### Requirement: `check` diffs intent against reality and `fix` reconciles what is reconcilable

`check` SHALL be read-only and SHALL report each finding with the command that resolves it.
`fix` SHALL regenerate the policy and re-wrap the governed files to the audiences declared, and SHALL name what needs a human rather than attempting it.

#### Scenario: A finding carries its remedy

- **WHEN** `check` reports a finding
- **THEN** the report includes the command that fixes it
- **AND** the report distinguishes what `fix` can reconcile from what requires a keyholder

#### Scenario: `check` needs no identity it does not have

- **WHEN** `check` examines a file the operator holds no identity for
- **THEN** it answers from the document's structure rather than its contents
- **AND** it does not attempt decryption

#### Scenario: `fix` is not revocation

- **WHEN** `fix` re-wraps files after an audience narrows
- **THEN** its output states that this aligns ciphertext with policy
- **AND** states that revoking means minting a new value, naming the command that does so

#### Scenario: `list` shows what a person holds

- **WHEN** `list` runs
- **THEN** it reports each secret's origin, the file it resolves to, whether it has a generator, and whether it is shared

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

### Requirement: check reports a value minted under a definition that has changed

`safix check` SHALL report a finding for every generated value whose recorded definition digest no longer matches the current declaration, naming the entry and both remedies: regenerate the value, or revert the declaration edit.
A value with no record predates the record and SHALL NOT be a finding.

#### Scenario: An edited definition is reported

- **WHEN** a generator's declaration changes after its value was minted
- **THEN** `check` reports the entry as minted under a definition that no longer exists
- **AND** the finding names regeneration and reverting the edit as the remedies
- **AND** the finding carries no value

#### Scenario: Regeneration clears the finding

- **WHEN** the value is regenerated under the current declaration
- **THEN** the finding is gone on the next `check`

#### Scenario: A capability grant is a definition change

- **WHEN** a generator's network grant flips after its value was minted
- **THEN** `check` reports the entry as minted under a definition that no longer exists
- **AND** the record stays put until a regeneration adopts the grant or the flip is reverted

#### Scenario: What is out of scope stays quiet

- **WHEN** an entry has no generator, or a generated value has no record
- **THEN** no drift finding exists for it

### Requirement: set reads a stream when one is offered

`safix set` SHALL read the value from standard input when standard input is not a terminal, storing the bytes exactly as received, with no prompt and no confirmation.
When standard input is a terminal, the hidden double prompt SHALL remain the behaviour.

#### Scenario: A piped value is stored as its own bytes

- **WHEN** a value is piped into `safix set <name>`
- **THEN** the stored bytes are exactly the piped bytes
- **AND** nothing prompts and nothing asks for confirmation
- **AND** the value reaches no argument vector and no environment variable

#### Scenario: Empty input keeps its refusal

- **WHEN** the pipe yields no bytes
- **THEN** the write is refused as an empty value, exactly as an empty prompt is

#### Scenario: The terminal path is unchanged

- **WHEN** standard input is a terminal
- **THEN** the hidden prompt, the confirmation, and the single-line contract hold as they do today

### Requirement: Scaffolding verbs honour the delegation records

When the target person declares `managedBy`, `enroll` and the record-editing half of onboarding SHALL refuse an acting identity that is not among that organization's managers, reading the acting identity from the same git identity the resulting commit carries, and a permitted scaffold's commit SHALL record the organization context.
When the target declares no `managedBy`, the verbs SHALL behave exactly as before.

#### Scenario: A manager scaffolds and the record says so

- **WHEN** alice, a manager of acme, enrolls a card for bob, who declares `managedBy` acme
- **THEN** the scaffold proceeds and its commit records the acme context

#### Scenario: An outsider is refused before anything is edited

- **WHEN** mallory, no manager of acme, attempts the same scaffold
- **THEN** the verb refuses before editing any file, naming the organization and where its managers are declared

#### Scenario: Unmanaged people are untouched by the feature

- **WHEN** the target person declares no `managedBy`
- **THEN** the verb neither reads nor mentions delegation

### Requirement: Group membership is a verb with the narrowing disclosure

`safix group add <group> <subject>` and `safix group remove <group> <subject>` SHALL edit the group's declaration as text, parsed by the real parser before staging and committed, with `remove` printing the not-retroactive disclosure and naming the revocation report that will carry the shrink.
Both SHALL honour the delegation records over groups the organization's silo declarations cover, and both SHALL refuse a subject or group the fleet does not declare.

#### Scenario: An addition is one inserted line

- **WHEN** alice runs `safix group add oncall bob`
- **THEN** the group's declaration gains bob as one inserted line, parsed before staging, and the commit names the act

#### Scenario: A removal says what it does not undo

- **WHEN** alice runs `safix group remove oncall bob`
- **THEN** the edit lands and the verb prints that bob has seen what the group could read and rotation is the remedy
- **AND** the next `check` reports the shrink as the revocation it is

### Requirement: Retired and reserved verbs are recorded rather than left mysterious

The command's help SHALL state which verbs from the tool this package replaces do not exist here, and why.
Where a word this package's own vocabulary once used is retired outright, the help SHALL say so; where one is reserved for a feature not yet built, the help SHALL say that instead, so a reservation is never mistaken for an oversight.
Where a verb exists but is invoked by a machine rather than by an operator, the help SHALL say that too.

#### Scenario: The recorded absences

- **WHEN** the help text is read
- **THEN** it states that no verb exists for ongoing secret delivery, because activation already delivers what it would
- **AND** it states that no export verb exists, naming clan's own `export` as the bulk plaintext dump safix's design refuses to build on either side of the boundary

#### Scenario: A reserved absence is told apart from a retired one

- **WHEN** the help for `import` is read
- **THEN** it states that `import` is reserved for a future, unbuilt feature — ingesting a value from an external plaintext source one entry at a time — rather than retired outright
- **AND** it states that this is distinct from `export`'s retirement, which is permanent

#### Scenario: `upload` exists under a name clan also uses, with narrower semantics

- **WHEN** the help for `upload` is read
- **THEN** it states that it moves only a machine's own host identity, once, before that machine's first activation
- **AND** states that it is not clan's ongoing vars-delivery verb of the same name, because activation already delivers what this package resolves for a machine that already has its identity
- **AND** it names this package's own `install` verb as what performs that delivery, rather than naming the framework this package no longer depends on

#### Scenario: A machine-facing verb is marked as one

- **WHEN** the help enumerates the verbs
- **THEN** `install` is described as the verb an activation runs rather than one an operator types
- **AND** it is the only verb so described, so the mark means something

### Requirement: A global entry file replaces the flake for evaluation

The command SHALL accept a global `--entry <file>` option and a `SAFIX_ENTRY` environment variable naming a plain nix file.
When either is set, every nix evaluation SHALL target that file — `nix eval --file <file> <attribute>` — instead of `<root>#<attribute>`, using the same attribute spellings the flake path uses.
`--entry` SHALL take precedence over `SAFIX_ENTRY` when both are given.
Fifteen of the sixteen subcommands SHALL behave identically under `--entry` as against a flake; `generate` is the documented exception.
`install` is identical under either because it evaluates no nix at all.
A sync target added to the runtime adds one attribute to the set every evaluation path reads, so this requirement SHALL be stated over each attribute the runtime evaluates rather than over a count of them, which a target change would falsify without changing any behaviour.

#### Scenario: The attribute spellings are unchanged

- **WHEN** each attribute the runtime evaluates is read under `--entry`
- **THEN** the string naming it is identical to the one read against a flake
- **AND** only how the target is built — `--file <entry> <attribute>` rather than `<root>#<attribute>` — differs

#### Scenario: Nested payload shapes evaluate identically under either path

- **WHEN** an entry file's declarations include at least one generator, one bridge mapping, one keepassxc mapping, and one pass mapping declaring metadata fields
- **THEN** the JSON `--entry` evaluation emits for `generatorPlan`, `bridge`, `keepassxc`, and `pass` deserializes without an unknown-field error against the same `#[serde(deny_unknown_fields)]` structs the flake path deserializes against
- **AND** the same fixture, evaluated against an equivalent flake, produces byte-identical JSON apart from any store path a generator's tooling reference embeds

#### Scenario: A verb unaffected by --entry works exactly as it does against a flake

- **WHEN** `list`, `get`, `set`, `check`, `fix`, `keygen`, `adduser`, `enroll`, `group`, `audit`, `sync`, `upload`, or `edit` is run under `--entry`
- **THEN** its behaviour, output, and exit code match the same invocation against an equivalent flake
- **AND** neither git operations nor workspace root discovery change, because `--entry` governs only how declarations are evaluated

#### Scenario: A verb that evaluates nothing is trivially unaffected

- **WHEN** `install` is run with `--entry` or `SAFIX_ENTRY` set
- **THEN** it behaves exactly as without them, because its whole input is the manifest path it was given
- **AND** it performs no workspace discovery either, so a manifest installs on a machine that holds no declaring repository at all

#### Scenario: The workspace root is still found by git

- **WHEN** `--entry` or `SAFIX_ENTRY` is set
- **THEN** the repository root a run stages and commits into is still the one git reports for the current directory
- **AND** the entry file need not be inside that repository, though every declaration it makes about paths under `root` still resolves relative to whatever `root` its own expression names

### Requirement: generate requires a flake or a declared nixpkgs reference

`safix generate` SHALL refuse, before running any generator, when it is invoked under `--entry` or `SAFIX_ENTRY`, no `--nixpkgs <flake-ref>` or `SAFIX_NIXPKGS` is given, and the target user's `generatorPlan.order` is non-empty.
The refusal SHALL name the reason — that the generator sandbox resolves its tools through a flake — and both remedies: dropping `--entry` to run against the declaring flake, or supplying `--nixpkgs`.
A user whose `generatorPlan.order` is empty SHALL succeed under `--entry` with no nixpkgs reference, unchanged from today.

#### Scenario: A declared generator refuses without a flake or a nixpkgs reference

- **WHEN** `safix generate` is run under `--entry`, the target user's generator order is non-empty, and neither `--nixpkgs` nor `SAFIX_NIXPKGS` is set
- **THEN** the command refuses before any generator runs
- **AND** the refusal names both remedies

#### Scenario: A user with no generator is unaffected

- **WHEN** `safix generate` is run under `--entry` for a user whose `generatorPlan.order` is empty
- **THEN** the command succeeds, having done nothing, exactly as it does against a flake

#### Scenario: A declared nixpkgs reference lifts the refusal

- **WHEN** `safix generate` is run under `--entry` with `--nixpkgs <flake-ref>` or `SAFIX_NIXPKGS` set, and the target user's generator order is non-empty
- **THEN** the sandbox resolves its tools against the declared reference instead of `--inputs-from <root>`
- **AND** every generator in the order runs exactly as it would against a flake

#### Scenario: Flake mode is unaffected

- **WHEN** `safix generate` is run with neither `--entry` nor `SAFIX_ENTRY` set
- **THEN** the refusal never fires, and `--nixpkgs` and `SAFIX_NIXPKGS`, if given, are ignored, because `--inputs-from <root>` already resolves the sandbox's tools

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

### Requirement: `install` takes a manifest, a check mode, and two switches

`install` SHALL accept a manifest path, `--check-mode=off|manifest|document`, `--ignore-passwd`, and `--dry-run`, and SHALL treat the host's dry-activation signal in the environment as implying `--dry-run`.
`--check-mode=manifest` SHALL validate structure without reading any ciphertext; `--check-mode=document` SHALL additionally decrypt each named document and verify each declared key resolves in it.

#### Scenario: The two checking tiers are distinguishable

- **WHEN** `install --check-mode=manifest` is run against a manifest whose documents are absent or undecryptable
- **THEN** it succeeds, because it read none of them
- **WHEN** `install --check-mode=document` is run against the same manifest
- **THEN** it fails naming the document it could not read
- **AND** the two are exercised over one manifest, so the tiers are held apart rather than asserted

#### Scenario: A check mode installs nothing

- **WHEN** either checking mode runs
- **THEN** no file is written into the store, no filesystem is mounted, and no symlink moves
- **AND** the mode is therefore usable inside a build, which is what the manifest derivation's own check phase does with it

#### Scenario: The passwd switch skips resolution rather than guessing

- **WHEN** `--ignore-passwd` is given
- **THEN** no user, group, or keys-group lookup is performed and every entry's owner and group are zero
- **AND** the reason is recorded: a sandboxed build has no such users, so a mode that guessed them would be validating a resolution it cannot perform

### Requirement: `sync` and `audit` read `pass` as a target keyword

Both verbs SHALL accept `pass` as a first argument naming a target, alongside the target keywords they already accept, and SHALL narrow the run to that target's own mappings when it is given.
Naming a mapping of one target together with a direction or mode word that target does not use SHALL be refused naming the target, as it already is for every other target.
Both verbs' forms, help text and reports SHALL name this target in the same shape they name the existing ones, so the target list a reader sees in the help is the target list the dispatch accepts.

#### Scenario: The keyword narrows the run

- **WHEN** `sync pass` or `audit pass` is run
- **THEN** only pass mappings are acted on or compared
- **AND** no other target's mappings appear in the report

#### Scenario: The help and the dispatch agree

- **WHEN** either verb's form and help text are read
- **THEN** `pass` appears among the target keywords, with its own section stating its modes and what it converges
- **AND** the keywords the help lists are exactly the keywords the dispatch accepts

#### Scenario: A bare run still covers every target

- **WHEN** either verb is run with no target named
- **THEN** pass mappings are acted on or compared alongside every other target's
- **AND** a run over a consumer declaring no pass mapping reports nothing for this target rather than refusing

### Requirement: Every value path preserves bytes or refuses before mutation

Safix SHALL preserve arbitrary bytes on its own secret storage, generator and installation paths. Intentional empty values SHALL remain distinguishable from missing values and placeholders. Text-only external destinations SHALL refuse values they cannot represent before changing the destination.

#### Scenario: A non-UTF-8 piped value
- **WHEN** set receives bytes 00 ff fe 41 0a
- **THEN** get and installation return exactly those bytes

#### Scenario: A text-only destination
- **WHEN** synchronization sends a non-UTF-8 value to a text-only API
- **THEN** it refuses before modifying the destination

### Requirement: `migrate` recovers its own interruptions

`safix migrate <plan.json>` SHALL resume an interrupted run of the same plan, and `safix migrate --abandon <plan.json>` SHALL discard one. The help for `migrate` SHALL state that an interrupted run resumes on rerun, that `--abandon` removes what the journal records, and that a journal from a different plan is refused. It SHALL NOT advise inspecting partial outputs by hand.

#### Scenario: The help states the recovery contract
- **WHEN** the help for `migrate` is read
- **THEN** it names the journal, the rerun-to-resume behaviour and `--abandon`
- **AND** it states that sources are retained on every path, including abandonment

#### Scenario: Abandon takes exactly one plan
- **WHEN** `--abandon` is given without a plan path or with more than one
- **THEN** the verb refuses as a usage error naming the expected form

### Requirement: The picker edits its query at a caret

The picker SHALL keep a caret inside the query. Typed characters SHALL insert at the caret; Backspace SHALL delete the character before it and Delete the character under it; Left and Right SHALL move it one character; Home and End SHALL move it to either end. Ctrl+Left and Ctrl+Right SHALL scroll the columns. Every edit SHALL re-narrow the rows exactly as appending did. The help line SHALL name these keys. Escape and Ctrl+C SHALL still cancel, and an escape sequence the picker does not bind SHALL be ignored rather than cancelling.

#### Scenario: A mistake in the middle is fixed in place
- **WHEN** the operator moves the caret left past a mistyped character, deletes it and types the correction
- **THEN** the query reads with the correction in that position
- **AND** the rows are narrowed to the corrected query

#### Scenario: Arrows no longer scroll
- **WHEN** Left or Right is pressed with columns scrolled off-screen
- **THEN** the columns do not scroll and the caret moves
- **AND** Ctrl+Left or Ctrl+Right scrolls them

#### Scenario: An unbound sequence is not a cancel
- **WHEN** a key the picker does not bind arrives as an escape sequence, such as Page Down
- **THEN** the picker keeps running with nothing changed

### Requirement: The picker separates the list from the value and titles the list from below

The column titles SHALL be the last line of the list, directly below the alphabetically first row. Below the titles the picker SHALL draw one blank line, one rule spanning the terminal width carrying the pane's title, the decrypted value, one blank line, the query and the help. A terminal too short for the pane SHALL keep the titles beneath the rows and drop the pane, the rule and its padding together.

#### Scenario: The frame reads upward from its headings
- **WHEN** the picker draws on a terminal tall enough for the pane
- **THEN** the titles sit under the rows, the rule sits one blank line under the titles, the value follows the rule, and one blank line separates the value from the query

#### Scenario: A short terminal keeps the titles
- **WHEN** the terminal has no room for the pane
- **THEN** the rows and their titles are drawn and nothing of the pane, the rule or its padding is

### Requirement: The picker shows what matched and where it looked

For every row the query admits, the characters each term matched SHALL be emphasised inside the cell they matched. The title of every column the query searches SHALL be emphasised: a term with a field emphasises that field's title, a term without one emphasises every title, an empty query emphasises none. Exclusion terms SHALL emphasise nothing. Emphasis SHALL compose with the cursor's reverse video and the row tints, and a line wider than the terminal SHALL be cut by visible width so no emphasis sequence is split.

#### Scenario: A field term lights one column
- **WHEN** the query is `name:tok`
- **THEN** only the name title is emphasised
- **AND** in each admitted row the letters `tok` inside the name cell are emphasised and nothing else is

#### Scenario: A bare term lights every column it could match
- **WHEN** the query is `tok`
- **THEN** every column title is emphasised
- **AND** every cell containing `tok` has those letters emphasised, case-insensitively

#### Scenario: An exclusion emphasises nothing
- **WHEN** the query is `!tok`
- **THEN** admitted rows carry no emphasis and no title is emphasised for that term

#### Scenario: Emphasis survives the cut
- **WHEN** an admitted row is wider than the terminal
- **THEN** the drawn line is cut to the terminal width counted in visible characters
- **AND** every emphasis it opened is closed before the line ends

### Requirement: `rotate` and `rotation` join the closed subcommand set

The command SHALL accept `rotate` and `rotation` as subcommands, listed in its help with one line each, and `list` SHALL carry the rotation deadline as a column beside the timestamps.

#### Scenario: The verbs are discoverable
- **WHEN** the help enumerates the verbs
- **THEN** `rotate` and `rotation` appear with their forms and one-line meanings

#### Scenario: `list` carries the deadline
- **WHEN** `list` runs
- **THEN** each row shows the remaining time, `due`, or the absent marker in a column of its own

### Requirement: The encrypting backend is denied every key-service address unless the operator says otherwise

Every invocation of the encrypting backend SHALL carry `SOPS_HC_VAULT_ALLOWLIST=none` unless the operator's environment sets it to a non-empty value, which SHALL reach the backend unchanged.
An invocation for a run that must not inherit ambient identity material SHALL additionally carry neither `VAULT_TOKEN` nor `VAULT_ADDR`.
The environment reference SHALL name both behaviours and GHSA-jgf3-f6rg-8x3h.

#### Scenario: An unset allowlist is set to none

- **WHEN** a value is written to a declared secret whose file does not exist yet, with `SOPS_HC_VAULT_ALLOWLIST` unset
- **THEN** every backend invocation the run makes, the one creating the file and the one writing the value, carries `SOPS_HC_VAULT_ALLOWLIST=none`

#### Scenario: An operator's allowlist wins

- **WHEN** the same write runs with `SOPS_HC_VAULT_ALLOWLIST` set to a non-empty value by the operator
- **THEN** every backend invocation carries exactly that value
- **AND** none carries `none`

#### Scenario: An isolated run hands the backend no Vault credentials

- **WHEN** an installation decrypts with `VAULT_TOKEN` and `VAULT_ADDR` present in its own environment
- **THEN** the backend invocation carries neither variable
- **AND** it carries `SOPS_HC_VAULT_ALLOWLIST=none`

### Requirement: The packaged command carries the card plugin as a fallback on its search path

The packaged command SHALL append `age-plugin-yubikey` from its own closure to the end of its executable search path, after whatever the operator's search path already holds, so a write to a card's recipient succeeds without the operator installing the plugin and an operator's own plugin is still the one found first.
The packaged command SHALL evaluate on every system the flake declares.

#### Scenario: The plugin is found with nothing else installed

- **WHEN** the packaged command runs with an operator search path that holds no `age-plugin-yubikey`
- **THEN** the search path its children inherit ends with the directory holding the packaged plugin

#### Scenario: The operator's plugin comes first

- **WHEN** the packaged command runs with an operator search path
- **THEN** every operator entry precedes the packaged plugin's directory in the search path its children inherit

### Requirement: keygen names its identity file to standalone tools where their default differs

On macOS, after appending an age identity, `keygen` and `keygen --kind age` SHALL print an `export SOPS_AGE_KEY_FILE=<file>` line naming the file written, unless that file is the one standalone sops reads by default or `SOPS_AGE_KEY_FILE` already names it.
Standalone sops's default on macOS is `$XDG_CONFIG_HOME/sops/age/keys.txt` where `XDG_CONFIG_HOME` is non-empty and `$HOME/Library/Application Support/sops/age/keys.txt` otherwise.
Where safix itself looks for identities SHALL NOT change.

#### Scenario: A macOS identity under the XDG default is named

- **WHEN** `keygen` appends an identity on macOS with `XDG_CONFIG_HOME` and `SOPS_AGE_KEY_FILE` unset
- **THEN** the output includes `export SOPS_AGE_KEY_FILE=` followed by the file it appended to

#### Scenario: Nothing is printed where standalone sops already looks

- **WHEN** `keygen` appends an identity on macOS with `XDG_CONFIG_HOME` set, so the file written is the one standalone sops reads
- **THEN** the output includes no `export SOPS_AGE_KEY_FILE=` line

#### Scenario: Nothing is printed off macOS

- **WHEN** `keygen` appends an identity on Linux
- **THEN** the output includes no `export SOPS_AGE_KEY_FILE=` line

#### Scenario: safix's own discovery is unchanged

- **WHEN** a run on macOS discovers its age identity file
- **THEN** it consults `SOPS_AGE_KEY_FILE`, then `SAFIX_AGE_KEY_FILE`, then `$XDG_CONFIG_HOME/sops/age/keys.txt` falling back to `$HOME/.config`, exactly as on Linux

### Requirement: `keygen --kind age --hybrid` mints a hybrid post-quantum identity

`keygen --kind age --hybrid` SHALL mint one hybrid X25519 + ML-KEM-768 identity through the upstream `age-keygen`, append it to the identity file plain `--kind age` appends to, and print only its public recipient.
Plain `--kind age` SHALL keep minting an X25519 identity.
The help text SHALL state the per-recipient ciphertext cost and the sops and age versions every reader needs.

#### Scenario: A hybrid identity is appended beside an existing one

- **GIVEN** an identity file that already holds an X25519 identity
- **WHEN** `keygen --kind age --hybrid` runs
- **THEN** the file holds the X25519 identity unchanged and one new `AGE-SECRET-KEY-PQ-1…` identity after it
- **AND** the printed recipient begins `age1pq1` and is the one `age-keygen -y` derives from the new identity

#### Scenario: The private half is never printed

- **WHEN** `keygen --kind age --hybrid` runs
- **THEN** neither standard output nor standard error carries the `AGE-SECRET-KEY-PQ-1…` line

#### Scenario: `--show` names the hybrid recipient after a hybrid mint

- **WHEN** `keygen --show` runs after `keygen --kind age --hybrid`
- **THEN** it prints the hybrid recipient, because that identity was appended last

#### Scenario: `--hybrid` outside the age profile is a usage error

- **WHEN** `--hybrid` is given with `--kind pgp`, or without `--kind age`
- **THEN** the command refuses with its usage form and mints nothing

#### Scenario: The cost is stated

- **WHEN** the `keygen` help text is read
- **THEN** it states that a hybrid recipient is about 2,000 characters long and adds about 4.8 KB to every sops file wrapped to it
- **AND** it states that every reader needs sops 3.12.0 or later, or age 1.3.0 or later

### Requirement: `adduser` admits a hybrid recipient by shape

`adduser` SHALL accept `age1pq1` followed by exactly 1952 bech32 characters as a well-formed recipient, beside the X25519, SSH and `pgp:` shapes it already accepts.
It SHALL keep refusing `age1yubikey1…` as a hardware recipient, and SHALL refuse a hybrid-prefixed string of any other length or alphabet as malformed.

#### Scenario: A hybrid recipient is declared

- **WHEN** `adduser` is given a well-formed hybrid recipient
- **THEN** the scaffold records it as the person's recipient and the regenerated policy names it

#### Scenario: A truncated hybrid recipient is refused

- **WHEN** `adduser` is given `age1pq1` followed by fewer or more than 1952 bech32 characters, or by a character outside the bech32 alphabet
- **THEN** it refuses as a malformed recipient and writes nothing

#### Scenario: The refusal names both age shapes

- **WHEN** a malformed recipient is refused
- **THEN** the message describes the X25519 shape and the hybrid shape

### Requirement: check reports a derived value whose dependency changed after it

`safix check` SHALL report a finding for each generator, per holder, when a dependency it declares has an `updated` stamp later than the earliest `updated` stamp among the values that generator writes.
The finding SHALL name the holder, the generator's entry and the dependency, SHALL name `safix generate --regenerate <user> <entry>` as the remedy, and SHALL fail the run.
It SHALL be answered from the stamp records alone.

#### Scenario: A dependency set by hand after the derivation is reported

- **WHEN** a value a generator reads is written by `set`, `edit` or `sync` after that generator last ran
- **THEN** `check` reports the generator's entry as derived from a dependency that has changed since, naming the dependency
- **AND** the remedy is `safix generate --regenerate <user> <entry>` naming the dependent entry, never the dependency
- **AND** `check` exits non-zero
- **AND** no ciphertext is opened and the finding carries no value

#### Scenario: A regeneration cascade leaves nothing to report

- **WHEN** a dependency is regenerated through `generate --regenerate` or `rotate`, which re-run every generator derived from it
- **THEN** no staleness finding exists for any generator in that cascade

#### Scenario: The remedy clears the finding

- **WHEN** the named remedy runs
- **THEN** the finding is gone on the next `check`

#### Scenario: A missing stamp makes no claim

- **WHEN** the dependency or every value the generator writes has no stamp record
- **THEN** no staleness finding exists for that pair

#### Scenario: Equal stamps are not stale

- **WHEN** a dependency's `updated` stamp equals the dependent's, as a cascade completing within one second leaves them
- **THEN** no staleness finding exists for that pair

### Requirement: check judges a raw-age file's SSH recipients by their tags

`safix check` SHALL read each existing raw-age governed file's header, binary or ASCII-armored, without decrypting it, and SHALL compare each SSH stanza's tag against the tags of the SSH recipients the file's declared audience names.
A missing, extra or duplicated SSH stanza SHALL be a failing finding naming `safix fix --yes`.
A header that cannot be read SHALL refuse the run naming the file.

#### Scenario: An extra SSH recipient is reported

- **WHEN** a raw-age file carries an `ssh-ed25519` or `ssh-rsa` stanza whose tag matches no declared SSH recipient of its audience
- **THEN** `check` reports the file's SSH recipients as drifted, listing the stanza as extra
- **AND** names the declared subject holding a key with that tag when exactly one declared key anywhere has it, and the stanza's type and tag otherwise
- **AND** states that re-wrapping is not revocation and names what mints a new value
- **AND** `check` exits non-zero

#### Scenario: A missing SSH recipient is reported

- **WHEN** a declared SSH recipient of the audience has no stanza with its tag in the file
- **THEN** `check` reports that recipient as missing and exits non-zero

#### Scenario: A duplicated SSH stanza is reported

- **WHEN** more stanzas carry one type and tag than the audience declares distinct keys with that type and tag
- **THEN** `check` reports the surplus as duplicated and exits non-zero

#### Scenario: Armored and binary files are read alike

- **WHEN** a raw-age file is ASCII-armored
- **THEN** its header is decoded from the armor and judged exactly as the binary form would be

#### Scenario: Grease stanzas are ignored

- **WHEN** a stanza's type ends in `-grease`
- **THEN** it is neither counted as a recipient nor as unverifiable

#### Scenario: An unreadable header refuses

- **WHEN** a raw-age governed file does not begin with the age v1 header, its armor does not decode, or its header has no closing MAC line
- **THEN** `check` refuses with `safix::recipients_unreadable` naming the file, caused by `safix::age_header_unreadable`

### Requirement: An exact SSH tag match is a note, never a pass

When every stanza of a raw-age file is an SSH stanza and the stanzas match the declared SSH recipients exactly, `safix check` SHALL report a note that the roster matched by 32-bit tag only and that a tag match does not establish which keys the file is wrapped to.
The note SHALL NOT fail the run.

#### Scenario: A matching all-SSH file is a note

- **WHEN** a raw-age file's audience declares only SSH recipients and the file's stanzas match their tags one for one
- **THEN** `check` prints a note naming the file and stating that the match is by a four-byte tag anyone can collide with
- **AND** that note alone does not make `check` exit non-zero

#### Scenario: Any non-SSH recipient keeps the file unverifiable

- **WHEN** a raw-age file carries an X25519, plugin, scrypt or other non-SSH stanza, or its audience declares a non-SSH recipient
- **THEN** `check` reports the file's roster as unverifiable, failing the run as it does today
- **AND** any SSH stanza disagreement in the same file is reported as well, as its own finding

### Requirement: check's exit status follows its failing findings

`safix check` SHALL exit non-zero when at least one failing finding is reported and zero otherwise.
Every finding SHALL fail except the SSH tag-match note.
The closing line SHALL count failing findings only.

#### Scenario: Notes alone exit zero

- **WHEN** the only findings are tag-match notes
- **THEN** `check` prints them, closes by stating that no drift was found and that the notes above qualify it, and exits zero

#### Scenario: A failing finding beside a note exits non-zero

- **WHEN** a note and a failing finding are both reported
- **THEN** the closing line counts the failing finding and not the note
- **AND** `check` exits non-zero

### Requirement: check reports a raw-age file wrapped to an audience that is no longer declared

When a raw-age file's audience record exists, is in a format this version reads, and records the SHA-256 of the file's current bytes, `safix check` SHALL report a failing finding if the audience digest it records differs from the digest of the audience declared now, naming the file and `safix fix --yes`.
An absent or unreadable record, or one recording other bytes, SHALL produce no finding.

#### Scenario: A narrowed audience over a recorded raw-age file is reported

- **WHEN** `fix --yes` re-wrapped a raw-age file and wrote its record, and a recipient is then removed from the file's declared audience
- **THEN** `check` reports the file as wrapped to an audience that is no longer declared
- **AND** states that re-wrapping is not revocation
- **AND** exits non-zero

#### Scenario: An absent record is not a finding

- **WHEN** a raw-age file has no audience record, as every file written before this change has none
- **THEN** no audience-record finding exists for it

#### Scenario: A record about other bytes is silent

- **WHEN** the file was rewritten after its record, by `set`, `edit`, `generate`, `sync` or a tool outside safix
- **THEN** the record says nothing about the current file and no audience-record finding exists for it
