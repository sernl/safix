# Tasks: configurable-storage-roots

Citations are as read while designing this change, on 2026-09-15; re-read the named lines before editing, since implementation may land after sibling changes and line numbers drift.
The symbol name, not the line number, is the anchor.
No real fleet identifier, hostname, or recipient enters this repository; fixtures use `alice`, `bob`, and `carol`, and synthetic `age1` strings, matching the existing fixtures.
Where a task says "hold", add a check that fails when the claim stops being true, not a sentence asserting it.
Throughout, `storage` means the resolved attrset `{ encrypted; plaintextOutputs; generatorRecords; }` threaded from `flake.safix.storage`, and every task that says "derive from `storage.<x>`" means string-interpolating that value where a literal stands today, with no trailing slash added (the option refuses one, so every join writes its own `/`).

## 1. The `storage` option and its two evaluation refusals

- [ ] 1.1 Add `storage` to `modules/flake/safix/options.nix`, beside `vault` (`options.nix:242-333` is the shape to follow): `lib.mkOption { type = lib.types.submodule { options = { encrypted = ...; plaintextOutputs = ...; generatorRecords = ...; }; }; default = { }; }`, each sub-option `lib.types.str` with `default = "secrets/safix"` / `"public/safix"` / `"state/safix/definitions"` respectively
- [ ] 1.2 Write `storage.encrypted`'s description: it holds every ciphertext document safix places, one file per distinct audience, in a directory named for that audience; everything under it is ciphertext without qualification, which is the sentence a backup policy, an `rsync --exclude` and a reviewer are written against; the name is the consumer's, and naming it something that does not say "encrypted" moves that promise onto whatever name they pick
- [ ] 1.3 Write `storage.plaintextOutputs`' description: it holds generator outputs declared `secret = false`, written in the clear so a nix module can read one at evaluation, never handed to sops and never given a creation rule; it is separate from the encrypted tree so that a rule, an exclusion or a search scoped to one cannot reach the other
- [ ] 1.4 Write `storage.generatorRecords`' description: it holds one plaintext line per generated value, a digest of the generator definition that minted it, carrying no value and no derivative of a value — which is what licenses committing it in the clear — so that `safix check` can answer definition drift without decrypting anything; it is separate from the plaintext-output tree because that tree means "declared public outputs a nix module reads", and bookkeeping there would dilute it into "plaintext things safix wrote"
- [ ] 1.5 Add to each of the three descriptions the one-parent worked example (`.safix/encrypted`, `.safix/plaintext-outputs`, `.safix/generator-records`) with the sentence that setting all three under one directory buys one ignore entry, one backup rule and one directory to move; and the caveat that `rg` and `fd` skip dot-directories by default, so an audit of a hidden root needs `--hidden`
- [ ] 1.6 Add `wellFormedStorageRoot` to `modules/flake/safix/resolve.nix`, following the shape `wellFormedNamingKey` establishes: false for an empty string, a string beginning with `/`, a string with a `..` path component, or a string ending in `/`
- [ ] 1.7 Add `storageOverlap = a: b: a == b || lib.hasPrefix "${a}/" b || lib.hasPrefix "${b}/" a;` beside it — the `/`-suffixed comparison is what makes `secrets/safix` and `secrets/safix-public` disjoint while `secrets/safix` and `secrets/safix/pub` overlap
- [ ] 1.8 Add `storageViolations = storage: ...` returning one message per malformed root (naming `flake.safix.storage.<name>` and the value) and one per overlapping pair (naming both option paths and both values), and add it to `resolve.nix`'s exports list beside `vaultViolations` (`resolve.nix:2418`)
- [ ] 1.9 Concatenate `resolve.storageViolations cfg.storage` into `violations` in `modules/flake/safix/default.nix`, alongside `resolve.violations registry`, `resolve.generatorViolations registry` and `resolve.vaultViolations cfg.vault`
- [ ] 1.10 Thread `storage` into `bound`/`registry` in `default.nix:41-58`, the way `namingKey` is threaded, so every `resolve.*` entry point reaches it without a second plumbing mechanism
- [ ] 1.11 Add `modules/flake/checks/storage.nix` with a check named `safix-storage` and register it where the sibling check files are registered; it carries every fixture assertion this change's groups name
- [ ] 1.12 In `safix-storage`, assert the default configuration: a fixture with no `storage` set resolves every `sopsFile`, `publicValue` path, `publicPathsOf` entry, `definitionRecord` and generated `pathRegex` byte-identically to the literal spellings `secrets/safix/...`, `public/safix/...`, `state/safix/definitions/...` — this is the check that makes "defaults preserve the layout exactly" a fact rather than a claim
- [ ] 1.13 In `safix-storage`, assert the four malformed-root refusals (empty, absolute, `..` component, trailing slash) fire for each of the three options, and that a well-formed root evaluates without refusal
- [ ] 1.14 In `safix-storage`, assert the overlap refusals: two roots equal; `encrypted` a component-prefix of `plaintextOutputs`; `generatorRecords` nested under `encrypted`; and the accept case where one root's string is a non-component prefix of another (`secrets/safix` with `secrets/safix-public`)
- [ ] 1.15 Severity drill: dropping the `"${a}/"` suffix from 1.7's comparison turns 1.14's accept case red (it starts refusing a legal configuration); dropping the `a == b` clause turns the equal-roots case green when it should refuse. Record both in `storage.nix`'s drill commentary
- [ ] 1.16 Severity drill: removing `storageViolations` from 1.9's concatenation turns every case in 1.13 and 1.14 green, which is the evidence the refusals reach evaluation rather than only existing as a function
- [ ] 1.17 Verify: `nix build .#checks.x86_64-linux.safix-storage` green; drills in 1.15 and 1.16 observed

