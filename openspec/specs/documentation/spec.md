# documentation Specification

## Purpose
The contract the operator-facing README is held to as a document: that every verb and every option is documented exactly once, that no name only a contributor can use appears in it, that its prose is plain enough to be read once, that the chapter covering the sync targets carries one same-shaped subsection per target, and that contributor-facing status lives with the contributors.

## Requirements

### Requirement: Every verb and every option is documented exactly once

The documentation set — the README together with `docs/` — SHALL document each of the command's subcommands and each option of the declaration and profile namespaces, and SHALL document each of them in exactly one place. The README SHALL be a quickstart that walks one example end to end and SHALL link to the reference page for anything it mentions without expanding. It SHALL NOT state how many subcommands there are.

#### Scenario: The verb inventory is one table

- **WHEN** a reader looks for what the command can do
- **THEN** one table in the reference carries one row per subcommand, stating what it does, what it reads, what it writes, and whether it needs a terminal
- **AND** the subcommand that no operator types is present and marked as such, rather than omitted from the inventory and documented elsewhere

#### Scenario: A verb documented twice is a defect

- **WHEN** a subcommand's behaviour is described outside the row and the one page that expands it
- **THEN** that is a defect of the same class as a contradiction, because two sites drift and the reader cannot tell which is current
- **AND** a README snippet that shows a verb in use is not a second site, because it states no behaviour the reference does not

#### Scenario: No count of verbs appears

- **WHEN** the documentation set is read for a number of subcommands
- **THEN** no sentence states one

#### Scenario: Every option has exactly one documentation site

- **WHEN** an option of the declaration namespace or of either scope's profile namespace is looked up
- **THEN** it is found in exactly one reference page
- **AND** an option present in no reference page is undocumented, which is a defect rather than an omission left to the option's own description

### Requirement: The README names no check

The README SHALL NOT name any check of this repository's own suite.

#### Scenario: A guarantee is stated rather than attributed

- **WHEN** the README states a guarantee that a check holds
- **THEN** it states the guarantee, including what it does not cover
- **AND** it does not name the check, because a reader cannot act on a check name and the command that runs the suite prints the names itself

#### Scenario: Contributor evidence lives with the contributors

- **WHEN** a mapping from guarantee to check is wanted
- **THEN** the contributing document is where it belongs
- **AND** the reason is recorded: the audience that runs the suite is the audience that document addresses

#### Scenario: A published function is not a check name

- **WHEN** the README documents the helper a consumer calls to obtain checks of their own fleet
- **THEN** naming that function is not a violation of this requirement
- **AND** the distinction is that a consumer calls it, where a check of this repository is something only a contributor runs

### Requirement: The prose is plain, and each load-bearing rule is stated once

No sentence in the README or in `docs/` SHALL exceed forty words.
Each rule the model rests on SHALL be stated in exactly one place, with every later mention naming that place rather than restating the rule.

#### Scenario: Sentence length is bounded

- **WHEN** each sentence of the README and of every page under `docs/` is measured
- **THEN** none exceeds forty words

#### Scenario: The revocation rule is stated once

- **WHEN** the rule that narrowing an audience does not retroactively revoke what was already encrypted is looked for
- **THEN** it is stated once, in the concept page that introduces custody
- **AND** every other mention is a clause naming that page

#### Scenario: The namespace rule is stated once

- **WHEN** the rule that the package reads no option outside its own namespace is looked for
- **THEN** it is stated once
- **AND** every other mention names that place

#### Scenario: A rule's one statement carries its consequence

- **WHEN** a rule is stated in its one place
- **THEN** the statement carries what the rule does not give the reader, which is the part a cross-reference cannot restate safely

### Requirement: One sync chapter, with one same-shaped subsection per target

The documentation set SHALL carry exactly one page covering synchronisation with other stores, under the guides.
That page SHALL state the shared mapping shape, the modes, the direction reading, the conflict judgement and the never-delete rule once, and SHALL then carry one subsection per declared target, each with the same headings, stating only what differs. The README SHALL mention synchronisation in at most one sentence that links to that page.

#### Scenario: The shared shape is stated once

