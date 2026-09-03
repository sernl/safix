# A secrets vault that is its own repository

## Amendment, 2026-09-03

This proposal is amended before adoption, per `docs/notes/research/zero-knowledge-vault.md`, option O3.
The vault layout becomes opaque by construction, and the recipient policy file never enters the vault, so a vault host or a reader holding only the vault learns none of the audience, key, or secret names the declaring repository holds.
The prior draft's `secrets-vault` scenario "A vault declared" asserted that the same audience still derives the same relative file name in vault mode; that assertion is reversed.
The prior draft's `recipient-policy` modification, which moved the committed policy file into the vault, is reversed: the committed file stays at the declaring root in every case, and only a disposable, uncommitted rendering of its creation rules ever reaches the vault working tree, for the two sops invocations that need one.
Scope is vault-only: the in-repository layout, used when no vault is declared, is unchanged by this amendment.
No code changes land in this proposal; this remains a design amendment only.

## Why

Every ciphertext path, `.sops.yaml`, and generated public value safix writes today is rooted at `self` — the declaring flake's own source (`modules/flake/safix/default.nix:47`) — so a consumer who wants encrypted material and declarations under different access control, different collaborators, or a different retention policy has no way to express it: the two are structurally the same repository.
The operator has settled that a separate repository may hold the vault, reached as a flake input and accepted at the cost of a lock bump per secret (contract D3), landing after `support-plain-nix-consumers` has established the flakeless entrypoint the same program depends on (contract D7).
This change is the mechanism that makes D3 real: the declaring option, the two roots the runtime carries, and what replaces the single-repository commit atomicity every write path relies on today.

## What Changes

- Add `flake.safix.vault`, `nullOr (submodule { root; namingKey; })`, defaulting to `null`.
  `root` carries the meaning the unamended draft gave the option itself: when set, every audience file's `sopsFile` resolves rooted there rather than at the declaring flake's own source.
  `namingKey` is a string — never a path — of at least 64 lowercase hexadecimal characters, minted once by the operator (`openssl rand -hex 32`, documented at the option); evaluation refuses a vault with no naming key, a short one, or a non-hexadecimal one.
  The option's own description and this document both state that the naming key is visible to anyone who can evaluate the declaring flake, because nix has no keyed hash and the key must itself be an evaluation-time value: it hides names only from the vault host and from a reader holding only the vault, never from the store or the declaring repository.
- The command-line runtime resolves two independent repository roots instead of one: a declaration root, discovered from the repository it runs inside exactly as today, and a vault root, named by the operator through a new environment variable, defaulting to equal the declaration root when no vault is declared.
- Every ciphertext document, public output, and generator definition record the resolver places — never the recipient policy, which stays at the declaration root (see below) — moves to the vault root under an opaque name: a `sha256` hash of the naming key, a domain-separating tag, and today's readable name, computed once by `opaqueOf` beside `audienceFileOf` in `resolve.nix`.
  Each document's top-level key is opaque the same way, so a vault directory listing, a document's own keys, and a definition record's own path name nothing.
  Catalogue, user, and group declarations, the `.nix` scaffolds the runtime generates for them, `--inputs-from`, the flake evaluation target, and git authorship for every commit stay at the declaration root, exactly as the unamended draft stated.
  When no vault is declared, every name is exactly today's readable name; the in-repository layout is unedited by this amendment.
- The recipient policy, `.sops.yaml`, is committed at the declaring root in every case, reversing the unamended draft's decision to move it with the ciphertext.
  For the two sops invocations that need creation rules to reach vault-rooted documents — `encrypt` and `updatekeys` — the runtime renders a second, disposable copy of the rules into the vault's own working tree (sops matches `path_regex` relative to the config file it reads), passes it with `--config`, sweeps it through the existing scratch registry on every exit path, and the vault carries a committed `.gitignore` entry so the scratch file can never itself be committed.
  `decrypt` and `set` need no creation rules and are unaffected.
- Fourteen runtime touch points across `Workspace`, the sops driver, the git driver's callers, the scratch sweep floor, and three `.current_dir` sites are re-attributed between the two roots; each disposition is recorded in `design.md`.
- The single-commit atomicity every write path relies on today — one `git commit` naming a scaffold, a regenerated policy, and re-wrapped ciphertext together — no longer exists for an operation that spans both roots.
  It is replaced with an ordered two-commit sequence, vault root first, a preflight that checks both roots' cleanliness before either is written, and a named refusal for the state in which the vault commit landed and the declaration commit did not, stated to be safe to re-run.
