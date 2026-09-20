# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
