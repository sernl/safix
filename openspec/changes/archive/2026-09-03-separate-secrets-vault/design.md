# Design: a secrets vault that is its own repository

## Amendment, 2026-09-03

Amended per `docs/notes/research/zero-knowledge-vault.md`, option O3, before this change is adopted.
Seven new decisions, V8 through V14 below, make the vault layout opaque by construction and keep the recipient policy out of the vault entirely; each cites the file and line it rests on, in this repository's worktree or in the pinned reference clones the research note used (sops-nix `a8627b21`, sops `v3.13.3`, clan-core `56e35624`).
V10 reverses V3's row 4 and row 5 above: `.sops.yaml`'s write and read sites stay at the declaration root; only a disposable, gitignored rendering of its creation rules ever reaches the vault, for `encrypt` and `updatekeys` alone.
`secrets-vault`'s scenario "A vault declared" — "the same audience still derives the same relative file name" — is reversed by V9; see the delta spec.
The Migration Plan below is rewritten by V13: moving an existing consumer's ciphertext into a vault is a decrypt-then-encrypt of every leaf and every key name, not a `git mv`, because a vault's names are keyed by the naming key and a `git mv` would carry the readable name across.
Everything else in V1 through V7 and the rest of this Context is unchanged.

## Context

Two facts fix the shape of this change and neither is up for renegotiation here.

`sopsFile` is `lib.types.path` (`${sops-nix}/modules/sops/default.nix:136`) and a path-valued expression rooted at a different flake input resolves fine — that is the entire mechanism D3/C5 rest on, and it needs no cooperation from sops-nix.
`.sops.yaml` must sit in the directory sops runs from, because `updatekeys` and `--filename-override` resolve `path_regex` relative to the config file's own directory (`crates/safix-core/src/sops/mod.rs:292-294`), so the policy file moves into the vault with the ciphertext it governs, never staying beside the declarations that describe who should hold it.

Today, `root` in `modules/flake/safix/default.nix:47` is bound to `self` — the declaring flake's own source — and used at exactly two other sites: `publicValueIn` (`modules/flake/safix/resolve.nix:648`) and `materializeFor`'s two `sopsFile` constructions (`resolve.nix:2021`, `:2093`).
Everything else the resolver exposes to the command-line runtime — `placements`, `governedFiles`, `outputPath` — is a plain repository-relative string, with no `root` concatenated in nix at all; the Rust runtime does that joining itself, in `Workspace::absolute`.
That split is what makes this change tractable: the nix-side flip from `self` to `cfg.vault` (contract C5) touches three call sites, and the Rust runtime's fourteen touch points are about which of two filesystem roots each `join` targets, not about re-plumbing nix evaluation.

`flake.safix.vault` is a `flake = false` input in the ordinary nix sense: the vault is fetched as a plain tree and never itself evaluated as a flake, so `support-plain-nix-consumers`'s `mkVault` (contract C1) has nothing to do here.
This change's actual dependency on that program is narrower and indirect: it lands second (contract D7) and its two-root discovery has to compose with whatever entrypoint form — `<root>#attribute` or `--entry <file>` (contract C2) — that change establishes for evaluating the declaration root; §"What deliberately does not change" states exactly how.