## 2. The resolver derives every readable path from the roots

- [ ] 2.1 `resolve.nix:523-526` — `audienceFileOf` takes `storage` and emits `"${storage.encrypted}/users/${user}/secrets.yaml"` and `"${storage.encrypted}/shared/${members}/secrets.yaml"`
- [ ] 2.2 `resolve.nix:592-597` — `publicFileOf` takes `storage` and emits `"${storage.plaintextOutputs}/users/${user}/${name}/value"` and `"${storage.plaintextOutputs}/shared/${members}/${name}/value"`; the `<name>/value` leaf shape is unchanged, it is clan's shape
- [ ] 2.3 `resolve.nix:297-298` — the comment justifying the audience separator "as a path component after `secrets/…/`" now reads against the configured encrypted root rather than a literal
- [ ] 2.4 `resolve.nix:586-591` — rewrite the load-bearing rationale comment above `publicFileOf`. It currently argues that the public prefix is "a top-level sibling of the ciphertext tree… separable by prefix — which is what a `.gitignore`, an `rsync --exclude`, a backup policy and a reviewer all actually operate on". Keep the argument and move its ground: the trees are separable because they are refused at evaluation if they overlap, and the name carrying the "everything here is ciphertext" promise is the consumer's to choose. Cite `design.md` S3
- [ ] 2.5 `resolve.nix:611-614` — the comment describing vault ciphertext as "one flat file directly under `secrets/`" stays accurate (the vault buckets are literal, per design S6); add the sentence that this bucket is deliberately not `storage.encrypted` and why
- [ ] 2.6 Confirm and hold: `outputPathIn` (`resolve.nix:728-732`), `publicValueIn` (`:751-753`) and `publicPathsIn` (`:771-774`) consume `placement.public`/`placement.file` and need no edit. Add an assertion in `safix-storage` that all three follow a renamed root, so "derived" is checked rather than assumed
- [ ] 2.7 Confirm and hold: `policy.nix:206`'s `pathRegex = "^${a.dir}/[^/]*\\.yaml$"` derives from `audiences.<file>.dir`, itself `builtins.dirOf (audienceFileOf audience)` (`resolve.nix:543`), so every readable-mode rule follows a root rename with no edit. Add an assertion in `safix-storage` over a renamed fixture
- [ ] 2.8 Severity drill: reverting 2.1 to the literal while leaving 2.2 in place turns 2.6's renamed-root assertion red on the ciphertext half alone, which is the evidence the two trees are independently derived
- [ ] 2.9 Verify: `nix build .#checks.x86_64-linux.safix-storage` green including 2.6 and 2.7; drill in 2.8 observed

## 3. Nix emits `definitionRecord` on every placement; the runtime stops constructing it

