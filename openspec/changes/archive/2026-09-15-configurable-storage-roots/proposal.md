# The three storage roots become the consumer's to name

## Why

Every path safix places is rooted at one of three string literals compiled into the resolver and, for one tree, into the Rust runtime as well: `secrets/safix`, `public/safix`, and `state/safix/definitions` (`modules/flake/safix/resolve.nix:523-526`, `:592-597`; `crates/safix-core/src/definition.rs:71`).
A consumer whose repository already owns `public/` for a static site, or who wants everything safix writes under one directory their backup policy and `.gitignore` name once, has no way to say so, and the three trees are disjoint today only because three literals happen not to overlap — asserted in four scattered places (`crates/safix-core/src/public.rs:107-111`, `:172-175`, `definition.rs:548-550`, `modules/flake/checks/vault.nix:415-417`) rather than checked as an invariant.

## What Changes

- Add `flake.safix.storage`, a submodule with three independent options, each a repository-relative path string carrying today's spelling as its default: `encrypted` (`"secrets/safix"`), `plaintextOutputs` (`"public/safix"`), `generatorRecords` (`"state/safix/definitions"`).
  Three independent roots rather than one parent with fixed subtrees, because a consumer who wants one parent expresses it by setting all three under it (`.safix/encrypted`, `.safix/plaintext-outputs`, `.safix/generator-records`) while the converse — placing one tree somewhere the others are not, such as a public tree a static-site build can read — is inexpressible under a single-parent option.
  Defaults preserved means no existing consumer migrates and not one of the roughly ninety fixture and snapshot literals in this repository changes.
- Each option's description states, in plain words, what its tree holds and what a reader may assume about it, because those three propositions are exactly what an operator takes responsibility for when they rename a tree.
  `encrypted`'s says that everything under it is ciphertext without qualification, since that is the sentence a backup policy is written against.
  The one-parent example and the caveat that `rg` and `fd` skip dot-directories by default are documented on the options and in the README.
- Add an evaluation refusal for a malformed root: absolute, empty, containing a `..` component, or carrying a trailing slash.
- Add an evaluation refusal for pairwise overlap: no two of the three roots may be equal, and none may be a path-prefix of another at a `/` component boundary.
  The refusal names both options and the offending pair.
  This converts today's accidental disjointness into a checked invariant that holds for every configuration, including the default one.
- Every site in `modules/flake/safix/` that today writes a literal prefix derives it from the configured roots instead: `audienceFileOf`, `publicFileOf`, `checks.nix`'s `catchAllProbes`, `policy.nix`'s two committed header paragraphs, `default.nix`'s `extraGovernedFiles` example, and `types.nix`'s `secret = false` description.
  `catchAllProbes` in particular must be derived rather than left literal: left literal, the catch-all check would probe trees nothing uses and stop probing the real ones — a silent loss of the check.
