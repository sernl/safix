## Context

The evidence is the upstream survey of 2026-11-04, `docs/notes/research/upstream-survey-2026-11.md`, at clan-core `dbae210`, agenix `5e5b88a` (tags 0.16.0, 0.17.0, 0.18.0) and sops-nix `b18e01c`.
Its "Corrections after verification" section amends none of the findings this change draws on: clan-core B1, F1 and F6, agenix §3 items 1, 4 and 8 and §4, sops-nix §3.5 and §4, and agenix-rekey §4.
The safix lines cited below were re-read at the current tree, not taken from the survey.

Current state, as read:

- `crates/safix-core/src/migrate.rs:526-539` refuses `neededForUsers: true` for the agenix target alongside restart and reload units, with the message "agenix deployment does not support restartUnits, reloadUnits, or neededForUsers".
  `:550-563` refuses any early-user entry that is not root-owned, for every target; `:564-574` adds the owner-only mode rule for the safix target only.
- `deployment_fields` (`migrate.rs:658-708`) writes `owner` and `group` for agenix only when the plan names them, and turns a `uid` or `gid` into the string field when the named one is absent (`:675-692`).
  The declaration module's header is `{ config, ... }:` (`:604`).
- agenix's own `group` default is `(findFirst (u: u.name == config.owner) { } (attrValues users)).group or "0"` at 0.18.0 (`modules/age.nix:222` at `5e5b88a`, commit `9353cb2`); 0.16.0 and 0.17.0 dropped the `or "0"` and fail to evaluate for an owner whose user has no `group` attribute, which is every nix-darwin user.
- `crates/safix/tests/migrate.rs` exercises only `deploymentTarget = "safix"` (`write_plan`, `:174-218`); nothing in `crates/safix/tests` or `modules/flake/checks` runs the agenix or sops-nix targets.
- The clan-shaped fixture in `modules/flake/checks/installer.nix` places host keys at `/run/secrets/openssh/ssh.id_ed25519` and `…/ssh.id_rsa` (`:793-805`), asserts the derived list at `:1506`, and describes the shape in the header at `:93-100`.
  clan-core at `dbae210` deploys an age-backend var at `/run/secrets/${file.config.rel_dir}/${file.config.name}` (`nixosModules/clanCore/vars/secret/age.nix:249`), where `rel_dir` is `per-machine/${config.machineName}/${generator.config.name}` for an unshared generator (`nixosModules/clanCore/vars/interface.nix:76-80`).
- A grep of `docs/`, `openspec/specs/`, `crates/`, `modules/` and `README.md` for `age.nix:` finds no live citation into clan's `age.nix`; the only hits are in the survey itself and in archived changes, which stay as written.
- The "commits in the clan repository" wording sits in `crates/safix-core/src/bridge.rs:17-24`, `:515-517`, `:545-548` and `:669-671`, in `openspec/specs/bridge-sync/spec.md:154`, and in `openspec/specs/bridge-transfer/spec.md:97` and `:195`.
  `crates/safix-core/src/clan.rs:173-193` starts clan without clearing the environment, so an exported `CLAN_AGE_STORE_DIR` reaches it.
- `safix enroll` registers a card with clan through `clan secrets users add` and `add-key --age-key` (`crates/safix-core/src/clan.rs:245-282`), which on an age-backend clan creates a sops user with no access to vars.
- `flake.lock` pins nixpkgs `0e251e2` (2026-08-13), home-manager `83b7606`, crane `2c71e19`, flake-parts `427bf4b`, treefmt-nix `ae79109` and advisory-db `69f93e1`.
  Through it the flake gets rustc 1.97.1, sops 3.13.3, age 1.3.1 and bitwarden-cli 2026.7.0; `Cargo.toml:15` declares `rust-version = "1.97"`, and the comment above it says the field states the toolchain the locked nixpkgs provides.

## Goals / Non-Goals

**Goals:**

- Every recorded claim the survey found stale for this scope is corrected in the same change as the code that makes the corrected claim true.
- The agenix target's declarations evaluate under agenix 0.16.0 and later on NixOS and nix-darwin, and the documented floor is 0.18.0.

**Non-Goals:**

- No check that evaluates generated declarations against a real agenix or sops-nix; the flake has neither as an input, and adding one is a separate decision (agenix survey §5 question 5).
- No refusal for a sops-nix JSON destination with a nested key; the version note documents the requirement instead.
- No change to how `safix enroll` registers a card with clan, and no write to clan's `vars.settings.recipients`; safix does not edit clan's files.
- No migration of agenix-rekey's per-host `rekeyed/` outputs and no agenix-rekey-specific plan fields.
- The other survey findings (installer ordering, tmpfs fallback, editor hardening, check staleness, hybrid recipients, FIDO2) belong to other changes.