- [ ] 3.1 `resolve.nix:698-699` — `placementsIn`'s `definitionRecord` becomes unconditional: `"${storage.generatorRecords}/shared/${audienceDir}/${name}"` for a shared entry and `"${storage.generatorRecords}/${owner}/${name}"` otherwise, replaced by `"state/${opaqueOf r.namingKey "state" logicalRecord}"` when `r.namingKey != null`, mirroring exactly how `file`/`public` already fork
- [ ] 3.2 `resolve.nix` — emit `logicalRecord` on every placement alongside `logicalFile`/`logicalKey`/`logicalPublic` (`:700-702`), non-null only in vault mode, carrying the readable record path the opaque one was hashed from. This is what replaces `definition::logical_record_path`
- [ ] 3.3 `crates/safix-core/src/model.rs` — `Placement::definition_record` changes from `Option<String>` to `String` (`model.rs:178`); add `logical_record: Option<String>` beside `logical_public` (`:196`). Update the placement fixtures at `model.rs:1014-1018`
- [ ] 3.4 `crates/safix-core/src/definition.rs:145-155` — `record_path(name, placement)` collapses to `placement.definition_record.clone()`; drop the `name` parameter if no caller still needs it, and update every call site
- [ ] 3.5 `crates/safix-core/src/definition.rs` — delete `PREFIX` (`:70-71`), `logical_record_path` (`:166-175`) and `audience_directory` (`:177-195`)
- [ ] 3.6 `crates/safix-core/src/definition.rs:10-27` — rewrite the module doc. It currently states the three-tree rationale as three literal prefixes; it now states that the record's path is the resolver's, carried on the placement, and that the record is under the generator-record root and under neither other root because evaluation refuses overlap
- [ ] 3.7 `crates/safix-core/src/relocation.rs:90-108` — `record_leaves` reads `placement.logical_record` instead of calling `definition::logical_record_path`, and can now be expressed through `leaves_by` (`:113-129`) like `public_leaves` is; collapse it if the dedup semantics are identical, keeping the doc comment's explanation of why a shared entry's record must not be queued twice
- [ ] 3.8 `crates/safix-core/src/definition.rs:497-534` — rewrite the tests that exercise the deleted derivation: they now assert that `record_path` returns exactly the placement's emitted record, over placements carrying both readable and opaque forms
- [ ] 3.9 `crates/safix-core/src/definition.rs:540-552` — delete the disjointness assertion (`!path.starts_with("secrets/")`, `!path.starts_with(crate::public::PREFIX)`). It asserts a property of three constants that are now inputs; group 1's evaluation refusal replaces it for every configuration
- [ ] 3.10 `crates/safix-core/src/definition.rs:562-608` — keep the vault-mode record tests; re-point them at the placement's emitted record rather than at the fallback derivation
- [ ] 3.11 Confirm and hold: no new `nix.rs` `Attribute` variant is needed (`nix.rs:64-90`). The record reaches the runtime on the placement, through the `placements` attribute already evaluated. Record this in a comment where the temptation to add one would arise
- [ ] 3.12 In `safix-storage`, assert `placementsIn` emits a non-null `definitionRecord` for every placement with no vault declared, and that it equals `state/safix/definitions/...` under the default roots and follows a renamed `generatorRecords` root
- [ ] 3.13 Severity drill: making 3.1 conditional again (emitting `null` outside vault mode) turns 3.12 red and makes `safix check` unable to locate any record, observable as a failing `crates/safix/tests/generators.rs` definition-drift case — this is the evidence the emission is load-bearing rather than decorative
- [ ] 3.14 Verify: `cargo test -p safix-core definition::` and `cargo test -p safix-core relocation::` green; `nix build .#checks.x86_64-linux.safix-storage` green; drill in 3.13 observed

## 4. `public::{PREFIX, LEAF, is_public_path}` and the property module are deleted

