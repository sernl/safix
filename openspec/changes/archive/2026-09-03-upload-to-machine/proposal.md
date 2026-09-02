# safix upload seeds a machine's identity before it can decrypt anything of its own

Revisions are named because every claim below was read at one.
`clan-core` is `56e35624d94e4f1ac55d36575ebab97cbd9b9cdd`, this flake's `inputs.clan-core`, the same pin the archived `own-secret-installer` change reasoned against.

## Why

A machine's declared recipient is the age form of an ed25519 host key it does not yet hold.
`flake.safix.machines.<m>.recipient` names "the age form of the host identity the system scope already decrypts with" (`modules/flake/safix/types.nix:548-576`), and declaring the machine "mints no second identity and adds no enrollment step" (`:557-558`) — the key has to already exist on the host for the sentence to be true.
On every machine this package has shipped to so far, that key existed because the machine had already booted at least once: `modules/consume/nixos.nix` derives the system-scope identity from `services.openssh.hostKeys` (`:46-54`), and a host that has never booted has no host keys to derive from.

A freshly installed machine is exactly the host that has not booted yet, and it is the one case safix's own installer cannot bootstrap by itself.
`secret-installation`'s own requirements assume the identity is already present or derivable before its installer runs — "The installer runs after the stores it is told to wait for" (`openspec/specs/secret-installation/spec.md:73-96`) orders safix's installer against another store that already exists, not against a host key that has never been generated.
Nothing in this package mints that key, by the same principle `custody-subjects` states for every subject: "the key a person holds stays theirs to generate" (README.md:816-818), applied to a machine's host identity rather than a person's.
So the operator generates or harvests the private half themselves, and today has no supported way to get it onto the machine's disk before the first boot that would need it — the gap this change closes.

`safix-cli`'s own requirement records the reason no such verb exists today: "the help states that no upload verb exists because activation already delivers what it would" (`openspec/specs/safix-cli/spec.md:144-146`).
That sentence is true of ongoing secret delivery — ciphertext rides the nix closure and safix's own installer decrypts it locally, so there is nothing to push after the first boot — and it is silent on what happens before the first boot, when no ciphertext has been decrypted yet because no identity exists to decrypt with.
This change adds the verb the sentence's reasoning does not cover, and narrows what the sentence claims accordingly.

## What Changes

- A new `safix upload <machine>` command, mirroring clan's own verb of the same name (`clan_cli/vars/cli.py:190-212`) in what it is for — getting material onto a machine before that machine can help itself — and diverging from it in what it moves: clan uploads generated secret values on an ongoing basis (`clan_lib/vars/upload.py`), safix uploads only the host identity material a machine needs once, before its first boot, because ongoing delivery is already solved by the closure.
- `--directory DIR` mode writes a pre-seed tree at `DIR`, holding the operator-supplied private host key at the path a fresh install's `services.openssh.hostKeys` will read it from, for consumption by `nixos-anywhere --extra-files` or equivalent offline media preparation — mirroring clan's own `--directory` split (`clan_cli/vars/upload.py:14-19`, `populate_secret_vars`).
- Remote mode (no `--directory`) connects over ssh to an operator-named address, mirroring clan's transport shape (`clan_lib/ssh/upload.py`): root, tar-over-ssh, `0400` files and `0700` directories, wipe-then-extract, with the path-depth safety clan's own transport enforces (`:9-11`, `:34-53`).
  Before writing anything it inspects what identity the target already presents.
- An honest no-op: when the target already presents an ed25519 host key that derives to the machine's declared recipient, remote mode writes nothing and reports that the machine already has what it needs, rather than performing a transfer that would either overwrite a live host's own key or silently do nothing while claiming success.
- `--identity PATH` supplies the private key material the operator harvested or minted; the command mints no machine identity itself, and refuses when the supplied key's derived recipient does not match the machine's declared one, so a pre-seed run cannot silently seed the wrong key.
- Refusals for an undeclared machine, a declared machine with no recipient, and a write that has no `--identity` to write.
- **BREAKING** (narrowing, not additive): `safix-cli`'s "Retired and reserved verbs are recorded rather than left mysterious" requirement (`rename-transfer-verbs`'s successor to "Absent verbs are recorded rather than left mysterious") stops stating that no upload verb exists; it states instead what the verb is scoped to, so the sentence stays true.

