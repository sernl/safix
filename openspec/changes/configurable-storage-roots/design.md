# Design: the three storage roots become the consumer's to name

## Context

See `proposal.md` — Why.
Four facts about the current worktree fix the shape of this change; each was read rather than assumed.

The resolver computes the ciphertext path and the public path for every placement, but computes the definition-record path only in vault mode (`modules/flake/safix/resolve.nix:523-526`, `:592-597`, `:698-699`).
In readable mode the record path is constructed in Rust, from `definition::PREFIX` plus `placement.owner` or `audience_directory(placement.file)` (`crates/safix-core/src/definition.rs:71`, `:145-155`).
That is a second implementation of the layout in the other language, and it is the one place the "one implementation of the layout" rule stated at `crates/safix-core/src/public.rs:30-34` is not honoured.

The recipient policy's readable-mode rules are already derived: `pathRegex = "^${a.dir}/[^/]*\\.yaml$"` (`modules/flake/safix/policy.nix:206`), where `dir` comes from `builtins.dirOf (audienceFileOf audience)` (`resolve.nix:543`).
A new root therefore produces a new regex with no code change, the drift check compares generated against committed so both sides move together, and `safix fix` writes the new file.
What is *not* derived is the header prose committed into every consumer's `.sops.yaml` (`policy.nix:56-60`, `:84-89`), which spells `secrets/safix/shared/<a>,<b>/` and `nested/secrets/safix/users/alice/x.yaml` as literals.

Every vault-mode name hashes the full readable path, which today begins with the literal root: `secretsFileOf ns a = "secrets/" + sha256(namingKey|"secrets"|audienceFileOf a) + ".yaml"` (`resolve.nix:615-616`), and `opaqueKeyOf` closes over the same `logicalFile` (`:625-631`, used at `:686`).
Renaming a readable root therefore changes every vault filename *and* every in-document key, and because sops binds each leaf's ciphertext to its key path as associated data, a key rename is a re-encryption rather than a rewrite.

Safix writes exactly one `.gitignore`, at the *vault* root, covering only `.sops-vault-rules.yaml` (`crates/safix-core/src/workspace.rs:485-491`, `fix.rs:479-490`).
It writes nothing at the declaration root.
So no `.gitignore` regenerates as part of this change; what a consumer's own ignore or backup rule should say becomes a documentation obligation instead.

## Goals / Non-Goals

**Goals:**

Make each of the three trees a declared, repository-relative root with today's spelling as its default, so that no existing consumer migrates and no fixture literal in this repository changes.
Turn the trees' disjointness from an accident of three string literals into a refusal that holds for every configuration.
Leave exactly one implementation of the layout, in the resolver.
Make a vault name a function of an entry's identity rather than of the readable tree's spelling, once, before any consumer depends on the current derivation.

**Non-Goals:**

A single-parent option, with or without overridable subtree names.
Three independent roots subsume the single-parent ergonomics by configuration (§S1), and a subtree-rename layer buys a capability nobody has asked for while reopening the overlap question.
Making the vault's own flat buckets configurable (§S6).
Any automatic migration from a previous root spelling (§S7).
Changing `secret-custody`'s audience-named-directory requirement or `recipient-policy`'s anchoring requirements: both were read against this change's touch points and are already root-agnostic — `secret-custody/spec.md:39-53` speaks of "a directory named for its members", `recipient-policy/spec.md:66-94` of anchors and directory levels, and neither names a prefix.

## Decisions

### S1. Three independent roots, not one parent

`flake.safix.storage` is a submodule with `encrypted`, `plaintextOutputs` and `generatorRecords`, each `lib.types.str`, defaulting to `"secrets/safix"`, `"public/safix"` and `"state/safix/definitions"`.

A consumer who wants one parent writes three strings sharing it:

```nix
flake.safix.storage = {
  encrypted        = ".safix/encrypted";
  plaintextOutputs = ".safix/plaintext-outputs";
  generatorRecords = ".safix/generator-records";
};
```

and gets one `.gitignore` line, one backup rule and one directory to move — the exact ergonomics a single-parent option would mandate.
The converse does not hold: no single-parent option can express "the public tree lives where a static-site build can read it".