- A vault root that is not the top level of a git repository is refused before anything is written, naming the path and what is missing.
- After a vault-root commit, the command discloses that the change is invisible to any consuming build until the declaring flake's lock entry for the vault is updated, naming the update command when the runtime can determine which input the vault is.
- **BREAKING** (within this repository's own use, not yet a public interface break): a consumer who inspects `.sops.yaml` or a governed ciphertext file's path relative to the declaring flake's own root, rather than through the resolved paths `safix.lib.placements` and `safix.lib.governedFiles` already report, finds ciphertext, public outputs, and definition records moved and renamed once a vault is declared — opaquely renamed, not merely relocated.
  No consumer of this package currently does so; the migration note in `design.md` records it for completeness.

Not in scope: the vault having its own flake, `mkVault`, or any evaluation of the vault's own nix expressions — the vault is a plain git-fetched tree (`flake = false` input), never itself evaluated as a flake, so `support-plain-nix-consumers`' `mkVault` (contract C1) is not a dependency of this mechanism.
Also not in scope: verifying the vault's local clone is in sync with its remote before writing, and any change to `secret-installation`, `secret-consumption`, or `secret-catalogue` — each was checked against this change's touch points and found to state no requirement that becomes false; the reasoning for each is recorded in `design.md`.
Also not in scope, permanently rather than deferred: hiding sops recipient public keys (`sops.age[].recipient`).
Every sops document lists them in the clear and `updatekeys` depends on reading them, so hiding them means leaving the sops document format entirely for age-native documents, which `sops-nix` cannot consume; this is option O4 in the research note, rejected there and not reopened here.

## Capabilities

### New Capabilities

- `secrets-vault`: the declaring option, the two-root model the runtime carries, what lands at which root, the git-repository requirement on the vault, the commit ordering that replaces single-commit atomicity, the preflight that checks both roots before either is written, the half-landed-state refusal and its safety to re-run, the lock-bump disclosure, the naming-key declaration and its refusals, the opaque derivation of every vault-rooted name, the scratch creation-rules rendering, and the residual metadata a vault host still sees.

### Modified Capabilities

- `recipient-policy`: the requirement "The policy file is generated and never hand-edited" gains vault-mode scenarios for the scratch creation-rules rendering the runtime produces for `encrypt` and `updatekeys`; the requirement itself — that the committed file lives in the repository the encryption tool reads it from — is unedited, because the committed file always lives at the declaring root, in every case, including vault mode.
- `secret-custody`: the requirement "The audience picks the file, and one audience gets one file", scenario "A multi-member audience", gains a vault-mode scenario stating that the file's name is opaque rather than a sorted member list when a vault is declared.
- `public-outputs`: the requirement "The plaintext store is separable from the ciphertext tree by path prefix" gains a vault-mode scenario for opaque leaves; prefix separation itself is unedited.

## Impact

Affected code (design only; no code changes land in this proposal):

- `modules/flake/safix/default.nix:47` — the `root` binding flips from `self` to `cfg.vault.root` when declared; `bound` also threads `namingKey` alongside `root`.
- `modules/flake/safix/options.nix` — gains the `vault` option, typed `nullOr (submodule { root; namingKey; })`.
- `modules/flake/safix/default.nix` — gains a `vaultDeclared` boolean under `flake.safix.lib`, read by the command-line runtime to cross-check against the operator-named vault root; `violations` gains a third concatenated list for the naming-key refusals.
- `crates/safix-core/src/workspace.rs` — `Workspace` carries a second root; `at`/`discover` resolve and cross-validate it; `absolute`/`read_relative` gain vault-rooted counterparts.
- `crates/safix-core/src/sops/mod.rs` — no change to the driver itself; its callers pass the vault root where they pass the declaration root today.
- `crates/safix-core/src/git.rs` — no change to the driver itself; every `commit_written_files` caller that spans both roots calls it twice, vault first.
- `crates/safix-core/src/fix.rs`, `adduser.rs`, `group.rs`, `check.rs`, `enroll/mod.rs`, `enroll/proof.rs`, `set.rs`, `bridge.rs`, `sync.rs`, `generate.rs`, `delegation.rs` — the write, read, and commit sites named in `design.md`'s touch-point table.
- `crates/safix-core/src/scratch.rs` — the sweep floor becomes two floors instead of one.
- `crates/safix-core/src/nix.rs` — a new `Attribute::VaultDeclared` variant; `--inputs-from` and the flake evaluation target are unchanged.
- `modules/flake/safix/resolve.nix` — gains `opaqueOf` and `opaqueKeyOf` beside `audienceFileOf`; `audienceFileOf`'s and `publicFileOf`'s vault-mode call sites route through them; `placementsIn`'s `key` field and `selectFor.forMachine`'s `sopsKey` field both apply `opaqueKeyOf` in vault mode; `placementsIn` gains a `definitionRecord` field, opaque in vault mode and `null` otherwise.
- `modules/flake/safix/policy.nix` — gains `renderVaultRules`, a second rendering of the same `plan` value `renderPlan` already renders, carrying no header, no `Audience:` comments, and no anchors, with `path_regex` matching the opaque ciphertext name literally.
- `crates/safix-core/src/nix.rs` — a new `Attribute::VaultCreationRulesText` variant beside `Attribute::PolicyText`.
- `crates/safix-core/src/model.rs` — `Placement` gains `definition_record: Option<String>`.
- `crates/safix-core/src/definition.rs` — `record_path` reads `placement.definition_record` when present (vault mode) and falls back to today's `audience_directory` derivation otherwise (unedited).

Affected checks: a new `modules/flake/checks/vault.nix` (or an addition to an existing check file, decided at implementation time) carrying the `root` flip, the `vaultDeclared` output, the naming-key format refusal, the opaque-derivation drill, and every refusal named above.
Every guarantee this change states gets a severity drill in `tasks.md`.
