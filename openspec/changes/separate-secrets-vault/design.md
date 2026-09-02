# Design: a secrets vault that is its own repository

## Context

Two facts fix the shape of this change and neither is up for renegotiation here.

`sopsFile` is `lib.types.path` (`${sops-nix}/modules/sops/default.nix:136`) and a path-valued expression rooted at a different flake input resolves fine — that is the entire mechanism D3/C5 rest on, and it needs no cooperation from sops-nix.
`.sops.yaml` must sit in the directory sops runs from, because `updatekeys` and `--filename-override` resolve `path_regex` relative to the config file's own directory (`crates/safix-core/src/sops/mod.rs:292-294`), so the policy file moves into the vault with the ciphertext it governs, never staying beside the declarations that describe who should hold it.

Today, `root` in `modules/flake/safix/default.nix:47` is bound to `self` — the declaring flake's own source — and used at exactly two other sites: `publicValueIn` (`modules/flake/safix/resolve.nix:648`) and `materializeFor`'s two `sopsFile` constructions (`resolve.nix:2021`, `:2093`).
Everything else the resolver exposes to the command-line runtime — `placements`, `governedFiles`, `outputPath` — is a plain repository-relative string, with no `root` concatenated in nix at all; the Rust runtime does that joining itself, in `Workspace::absolute`.
That split is what makes this change tractable: the nix-side flip from `self` to `cfg.vault` (contract C5) touches three call sites, and the Rust runtime's fourteen touch points are about which of two filesystem roots each `join` targets, not about re-plumbing nix evaluation.

`flake.safix.vault` is a `flake = false` input in the ordinary nix sense: the vault is fetched as a plain tree and never itself evaluated as a flake, so `support-plain-nix-consumers`'s `mkVault` (contract C1) has nothing to do here.
This change's actual dependency on that program is narrower and indirect: it lands second (contract D7) and its two-root discovery has to compose with whatever entrypoint form — `<root>#attribute` or `--entry <file>` (contract C2) — that change establishes for evaluating the declaration root; §"What deliberately does not change" states exactly how.

## Goals / Non-Goals

**Goals:**

Make the vault path the projection root when declared, with the one-line flip C5 already specifies.
Make the command-line runtime carry two independent, cross-validated roots, defaulting to one when no vault is declared.
Replace the single-commit atomicity every write path relies on today with an ordered, preflight-checked, safely-re-runnable two-commit sequence.
Refuse a vault that is not a git repository, before writing anything.
Disclose the lock-bump cost D3 already accepts, at the point the operator can act on it.

**Non-Goals:**

The vault having a flake, `mkVault`, or any nix evaluation of the vault's own tree — it is read-only, plain-tree content from nix's perspective.
Verifying the vault's local clone is fresh against its git remote before writing — that is a push-time concern this change's commit-ordering machinery does not need to solve to be correct, and folding it in would make every write path depend on network reachability it does not otherwise need.
Multiple vaults per consumer, or a vault scoped per-user or per-machine — `flake.safix.vault` is one option, and the ordinary nix module system's own conflicting-definition refusal is adequate for a plain `nullOr path`, the way it is for every other scalar option in `modules/flake/safix/options.nix` that is not `oneClanFlake`.
Any change to `secret-custody`, `secret-installation`, `secret-consumption`, `public-outputs`, or `secret-catalogue` — §"Capabilities considered and not touched" records why each was checked and left alone.

## Decisions

### V1. Two roots, cross-validated at discovery, defaulting to one

`Workspace` gains a `vault_root: PathBuf` field alongside `root`, and `Workspace::at` gains a `vault_root: PathBuf` parameter.
`Workspace::discover` resolves it from a new `SAFIX_VAULT_ROOT` environment variable, mirroring the existing `SAFIX_REPO_ROOT` override (`crates/safix-core/src/git.rs:60-79`) rather than inventing a second discovery mechanism.

