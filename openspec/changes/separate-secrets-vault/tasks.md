# Tasks: separate-secrets-vault

Citations are as read while designing this change; re-read the named lines before editing, since implementation may land after other changes in this program and line numbers drift.
No real fleet identifier, hostname, or recipient enters this repository; fixtures use `alice`, `bob`, and `carol`, and synthetic `age1` strings, matching the existing consumption fixtures.
Where a task says "hold", add a check that fails when the claim stops being true, not a sentence asserting it.

## 1. The vault option and the root flip

- [ ] 1.1 Add `vault` to `modules/flake/safix/options.nix`: `lib.mkOption { type = lib.types.nullOr lib.types.path; default = null; description = ...; }`, documenting that a value moves every `sopsFile` to resolve rooted there instead of at the declaring flake's own source
- [ ] 1.2 Flip `bound` in `modules/flake/safix/default.nix:47` from `root = self;` to `root = if cfg.vault != null then cfg.vault else self;`
- [ ] 1.3 Add `vaultDeclared = cfg.vault != null;` under `flake.safix.lib` in `default.nix`, alongside the other twelve `safix.lib.*` outputs
- [ ] 1.4 Add a check asserting that a fixture with no `vault` set resolves every `sopsFile` and `publicValue` path exactly as it does on `main` today (byte-identical resolved paths, over a fixture carrying at least one shared and one private secret)
- [ ] 1.5 Add a check asserting that a fixture with `vault` set to a synthetic fixture path resolves every `sopsFile` and `publicValue` path rooted there instead, and that `flake.safix.lib.vaultDeclared` is `true` on that fixture and `false` on the unset one
- [ ] 1.6 Severity drill: reverting 1.2's flip while leaving 1.3 in place turns 1.5 red on the path assertion but not on `vaultDeclared`, which is the evidence the two are independently held
- [ ] 1.7 Verify: `nix build .#checks.x86_64-linux.safix-vault-root` (or the chosen check name) green, drill in 1.6 observed

## 2. `Workspace` carries two roots