- [ ] 4.1 `crates/safix-core/src/public.rs:97-100` — delete `is_public_path`. It has no production call site (the runtime forks on `placement.public.is_some()` at `generate.rs:372-380` and `edit.rs:175-180`) and it answers `false` for every vault-mode public path, which has neither the prefix nor the `value` leaf
- [ ] 4.2 `crates/safix-core/src/public.rs:45-49` — delete `PREFIX` and `LEAF`; they exist only to feed 4.1 and its tests
- [ ] 4.3 `crates/safix-core/src/public.rs:107-123` — delete the two unit tests (`the_public_prefix_and_the_ciphertext_prefix_are_not_prefixes_of_each_other`, with its own `const CIPHERTEXT: &str = "secrets/safix/"`, and `a_path_is_public_only_under_the_prefix_and_at_the_leaf`)
- [ ] 4.4 `crates/safix-core/src/public.rs:126-191` — delete the property module: its `NAME` alphabet (`:133`), its reimplementation of `public_path` (`:140-150`), and all three properties (`:156-190`). It is a property test about a deleted function over a layout that is now configuration
- [ ] 4.5 `crates/safix-core/src/public.rs:10-34` — rewrite the module doc. Keep `:30-34`'s "one implementation of the layout" rule, which this change finally honours; replace `:22-28`'s literal-prefix argument with design S3's: the promise moved from the tree's name to the consumer's naming, and what stays mechanically enforced is the evaluation refusal on overlap plus `safix-public-no-rule` and `safix-no-catch-all`
- [ ] 4.6 Confirm unedited: `public::stage` (`:56-77`) and `public::holds_a_value` (`:84-87`) are the module's real surface and neither reads a prefix
- [ ] 4.7 Grep the workspace for `public::PREFIX`, `public::LEAF`, `is_public_path`, `definition::PREFIX`, `logical_record_path`, `audience_directory` and confirm zero remaining references, including in `modules/flake/checks/generators.nix`, which asserts the nix half against the Rust constants today
- [ ] 4.8 Severity drill: the claim these deletions rest on is that nix's two checks, not Rust, hold the separation. Plant `pathRegex = "^${storage.plaintextOutputs}/.*"` in a fixture with renamed roots and confirm both `safix-public-no-rule` and `safix-no-catch-all` fail. If either passes, the deletion is not yet safe and group 6 is incomplete
- [ ] 4.9 Verify: `cargo test -p safix-core public::` green (the module's remaining tests), `cargo build -p safix-core` clean with no dead-code warning; drill in 4.8 observed

## 5. The committed `.sops.yaml` header names the configured roots

- [ ] 5.1 `modules/flake/safix/policy.nix:56-60` — the header paragraph "A shared audience gets `secrets/safix/shared/<a>,<b>/`" interpolates `storage.encrypted`
- [ ] 5.2 `modules/flake/safix/policy.nix:84-89` — the anchoring paragraph's worked example, `nested/secrets/safix/users/alice/x.yaml` matching a rule written for `secrets/safix/users/alice/`, interpolates `storage.encrypted` in both spellings
- [ ] 5.3 Thread `storage` into `header` and into `commentLines "" header`'s call site so the two paragraphs can reach it without a second plumbing path
- [ ] 5.4 `modules/flake/checks/fixture-policy.yaml:22-26, 50-56` — regenerate the committed fixture header. Under the default roots the bytes are unchanged; regenerate rather than hand-edit, and confirm the diff is empty
- [ ] 5.5 Confirm unedited: `policy.nix:253-256`'s `renderVaultRules` keeps its literal `^secrets/`, per design S6; add the one-line comment stating that the vault's buckets are deliberately not `storage.encrypted`
- [ ] 5.6 In `safix-storage`, assert over a renamed-root fixture that the generated `.sops.yaml` header contains the renamed root in both worked examples and contains no occurrence of the default spelling
- [ ] 5.7 Severity drill: reverting 5.1 while keeping 5.2 turns exactly the shared-audience half of 5.6 red and leaves the anchoring half green, which is the evidence the two paragraphs are independently held
- [ ] 5.8 Verify: `nix build .#checks.x86_64-linux.safix-storage` and `.#checks.x86_64-linux.safix-policy-drift` green; `modules/flake/checks/fixture-policy.yaml` unchanged under the defaults; drill in 5.7 observed

## 6. `catchAllProbes` is derived, or the check silently stops meaning anything

- [ ] 6.1 `modules/flake/safix/checks.nix:235-242` — derive the six tree-shaped probes from the configured roots: `"${storage.encrypted}/users/UNCLAIMED/x.yaml"`, `"${storage.encrypted}/shared/UNCLAIMED/x.yaml"`, `"${storage.plaintextOutputs}/users/UNCLAIMED/x/value"`, `"${storage.plaintextOutputs}/shared/UNCLAIMED/x/value"`, `"${storage.generatorRecords}/UNCLAIMED/x"`, `"${storage.generatorRecords}/shared/UNCLAIMED/x"`. Keep the uppercase `UNCLAIMED` component: the name alphabet excludes it, so no declaration can ever make a probe real
- [ ] 6.2 Keep the four non-tree probes literal (`x.yaml`, `UNCLAIMED.yaml`, `UNCLAIMED/x.yaml`, `some/other/place/UNCLAIMED.yaml`); they are about paths outside every tree, which is exactly what they must stay
- [ ] 6.3 Thread `storage` into `catchAllProbes`/`catchAllMessagesOf`/`mkNoCatchAllCheck` (`checks.nix:224-262`), which today take only `plan`/`registry`
- [ ] 6.4 `modules/flake/safix/checks.nix:234-243` — rewrite the comment block. It currently says the record tree's two probes are "nix's only statement about the shape Rust writes"; after group 3, nix computes the record path, so the probes now guard the tree nix itself places
- [ ] 6.5 `modules/flake/safix/checks.nix:271-275` — the comment "The rules are anchored under `secrets/safix/` and terminate on `\.yaml$`" names the configured encrypted root instead of the literal
- [ ] 6.6 Confirm derived and hold: `publicRuleMessagesOf`/`mkPublicRuleCheck` (`checks.nix:264-300`) are fed `resolve.publicPathsOf`, so they follow a renamed root with no edit. Add a renamed-root assertion to `safix-storage`
- [ ] 6.7 Severity drill (the most dangerous line in this change): leave 6.1 literal, rename the roots in a fixture, plant a rule reaching the renamed encrypted tree, and confirm `safix-no-catch-all` stays green. Then apply 6.1 and confirm it goes red. A derived-probe implementation that does not reproduce this transition has not derived anything
- [ ] 6.8 Severity drill: deriving the probes but dropping the `UNCLAIMED` component (probing a name a declaration could really occupy) turns `safix-no-catch-all` red on a legitimate fixture, which is the evidence the excluded-alphabet component is load-bearing
- [ ] 6.9 Verify: `nix build .#checks.x86_64-linux.safix-no-catch-all`, `.#checks.x86_64-linux.safix-public-no-rule`, `.#checks.x86_64-linux.safix-storage` green; drills in 6.7 and 6.8 observed

## 7. The vault's opaque hash input becomes root-relative

- [ ] 7.1 `resolve.nix:615-616` — `secretsFileOf` hashes the audience file's name relative to `storage.encrypted` (`users/alice/secrets.yaml`) rather than the root-prefixed name. Introduce one helper, `relativeTo = root: path: lib.removePrefix "${root}/" path;`, and use it at every one of this group's sites so the stripping rule exists once
- [ ] 7.2 `resolve.nix:622-623` — `publicFileOfVault` hashes the public file's name relative to `storage.plaintextOutputs`
- [ ] 7.3 `resolve.nix:698-699` — the vault branch of `definitionRecord` hashes the record's name relative to `storage.generatorRecords`
- [ ] 7.4 `resolve.nix:625-631` — `opaqueKeyOf`'s `logicalPath` becomes the same root-relative audience-file name `secretsFileOf` hashes, at its call site (`:686`) and everywhere `selectFor.forMachine`'s `sopsKey` and `selectFor.forUser`'s `sopsKey` are computed, so the two derivations still agree bit for bit
- [ ] 7.5 `resolve.nix:599-610` — update the `opaqueOf` comment: the four tags were a convenience while the full path distinguished the uses; they are now the only separator, because two roots' relative names can coincide
- [ ] 7.6 Confirm unedited: the vault's own three buckets (`secrets/`, `public/`, `state/`) stay literal at the vault root
- [ ] 7.7 `modules/flake/checks/vault.nix:202-207` — `expectedOpaqueFile`/`expectedOpaquePublic`/`expectedDefinitionRecord` recompute against the root-relative input; the literal `secrets/`, `public/`, `state/` joins stay, because those are the vault buckets
- [ ] 7.8 `modules/flake/checks/vault.nix:253-254` — `expectedRegexOf`'s recomputation follows 7.7
- [ ] 7.9 `modules/flake/checks/vault.nix:415-417` — `prefixesStayDisjoint` asserts a property of the vault's literal buckets, which this change leaves literal; keep it, and add a comment distinguishing it from the declaration-side disjointness group 1 now refuses
- [ ] 7.10 Add a `safix-storage` (or `safix-vault`) assertion: a vault fixture whose `storage.*` roots are renamed resolves every opaque name, public leaf, definition record and in-document key identically to the same fixture under the default roots. This is the whole point of the change and the one assertion that would catch a missed `relativeTo`
- [ ] 7.11 Add a `safix-vault` assertion that the four tags remain load-bearing: two entries in different trees sharing one root-relative readable identity resolve to different opaque names
- [ ] 7.12 Severity drill: reverting any one of 7.1-7.4 to the root-prefixed input turns 7.10 red on exactly that tree (or, for 7.4, on the key), which is the evidence the four sites are independently held
- [ ] 7.13 Severity drill: collapsing two of `opaqueOf`'s tags to one turns 7.11 red on the coincidental-collision fixture
- [ ] 7.14 Verify: `nix build .#checks.x86_64-linux.safix-vault` and `.#checks.x86_64-linux.safix-storage` green; drills in 7.12 and 7.13 observed

## 8. The one-time opaque-name break relocates through the existing machinery

- [ ] 8.1 Confirm no code change is needed in `check.rs:217-260` (`vault_relocation`), `relocation.rs:44-129`, or `fix.rs:215-330` (`relocate`, `document_move`, `leaf_move`, `named_move`) beyond group 3's `logical_record` swap: every opaque name changing is the case this machinery already serves, and `named_move`'s "source absent or destination exists → `None`" predicate is what makes the relocation resumable
- [ ] 8.2 Confirm `Finding::VaultRelocationPending { file }` (`check.rs:164-180`) and its `render.rs:149-…` prose still read correctly for this cause; they name a pending document, not a cause, so no new variant and no snapshot change
- [ ] 8.3 Extend `crates/safix/tests/vault_migration.rs` with a drill for this break: seed a vault whose documents carry the pre-change opaque names, run `check` and observe one pending finding per document, public leaf and record, run `fix`, run `check` again and observe none. Use `:120-205`'s existing before/after shape
- [ ] 8.4 `crates/safix/tests/vault_opaque_names.rs:26-32` and `vault_migration.rs:19-38` — regenerate the six logical/opaque constants against the root-relative derivation. Recompute them, do not hand-edit: each is `sha256("<namingKey>|<tag>|<root-relative logical>")`
- [ ] 8.5 `crates/safix/tests/vault_scratch_rules.rs:102-105, 241-260` and `crates/safix-core/src/set.rs:417-420`'s `(&fixture.vault, "secrets/opaque.yaml")` — regenerate any opaque constant these carry
- [ ] 8.6 `modules/flake/checks/vault-projection.nix:271-274` — regenerate the expected `audiences` list if it carries an opaque name
- [ ] 8.7 Severity drill: running 8.3's `check` against a vault seeded with the *new* names must report nothing, and against the *old* names must report every document. A `check` that reports nothing in both cases means `vault_relocation` is no longer reached
- [ ] 8.8 Verify: `cargo test -p safix --test vault_migration --test vault_opaque_names --test vault_scratch_rules` green; drill in 8.7 observed

## 9. Remaining nix surfaces that spell a root

- [ ] 9.1 `modules/flake/safix/default.nix:72-79` — `extraGovernedFiles`' `example = [ "secrets/safix/users/alice/ops-tooling.yaml" ]` becomes a `lib.literalExpression` naming the configured root, or states in prose that the example uses the default spelling
- [ ] 9.2 `modules/flake/safix/types.nix:102-106` — the `files.<n>.secret` description, "false writes it to `public/safix/…/<name>/value`", names `flake.safix.storage.plaintextOutputs` and describes the shape below it rather than spelling the default
- [ ] 9.3 Confirm and hold: `default.nix:346-368`'s `governedFiles` is `extra ∪ required` with `required = attrNames audiences`, so it follows a renamed root with no edit. Add a renamed-root assertion to `safix-storage`
- [ ] 9.4 Confirm and hold: `default.nix:386-387`'s vault-rules rendering entry point needs no edit
- [ ] 9.5 Confirm unedited, with a stated reason: `modules/consume/{common,home,nixos,installer}.nix` carry no repository-layout prefix — they deal in `/run/safix.d`, `symlinkPath` and `sopsFile` values handed down by the resolver (`installer.nix:234-253`, `nixos.nix:134-143`) — so a root rename does not reach the consumption modules at all
- [ ] 9.6 Confirm unedited: `modules/flake/checks/collision-fixture/declaring-module.nix` carries no prefix
- [ ] 9.7 Verify: `nix eval .#safix.lib` style smoke over the renamed-root fixture resolves `governedFiles` under the new root; `nix build .#checks.x86_64-linux.safix-storage` green

## 10. Nix fixture and check literals

Under the preserved defaults not one of these changes value; each task is to confirm it still passes and, where the fixture asserts a *literal prefix* rather than a resolved path, to re-point it at the configured root so it does not go vacuous.

- [ ] 10.1 `modules/flake/checks/policy.nix:169-171` — `rulesWellFormed`'s `lib.hasPrefix "^secrets/" r.pathRegex` becomes `lib.hasPrefix "^${storage.encrypted}/" r.pathRegex`. Left literal, this assertion fails on any root rename while claiming to check anchoring
- [ ] 10.2 `modules/flake/checks/policy.nix:123` (`sharedFile`), `:186-190` (`siblingDirectory`/`nestedFile`/`prefixedPath`), `:222-261` (the five expected `^secrets/safix/...` regexes) — confirm green under the defaults; leave the literals, they are fixture expectations for a fixture that does not rename
- [ ] 10.3 `modules/flake/checks/generators.nix:263-268` (`publicLeaf`), `:282-285` (the deliberately reaching `pathRegex = "^public/safix/.*"`), `:512-523` (`pathOfThePublicHalf`, `pathOfTheSecretHalf`, `publicPaths`) — confirm green; the reaching-rule fixture is what group 4.8's drill extends to a renamed root
- [ ] 10.4 `modules/flake/checks/custody.nix:807-820, 865-878, 913-916, 959-968, 985-987` — confirm green
- [ ] 10.5 `modules/flake/checks/consumption.nix:632-634` (`sopsFile = "/secrets/safix/users/alice/secrets.yaml"`) — confirm green
- [ ] 10.6 `modules/flake/checks/exported.nix:276-283` (the six governed files a fixture fleet exports) — confirm green
- [ ] 10.7 `modules/flake/checks/materialization.nix:135-139` (`fileFromAudience`) — confirm green
- [ ] 10.8 `modules/flake/checks/portability.nix:574-582, 591-592, 601-602, 611-612, 631-632, 646-653, 770-773` — confirm green
- [ ] 10.9 `modules/flake/checks/subjects.nix:1757-1764, 1821-1846, 1866-1893, 1928-1979, 2007-2011` — confirm green, including the refusal message at `:2008` embedding `secrets/safix/shared/@contractors,alice/secrets.yaml`
- [ ] 10.10 `modules/flake/checks/cli.nix:486-490` — the prose about `state/safix/definitions/` names `flake.safix.storage.generatorRecords`; `:532-535` (`safix-governed-extras`) and `:1186-1190` (the evaluated attribute list) are behavioural, confirm green
- [ ] 10.11 `modules/flake/checks/examples.nix:53-57` — confirm green for both example consumers
- [ ] 10.12 Verify: `nix flake check` green for the check set this group names; every "confirm green" item observed rather than assumed

## 11. Rust fixtures, CLI help, and snapshots

- [ ] 11.1 `crates/safix/src/usage.rs:326-331` — `check`'s help text, "answered from state/safix/definitions/", names `flake.safix.storage.generatorRecords` and describes the tree rather than spelling the default
- [ ] 11.2 `crates/safix/src/usage.rs:64-68` — confirm `fix`'s help, which names the vault `.gitignore` write, is unaffected (the vault's `.gitignore` is the only one safix writes and it is not storage-rooted)
- [ ] 11.3 `crates/safix/src/render.rs:128-137` (`UngovernableExtra`, naming `extraGovernedFiles`), `:142-146` (`VaultGitignoreMissing`), `:149-…` (`VaultRelocationPending`), `:163-208` (recipient-drift, unclaimed-value) — confirm each reads correctly against a configured root; none spells a prefix, so none should change
- [ ] 11.4 `crates/safix/src/reporter.rs:259-573` — the sample errors feeding the snapshots carry path literals at `:260, 267, 272, 328, 335, 344, 353, 364, 367-368, 375, 379, 382, 386, 391, 397, 411, 456, 531, 573`. These are sample data under the default roots; confirm each still renders and regenerate only what group 8's opaque-name change moves
- [ ] 11.5 `crates/safix/src/snapshots/` — 30 files across the 22 named cases (`candidate_recipients_unreadable`, `conflict_entries`, `dependency_has_no_value`, `file_unreadable`, `file_unwritable`, `git_command_failed`, `no_creation_rule`, `no_value_yet`, `not_a_yaml_path`, `public_not_editable`, `recipient_drift`, `recipient_drift_one_sided`, `recipients_lost`, `recipients_unreadable`, `sops_create_failed`, `source_has_no_value`, `source_has_no_value_generated`, `source_unreadable`, `sync_source_empty`, `sync_source_empty_generated`, `uncommitted_changes`, `vault_relocation_unreadable`), each in `graphical-` and `plain-` form. Run `cargo insta test`; accept only diffs group 8's opaque rename explains, and investigate any other diff as a regression
- [ ] 11.6 `crates/safix/tests/harness/mod.rs:71-72` (`ALICE_FILE`), `:83-84` (`SHARED_FILE`), `:380-398` (the stub resolver's placements, audiences and `dir` fields at `:392`/`:397`), `:859-861` (the synthesised per-user file), `:2470-2476` (the stub `.sops.yaml`'s two `^secrets/safix/...` rules) — confirm green under the defaults; the stub resolver must also emit `definitionRecord` on every placement (group 3.1) and `logical_record` in vault mode (group 3.2), or every record-reading test breaks
- [ ] 11.7 `crates/safix/tests/` literal-carrying suites — confirm green: `abort_residue.rs:34-35, 175-178, 383-386`; `custody.rs:80-83`; `generators.rs:120-123, 714-715, 770-781, 837-840, 875-876, 944-945, 1012-1015`; `read_path.rs:107-110, 154-157, 222-226`; `subjects.rs:152-153, 261-262`; `sync_path.rs:535-536, 562-565`; `write_path.rs:222-226, 239-243, 425-433, 536-544`; `vault_invariants.rs`, `vault_reads.rs`, `vault_workspace.rs`, `vault_commit_ordering.rs`, `vault_lock_bump.rs`
- [ ] 11.8 Confirm the remaining suites reach the layout only through `harness::ALICE_FILE`/`SHARED_FILE` and so carry no independent literal: `bridge*.rs`, `store_cli.rs`, `upload.rs`, `real_clan.rs`, `audit.rs`, `value_pipe.rs`, `value_source.rs`, `enrollment.rs`, `group.rs`, `harness_check.rs`, `memory_backing.rs`, `sandbox.rs`, `shared_entries.rs`, `syscall_proof.rs`, `channel_drills.rs`, `delegation.rs`, `editor.rs`
- [ ] 11.9 Severity drill: point `harness::ALICE_FILE` at a path the stub resolver does not place and confirm the record-reading tests in `generators.rs` go red rather than silently skipping — the evidence that group 3's emitted record is read rather than reconstructed
- [ ] 11.10 Verify: `cargo test --locked --workspace` green; snapshot diffs in 11.5 reviewed line by line; drill in 11.9 observed