Discovery cross-validates against a new nix output, `flake.safix.lib.vaultDeclared` (a boolean, exposed the way the other twelve `safix.lib.*` attributes are), evaluated once at declaration root exactly as every other attribute is:

- Neither `SAFIX_VAULT_ROOT` nor `vaultDeclared` set: `vault_root = root`, today's behaviour, unchanged.
- Both set: `vault_root` is the named path, used for every vault-rooted touch point below.
- `vaultDeclared` true, `SAFIX_VAULT_ROOT` unset: refuse before evaluating or writing anything, naming `flake.safix.vault` and the environment variable.
- `SAFIX_VAULT_ROOT` set, `vaultDeclared` false: refuse, naming the same two things, because a root named for a vault nix does not know about would silently do nothing — every vault-rooted write would land somewhere no consuming build ever reads from.

**Alternative rejected**: deriving the vault's live checkout path from the flake input itself, by reading `inputs.<name>.outPath` at eval time.
That path is the locked, store-copied snapshot — read-only, and stale until the very lock bump this change discloses — and cannot receive a git commit.
The runtime needs the operator's actual working tree, which nix's lock file structurally cannot name, so an explicit, operator-supplied root is not a workaround; it is the only correct source for a mutable path nix's own resolution model deliberately does not expose.

### V2. The vault must be a git repository, verified before writing

`Workspace` verifies, at first use of the vault root (folded into the same validation V1 performs), that `git -C <vault_root> rev-parse --show-toplevel` succeeds and its output, canonicalized, equals `vault_root`.
A failure to run git there, or a result naming a different top level, refuses naming the path found and the path named, before any file is written.

**Alternative rejected**: allowing a vault that is a plain directory or a tarball-style export, with its own bespoke sync logic.
That would mean building a second, parallel mechanism for the dirty-state refusal, the mid-operation refusal, and the conflict-entry refusal that `Git` already provides for the declaration root — doubling the surface this change has to get right for no benefit, since D3 already commits the vault to being a flake input, which is git-hosted by construction.

### V3. What moves to the vault root, in one pass across the fourteen touch points

Every touch point named in this change's scope is disposed of below.
Citations are to the lines read while designing this change; a caller's line number may have moved by the time this is implemented, and the symbol name is the anchor that survives that drift.