**Alternative rejected**: one `storage.root` with fixed subtrees `<root>/encrypted`, `<root>/plaintext-outputs`, `<root>/generator-records`.
Its default cannot be `"secrets"` without re-breaking the promise that a path named for secrets holds only ciphertext, so its default is a forced migration for every existing consumer and a rewrite of roughly ninety fixture and snapshot literals in this repository — converting a scoped change into whole-suite churn where every diff line must be read to confirm it is only a rename.
It also loses the top-level `secrets/` signpost: someone grepping an unfamiliar repository for `secrets/` finds nothing.

**Alternative rejected**: `storage.root` plus `storage.subtrees.{encrypted,plaintextOutputs,generatorRecords}`.
Strictly more surface than the parent option with no capability it lacks except renaming a subtree, which reopens the overlap question the fixed-subtree form closes by construction.
Under this repository's own discipline — complexity pays rent in invariants or in value to the consumer — it pays a second option's rent to buy a rename nobody needs, and it still cannot place a tree outside the parent.
Its one real argument, that `root = "."` with today's spellings as subtrees reproduces today's layout, is this decision wearing a parent it does not use.

### S2. The option names, and why the tree names change

| tree | option | rejected | why |
|---|---|---|---|
| ciphertext | `storage.encrypted` | `secrets`, `ciphertext`, `vault` | `encrypted` names the invariant the checks actually enforce; `secrets` names the subject matter, which is also what the *plaintext* outputs are about; `vault` is taken by `flake.safix.vault` |
| public outputs | `storage.plaintextOutputs` | `public`, `plaintext`, `published`, `outputs` | `public` is a security-property word used as a tree name and reads as "public key"; `plaintext` alone invites "so the other tree's records are not plaintext?"; `outputs` alone collides with flake outputs. `plaintextOutputs` says both halves: in the clear, and holding generator outputs |
| definition records | `storage.generatorRecords` | `state`, `definitions`, `records` | `state` is today's name and says nothing to a newcomer, which is why `README.md:547` has to explain it in a sentence; `definitions` reads as "where I define things", the opposite of what it holds |

The namespace is `storage` rather than `paths` or `layout`: `paths` is what the resolver's *outputs* are (`outputPathOf`, `publicPathsOf`, `sopsFile`), so reusing it for inputs invites "is `safix.paths` where they are, or where I want them?"; `layout` reads as shape rather than location.
`storage` says where things are kept, is unused elsewhere in the option surface, and reads correctly both in a declaration and in a refusal message: "`flake.safix.storage.encrypted` and `flake.safix.storage.plaintextOutputs` overlap".

Each description states what the tree holds and what a reader may assume about it, because those propositions are what the operator takes responsibility for when they rename the tree, and because a newcomer must not have to read the README to set the option.
The three propositions, stated today at `public.rs:22-28`, `definition.rs:17-26` and `README.md:474`, are:

- `encrypted` — everything under it is ciphertext, without qualification; one file per distinct audience; the directory states the audience without opening the file.
- `plaintextOutputs` — generator outputs declared not secret, written in the clear, readable at evaluation, never handed to the encrypting backend and never given a creation rule.
- `generatorRecords` — one plaintext digest per generated value, recording which generator definition minted it; no value and no derivative of a value, which is what licenses committing it in the clear.

### S3. Why the trees stay separate once the names are the consumer's

The proposition "everything under `secrets/` is ciphertext" was carried by the *name*, not by any mechanism: nothing in safix behaves differently, and the argument at `public.rs:22-28` is precisely that a backup rule, an `rsync --exclude`, an `rg` invocation and a reviewer all apply that proposition to a path component called `secrets`.
Once the root is the consumer's, a root named `.vault/` or `store/` carries no promise at all — so the promise moves to the subtree name, and `encrypted` carries it more precisely than `secrets` ever did, because `secrets` describes the subject matter while `encrypted` describes the representation, which is the actual invariant.

What remains load-bearing and mechanically checkable is that the three trees do not overlap.
That becomes S4.

### S4. Two evaluation refusals: well-formedness and pairwise overlap