## Decisions

### D2: an agenix early-user entry is emitted, and the flag is dropped

Decision taken by the project owner.
For `deploymentTarget = "agenix"`, `neededForUsers: true` stops being refused; the entry is emitted as a plain `age.secrets` entry and no `neededForUsers` field is written.
agenix installs every secret before user creation: under activation scripts through `users.deps = [ "agenixInstall" ]`, present before 0.16.0, and under sysusers or userborn since 0.16.0 through `before = [ "systemd-sysusers.service" ]` (de100bf).
The guarantee the flag asks for therefore holds without the flag, and writing it would name an option agenix does not have.
The root-ownership rule at `migrate.rs:550-563` already applies to every target and keeps applying; the safix-only mode rule stays safix-only.
The receipt keeps recording the plan's deployment metadata unchanged, so the request remains reviewable even though the declaration does not carry it.
The migration guide states agenix >= 0.18.0 as the floor for the agenix target, which covers the sysusers ordering, the darwin group fix and the userborn option guard (c37a8b4).

Alternative considered: keep refusing, as the survey's question 1 offered.
Rejected by the owner, because the refusal would block a semantic the target provides.

### The agenix emitter names a group for every entry

`deployment_fields` writes `group` for every agenix entry:

- the plan's `group`, as today;
- else the plan's `gid` as a string, as today;
- else, when the entry has an owner (from `owner`, or from `uid` rendered as the owner string), agenix 0.18.0's own default written with builtins and the owner as a literal: `(builtins.head (builtins.filter (u: u.name == "<owner>") (builtins.attrValues config.users.users) ++ [ { } ])).group or "0"`;
- else `"0"`, which is what every agenix release defaults to for the default owner `"0"`.

The expression keeps agenix 0.18.0's semantics on NixOS, where the owner's primary group is found by the user's `name` rather than the attribute name, and evaluates to `"0"` on nix-darwin under 0.16.0 and 0.17.0, whose own default would fail.
It uses `builtins` only, so the module header `{ config, ... }:` does not change; the safix and sops-nix targets are untouched.

Alternatives considered:

- Document "agenix >= 0.18.0 on nix-darwin" and emit nothing new.
  Rejected because the task asks for declarations that 0.16.0 and 0.17.0 evaluate, and the floor alone would leave a darwin user on 0.17.0 with a module that fails to evaluate.
- Emit `group = "0"` whenever the plan names none.
  Rejected because on NixOS it would replace the owner's primary group with root's, a semantic the migration would silently drop.
- `config.users.users."<owner>".group or "0"`.
  Rejected because it keys on the attribute name, which differs from agenix's lookup by `name` when the two differ.

### Clan fixture paths move to the `per-machine` shape

The two foreign host keys become `/run/secrets/per-machine/server/openssh/ssh.id_ed25519` and `/run/secrets/per-machine/server/openssh/ssh.id_rsa`, and the assertion at `:1506` follows.
`server` is a fixture hostname the checks already use, so no real hostname enters the tree.
The header comment at `:93-100` says the fixture follows clan's age-backend deployed path, `/run/secrets/<rel_dir>/<file>`, and the drill sentence at `:150-153` is reworded to the new paths.
The check's subject, the `/run/safix` exclusion, is unaffected; only the realism of the foreign placement changes.

### No live `age.nix` citation needs re-anchoring

The grep in Context returned none, so the re-anchoring step is a recorded negative rather than an edit.
The archived changes that cite `age.nix:218-227` and `:141` stay as history.

### The external-store wording names clan's store repository

Where the runtime comments and specs say clan commits in "the clan repository" or "its own repository", they say clan's store repository: the clan flake, or the directory `CLAN_AGE_STORE_DIR` names when `vars.settings.age.externalStore` is set, where clan commits nothing if that directory is not a git repository (`clan_lib/vars/secret_modules/age.py`, `_commit_store_files`, at `dbae210`).
The read-first comparison's rationale holds in both placements, because a re-encrypting write would otherwise produce a commit wherever the store lives.
`bridge-transfer` carries the same claim at two scenarios, so it receives a delta alongside `bridge-sync`.
The operator note goes in the clan section's "How it unlocks" subsection of `docs/guides/syncing-password-managers.md`, because the variable is part of what the run needs from its environment.