Four more facts fix the shape of this amendment, read directly against the pinned revisions the research note used.
`builtins.hashString` accepts exactly `md5`, `sha1`, `sha256`, `sha512` (plus `blake3` behind an experimental feature), returns lowercase hex, and is a pure function of its two string arguments with no environment access (nix 2.34.8 `src/libexpr/primops.cc:4593-4605`; confirmed end to end against a throwaway flake by one of the research reports this amendment draws on).
A `flake = false` input's `outPath` is a string carrying store-path context rather than a `path` value, so `vault.root + "/" + name` is itself a string; `builtins.pathExists` and `builtins.readFile` both accept it, and sops-nix's own `sopsFile` validation accepts a store-prefixed string as well as a path (`sops-nix modules/sops/manifest-for.nix:11-28`).
Every leaf's ciphertext is bound to its own key path as AES-GCM associated data — `pathString := strings.Join(path, ":") + ":"` (`sops` `sops.go:604-611`) — so an opaque key name is fixed at encrypt time and a later rename is a re-encryption, never a rewrite.
sops's global `--config` flag disables `.sops.yaml` discovery and accepts a path anywhere on the filesystem; `path_regex` is then matched against the document's absolute path with the *config's own directory* trimmed as a prefix when the document sits under it, and against the full absolute path otherwise (`sops` `config/config.go:576-602`; `cmd/sops/main.go:1845-1849`); `sops decrypt` and `sops set` on an already-encrypted file need no creation rule at all — `set`'s own `loadConfig` failure is explicitly tolerated (`cmd/sops/main.go:1944-1949`, `:1985`) because both re-wrap against the document's own existing metadata rather than against a freshly matched rule.

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
Hiding sops recipient public keys is a permanent non-goal, not a deferral: every sops document lists `sops.age[].recipient` in the clear (`sops` `age/keysource.go:285-290`; `stores/stores.go:106-109`) and `updatekeys` depends on reading them, so recipient hiding requires age-native documents, which sops-nix cannot consume (`decrypt.File` requires sops metadata, `sops-nix pkgs/sops-install-secrets/main.go:340-346`); this is option O4 in the research note, rejected there and not reopened here.

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

### V8. The vault is declared as a naming-keyed submodule

`flake.safix.vault` becomes `nullOr (submodule { options = { root = path; namingKey = str; }; })` rather than the unamended `nullOr path`; `root` carries exactly the meaning the unamended proposal gave `vault` itself.
`namingKey` SHALL be a string, never a `path` — a `path`-typed option is store-copied at evaluation like every other path in the flake, so a `path`-typed key would carry the identical store exposure a plain string does while adding a second file the operator must keep in sync — of at least 64 lowercase hexadecimal characters (32 bytes of entropy, hex-encoded).
The option's own description documents `openssl rand -hex 32` as the minting recipe, and `head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n'` as the coreutils-only fallback for a minimal closure without `openssl`.

The refusal is added to the existing violations mechanism (`default.nix:174`, `violations = resolve.violations registry ++ resolve.generatorViolations registry;`) as a third concatenated list, `resolve.vaultViolations cfg.vault`, a new function in `resolve.nix` following the shape `noRecipientKey` already establishes (`resolve.nix:1701-1704`, `lib.concatMap (...) (lib.optional (cond) "message")`): `wellFormedNamingKey = k: builtins.stringLength k >= 64 && builtins.match "[0-9a-f]+" k != null;`, refusing when a vault is declared with no naming key, a naming key under 64 characters, or one containing anything outside `[0-9a-f]`.
A drill perturbs a fixture's naming key to 63 characters, then to a mixed-case or non-hex string, and each turns the refusal red independently, matching the house style every other `violationsOf` entry in this codebase drills.

Stated plainly, in the option description and here: the naming key is visible to anyone who can evaluate the declaring flake — every local user of a machine that has it in the nix store — because nix has no keyed hash and the naming key must itself be an evaluation-time value (`builtins.hashString`'s only inputs are its two string arguments, `primops.cc:4593-4605`).
It hides names only from the vault host and from a reader holding only the vault, never from the store or the declaring repository — exactly the bound the research note's Summary and Recommendation state.

**Alternative rejected**: typing `namingKey` as `path`, so the operator points at a file holding the key rather than pasting it inline.
Rejected because a `path`-typed option is store-copied at evaluation exactly like every other path in the flake, so the key would enter the nix store in the clear regardless — the same exposure the string form has — while adding a second file the operator must keep in sync with the option and a second thing that can go missing.

### V9. Opaque physical layout, vault mode only

`opaqueOf`, defined once beside `audienceFileOf` in `resolve.nix`:

```
opaqueOf =
  namingKey: tag: logicalPath:
  builtins.hashString "sha256" "${namingKey}|${tag}|${logicalPath}";