- [ ] 2.1 Add `vault_root: PathBuf` to `Workspace` (`crates/safix-core/src/workspace.rs:29-43`); add `vault_root` as a parameter to `Workspace::at`
- [ ] 2.2 Add `Workspace::vault_root(&self) -> &Path` and `Workspace::vault_absolute(&self, relative: &str) -> PathBuf`, mirroring `root`/`absolute`
- [ ] 2.3 Add `Workspace::read_vault_relative(&self, relative: &str) -> Result<Option<String>>`, identical in shape to `read_relative` but reading under `vault_root`
- [ ] 2.4 Implement `Workspace::discover`'s vault resolution per design V1: read `SAFIX_VAULT_ROOT`, evaluate `flake.safix.lib.vaultDeclared` at the declaration root, and resolve `vault_root = root` when neither is set
- [ ] 2.5 Add the two mismatch refusals from design V1 — `vaultDeclared` true with no `SAFIX_VAULT_ROOT`, and `SAFIX_VAULT_ROOT` set with `vaultDeclared` false — as new `Error` variants naming both the option and the environment variable
- [ ] 2.6 Add unit tests constructing a `Workspace` via `at` with `vault_root != root` and asserting `vault_absolute`/`read_vault_relative` join against `vault_root` while `absolute`/`read_relative` still join against `root`
- [ ] 2.7 Add integration-level tests (following this crate's existing `SAFIX_REPO_ROOT`-driven fixture pattern) for `discover`'s four resolution branches in 2.4, and both refusals in 2.5
- [ ] 2.8 Severity drill: setting only one of the two signals in each of the four fixtures from 2.7 turns the corresponding refusal test red; setting neither leaves `vault_root == root` observed by the test in 2.6's style
- [ ] 2.9 Verify: `cargo test -p safix-core workspace::` green, both refusal tests and the drill in 2.8 observed

## 3. The vault-is-a-git-repository refusal

- [ ] 3.1 Add a check, folded into `Workspace` discovery, running `git -C <vault_root> rev-parse --show-toplevel` and comparing the canonicalized result against `vault_root`
- [ ] 3.2 Add a new `Error` variant naming the vault path and stating it must be a git repository's top level, raised on a failed git invocation there
- [ ] 3.3 Add a second variant for the top-level mismatch case, naming both the path git found and the path named, raised when the two disagree
- [ ] 3.4 Add fixture tests: a `vault_root` pointing at a plain directory (3.2's case), a `vault_root` pointing at a subdirectory of a real git repository (3.3's case), and a `vault_root` that is itself a repository's top level (passes)
- [ ] 3.5 Severity drill: pointing `vault_root` at the subdirectory case while asserting only 3.2's variant fires turns the drill red, which is the evidence the two refusals are distinguished rather than collapsed into one message
- [ ] 3.6 Verify: `cargo test -p safix-core workspace::vault_repository` green, drill in 3.5 observed

## 4. Vault-rooted artifacts

- [ ] 4.1 Move `.sops.yaml` write sites to `vault_root`/`vault_absolute`: `fix.rs:101-119`'s `write_policy`, `adduser.rs:168-180`, `group.rs:219-233`'s `regenerate_policy`
- [ ] 4.2 Move `.sops.yaml`'s read site to `vault_absolute` in `check.rs:186-206`'s `policy` function
- [ ] 4.3 Update every caller of `Sops::create_empty_document` and `Sops::update_keys_command` to pass `workspace.vault_root()` as the `root` argument instead of `workspace.root()`; update the doc comment at `sops/mod.rs:292-294` to name the vault root
- [ ] 4.4 Re-attribute every `read_relative` caller per design V3's row 3: `group.rs:94-98` and `enroll/mod.rs:315-319` stay `read_relative`; `edit.rs:227`, `generate.rs:462`, `bridge.rs:443`, and `check.rs:477`/`:511` move to `read_vault_relative`
- [ ] 4.5 Move `enroll/proof.rs:150-173`'s `decrypt_with` to `workspace.vault_root()`/`workspace.vault_absolute(relative)`
- [ ] 4.6 Add a check or test asserting every governed file, `.sops.yaml`, and generator definition record a fixture run produces lands under `vault_root` and nothing of that kind lands under `root` when the two differ
- [ ] 4.7 Severity drill: reverting 4.1 alone while keeping 4.2 turns 4.6 red on a `.sops.yaml`-not-found-at-vault-root assertion, which is the evidence the write and read sites are held as a pair rather than independently
- [ ] 4.8 Verify: `cargo test -p safix-core` green for the touched modules, drill in 4.7 observed

## 5. Commit ordering and the preflight

- [ ] 5.1 Generalize `refuse_bad_repository_state` (`set.rs:224-250`) into a function taking a `Workspace` and the set of `(root, relative)` pairs an operation will touch, checking each in order and refusing on the first failure before any write
- [ ] 5.2 Wire the generalized preflight into `adduser.rs`, `group.rs`, `enroll/mod.rs`, `set.rs`, `fix.rs`, and `generate.rs`, each naming the roots and paths it actually touches (vault-only for `set`/`fix`/`generate`; both for `adduser`/`group`/`enroll`)
- [ ] 5.3 Split the `written` vector at `adduser.rs:163`, `group.rs:187`, and `enroll/mod.rs:373` into `written_vault` and `written_declaration`
- [ ] 5.4 Change `set.rs:196-202` and `generate.rs:855-863`'s single `commit_written_files` call to pass `workspace.vault_root()`
- [ ] 5.5 Change `adduser.rs:182-191`, `group.rs:182-186`, and `enroll/mod.rs:390-396` to call `commit_written_files` twice: `vault_root` with `written_vault` first, then `root` with `written_declaration`, the second message carrying a `Safix-Vault: <short-id>` trailer built from `Git::head_short(vault_root)` read after the first commit lands
- [ ] 5.6 Add a new `Error` variant for the half-landed state, carrying the vault commit id and the pending declaration-root paths, raised when the vault-root commit in 5.5 succeeds and the declaration-root commit fails
- [ ] 5.7 Add a fixture test driving `adduser` (or `group`) with the declaration-root commit forced to fail (a stubbed git driver refusing the second `commit_paths` call), asserting the vault-root commit is observed to have landed and the error names its id
- [ ] 5.8 Add a fixture test re-running the same operation after 5.7's failure with the stub restored to succeeding, asserting no second vault-root commit is made (`git log` in the vault fixture shows one commit for the operation, not two) and the declaration-root commit lands carrying the trailer
- [ ] 5.9 Add fixture tests for the preflight's two single-root-dirty cases and its both-clean case, asserting nothing is written at either root in the two dirty cases
- [ ] 5.10 Severity drill: skipping the preflight's vault-root check while leaving the declaration-root check in place turns the vault-dirty case in 5.9 green when it should refuse, which is the evidence the two checks are independently load-bearing
- [ ] 5.11 Verify: `cargo test -p safix-core` green for `adduser`, `group`, `enroll`, `set`, `fix`, `generate`; every test and drill in this group observed

## 6. The scratch sweep floor becomes two floors

- [ ] 6.1 Change `scratch::Registry.floor: Option<PathBuf>` to `floors: Vec<PathBuf>`; change `set_floor` to append de-duplicated rather than replace
- [ ] 6.2 Change `remove_empty_upwards`'s stop condition from equality against one floor to membership in `floors`
- [ ] 6.3 Update `set.rs:111`, `generate.rs:147`, `bridge.rs:229`, `sync.rs:246` to floor at `workspace.vault_root()`
- [ ] 6.4 Update `adduser.rs:104` and the two sites in `enroll/mod.rs` and `group.rs` that register scratch paths under both roots to call `set_floor` twice, once per root
- [ ] 6.5 Add a unit test registering scratch files under two distinct floors and asserting `cleanup`/`remove_empty_upwards` respects both, removing an empty directory at either without disturbing the other
- [ ] 6.6 Severity drill: reverting 6.1-6.2 while keeping 6.4's double `set_floor` call turns 6.5 red, since the second call would silently overwrite the first floor under the old `Option`-replacing behaviour rather than accumulate
- [ ] 6.7 Verify: `cargo test -p safix-core scratch::` green, drill in 6.6 observed

## 7. What stays put, held rather than assumed

- [ ] 7.1 Add a check or test asserting `Nix::shell` and the `nix eval` target in `nix.rs` are constructed from `declaration_root` alone, unaffected by whether `vault_root` differs from it, over a fixture pair identical except for `vault_root`
- [ ] 7.2 Add a fixture test asserting `Git::author_identity` reads from `declaration_root`'s git configuration even when a commit's content is entirely vault-rooted (a `set` operation, whose commit lands only at `vault_root`, still authored per `declaration_root`'s identity)
- [ ] 7.3 Add a fixture test asserting the onboarding hook (`adduser.rs`) and the enroll hook (`enroll/mod.rs`) both run with `current_dir` at `declaration_root`, unchanged from today, over a fixture where the two roots differ
- [ ] 7.4 Severity drill: for each of 7.1-7.3, flipping the corresponding site to the wrong root turns that test red and no other test in this group red, which is the evidence each is independently held
- [ ] 7.5 Verify: all three checks/tests and the drill in 7.4 observed; this group intentionally adds no new behaviour, only coverage for behaviour this change must not disturb

## 8. The lock-bump disclosure

- [ ] 8.1 Add a function computing the disclosure message: on success of a vault-root commit, run `nix flake metadata <declaration_root> --json`, search `.locks.nodes` for a node whose locked source matches `vault_root` (by path, falling back to a freshly computed NAR hash comparison), and format the specific or the general message per design V6
- [ ] 8.2 Wire the disclosure into every vault-root commit's success path: `set.rs`, `fix.rs`, `generate.rs`, and the vault half of `adduser.rs`/`group.rs`/`enroll/mod.rs`
- [ ] 8.3 Add a fixture test with a declaring flake whose lock names exactly one input matching the vault fixture, asserting the disclosure names that input and the exact `nix flake lock --update-input` command
- [ ] 8.4 Add a fixture test with a lock naming zero or more than one matching input, asserting the disclosure falls back to the general phrasing and names no input
- [ ] 8.5 Severity drill: corrupting the fixture lock's matching entry in 8.3 turns that test red and 8.4 unaffected, which is the evidence the two paths are independently reached
- [ ] 8.6 Verify: both fixture tests and the drill in 8.5 observed; measure `nix flake metadata`'s wall time against this repository's own flake and record it beside the open question in `design.md`, resolving it one way or the other before this task is checked off

## 9. Capabilities checked and left alone

- [ ] 9.1 Re-run `secret-custody`'s existing check suite unmodified against a fixture with `vault` set, asserting every existing assertion still passes with no new failure, holding "nothing in `secret-custody` names a repository" as a fact rather than an inference
- [ ] 9.2 Do the same for `secret-installation` and `secret-consumption`'s existing suites, over a fixture that both declares a vault and consumes a resolved set at both scopes
- [ ] 9.3 Do the same for `public-outputs`'s existing suite, specifically its prefix-separation and catch-all-policy assertions, over a vault fixture
- [ ] 9.4 Verify: 9.1-9.3 all green with zero modification to the check files those capabilities already own, which is the evidence this change's "not touched" list in `design.md` holds

## 10. Documentation

- [ ] 10.1 Document `flake.safix.vault` at its option declaration: what it does, its default, and the lock-bump cost accepting it incurs
- [ ] 10.2 Document `SAFIX_VAULT_ROOT` beside `SAFIX_REPO_ROOT` wherever the latter is documented for operators
- [ ] 10.3 Document the two-commit sequence, the half-landed refusal, and its safe-to-re-run guarantee in the README's secrets-lifecycle section
- [ ] 10.4 Document the migration steps from `design.md`'s Migration Plan for an existing consumer adopting a vault
- [ ] 10.5 Verify: every guarantee stated in the README names a check or test in this repository that holds it

## 11. Verification

- [ ] 11.1 `openspec validate separate-secrets-vault --strict`
- [ ] 11.2 `openspec validate --all --strict`, compared against the baseline recorded when this change was proposed
- [ ] 11.3 `nix eval .#checks.x86_64-linux --apply builtins.attrNames` lists every check named in groups 1-9
- [ ] 11.4 `nix flake check` green
- [ ] 11.5 `cargo test` green
- [ ] 11.6 `rg` the whole tree for any real fleet identifier and confirm none