### The hardware keys guide states what the clan registration does not do

One sentence after `docs/guides/hardware-keys.md:33`: on a clan whose vars use the age backend, the `clan secrets users` registration grants no access to vars, the recipient has to be listed in clan's `vars.settings.recipients`, and safix does not write that setting.
No spec changes: `hardware-enrollment` requires the registration to go through clan's command, which remains true.

### The migration documentation gains version notes and an agenix-rekey subsection

The guide and `docs/reference/migration-plan.md` gain:

- agenix >= 0.18.0 as the floor for the agenix target;
- that agenix 0.16.0 renamed its rules file to `agenix-rules.nix`, which `safix migrate` does not read because the plan names every path;
- that safix's quickstart declarations file is also named `secrets.nix`, which agenix still auto-discovers as a legacy rules file, so running `agenix` in that directory picks up the wrong file;
- that a JSON destination with a `/`-nested key under the sops-nix target needs sops-nix at or after commit `311ef94` (2026-10-09), because earlier sops-nix fails at activation with "does not refer to a dictionary".

The agenix-rekey subsection says: the master-encrypted `rekeyFile` sources are what a plan names as sources; the per-host `rekeyed/` outputs are derived and are not migrated; and a source identity that is a plugin identity (`AGE-PLUGIN-YUBIKEY-…`) needs the plugin binary on `PATH` for the run, because `safix migrate` decrypts raw age through the `age` command (`crates/safix-core/src/ciphertext.rs:276-288`), which looks plugins up on `PATH`.
The guide's sentence at `:84` that every generated module carries the units and the early-user flag is narrowed: the safix and sops-nix targets carry them; the agenix target carries neither field, refuses hooks, and satisfies an early-user request through agenix's own ordering.

### The flake inputs move together, verified by the documented core set

`nix flake update` moves all six inputs in one commit.
Verification is the set `CONTRIBUTING.md` documents for local work: the four commands under "Before pushing" (`CONTRIBUTING.md:140-145`) in `nix develop`, the rust gates and the integration suite as nix builds, and an evaluation of every check on `x86_64-linux` without building them, which surfaces option renames without running the VM checks.
The cross-platform matrix stays with CI.

## Risks / Trade-offs

- [No check evaluates the declaration against agenix] The flake has no agenix input → the migrate integration test parses the whole declaration with `nix-instantiate --parse`, as `crates/safix/tests/custody.rs:52-56` already does for generated nix, and evaluates the emitted group expression with `nix-instantiate --eval --strict` over two user sets, one whose matching user has a `group` attribute and one whose user has none; the whole module is not evaluated, because its `builtins.path` needs a store.
- [A plan that relied on agenix's default group now names it] The expression is evaluated when the system is built, not when the migration runs, so a later change to the owner's primary group is still followed → no mitigation needed; it is agenix's own default lifted into the declaration.
- [The rust toolchain moves] nixpkgs unstable may now ship rustc 1.98 or later; the manifest comment at `Cargo.toml:8-14` says `rust-version` names the toolchain the locked nixpkgs provides → set `rust-version` to the new toolchain's minor version in the bump commit, and fix any new `clippy::pedantic` findings that `--deny warnings` turns into failures.
- [sops or age change behaviour] sops is at 3.13.3 and its main branch carries 39 unreleased commits (sops survey §1); age is at 1.3.1 → record the versions after the bump; if either moved, the integration suite is the detector, because its tests match the tools' refusal text.
- [bitwarden-cli moves] The pin is 2026.7.0 while `modules/flake/checks/bitwarden.nix:89-90` names 2026.8.0 as "this flake's pin" for a measurement → re-read that comment after the bump and correct the version it names, without restating a measurement that was not repeated.
- [home-manager renames or removes options] The home-scope module and several checks evaluate home-manager → the evaluation sweep in the verification step fails on a removed option and prints a rename warning; a rename is followed in the same commit.
- [advisory-db adds an advisory] `safix-rs-audit` turns red on a new advisory against a locked crate → the advisory is handled in the bump commit, by updating the crate or by a reasoned entry in `deny.toml`.
- [The survey's clan pin question] Whether the fleet's clan moves to `dbae210` is open (clan-core survey §5 question 1); the fixture change is correct either way, because it only changes the foreign placement's shape.

## Migration Plan

No consumer action is needed.
A plan that targets agenix and was refused for `neededForUsers` now succeeds; a plan that succeeded before produces the same declaration plus a `group` line per entry.
Rollback is reverting the commits; the lock bump is its own commit so it can be reverted alone.