```

`|` is the separator: it appears in none of the three inputs — `namingKey` is constrained to `[0-9a-f]` by V8, `tag` is one of four fixed strings this decision defines below, and `logicalPath` is drawn from the same name alphabet and audience-marker set `audienceMarkers` already excludes it from (`resolve.nix:267-293`) — so no two distinct `(namingKey, tag, logicalPath)` triples concatenate to the same string.

Four call sites, one tag each, every one reusing the readable, injective name the unamended proposal already computes as the *input* to the hash rather than as the output:

- Ciphertext: `secretsFileOf = namingKey: audience: "secrets/${opaqueOf namingKey "secrets" (audienceFileOf audience)}.yaml"`, replacing `audienceFileOf audience` as the value bound to `sopsFile` at the two `selectFor` sites (`resolve.nix:2021`, `:2093`) when `cfg.vault != null`, threaded the same way `root` is threaded today (`default.nix:47`'s `bound` gains a `namingKey` field alongside `root`).
- Public outputs: the vault-mode counterpart of `publicFileOf` is `"public/${opaqueOf namingKey "public" (publicFileOf audience name)}"` — a file, not a `<name>/value` directory, because the leaf no longer needs a directory to disambiguate once the name itself is a hash.
- Definition records: see V14; the record path is nix-computed and opaque in vault mode, never derived by Rust string-parsing.
- The in-document key: below.

Each tag (`"secrets"`, `"public"`, `"state"`, `"key"`) domain-separates the four uses, so a coincidental collision between, say, a public output's logical path and a ciphertext's logical path never produces the same hash under two different meanings.

The in-document key. `placementsIn`'s `key` field (`resolve.nix:595`, `key = if entry.sopsKey != null then entry.sopsKey else name;`) is the canonical formula every consumer traces to: the Rust runtime reads it directly from `flake.safix.lib.placements` for `safix set`/`get`/`generate`/`decrypt --extract`, and `selectFor.forMachine` (`resolve.nix:2027`) recomputes the identical formula independently for the system-scope sops-nix option.
In vault mode both sites apply `opaqueKeyOf namingKey (audienceFileOf audience) logicalKey` — where `logicalKey` is today's `entry.sopsKey != null then entry.sopsKey else name` and `audienceFileOf audience` is the same readable file identity `secretsFileOf` above hashes — so a key derived at one site and a key derived at the other agree bit for bit, because both close over the same `audience` value the file name itself is keyed by.
`opaqueKeyOf = namingKey: logicalPath: logicalKey: opaqueOf namingKey "key" "${logicalPath}#${logicalKey}";` — `#` plays the same non-collision role `|` does above and is likewise outside the name alphabet.

One further site needs a non-optional change rather than a value change: `materializeFor`'s key emission, `lib.optionalAttrs (secret.sopsKey != null) { key = secret.sopsKey; }` (`resolve.nix:2249`), is conditional on the author having declared a custom `sopsKey`; when it is not declared, sops-nix defaults `key` to the attribute name — the *readable* secret name (`sops-nix modules/sops/default.nix:63-71`, `default = ... config._module.args.name`) — which would defeat key opacity for every entry that does not carry an explicit `sopsKey`.
In vault mode this optionality is dropped: `key` is always emitted, carrying the same opaque value `placementsIn` computed, so the system-scope activation path and the Rust CLI's own writes never disagree about what string sits inside the document.

Inside a vault document, the top-level key for each secret is therefore its own opaque hex, distinct from the file name's hex (different tag, different logical input), so key names name nothing — the research note's ingredient 2, confirmed feasible against sops-nix's own typing (`key : lib.types.str`, no name-checking, `sops-nix modules/sops/default.nix:63-71`) and against `-check-mode=sopsfile`, which verifies only that the key exists in the ciphertext, never that it means anything (`sops-nix pkgs/sops-install-secrets/main.go:556-563`).

When `vault == null`, `secretsFileOf`, `opaqueKeyOf`, and the definition-record hash are never invoked; every name is exactly today's readable name, and the "path states who can open the file without opening it" opinion (`resolve.nix:63-66`; `secret-custody/spec.md`'s "A multi-member audience" scenario) stays true for that case, unedited.