## 12. Documentation

- [ ] 12.1 `README.md:466-476` — rewrite the public-store section against `flake.safix.storage.plaintextOutputs`. Keep the "why separate" argument and move its ground per design S3; update the two-line layout block and keep the two checks named
- [ ] 12.2 `README.md:533-550` — rewrite the definition-record section against `flake.safix.storage.generatorRecords`. The "third top-level tree" framing and the "`state/` says what it is" sentence are replaced: the option name now says what it is, and the tree is no longer necessarily top-level
- [ ] 12.3 `README.md:1203-1208` — the "where things live" table gains an option column; each row names the option, its default, and its vault-mode spelling
- [ ] 12.4 `README.md:1167-1171` — the anchoring paragraph names the configured encrypted root
- [ ] 12.5 `README.md:119-121, 190-192, 223-225, 241-243, 307-309, 577-579` — the six `sops secrets/safix/...` examples (including `credsCommand`'s at `:578`) gain "with the default spelling" or equivalent, so a reader who has renamed a root is not misled
- [ ] 12.6 Add a README passage on what a consumer's own `.gitignore`, backup rule or `rsync --exclude` should say once the roots are theirs, the one-parent worked example, and the hidden-directory caveat. State that safix writes no `.gitignore` at the declaration root and exactly one at the vault root, covering only the scratch rules file
- [ ] 12.7 Add a README passage on renaming a root: change the option, `git mv` the tree, `safix fix`, `safix check`. State that no re-encryption is involved in readable mode because a sops document does not embed its own path, and that `safix check` reports any governed file `.sops.yaml` no longer names, which is what a half-finished rename looks like
- [ ] 12.8 `CHANGELOG.md` — add one `[Unreleased]` entry for the three options and their two refusals, and one **BREAKING** `[Unreleased]` entry for the root-relative opaque-name derivation, naming `safix check` and `safix fix` as the migration and stating that the vault feature is itself unreleased so no released consumer is affected
- [ ] 12.9 Confirm unedited: `CHANGELOG.md:269-274`, `:333-341`, `:485-489` are history and are not rewritten
- [ ] 12.10 Confirm unedited: `examples/README.md`, `examples/plain-nix/*`, `examples/dendritic/**` carry no prefix literal — they declare only `flake.safix.*` records — so the examples demonstrate the default layout with no edit
- [ ] 12.11 Verify: every path spelled in the README's storage sections either names an option or is marked as the default spelling. There is no documentation check in `modules/flake/checks/` — verified by grep — so this one is read rather than built; `modules/flake/checks/cli.nix:486-490`'s prose assertion is the closest mechanical cover and is already named in 10.10

## 13. Spec reconciliation and whole-change verification

- [ ] 13.1 Confirm the four delta specs in this change match what landed: `public-outputs` (the three options, the two refusals, the rewritten separability requirement, the derived probes), `secret-generators` (the record's single implementation and configured root), `secrets-vault` (the root-relative hash input), `recipient-policy` (the header's derived examples)
- [ ] 13.2 Confirm unedited, with the reason recorded: `openspec/specs/secret-custody/spec.md:39-53` (audience-named directory, and its vault-mode exception) and `openspec/specs/recipient-policy/spec.md:75-90` (the anchoring requirements) are already root-agnostic — neither names a prefix — so neither needs a delta
- [ ] 13.3 `openspec validate configurable-storage-roots --strict` green (the CLI takes the change name positionally; `--change` is not a flag it accepts)
- [ ] 13.4 Verify: `nix flake check` green in full
- [ ] 13.5 Severity drill roll-up: re-run the drills in 1.15, 1.16, 2.8, 3.13, 4.8, 5.7, 6.7, 6.8, 7.12, 7.13, 8.7 and 11.9 against the landed tree and record each one's observed red/green transition in the check file's drill commentary, the way `modules/flake/checks/vault.nix` records its own. A drill that does not reproduce is a finding, not a formality: record why, as `vault.nix:4.9` does