- **BREAKING** (within this repository's own `[Unreleased]` surface, not a released interface): in vault mode the opaque-name hash input becomes root-relative.
  `opaqueOf` is fed `users/alice/secrets.yaml` rather than `secrets/safix/users/alice/secrets.yaml`, keeping the existing domain-separating tag, so that renaming a readable root never re-hashes a vault name and never turns a rename into a decrypt-and-re-encrypt.
  This is a one-time break of every opaque file name, public leaf, definition record and in-document key; `safix check` reports every affected document as pending relocation and `safix fix` relocates.
  The vault feature is unreleased, so no released consumer is affected, and this is the last moment to take the break without paying for it twice.
- Delete the runtime's second implementation of the layout.
  Nix populates `definitionRecord` on every placement, not only in vault mode, so `crates/safix-core/src/definition.rs`'s `record_path` collapses to `placement.definition_record.clone()` and `PREFIX`, `logical_record_path` and `audience_directory` are removed.
  `crates/safix-core/src/public.rs`'s `PREFIX`, `LEAF` and `is_public_path` are deleted with their tests and their property module: `is_public_path` has no production call site — the runtime forks on `placement.public.is_some()` (`generate.rs:373`, `edit.rs:176`) — and it is additionally wrong in vault mode, where `public/<hash>` has neither the prefix nor the `value` leaf.
- Rewrite the README sections that explain the three trees against the new option names, keeping the argument for why they are separate, and add what a consumer's own `.gitignore` or backup rule should say once the roots are theirs to name.

Not in scope: a `storage.previous` option, a previous-root `Finding`, or any automatic detection that a governed file sits under a spelling a root used to have.
No record of a previous spelling exists anywhere safix can read, and inventing an option to carry one buys a migration that a `git mv` followed by `safix fix` already performs; `safix check` already reports any governed file `.sops.yaml` no longer names, which is the signal a half-finished rename produces.
Also not in scope: making the vault's own three buckets (`secrets/`, `public/`, `state/`, flat at the vault root) configurable.
The vault is a dedicated repository whose root is the vault root, so a second layer of naming there buys nothing, and with the hash input made root-relative the buckets become genuinely independent of the declaration-side spelling.
Also not in scope: changing any default.
A breaking default would buy aesthetics and cost every consumer a migration and this repository its entire fixture corpus.

## Capabilities

### New Capabilities

None.
Every requirement this change adds belongs to a capability that already owns the tree it is about.

### Modified Capabilities

- `public-outputs`: the requirement "The plaintext store is separable from the ciphertext tree by path prefix" is rewritten from "a top-level prefix distinct from the one holding encrypted material" to "a distinct, configurable root that does not overlap the encrypted tree's, with non-overlap refused at evaluation"; it gains the three-option declaration, the defaults, the malformed-root refusal, the pairwise-overlap refusal, and the requirement that each option's description states what its tree holds.
  The "reason is recorded" scenario is restated: the tree holding ciphertext must be named for that, and the naming is the consumer's.
- `secret-generators`: the requirement "Minting records the definition it minted under", scenario "The record does not live where its meaning would lie", is rewritten against the configured roots rather than against literal path meanings, and the requirement gains a scenario stating that the record's path is computed in exactly one place — the resolver — and carried to the runtime on every placement rather than reconstructed there.
- `secrets-vault`: the requirement "Every vault-rooted name is opaque" gains the statement that the hash input is the logical name relative to its configured root, so a vault name is a function of the entry's identity and never of the readable tree's spelling.
- `recipient-policy`: the requirement "The policy file is generated and never hand-edited" gains a scenario stating that the header's worked examples name the configured roots, because a committed header documenting a layout the repository does not have is a false statement in a file every reviewer reads.

## Impact

Affected nix:

- `modules/flake/safix/options.nix` — gains the `storage` submodule option beside `vault` (`options.nix:242-333` is the naming precedent).
- `modules/flake/safix/resolve.nix` — `audienceFileOf` (`:523-526`), `publicFileOf` (`:592-597`), `secretsFileOf` (`:615-616`), `publicFileOfVault` (`:622-623`), `definitionRecord` (`:698-699`), and the rationale comments at `:297-298`, `:586-591`, `:611-614`; a new `storageViolations` beside `vaultViolations` in the exports list (`:2418`); `placementsIn` emits `definitionRecord` unconditionally.
- `modules/flake/safix/policy.nix` — the two committed header paragraphs (`:56-60`, `:84-89`); `renderVaultRules`' literal `^secrets/` (`:253-256`) stays, per the vault-buckets non-goal.
- `modules/flake/safix/checks.nix` — `catchAllProbes` (`:235-242`) derived; the comment at `:271-275`.
- `modules/flake/safix/default.nix` — `extraGovernedFiles`' example (`:75`); `storageViolations` concatenated into `violations` (`:346-368` region).
- `modules/flake/safix/types.nix` — the `files.<n>.secret` description (`:103`).
- `modules/flake/checks/policy.nix` — `rulesWellFormed`'s literal `^secrets/` prefix assertion (`:169-171`); `modules/flake/checks/vault.nix` — `prefixesStayDisjoint`'s literals (`:415-417`); `modules/flake/checks/fixture-policy.yaml` — the regenerated header prose (`:22-26`, `:50-56`).

Affected Rust:

- `crates/safix-core/src/definition.rs` — `PREFIX` (`:71`), `record_path` (`:145-155`), `logical_record_path` (`:166-175`), `audience_directory` (`:177-195`), the module doc (`:10-27`), and the tests at `:497-608`.
- `crates/safix-core/src/public.rs` — `PREFIX` (`:45-46`), `LEAF` (`:48-49`), `is_public_path` (`:97-100`), the tests at `:107-123`, and the property module (`:126-191`).
- `crates/safix-core/src/relocation.rs`, `check.rs`, `fix.rs` — unchanged in shape; the vault relocation they already implement is what carries the one-time opaque-name break.
- `crates/safix/src/usage.rs:326-331` — `check`'s help text naming `state/safix/definitions/`.

Affected docs: `README.md:119-121`, `:190-192`, `:223-225`, `:241-243`, `:307-309`, `:466-476`, `:533-550`, `:577-579`, `:1167-1171`, `:1203-1208`; a new `CHANGELOG.md` `[Unreleased]` entry, with the historical entries at `:269-274`, `:333-341`, `:485-489` left as history.

Every guarantee this change states gets a severity drill in `tasks.md`.