**Alternative rejected**: an HMAC construction (`HMAC-SHA256(namingKey, tag || logicalPath)`, built from two nested `hashString` calls per RFC 2104's ipad/opad construction) in place of the plain keyed-prefix hash above.
Nix's `hashString` takes only two string arguments and produces a hex digest with no byte-level access to XOR against an ipad/opad constant, so an in-nix HMAC would need to hex-decode, XOR, and re-encode by hand — a materially larger, harder-to-verify surface for a property the threat model does not need: the output here is never presented for verification against a MAC, only used as a directory or file name, so the length-extension weakness a bare prefix-hash carries has no exploitable consequence here — an attacker who computes `H(key‖m‖pad‖m')` from `H(key‖m)` learns the hash of a path with unreadable padding appended, which names nothing anyone is looking for.
The real risk a public salt carries — dictionary attack against a small, guessable name space — is closed the same way either construction closes it: by keeping `namingKey` itself secret from the vault host, which V8 already establishes as the property in force.

### V10. The policy never enters the vault

`.sops.yaml` stays committed at the declaring root in every case — this reverses V3's row 4 and row 5, which moved the write and read sites to `vault_root`.
`fix::write_policy` (`fix.rs:101-119`) keeps writing to `<declaration root>/.sops.yaml`; `check::policy` (`check.rs:186-206`) keeps reading it there; the drift check `mkDriftCheck` (`policy.nix:273-292`) keeps comparing the committed file at the declaring root against the generated one; and `recipient-policy`'s existing requirement — "SHALL be committed to the consumer's repository because the encryption tool reads it from the filesystem" — stays true of the declaring repository unedited, because the committed file the check governs is that repository's own.

What changes is only the two sops invocations that need creation rules to run against vault-rooted documents: `encrypt` (`Sops::create_empty_document`, `sops/mod.rs:173-235`) and `updatekeys` (`Sops::update_keys_command`, `sops/mod.rs:295-303`).
Both currently run with `current_dir(root)` set to whichever repository holds the target document (`sops/mod.rs:204`, `:301`), relying on sops's own upward `.sops.yaml` search from that directory (`sops` `config/config.go:41-81`); in vault mode `root` there is `vault_root`, which V10 no longer places a `.sops.yaml` in.

The runtime instead renders a second, disposable copy of the creation rules into the vault working tree and passes it with `--config`.
This is a second rendering of the same structured `plan` (`policy.nix:193-214`) the committed policy already renders from `renderPlan` (`policy.nix:216-233`) — not a derivation from the committed file, and not a second pass over the declarations.
The two renderings diverge exactly where opacity requires: `renderPlan`'s `path_regex` is a directory wildcard, `"^${a.dir}/[^/]*\\.yaml$"` (`policy.nix:209`), built from the readable `a.dir`; the vault-mode rendering's `path_regex` is the literal opaque filename, `"^secrets/${opaqueOf namingKey "secrets" (audienceFileOf a.audience)}\\.yaml$"`, because vault mode places exactly one flat ciphertext file per audience directly under `secrets/`, with no per-audience subdirectory left to wildcard over (V9).
The vault-mode rendering also carries no `Audience:` comment (`policy.nix:182-191`, `audienceNote`), no header prose (`policy.nix:34-111`), and no `keys:` block with named anchors (`policy.nix:219-222`): each rule's `key_groups` lists the recipients' raw age public keys inline, because a disposable file with no anchor block has nothing for `*anchor` to reference.
A new function, `renderVaultRules`, sits beside `renderPlan` in `policy.nix` and takes the same `plan registry` value `render` already computes, plus `namingKey`, so the committed file and the scratch file are two views of one evaluation rather than two evaluations that could disagree.

A new nix attribute, `flake.safix.lib.vaultCreationRulesText`, mirrors `policyText` and is `null` when `cfg.vault == null`; a matching `Attribute::VaultCreationRulesText` variant is added to `nix.rs` beside `Attribute::PolicyText` (`nix.rs:24-65`).

The runtime writes this text to a scratch file inside the vault working tree — not outside it, because `config.go:576-581`'s path-relative matching only strips the config's own directory as a prefix when the document lies under it, and an opaque `secrets/<hex>.yaml` path is relative to `vault_root`, so the config must sit at or above `vault_root` for the rules above to match as written.
The filename is `.sops-vault-rules.yaml`, chosen to read unmistakably as generated and disposable rather than as a second `.sops.yaml`.
It is registered with the existing scratch registry (`scratch::register_file`, `scratch.rs:48-51`) before creation, exactly as every other scratch artifact in this codebase is (`scratch.rs:17-19`'s own stated invariant), so `scratch::cleanup` (`scratch.rs:156-186`) sweeps it on normal return, on error, on panic, and — because `safix` already catches `SIGINT`/`SIGTERM` and calls `cleanup` from the handler (`scratch.rs:9-15`) — on signal, with no new signal-handling code.
`--config <scratch path>` is passed to both `create_empty_document` and `update_keys_command`.

Because a scratch file can exist for the duration of a `sops` subprocess call, sitting inside the same run whose later steps stage and commit the vault, the vault repository additionally carries a committed `.gitignore` entry for `.sops-vault-rules.yaml`, written once as part of the migration procedure (V13) and checked for by a new `check` finding — so a scratch file that happens to still exist at the moment `git add`/`git commit` runs (a crash between the sops call and the sweep, say) can never be staged, a second, independent guarantee beside the scratch registry's own sweep-on-every-exit-path guarantee, rather than a restatement of it.

`decrypt` and `set` need no creation rules and are unaffected: `sops decrypt` never consults them (`cmd/sops/main.go:1944-1949`, `needsCreationRule` excludes decrypt mode), and `sops set` on an already-encrypted document re-wraps against the document's own existing `sops.age[]` metadata rather than a freshly matched rule — its `loadConfig` failure is explicitly tolerated rather than fatal (`cmd/sops/main.go:1985`, `!isDecryptMode && !isRotateMode && !isSetMode`) — so `Sops::set_key` (`sops/mod.rs:253-281`), which already runs with no `current_dir` override and an absolute file argument, needs no vault-mode change at all.

**Alternative rejected**: deriving the scratch rules from the committed file at read time, parsing `.sops.yaml` back out of its rendered YAML, rather than rendering a second time from the structured `plan`.
Rejected because the committed file already carries prose, comments and anchor references purpose-built for a human reader (`policy.nix:34-111`, `:182-191`), so recovering a machine-usable rule list from it means writing a YAML-and-comment parser this codebase does not otherwise need, whereas the structured `plan` value the committed file is itself rendered from is already exactly the input a second renderer needs.

### V11. Residual visibility, stated as accepted

Stated as accepted residuals, in this document and in the delta specs' non-goals, rather than implied by omission: an opaque vault still shows its host the number of documents (one per audience, unchanged from today), the number of keys per document (one per secret, unchanged), each leaf's ciphertext length (sops does not pad; `document.rs:110-114`), each document's `sops.age[].recipient` list in the clear (`sops` `age/keysource.go:285-290`), one vault commit per write (unchanged from V4), and everything visible to anyone holding the declarations — the naming key included, per V8.

Recipient hiding requires age-native documents, which sops-nix cannot consume (`decrypt.File` needs sops metadata, `sops-nix pkgs/sops-install-secrets/main.go:340-346`), and even there a YubiKey (`age-plugin-yubikey` `piv-p256`) stanza still carries a static 4-byte tag equal to `SHA-256(recipient)[:4]` (`age-plugin-yubikey src/p256.rs:71-74`), so a YubiKey recipient stays linkable regardless of format.
This is option O4 in the research note, rejected there; it is named again here as a non-goal in this document's own Non-Goals section rather than left to be inferred from V9's silence about recipients.

### V12. Operations under opacity

A vault document is not browsable by hand: `sops <file>` against a vault-rooted document finds no `.sops.yaml` above it (V10) and needs the scratch config `safix` renders, so operator-facing documentation (task group below) directs `safix set`/`edit`/`get` at vault documents and states that a bare `sops` invocation there will not find rules on its own.

Renaming or re-audiencing an entry re-encrypts its leaf: the key path is bound into the AES-GCM associated data (`sops.go:604-611`, confirmed in Context above), so a key name cannot be renamed without a fresh encryption — this already held before opacity and is unchanged by it, restated here because opacity makes every key name look identical, which is exactly the situation in which an implementer might mistake a hash recomputation for a free rename.

Rotating the naming key is a full rename of the vault, performed by the same migration procedure V13 names for the initial move, because every physical name the vault holds is a function of the key: there is no partial or incremental rotation.
This is out of scope beyond stating it, matching the unamended proposal's own treatment of anything beyond the mechanism it introduces.

Losing the naming key never loses a secret: the declarations regenerate every name deterministically, so a naming key kept only in the operator's own record (never committed, since V8 makes it a plain evaluation-time string) is recoverable by re-declaring it, and only a *changed* naming key is a rotation rather than a loss.

### V13. Migration rewritten for the opaque layout

The unamended Migration Plan below states the move as `git mv` of `.sops.yaml` and every governed ciphertext file across the two repositories.
That is no longer correct once names are opaque: a `git mv` carries the *readable* name across, and the destination vault expects the *opaque* one, so the move is not a rename at all — it is decrypt each leaf under the readable layout, re-encrypt each into the opaque document its hash names, entirely in memory-backed staging, matching the staging discipline `plaintext-staging`'s existing requirements already state for every other transient plaintext this codebase produces.

Concretely, in order: mint a naming key (V8); create the vault repository; for every governed file in the existing readable layout, decrypt it under the operator's own identity and re-encrypt each of its keys into the opaque document and opaque key name `opaqueOf` computes for it, staged under the memory-backed root; move `public/` and `state/` the same way by copying each readable leaf's bytes to its opaque destination, since these are plaintext and need no decrypt/encrypt round trip; commit the assembled vault tree in one commit, together with the vault's `.gitignore` entry for `.sops-vault-rules.yaml` (V10); declare `flake.safix.vault` (`root` and `namingKey`) and set `SAFIX_VAULT_ROOT`; verify with `safix check` that the resolver's opaque view and the vault's committed state agree before the next write; only then remove the readable `.sops.yaml` and governed files from the declaring root.

This is a verb the runtime performs, not a manual `git mv` sequence, because the decrypt-then-encrypt step needs the operator's own identity and the resolver's opaque naming in the same place; unlike the unamended plan, it cannot be described as a sequence of plain git commands alone.
Which command performs it — a new `safix` verb, or a documented one-shot script this repository's own tooling ships — is a task-level decision (task group below), not a design one: the requirement is that it exists, runs entirely in memory-backed staging per `plaintext-staging`'s existing rules, and is idempotent enough that an interrupted run can be re-started from either state.

Rollback is symmetric in the same corrected sense: decrypt every leaf from the vault, re-encrypt into the readable layout at the declaring root, remove the vault declaration and `SAFIX_VAULT_ROOT`.
Both trees can coexist during the migration window, exactly as the unamended plan already allowed.

**Note, 2026-09-03**: two simplifications adopted at implementation time, each superseding the paragraph above in the one respect named.
First, the migration mechanism realizing this section is a `check` drift finding — `Finding::VaultRelocationPending`, one per readable-layout ciphertext document, public output, or definition record still present at the declaration root while a vault is declared — paired with a `fix` relocate phase, rather than a dedicated `safix` subcommand or a one-shot script; `fix` keeps its documented no-commit contract throughout the relocation, printing an informational note that names the vault-first commit order and the pending lock bump rather than committing or disclosing either itself.
Second, the rollback direction is a `fix` flag, `safix fix --vault-rollback`, run while the vault is still declared — the naming key needed to map an opaque name back to its readable one is reachable only through the still-standing declaration — rather than a mirrored `Finding::VaultOrphanedAfterUndeclare` covering the state after the declaration has already been removed.
That mirror finding is dropped entirely: the existing `Error::VaultRootWithoutDeclaration` refusal's prose instead gains one sentence telling the operator that a vault whose declaration was removed without first running the rollback is recovered by re-declaring `flake.safix.vault` and running `safix fix --vault-rollback`.

### V14. Runtime reversibility, traced site by site

Every place the runtime or nix reverses a physical name back to an audience or a grant, traced and disposed of individually.

`refOfElement` (`resolve.nix:313-324`) turns an *audience element string* — one entry of the in-memory `audience` list nix already carries during evaluation, such as `"@oncall"` or `"alice"` — back into a subject reference.
Its two call sites, `audienceKeysOf` (`resolve.nix:435-438`) and the silo-membership computation (`resolve.nix:1748-1751`), both consume `audience` — the list itself, never a parsed directory name — so `refOfElement` is never fed a physical path in either mode and needs no change.
`opaqueOf` transforms only the *rendered* file, public, and definition-record names; it never touches the `audience` list those names are rendered from, so `refOfElement` and everything upstream of `elementOf`/`isMarkedElement` continue to operate on logical data exactly as they do today.

`check.rs`'s shared/stray logic (`shared`, `check.rs:291-375`) reads `placement.file` and `placement.key` as opaque strings handed over by `workspace.placements()` — `shared_files.insert(name.clone(), (placement.file.clone(), placement.key.clone()))` (`check.rs:316-319`) — and never parses either string's structure; every comparison is string equality against other nix-provided strings (`audiences.for_file(audience_file)`, `check.rs:328-330`).
Opacity changes what these strings *are* but not how `check.rs` uses them, so this site needs no change in either mode.

`definition::record_path` (`definition.rs:137-143`) is the one site that genuinely reverses a physical name: it calls `audience_directory(&placement.file)` (`definition.rs:153-164`), which extracts the last path component of `placement.file`'s directory — today, an audience's own readable directory name — to build `state/safix/definitions/shared/<audience>/<name>`.
In vault mode `placement.file` is `secrets/<hex>.yaml`, whose directory has no last component to extract (V9 places every ciphertext file one level deep, directly under `secrets/`), so `audience_directory` cannot recover an audience from it, by construction rather than by omission.

The disposition is a new nix-supplied field rather than a Rust-side mapping table: `flake.safix.lib.placements.<user>.<name>` gains `definitionRecord : nullOr str`, populated only when `cfg.vault != null`, carrying `"state/${opaqueOf namingKey "state" logical}"` where `logical` is the same `shared/${lib.concatStringsSep audienceSeparator audience}/${name}` or `${owner}/${name}` string `audience_directory`'s two branches already compute today.
Nix has the `audience` list in hand inside `placementsIn` (`resolve.nix:585-613`, `audience = audienceOf r src.owner name;` at `:589`) exactly where `file` and `public` are already computed (`:593`, `:596`), so the parallel field costs one more line there, not a second pass.
A matching field, `pub definition_record: Option<String>`, is added to `Placement` (`model.rs:147-169`, `#[serde(deny_unknown_fields)]`, so the schema task is mechanical: add the field, update every fixture literal that constructs a `Placement`).
`definition::record_path` becomes: return `placement.definition_record.clone()` when it is `Some`; otherwise fall back to today's `audience_directory`-based derivation, unedited — so the non-vault path is untouched byte for byte, and nothing in `crates/safix-core` computes a hash at any point, matching the constraint that hashing is nix's job alone.