- **WHEN** the page is read
- **THEN** what a mapping is, the four modes, how a direction is read, how a conflict is judged and what is never deleted are each stated once, before the per-target subsections
- **AND** no subsection restates any of them

#### Scenario: Every target has a subsection, and they are the same shape

- **WHEN** the per-target subsections are compared
- **THEN** there is one per target the declaration surface carries
- **AND** each has the same headings in the same order: what it addresses, how it is declared, what it can carry, how it unlocks, what it refuses

#### Scenario: The number of targets appears nowhere

- **WHEN** the documentation set is read for a count of sync targets
- **THEN** no sentence states one

#### Scenario: A target with nothing to say under a heading says so

- **WHEN** a target has no unlock step of its own, because its agent is ambient
- **THEN** its subsection keeps the heading and states that
- **AND** it does not drop the heading, because a missing heading reads as an oversight rather than as an absence

### Requirement: Contributor-facing status lives in the contributing document

Narration of this repository's own suite — which platforms it runs on, which parts are conditional, what was retired — SHALL live in the contributing document and SHALL NOT appear in the README.

#### Scenario: The status section is not in the README

- **WHEN** the README's chapters are enumerated
- **THEN** none of them narrates this repository's continuous integration
- **AND** the criterion is recorded: a section that changes nothing an operator types is not the operator's

#### Scenario: It is moved rather than deleted

- **WHEN** the contributing document is read
- **THEN** it carries which checks are platform-conditional and why
- **AND** the reason is recorded: a contributor who does not know a check is skipped on their platform reads a skipped check as a passing one

### Requirement: The examples' own document states only what a check holds

The document describing the worked examples SHALL describe only what a check evaluates, and every coverage claim it makes SHALL name the assertion that holds it.

#### Scenario: A claim about what is evaluated is true of a check

- **WHEN** the examples' document claims that a file is read by a check
- **THEN** a check reads it
- **AND** a file that no check reads is described as such, because the previous document claimed every file under both examples was evaluated while one of them was evaluated by nothing

#### Scenario: How a file reaches its projection is stated as it is

- **WHEN** the document explains how the flakeless example obtains the resolver
- **THEN** it states the mechanism the file uses
- **AND** a mechanism the file deliberately no longer uses is not described as current

### Requirement: README snippets are lifted from a checked example

Every fenced code block in the README that shows nix SHALL name the file, or the named region of a file, under `examples/quickstart/` it was lifted from, and SHALL be byte-identical to it. A check SHALL hold that equality, and a separate check SHALL evaluate the example's host and home profiles for real. A block that names nothing SHALL be a shell transcript, not nix.

#### Scenario: A README block matches its source
- **WHEN** a nix block in the README names `examples/quickstart/secrets.nix`
- **THEN** its body is byte-identical to that file, or to the region the block names

#### Scenario: Drift fails a check
- **WHEN** the example file changes and the README does not
- **THEN** the snippet check fails naming the block and the file

#### Scenario: The example builds
- **WHEN** the example's host profile and home profile are evaluated
- **THEN** each materializes the secrets the README says it does

### Requirement: The docs tree follows Diátaxis and the flake-parts route is documented as supported

`docs/` SHALL carry `tutorials/`, `guides/`, `concepts/` and `reference/`, each page with a YAML `title` in front matter. One guide SHALL document the flake-parts route as fully supported, naming the flake module and the worked example, and the README SHALL say the same in one section. Three guides SHALL carry the custody guidance: separating machine identities from human identities for unattended hosts, choosing an inspectable recipient roster over raw age where auditing matters, and rotating a value after removing access.

#### Scenario: A reader finds the page kind by the directory
- **WHEN** the tree is listed
- **THEN** every page sits under exactly one of the four directories and carries a front-matter title

#### Scenario: flake-parts is a supported route, not a footnote
- **WHEN** a flake-parts user reads the README
- **THEN** one section states the route is fully supported and points at the guide and the example

#### Scenario: The custody guidance has a home
- **WHEN** an operator asks how an unattended host decrypts, why a raw-age roster cannot be audited, or what to do after removing a person
- **THEN** one guide each answers, and the README links to them from the step where the question arises