| # | Touch point | Current behaviour | Disposition |
|---|---|---|---|
| 1 | `Workspace` struct and `Workspace::at`/`discover` (`workspace.rs:29-79`) | One `root: PathBuf` field; `at` takes it, `discover` resolves it from git | Add `vault_root: PathBuf`; `at` takes it as a parameter; `discover` resolves and cross-validates it per V1 and V2 |
| 2 | `Workspace::absolute` (`:104-106`) | `self.root.join(relative)` | Unchanged; gains a sibling `vault_absolute(&self, relative: &str) -> PathBuf` doing `self.vault_root.join(relative)`, used wherever a touch point below reads or writes a vault-rooted artifact |
| 3 | `Workspace::read_relative` (`:352-364`) | Reads a `root`-relative file, used today for both declaration files (group and user declarations) and vault-rooted ones (ciphertext key indices, generator definition records) | Split: `read_relative` keeps reading declaration-rooted files (group.rs's group declaration, enroll/mod.rs's user declaration); a new `read_vault_relative` reads vault-rooted ones (edit.rs, generate.rs's `Target::Secret`, bridge.rs's mapping documents, check.rs's definition records and governed-file cache). Each caller is re-attributed by which kind of file it reads, not left to guess from one method |
| 4 | `.sops.yaml` write sites (`fix.rs:102-104`, `adduser.rs:168-176`, `group.rs:220-228`) | `workspace.root().join(".sops.yaml")`, staged and renamed into place | `workspace.vault_root().join(".sops.yaml")`; the staging-then-rename pattern is unchanged |
| 5 | `.sops.yaml` read site (`check.rs:188`) | `workspace.absolute(".sops.yaml")` | `workspace.vault_absolute(".sops.yaml")` |
| 6 | sops working directory and `--filename-override` (`sops/mod.rs:202-204`, `:301`) | `current_dir(root)` where callers pass `workspace.root()` | Driver unchanged (already takes `root: &Path` per call); every caller passes `workspace.vault_root()` instead. The doc comment at `:292-294`, which states the working directory is the repository root because sops resolves `path_regex` relative to the config it read, is updated to say the vault root |
| 7 | git driver root (`git.rs:198-220`, `Git`'s per-call `root: &Path` pattern) and every `commit_written_files` caller (`adduser.rs`, `group.rs`, `set.rs`, `generate.rs`, `enroll/mod.rs`) | One call per operation, one root, one flat list of relative paths | `Git` itself is unchanged — it already takes `root` per call. `set.rs` and `generate.rs` (vault-only writers) switch their single call from `workspace.root()` to `workspace.vault_root()`. `adduser.rs`, `group.rs`, and `enroll/mod.rs` (cross-root writers) call it twice: vault root first with vault-relative paths, declaration root second with declaration-relative paths, per V4 |
| 8 | single-commit atomicity (`adduser.rs:163`, `group.rs:187`, `enroll/mod.rs:373`, the `written` vector each site builds) | One vector mixing a `.nix` scaffold or edit with `.sops.yaml` and re-wrapped governed files | Split into `written_vault` (ciphertext, `.sops.yaml`, re-wrapped governed files) and `written_declaration` (the scaffold or edited declaration file), staged and committed separately per V4 |
| 9 | `refuse_bad_repository_state` (`set.rs:224-250`) | Checks one root's mid-operation state, conflict entries, and dirty status for one relative path | Generalized into a preflight run before any write: for a vault-only operation, run it once against the vault root; for a cross-root operation, run it against both roots for the paths each will touch, refusing on the first failure, before either root is written (V4) |
| 10 | `Git::author_identity` (`git.rs:198-220`, read at `delegation.rs:261`) | `workspace.git().author_identity(workspace.root())` | Unchanged. Authorship is a declaration-root question — who is editing the declarations — regardless of which root a commit's content lands in, matching the `secrets-vault` capability's explicit requirement. Recorded here because an implementer reaching for symmetry with touch point 7 is the likely mistake |
| 11 | scratch sweep floor `scratch::set_floor` (`set.rs:111`, `adduser.rs:104`, `generate.rs:147`, `bridge.rs:229`, `sync.rs:246`) | One global floor, `Registry.floor: Option<PathBuf>`, compared for exact equality as `remove_empty_upwards` walks up from a leaf | V5 |
| 12 | (folded into 11) `bridge.rs:229`, `sync.rs:246` specifically | Both floor at `workspace.root()` | Both floor at `workspace.vault_root()` alone: `bridge`'s `one_import`/`one_export` and `sync`'s push/pull decisions write only ciphertext, through the same path `set.rs` does, never a declaration scaffold |
| 13 | `--inputs-from` and the eval reference (`nix.rs:91` region / `shell()` at `:205-213`, `target()` at `:93-98`) | Both take `root`, the declaration root | Unchanged. Nix evaluation of `safix.lib.*` always targets the declaration root; the vault only ever appears as a `root`-prefixed path *inside* that evaluation's result (touch points 2 and elsewhere), never as a second evaluation target. Recorded here because the temptation to route evaluation through the vault, since it holds ciphertext, is the likely mistake — verified against the resolver, not assumed |
| 14 | `.current_dir` sites (`enroll/proof.rs:158-159`, `adduser.rs:379-380`, `enroll/mod.rs:495-496`) | All three run at `workspace.root()` | `enroll/proof.rs`'s `sops decrypt` operates on a governed ciphertext file and moves to `workspace.vault_root()`/`workspace.vault_absolute()`. `adduser.rs`'s onboarding hook and `enroll/mod.rs`'s enroll hook stay at `workspace.root()`: both are consumer-supplied scripts that fire after a declaration-root commit and are documented, in `secret-consumption`'s neighbourhood, as declaration-side extension points, not ciphertext operations |

Touch points 2 and 3 are the two `Workspace` accessors every other row routes through; row 1 is where both roots become available in the first place.
Nothing in this table is left undecided.

### V4. Commit ordering, the preflight, and the half-landed refusal

The write-then-stage ordering that exists today for a stronger reason — an evaluation must see a scaffold staged before regenerating a policy that should include it (`adduser.rs:157-162`'s own comment) — is unchanged: the declaration-root scaffold is written and staged (`git add`, not committed) before the vault-root policy is regenerated, because that evaluation still runs at the declaration root and still needs to see the staged file.
What changes is the **commit** order, which flips relative to the stage order: vault root commits first, declaration root commits second.

The reason is a safety argument, not a convention, and it is the one the `secrets-vault` capability states as a requirement rather than leaving in this document alone: the declaring flake must never be able to claim, through a committed declaration, a custody grant that the vault's committed policy has not yet been re-wrapped for.
Consider `group add`, widening a shared secret's audience.
Declaration-first: the group membership commits, and if the vault-side policy regeneration then fails, the declaring flake now asserts an audience the committed `.sops.yaml` does not yet grant — a `safix set` run against that file in the interim would silently wrap the new value to the *old*, narrower recipient list, which is a custody gap no refusal catches, because nothing has refused yet.
Vault-first: if the declaration commit then fails, the vault has committed content — an anchor, a wider rule, or ciphertext — that no live declaration references yet, which is inert by the same reasoning `recipient-policy`'s own "a declared person holding nothing yet" scenario already gives: extra capacity nobody has claimed costs nothing.

Concretely, per operation that touches both roots:

1. Preflight both roots for the paths this operation will touch — the generalized `refuse_bad_repository_state` from touch point 9 — vault root first, refusing immediately and writing nothing at either root if either check fails.
2. Perform the declaration-root scaffold write and `git add` (not commit), as today, so the vault-root evaluation below sees it.
3. Perform the vault-root writes (ciphertext, `.sops.yaml`, re-wrapped governed files) and commit them via `commit_written_files` against `vault_root`.
   Because that function already stages exactly the named paths and reports "nothing to commit" when content is unchanged (`git.rs:270-291`), a retry whose vault content already matches produces no duplicate commit — this is the entire mechanism that makes re-running safe, and it needed no new code, only reuse.
4. Read the vault commit's short id (`Git::head_short`, already `git.rs:188-196`) and commit the declaration-root scaffold via `commit_written_files` against `root`, with a trailer line `Safix-Vault: <short-id>` appended to the message.
5. If step 3 succeeds and step 4 fails, return a named error carrying the vault commit id and the declaration-root paths still staged, whose message states that re-running the same command completes the operation and will not repeat the vault commit.

**Alternative rejected, for the ordering**: committing declaration-first.
Argued above; it is the direction in which a failure is a silent custody gap rather than an inert extra.

**Alternative rejected, for detecting the half-landed state**: a separate marker file recording "a two-phase operation is in progress."
Unnecessary — git is already the marker.
The vault's own history is the record of which phase completed; a re-run's write-then-stage of identical content against an unchanged vault HEAD is itself the check, and `commit_written_files`'s existing idempotence is what makes it free.

**Alternative rejected, for the preflight**: checking each root's cleanliness only when that root is about to be written, rather than both up front.
That risks a vault-root write and commit completing before discovering the declaration root is mid-rebase — turning an avoidable refusal into the very half-landed state step 5 exists to name, on every run where the declaration root happens to be the dirty one.
Checking both first is strictly cheaper: it costs one extra `git` invocation against a root that was going to be touched anyway, and it never leaves a partial write on the floor.

### V5. The scratch sweep floor becomes two floors

`Registry.floor: Option<PathBuf>` becomes `floors: Vec<PathBuf>`, holding at most two entries — one per root a run actually registers scratch paths under.
`set_floor` appends (de-duplicated by equality) instead of replacing.
`remove_empty_upwards`'s stop condition changes from `floor == Some(current)` to `floors.iter().any(|f| f == &current)`; nothing else about the walk changes.

This closes a real gap rather than a hypothetical one: with a single floor set to the declaration root, a scratch file created under the vault root (a candidate ciphertext file, say) sweeps upward comparing against a floor it will never reach, relying entirely on the vault root's own `.git` directory making `remove_dir` fail there by incidence — the module's own doc calls the floor comparison "belt and braces" *on top of* that incidental protection, not a replacement for it, and a two-root runtime is exactly the case where the incidental protection is the only one actually in force for whichever root the floor was not set to.
`adduser.rs`, `group.rs`, and `enroll/mod.rs` (which register scratch paths under both roots) call `set_floor` twice; `set.rs`, `generate.rs`, `bridge.rs`, and `sync.rs` (vault-only) call it once, at `vault_root`.

**Alternative rejected**: leaving one floor and accepting the incidental `.git`-directory protection as sufficient.
It happens to hold today because both roots are git repositories, but relying on an incidental property to stand in for a stated invariant is exactly the pattern this codebase's own comments avoid elsewhere — the fix is one field becoming a two-element vector and one equality check becoming an `any`, which is cheaper than the audit trail needed to keep re-justifying the incidental version.

### V6. The lock-bump disclosure

After step 4 of V4 commits (or after `set.rs`'s or `fix.rs`'s single vault-only commit), the runtime prints that the change is not visible to any consuming build until the declaring flake's lock entry for the vault is updated.
It attempts to name the exact remedy by running `nix flake metadata <declaration_root> --json`, reading `.locks.nodes`, and searching for exactly one node whose locked source resolves to `vault_root` (by path or by matching NAR hash against a freshly computed one).
When exactly one match is found, the disclosure names that input and prints `nix flake lock --update-input <name>`.
When none or more than one match is found, the disclosure states the same requirement in terms general enough to remain true without guessing a name.

**Alternative rejected**: staying silent and trusting the operator to remember.
Inconsistent with this codebase's own standard — `own-secret-installer`'s D7 states a refusal in the package's own words specifically because leaving a gap to the provisioner's unrelated assertion left no refusal at all — and the cost here is exactly symmetric: a `set` that succeeds with no disclosure is a `nixos-rebuild` away from an operator asking why nothing changed.

**Alternative rejected**: automatically running the lock update as part of every vault-touching command.
That would rewrite the declaring flake's `flake.lock` as a side effect of an operation whose stated scope is the vault, mixing two repositories' version-control history into one action the operator did not ask for, and a lock update can itself conflict with a concurrent edit to `flake.nix` that the runtime has no visibility into.

### V7. What deliberately does not change

Recorded here because each is the shape of the likely wrong turn, not because any of the four needed a decision beyond confirming it stays put.

Nix evaluation of `safix.lib.*` always targets the declaration root (touch point 13); the vault never becomes a second evaluation target, because every nix-level use of `root` (V3's opening paragraph) is a path concatenation *inside* that one evaluation, not a second `nix eval` invocation.
Git author identity is always read from the declaration root (touch point 10), because authorship is a question about who edited the declarations, independent of which root a commit's content lands in.
The two consumer-supplied hooks (onboarding, enroll) run at the declaration root (touch point 14's other two sites), because both are declaration-side extension points that fire after a declaration-root commit, not ciphertext operations.
The composition with `support-plain-nix-consumers`'s flakeless entrypoint (contract C2) needs no new mechanism in flake mode: `--inputs-from` and the eval target both name the declaration root exactly as they do today, and the vault's resolution happens entirely inside that one evaluation's result, so nothing about the vault changes what `nix.rs` passes to `nix eval`.
In flakeless mode (`--entry <file>` / `SAFIX_ENTRY`), the operator's own entry file is what would supply `flake.safix.vault` as a literal `nullOr path`, exactly as it supplies every other `flake.safix.*` value in that mode; nothing about vault discovery requires the flake-input override machinery V1's rejected alternative considered and discarded.

## Capabilities considered and not touched

`secret-custody` states audience algebra, revocation, and the user record's fields; none of it names a repository, a file location, or a commit, so nothing in it becomes false when ciphertext moves to a second repository.
`secret-installation` and `secret-consumption` state how a resolved entry arrives on a machine; `sopsFile` is a path there exactly as it is here, and neither spec states where that path is rooted, so C5's flip does not touch either.
`public-outputs` states that a public output "is stored under a top-level prefix distinct from the one holding encrypted material" and "is stored as plaintext in the repository" — both survive unedited, because the requirement is repository-relative and repository-agnostic: it is still one repository, still one prefix separation, and that repository is now the vault when one is declared, which the requirement never named in the first place.
`secret-catalogue`'s known defect — that it requires the catalogue be "an attribute set option on a flake-parts module" — belongs to `support-plain-nix-consumers`, which relaxes flake-parts as a hard dependency; nothing in this change touches that requirement.
`safix-cli` states the subcommand surface, the pipe discipline, and the read/write split; none of its requirements assert single-repository or single-commit behaviour by name, so the two-commit sequence V4 introduces is new behaviour best stated once, in `secrets-vault`, rather than as an edit to a capability that never made the claim being changed.

## Risks / Trade-offs

Two commits instead of one means two objects appear in `git log` for one logical operator action, in two different repositories — a reviewer of the declaration repository's history sees "declare alice and regenerate the recipient policy" without the ciphertext diff, which lives in a vault they may or may not have read access to.
That is not incidental; it is the entire point of a separately-permissioned vault, and the `Safix-Vault:` trailer (V4) is the mitigation, giving anyone with declaration-root access a durable pointer to the vault commit without granting them the ability to read it.

The half-landed state (V4, step 5) is a real operational state an operator can observe, not merely a theoretical one — a re-run resolves it, but an operator who does not re-run leaves a vault ahead of a declaration that never lands, silently accumulating inert vault history.
No check in this program can force a re-run; the refusal states the remedy and that is the limit of what a command-line tool can guarantee.

The lock-bump discovery in V6 runs `nix flake metadata`, an extra process per vault-touching commit; on a large flake this is a real latency cost, and if it proves objectionable at implementation time the fallback (a general disclosure naming no input) is always correct, just less specific — this is deferred to measurement, not decided here, since neither answer changes the specs, the approach, or the task breakdown.

Coexistence with `support-plain-nix-consumers`'s flakeless mode is asserted, not yet measured, because that change's artifacts do not exist yet at the time this one is authored; V7's last paragraph states why no new mechanism is expected to be needed, and the task list drills it once both changes have landed.

## Migration Plan

Within this repository, the flip is additive at the option level: `flake.safix.vault` defaults to `null`, so every existing consumer, including this repository's own checks, is unaffected until a vault is declared.
Declaring a vault for the first time on an existing consumer requires, in order: creating the vault repository, moving the existing `.sops.yaml` and every governed ciphertext file into it via `git mv` across the two repositories (a manual step this change does not automate, since it is a one-time migration rather than a runtime concern), committing that move in the vault, declaring `flake.safix.vault` and setting `SAFIX_VAULT_ROOT`, and then verifying with `safix check` that the resolver's view and the vault's committed state agree before the next write.

Rollback is symmetric: unset `flake.safix.vault`, move `.sops.yaml` and the governed files back with `git mv`, and unset `SAFIX_VAULT_ROOT`.
Nothing in this change makes that reversible path harder than it already reads as; no data format changes, only which repository holds which file.

## Open Questions

Whether the `nix flake metadata`-based input-name discovery in V6 is worth its per-command latency, or whether the disclosure should always use the general phrasing and let the operator supply the exact input name from their own knowledge of their flake.
Neither answer changes the `secrets-vault` capability's stated requirement — both scenarios in "A vault commit discloses the lock bump it requires" already account for the input name being determined or not — so this is deferred to an implementation-time measurement against a representative flake, not resolved here.