No other reversal site exists: `flake.safix.lib`'s exported `elementOf`/`refOfElement`/`publicFileOf`/`recipientsOf`/`custodyOf` (`resolve.nix:2268-2272`) are the resolver's own public surface for a consumer's nix code, not the Rust runtime's, and none of the four is fed a vault-rooted physical name by anything in `crates/safix-core`.

## Capabilities considered and not touched

`secret-custody` states audience algebra, revocation, and the user record's fields; none of it names a repository, a file location, or a commit, so nothing in it becomes false when ciphertext moves to a second repository. This amendment adds a vault-mode scenario to "A multi-member audience" stating the file name is opaque rather than a sorted member list when a vault is declared; the requirement's truth is unaffected, only its file-naming clause gains a vault-mode alternative.
`secret-installation` and `secret-consumption` state how a resolved entry arrives on a machine; `sopsFile` is a path there exactly as it is here, and neither spec states where that path is rooted, so C5's flip does not touch either.
`public-outputs` states that a public output "is stored under a top-level prefix distinct from the one holding encrypted material" and "is stored as plaintext in the repository" — both survive unedited, because the requirement is repository-relative and repository-agnostic: it is still one repository, still one prefix separation, and that repository is now the vault when one is declared, which the requirement never named in the first place. This amendment adds a vault-mode scenario to "The layout distinguishes shared from per-user" for opaque leaves; prefix separation itself is unedited.
`secret-catalogue`'s known defect — that it requires the catalogue be "an attribute set option on a flake-parts module" — belongs to `support-plain-nix-consumers`, which relaxes flake-parts as a hard dependency; nothing in this change touches that requirement.
`safix-cli` states the subcommand surface, the pipe discipline, and the read/write split; none of its requirements assert single-repository or single-commit behaviour by name, so the two-commit sequence V4 introduces is new behaviour best stated once, in `secrets-vault`, rather than as an edit to a capability that never made the claim being changed.