A new `storageViolations` beside `vaultViolations` in `resolve.nix`'s exports (`resolve.nix:2418`), concatenated into `violations` in `default.nix` the way `resolve.vaultViolations cfg.vault` already is, refuses:

1. **Malformed root** — a root that is empty, absolute (begins with `/`), carries a `..` component, or ends in `/`.
   Each is refused naming the option and the value.
   The trailing-slash case is refused rather than normalised because a silently normalised value makes the overlap comparison in (2) depend on normalisation the operator cannot see, and because every path in this system is joined with an explicit `/`.
2. **Pairwise overlap** — no two roots may be equal, and none may be a prefix of another at a `/` component boundary.
   Comparison is on component boundaries, not raw string prefixes, so `secrets/safix` and `secrets/safix-public` are accepted (neither contains the other as a directory) while `secrets/safix` and `secrets/safix/pub` are refused.
   The refusal names both options and both values.

**Alternative rejected**: keeping the four scattered disjointness assertions (`public.rs:107-111`, `:172-175`, `definition.rs:548-550`, `modules/flake/checks/vault.nix:415-417`) and adding a fifth for the configured case.
Those four assert a property of three constants; once the constants are inputs, an assertion about the constants says nothing about a consumer's configuration.
They collapse into one refusal plus one check that perturbs the option and observes the refusal.

**Alternative rejected**: refusing at check time rather than at evaluation.
An overlapping configuration would then place a plaintext output inside a tree a creation rule governs, and the failure would surface at `nix flake check` rather than at the point the operator wrote the value.
Evaluation refusal is also what makes the invariant free for the default configuration: it holds without anyone running a check.

### S5. The vault hash input becomes root-relative

`opaqueOf namingKey tag logicalPath` keeps its shape and its four tags.
What changes is `logicalPath`: the four call sites feed the logical name *relative to its configured root* rather than the full readable path.

```
secretsFileOf ns a = "secrets/" + sha256(namingKey|"secrets"|"users/alice/secrets.yaml") + ".yaml"
```

rather than today's `"secrets/safix/users/alice/secrets.yaml"` as the hashed input.
`opaqueKeyOf` closes over the same root-relative logical file, so the key derivation moves with it and `placementsIn`'s `key` and `selectFor.forMachine`'s `sopsKey` continue to agree bit for bit.

The tags stay: they are what keeps a ciphertext file and a public output that share one readable identity from colliding, and root-relativity does not make them redundant — it makes them the only separator, since two roots' relative names can now coincide.

**Alternative rejected**: deferring this.
Every `storage.*` change in vault mode would then re-hash every name, and because sops binds a leaf's ciphertext to its key path as associated data, an in-document key rename is a decrypt-and-re-encrypt of every leaf.
A root rename would be a re-encryption forever rather than a rename, and the one-time break would still have to be taken the first time anyone renamed a root — twice the cost, later, with released consumers.

**Alternative rejected**: keeping the full path in the hash and forbidding root changes in vault mode.
That is a refusal whose only justification is an implementation detail, and it makes the vault the one mode in which the new option does not work.

The break is one-time and total: every opaque file name, public leaf, definition record and in-document key changes.
§"Migration Plan" states how it is taken.

### S6. The vault's own buckets stay literal

`secrets/`, `public/` and `state/`, flat at the vault root, are not configurable, and `policy.nix:253-256`'s `^secrets/` in `renderVaultRules` stays a literal.

The vault is a dedicated repository whose root *is* the vault root (`options.nix:247-262`, `workspace.rs:549-565`), so a second naming layer inside it buys nothing a consumer can use.
With S5 in place the buckets are independent of the declaration-side spelling, so there is no coupling left to configure away.
A consumer who wants different vault buckets can say so later; nothing in this change forecloses it.

### S7. Migration is a `git mv`, not an option

There is no `storage.previous`, no `Finding` for a leaf sitting under a previous root spelling, and no detection of one.

Nothing in the repository records what a root used to be called.
Inventing an option to carry that record means a consumer sets it, runs one `safix fix`, and deletes it — three steps to replace a `git mv` and one `safix fix`, and a new nullable option on the surface forever to serve a one-command operation.
The half-finished state is already visible: `safix check` reports every governed file `.sops.yaml` no longer names, and the recipient-policy drift check reports the regenerated policy against the committed one.