Not in scope: a `safix upload` mode for person identities — this verb parses a machine name only, refusing a person's name the way it refuses an undeclared machine; bootstrapping a person's own first identity is separate, deferred design work outside this repository (the dotfiles repository's `one-unlock-bootstrap` design, `CHANGELOG.md:169`).
Not in scope: a systemd-credentials delivery path for the same material — the operator has deferred that feature entire; this change records it as a named non-goal and states the one-sentence extension point rather than building toward it.
Not in scope: any deploy or rebuild verb — safix is pull-model, so a provisioned machine's own next `nixos-rebuild switch` (or equivalent) is what activates the closure that already carries its ciphertext, and this change adds no verb that triggers one.
Not in scope: any change to `secret-installation` or `consumer-integration`'s requirements — both are read below and neither needs a requirement changed; see design.md's Context for why the boundary holds.

## Capabilities

### New Capabilities

- `machine-provisioning`: the `safix upload` command — its two write modes and their inputs, the honest no-op that a machine's own answer produces, the transport it mirrors from clan, the refusals that keep a pre-seed run from writing the wrong key or writing into a live machine's identity, and the boundary that keeps it out of deploy and out of person identity.

### Modified Capabilities

- `safix-cli`: `rename-transfer-verbs`'s "Retired and reserved verbs are recorded rather than left mysterious" requirement's recorded absence of an upload verb is no longer true as stated and is narrowed to what remains true — no verb exists for ongoing secret delivery, because activation already delivers what it would.
- `plaintext-staging`: its one requirement scoping staged plaintext to "generation or editing" gains a third occasion — the transient tarball remote mode assembles before streaming it over ssh — and a scenario stating that a destination the operator named on the command line, `--directory DIR` among them, is not staging and is not held to the memory-backed rule.

## Impact

Affected code (rust crate, `crates/safix-core/src` and `crates/safix/src`): a new `upload` module in `safix-core` (machine resolution, the two write modes, the presented-identity probe, the transport), a new `upload_command` dispatch arm in `crates/safix/src/main.rs` beside the other custody-touching verbs, and a new entry in `crates/safix/src/usage.rs`'s scaffold table.

Affected checks: no new `modules/flake/checks/*.nix` file, because this change adds no nix option; `crates/safix/tests/upload.rs`'s integration tests are wired one per claim through `modules/flake/checks/cli.nix`'s `mode` helper — the honest no-op that cannot become a silent write (the severity property this change cares most about), the recipient-mismatch refusal, the path-depth safety on the fixed pre-seed destination, and the plaintext-staging scenario distinguishing an operator-named destination from a staging root — each with its own perturbation drill recorded in that file's ledger.

Sequencing: this change is written to apply after `rename-transfer-verbs`, for a spec-level reason rather than a scheduling preference.
`rename-transfer-verbs` REMOVES `safix-cli`'s "Absent verbs are recorded rather than left mysterious" requirement and ADDS "Retired and reserved verbs are recorded rather than left mysterious" in its place, and this change's own `safix-cli` delta MODIFIES that successor requirement rather than the one `rename-transfer-verbs` removes.
Both changes also edit the same `crates/safix/src/usage.rs` block — `rename-transfer-verbs` task 5.3 rewrites it first, and this change's task 6.2 narrows the sentence it writes there — so landing `rename-transfer-verbs` first means this change's diff to that block is written against its post-rewrite shape once, rather than against a moving target.

No machine on any fleet this package has shipped to has ever been provisioned through this verb; every claim above was read from the cited sources rather than measured against a deployment.