## Risks / Trade-offs

Two commits instead of one means two objects appear in `git log` for one logical operator action, in two different repositories — a reviewer of the declaration repository's history sees "declare alice and regenerate the recipient policy" without the ciphertext diff, which lives in a vault they may or may not have read access to.
That is not incidental; it is the entire point of a separately-permissioned vault, and the `Safix-Vault:` trailer (V4) is the mitigation, giving anyone with declaration-root access a durable pointer to the vault commit without granting them the ability to read it.

The half-landed state (V4, step 5) is a real operational state an operator can observe, not merely a theoretical one — a re-run resolves it, but an operator who does not re-run leaves a vault ahead of a declaration that never lands, silently accumulating inert vault history.
No check in this program can force a re-run; the refusal states the remedy and that is the limit of what a command-line tool can guarantee.

The lock-bump discovery in V6 runs `nix flake metadata`, an extra process per vault-touching commit; on a large flake this is a real latency cost, and if it proves objectionable at implementation time the fallback (a general disclosure naming no input) is always correct, just less specific — this is deferred to measurement, not decided here, since neither answer changes the specs, the approach, or the task breakdown.

Coexistence with `support-plain-nix-consumers`'s flakeless mode is asserted, not yet measured, because that change's artifacts do not exist yet at the time this one is authored; V7's last paragraph states why no new mechanism is expected to be needed, and the task list drills it once both changes have landed.

