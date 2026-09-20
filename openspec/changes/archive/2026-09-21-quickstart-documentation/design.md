# Design

## Context

See proposal.md. Consumers reach declarations through `lib.mkVault { modules; root; }` with no flake-parts; `nixosModules.safix` and `homeModules.safix` are the consumption modules; `config.safix.secrets.<name>.path` is what a service reads. `examples/` holds one fleet declared twice plus two profiles, all checked. `docs/` is empty apart from one note.

## Goals / Non-Goals

**Goals:**
- A reader with an ordinary NixOS flake reaches a running secret without leaving the README, and every snippet they copy is code a check evaluated.
- Every fact lives in one page; the README links, it does not repeat.
- The docs tree is navigable by page kind, and prose in it reads once.

**Non-Goals:**
- A documentation site build. Markdown with front matter renders on the forge and is ready for a generator later.
- Rewriting `CONTRIBUTING.md` beyond the snippet-check paragraph.

## Decisions

### D1. One story, one example

The README follows one fleet: a person `alice`, a host `web`, a service `grafana`. `examples/quickstart/` holds it: `flake.nix` (inputs `nixpkgs`, `home-manager`, `safix`; outputs `nixosConfigurations.web`, `homeConfigurations.alice`), `secrets.nix` (the `flake.safix.*` declarations), `hosts/web.nix` (the NixOS profile: `safix.machine`, `identity.deriveHostKeys`, a service reading `config.safix.secrets.grafana-admin-password.path`, a template with `restartUnits`), `home/alice.nix` (the home profile with `safix.user`, `identity.keyFile`, and the rotation timer off by default with the option shown). Every README nix block names its source file; longer files are quoted by region so the README stays short.

### D2. Snippet regions

Regions are marked in the source with `# --8<-- [start:<name>]` / `# --8<-- [end:<name>]`, the mkdocs-snippets convention, so a future site generator can include them the same way. The README's fence carries the source as `nix title="examples/quickstart/secrets.nix#declare-people"`. The check `readme-snippets` extracts each block, resolves file and region, strips the marker lines from the region, and diffs. A nix block with no `title` fails the check.

### D3. The two checks

`examples-quickstart` evaluates `hosts/web.nix` through `nixosSystem` and `home/alice.nix` through home-manager's library (the way `safix-examples-profiles` does), asserting the materialized entries, the template and the rotation option. `flake.nix` is read as text, as `examples/dendritic/flake.nix` is, so no second input closure is resolved inside the sandbox. `readme-snippets` is a pure derivation over `README.md` and `examples/quickstart/`.

### D4. Docs tree

```
docs/
  tutorials/first-secret.md            # README quickstart continued into home-manager and a second host
  guides/unattended-hosts.md           # machine keys vs human keys, deriveHostKeys, GnuPG caveats
  guides/inspectable-recipients.md     # SOPS over raw age where the roster must be auditable; fix --yes
  guides/after-removing-access.md      # group remove → check → rotate; the not-retroactive rule by reference
  guides/rotating-secrets.md           # policies, rotation set, rotate --due, the timer
  guides/generators.md                 # scripts, prompts, dependencies, multi-output, validation, sandbox
  guides/templates-and-services.md     # templates, restartUnits/reloadUnits, early-user secrets
  guides/migrating-from-sops-nix-and-agenix.md  # plan JSON walk-through, recovery, abandon
  guides/hardware-keys.md              # enroll, upload, the proof
  guides/identity-backup.md            # keygen, backup, restore, the independence rule
  guides/vault.md                      # opaque names, two roots, the commit order
  guides/syncing-password-managers.md  # the one sync chapter, five same-shaped subsections
  guides/flake-parts.md                # the flake module, dendritic layout, fully supported
  concepts/model.md                    # the three questions; audiences → files
  concepts/custody.md                  # subjects; the revocation rule stated once
  concepts/storage-layout.md           # the three roots, ignore rules, renaming a root
  reference/cli.md                     # the verb table, one row per verb, machine-facing verb marked
  reference/declarations.md            # flake.safix.* option reference
  reference/profile-options.md         # safix.* at both scopes
  reference/migration-plan.md          # the plan schema
  reference/environment.md             # SAFIX_* and honoured upstream variables
  reference/refusals.md                # stable refusal codes
```

Front matter `title:` on every page; H2 starts the body.

### D5. Style contract for every page

Plain sentences under forty words; one term per concept (`entry`, `audience`, `placement`, `profile`, `host`); code blocks that name their file; no marketing, no "simply", no rhetorical questions; a reader-oriented opening line stating what the page lets them do. Options are documented once in `reference/`; guides link to the option rather than restating its description. The revocation rule and the namespace rule are stated in `concepts/custody.md` and `concepts/model.md` respectively and referenced everywhere else.

### D6. What the README keeps from the old one

The pitch (three sentences), the install step, the declaration step, the first three commands, the host step, one generator, one rotation policy, one template, the picker, a migration pointer, the flake-parts section, the "opinions" list reduced to five lines with a link, licence. Everything else moves to the page D4 names. The old "where did section X go" list is dropped: the docs index is the map now.

## Risks / Trade-offs

- [Sibling changes land text into pages this change creates] → Each sibling's tasks name the page and section they own; this change creates the pages with the paragraphs that hold before the siblings land, and the siblings replace them. Order of landing does not matter because each sibling owns whole sections.
- [The README snippet check is brittle to whitespace] → It diffs bytes after stripping only marker lines; the examples are formatted by the repository's formatter, and the README blocks are copied from formatted files. Brittleness is the point.
- [The example's `flake.nix` cannot be evaluated in the sandbox] → It is checked as text for the paths it references, and the two profiles it wires are evaluated directly, the same compromise `examples/dendritic` already makes.
- [Twenty-two pages is a lot to write in one change] → The tree is fixed here; the pages are authored in parallel by page kind at implementation, each against D5, and reviewed as a set.