**Alternative rejected**: generalising `Finding::VaultRelocationPending { file }` into `Finding::RelocationPending { from, to }` and adding a `previous` option to populate `previousFile`/`previousPublic`/`previousRecord` on every placement, mirroring how `logicalFile` and friends are populated in vault mode.
The machinery exists and would work — `relocation.rs:113-129`'s `leaves_by` is already generic over `(opaque, logical)` pairs and needs only a different selector.
It is rejected on cost, not on feasibility: a readable-mode root rename changes neither ciphertext nor keys nor recipients (a sops document does not embed its own path), so every move is a plain rename that `git mv` performs correctly and observably, and building a safix-side mechanism for it means carrying a second finding, a second option, a second set of placement fields and a second relocation branch to automate one shell command.

### S8. Nix emits `definitionRecord` on every placement; Rust stops constructing paths

`placementsIn` sets `definitionRecord` unconditionally, computed from `storage.generatorRecords` the same way `audienceFileOf` and `publicFileOf` compute theirs, rather than only when `namingKey != null` (`resolve.nix:698-699`).

`definition.rs::record_path` then collapses to `placement.definition_record.clone()`, and `PREFIX`, `logical_record_path` and `audience_directory` are deleted, along with the disjointness assertions at `:540-552` (subsumed by S4) and the tests that exercise the deleted derivation.

This honours the "one implementation of the layout" rule stated at `public.rs:30-34` and removes the only tree whose readable shape nix could not state.
`relocation.rs::record_leaves` (`:90-108`) currently pairs `record_path` with `logical_record_path`; with the record emitted on every placement, `logical_record_path` is replaced by the placement's own logical record field, emitted beside `logicalFile`/`logicalKey`/`logicalPublic` exactly as those are.

**Alternative rejected**: keeping the Rust derivation and feeding it the configured root through a new nix output.
That carries the layout across the language boundary as data *and* as code, leaves two implementations to keep in agreement, and gains nothing: the resolver already computes two of the three trees and has every input it needs for the third.

### S9. `public::{PREFIX, LEAF, is_public_path}` and the property module are deleted

`is_public_path` has no production call site — grep over `crates/safix-core/src` and `crates/safix/src` finds it only in `public.rs` and its own tests, and the runtime forks on `placement.public.is_some()` (`generate.rs:372-380`, `edit.rs:175-180`).
It is additionally wrong in vault mode, where a public path is `public/<hash>` with neither the `public/safix/` prefix nor the `value` leaf, so it answers `false` for every public path a vault holds.
`PREFIX` and `LEAF` exist only to feed it and its tests.

The property module (`public.rs:126-191`) reimplements `public_path` in Rust (`:140-150`) and asserts three properties over that reimplementation, including a non-overlap property (`:172-175`) built from a literal `secrets/safix/users/…`.
It is a property test about a function this change deletes, over a layout this change moves to configuration; it goes with them.

What is *not* lost: the separation claim is held by `safix-public-no-rule` and `safix-no-catch-all` (`checks.nix:264-300`, `:224-262`), which match generated rules against real resolved public paths behaviourally and which gate a build; and the non-overlap claim is held by S4's evaluation refusal, which holds for configurations the deleted property test could not reach.

### S10. `catchAllProbes` must be derived, and that is the change's most dangerous line

`checks.nix:235-242`'s probes are ten literal paths, six of which spell the three trees.
Left literal while the roots become configurable, `safix-no-catch-all` would probe trees nothing uses and stop probing the trees the consumer actually has — the check stays green and stops meaning anything.
A silent loss of a check is worse than a broken one, so the probes are derived from the configured roots (`"${storage.encrypted}/users/UNCLAIMED/x.yaml"` and so on), keeping the uppercase `UNCLAIMED` component, which the name alphabet excludes so no declaration can ever make a probe real.
The four non-tree probes (`x.yaml`, `UNCLAIMED.yaml`, `UNCLAIMED/x.yaml`, `some/other/place/UNCLAIMED.yaml`) stay literal: they are about paths outside every tree, which is exactly what they must remain.