The naming key V8 introduces is visible to anyone who can evaluate the declaring flake, so it does not create a new secret in the cryptographic sense — losing it loses no data (V12) — but it does mean this amendment's opacity guarantee is scoped exactly as narrowly as the research note states: against the vault host and a vault-only reader, never against the store or the declaring repository. A reader who wants that broader guarantee needs a mechanism this amendment does not build, and the amendment note above and V8 say so at the point the key is declared, not only here.

## Migration Plan

Within this repository, the flip is additive at the option level: `flake.safix.vault` defaults to `null`, so every existing consumer, including this repository's own checks, is unaffected until a vault is declared.
Declaring a vault for the first time on an existing consumer requires, in order (rewritten by V13 for the opaque layout): minting a naming key, creating the vault repository, decrypting every governed file under the operator's own identity and re-encrypting each of its keys into the opaque document and opaque key name `opaqueOf` computes for it — entirely in memory-backed staging, per `plaintext-staging`'s existing requirements — copying `public/` and `state/` to their opaque destinations as plaintext, committing the assembled vault tree together with its `.gitignore` entry for `.sops-vault-rules.yaml`, declaring `flake.safix.vault` and setting `SAFIX_VAULT_ROOT`, and then verifying with `safix check` that the resolver's opaque view and the vault's committed state agree before the next write.
This is a verb the runtime performs rather than a `git mv` sequence, because the decrypt-then-encrypt step needs the operator's own identity and the resolver's opaque naming in the same place; V13 records why and what performs it.

Rollback is symmetric in the corrected sense V13 gives it: decrypt every leaf from the vault, re-encrypt into the readable layout at the declaring root, remove the vault declaration and `SAFIX_VAULT_ROOT`.
Both trees can coexist during the migration window.

## Open Questions

Resolved, 2026-09-03, at implementation time: `nix flake metadata . --json` against this repository's own flake, warm-cache, ran in 93-99 ms across three runs — a small fraction of the several-hundred-millisecond floor a `set`, `generate` or `enroll` run already pays for its own `sops`, `age` and `git` subprocesses, and well under the latency an operator would notice as a pause distinct from the rest of the command.
The discovery stays: task 9's `Nix::flake_metadata` runs it once per vault-root commit, and only ever against the declaration root, which is the same flake every other evaluation in the run already targets and therefore already warm in the evaluation cache by the time the disclosure asks.
The `None`-on-any-failure shape [`Nix::flake_metadata`] and [`Nix::nar_hash`] both take — degrading to the general phrasing rather than propagating an error — is what keeps this measurement from being a correctness question as well as a latency one: a slow or failing lookup costs the specific input name, never the write that already committed.
