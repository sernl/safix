# A secrets vault that is its own repository

## Why

Every ciphertext path, `.sops.yaml`, and generated public value safix writes today is rooted at `self` — the declaring flake's own source (`modules/flake/safix/default.nix:47`) — so a consumer who wants encrypted material and declarations under different access control, different collaborators, or a different retention policy has no way to express it: the two are structurally the same repository.
The operator has settled that a separate repository may hold the vault, reached as a flake input and accepted at the cost of a lock bump per secret (contract D3), landing after `support-plain-nix-consumers` has established the flakeless entrypoint the same program depends on (contract D7).
This change is the mechanism that makes D3 real: the declaring option, the two roots the runtime carries, and what replaces the single-repository commit atomicity every write path relies on today.

## What Changes

- Add `flake.safix.vault`, `nullOr path`, defaulting to `null`.
  When set, every audience file's `sopsFile` resolves rooted at the vault rather than at the declaring flake's own source — a one-line flip of the `root` binding at `modules/flake/safix/default.nix:47`, from `self` to `cfg.vault`.
- The command-line runtime resolves two independent repository roots instead of one: a declaration root, discovered from the repository it runs inside exactly as today, and a vault root, named by the operator through a new environment variable, defaulting to equal the declaration root when no vault is declared.
- Every artifact the encrypting backend or the resolver's ciphertext-adjacent output touches — governed ciphertext files, `.sops.yaml`, generated public values, and generator definition records — moves to the vault root.
  Catalogue, user, and group declarations, the `.nix` scaffolds the runtime generates for them, `--inputs-from`, the flake evaluation target, and git authorship for every commit stay at the declaration root.
- Fourteen runtime touch points across `Workspace`, the sops driver, the git driver's callers, the scratch sweep floor, and three `.current_dir` sites are re-attributed between the two roots; each disposition is recorded in `design.md`.
- The single-commit atomicity every write path relies on today — one `git commit` naming a scaffold, a regenerated policy, and re-wrapped ciphertext together — no longer exists for an operation that spans both roots.
  It is replaced with an ordered two-commit sequence, vault root first, a preflight that checks both roots' cleanliness before either is written, and a named refusal for the state in which the vault commit landed and the declaration commit did not, stated to be safe to re-run.
- A vault root that is not the top level of a git repository is refused before anything is written, naming the path and what is missing.
- After a vault-root commit, the command discloses that the change is invisible to any consuming build until the declaring flake's lock entry for the vault is updated, naming the update command when the runtime can determine which input the vault is.
- **BREAKING** (within this repository's own use, not yet a public interface break): a consumer who inspects `.sops.yaml` or a governed ciphertext file's path relative to the declaring flake's own root, rather than through the resolved paths `safix.lib.placements` and `safix.lib.governedFiles` already report, finds them moved once a vault is declared.
  No consumer of this package currently does so; the migration note in `design.md` records it for completeness.

Not in scope: the vault having its own flake, `mkVault`, or any evaluation of the vault's own nix expressions — the vault is a plain git-fetched tree (`flake = false` input), never itself evaluated as a flake, so `support-plain-nix-consumers`' `mkVault` (contract C1) is not a dependency of this mechanism.
Also not in scope: verifying the vault's local clone is in sync with its remote before writing, and any change to `secret-custody`, `secret-installation`, `secret-consumption`, `public-outputs`, or `secret-catalogue` — each was checked against this change's touch points and found to state no requirement that becomes false; the reasoning for each is recorded in `design.md`.

## Capabilities

### New Capabilities

- `secrets-vault`: the declaring option, the two-root model the runtime carries, what lands at which root, the git-repository requirement on the vault, the commit ordering that replaces single-commit atomicity, the preflight that checks both roots before either is written, the half-landed-state refusal and its safety to re-run, and the lock-bump disclosure.

### Modified Capabilities

- `recipient-policy`: the requirement that the policy file "SHALL be committed to the consumer's repository because the encryption tool reads it from the filesystem" is no longer accurate as stated once a vault exists — the encryption tool reads it from whichever repository it runs in, which is now the vault root when one is declared.

## Impact

Affected code (design only; no code changes land in this proposal):

- `modules/flake/safix/default.nix:47` — the `root` binding flips from `self` to `cfg.vault` when declared.
- `modules/flake/safix/options.nix` — gains the `vault` option.
- `modules/flake/safix/default.nix` — gains a `vaultDeclared` boolean under `flake.safix.lib`, read by the command-line runtime to cross-check against the operator-named vault root.
- `crates/safix-core/src/workspace.rs` — `Workspace` carries a second root; `at`/`discover` resolve and cross-validate it; `absolute`/`read_relative` gain vault-rooted counterparts.
- `crates/safix-core/src/sops/mod.rs` — no change to the driver itself; its callers pass the vault root where they pass the declaration root today.
- `crates/safix-core/src/git.rs` — no change to the driver itself; every `commit_written_files` caller that spans both roots calls it twice, vault first.
- `crates/safix-core/src/fix.rs`, `adduser.rs`, `group.rs`, `check.rs`, `enroll/mod.rs`, `enroll/proof.rs`, `set.rs`, `bridge.rs`, `sync.rs`, `generate.rs`, `delegation.rs` — the write, read, and commit sites named in `design.md`'s touch-point table.
- `crates/safix-core/src/scratch.rs` — the sweep floor becomes two floors instead of one.
- `crates/safix-core/src/nix.rs` — a new `Attribute::VaultDeclared` variant; `--inputs-from` and the flake evaluation target are unchanged.

Affected checks: a new `modules/flake/checks/vault.nix` (or an addition to an existing check file, decided at implementation time) carrying the `root` flip, the `vaultDeclared` output, and every refusal named above.
Every guarantee this change states gets a severity drill in `tasks.md`.