The severity drill for this is explicit in `tasks.md`: a fixture that renames the roots must turn the check red when a rule reaching a renamed tree is planted, and that must fail if the probes are left literal.

### S11. The committed `.sops.yaml` header is parameterized

`policy.nix:56-60` and `:84-89` are prose committed into every consumer's `.sops.yaml`, and they name `secrets/safix/shared/<a>,<b>/` and `nested/secrets/safix/users/alice/x.yaml` as worked examples.
After a rename the committed file would document a layout the repository does not have — a false statement in the one file a reviewer reads to learn who can open what.
Both paragraphs interpolate the configured `encrypted` root, and `modules/flake/checks/fixture-policy.yaml:22-26`, `:50-56` is regenerated content that moves with them.

The rendered *rules* need no change: `policy.nix:206` already derives `pathRegex` from `audiences.<file>.dir`.

## Risks / Trade-offs

A consumer sets `storage.plaintextOutputs` to a path a creation rule elsewhere in their own hand-written `.sops.yaml` covers → out of safix's reach by construction: safix owns the generated rules and refuses on any generated rule matching a public path (`safix-public-no-rule`), but a consumer's rules in a different config file are theirs. The `plaintextOutputs` option description says so, and `safix check`'s `UngovernableExtra` finding already names files safix cannot govern.

A consumer picks a hidden root (`.safix/…`) and later audits with `rg` or `fd`, both of which skip dot-directories by default → stated in the option descriptions and the README, not refused. It cuts both ways: fewer accidental ciphertext matches, and an audit needs `--hidden`. Refusing a valid choice because one tool's default surprises is worse than documenting the surprise.

The S5 opaque-name break lands on any consumer already running a vault from the unreleased branch → corrected after implementation (task 8.3): the break is partial (definition records keep their opaque names, since the `state` input was already root-relative) and the relocation machinery cannot express an opaque-to-opaque move, so `safix check` reports every entry as holding no value at its new name rather than as pending relocation; the migration order is `safix fix --vault-rollback` before the update, then `safix fix` after it, and the changelog entry states exactly that.

The overlap refusal rejects a configuration someone considers reasonable — nesting the records tree inside the ciphertext tree, say → the refusal message names both options and both values, and the `generatorRecords` description states why the tree is separate. A consumer who genuinely wants a record inside the encrypted tree wants a different feature (encrypted records), which this change does not provide and does not foreclose.

Deriving `catchAllProbes` (S10) could itself weaken the check if the derivation is wrong → the drill in `tasks.md` perturbs the roots in a fixture and requires the check to go red on a planted reaching rule; a derivation that produced nothing, or probes under the default spelling, fails that drill.

## Migration Plan

**For every existing consumer, in readable mode: nothing.**
The defaults are today's spellings, so every resolved path, every generated rule, every fixture literal and every snapshot is byte-identical.
Task group 1's first check asserts exactly that.

**For a consumer who renames a root, in readable mode:**

1. Change the `flake.safix.storage.*` value.
2. `git mv` the tree.
3. `safix fix` — regenerates `.sops.yaml` (new `dir`, new `pathRegex`, new header prose) and commits it.
4. `safix check` — green; before step 2 it reports every governed file the regenerated policy no longer names.

No re-encryption: a sops document does not embed its own path, and readable-mode key names do not change.

**For a consumer running an unreleased vault (the S5 break):**

1. `safix fix --vault-rollback` while still on the pre-change input, which recovers every leaf into the readable layout under the old names.
2. Update the input.
3. `safix fix` — adopts the vault again through the existing relocation path (`fix.rs:215-285`), now under the root-relative names.

Updating first is recoverable but not automatic: the old opaque names are neither readable-layout sources `check` can queue nor destinations `named_move` recognises, so a vault carried across the update in place reports every entry as holding no value until the operator downgrades, rolls back, and repeats the order above.
`crates/safix/tests/vault_migration.rs` asserts both that the break is visible and that nothing relocates itself.

**Rollback:** reverting the change restores the literal roots and the full-path hash input.
A readable-mode consumer who has renamed a root reverses it with `git mv` and `safix fix`.
A vault consumer who has run step 3 reverses it in the same order: `safix fix --vault-rollback` on the new input, revert, `safix fix` on the old one.
