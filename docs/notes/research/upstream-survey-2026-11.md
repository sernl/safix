---
title: "Upstream survey, 2026-11-04"
---

Reference clones under `~/ghq` were updated on 2026-11-04 and five read-only surveys were run against the deltas since the revisions safix last read.
Each survey is reproduced below as delivered, followed by the corrections made after verifying its claims against safix's source.
Line anchors into upstream are at the new revision named in each section; anchors into safix are at v0.3.1 (`224478b`).

| Upstream | Read at | Now | Commits |
|---|---|---|---|
| clan-core | 56e3562 (2026-08-03) | dbae210 (2026-10-26) | 411 |
| sops-nix | f140661 (2026-07-04) | b18e01c (2026-11-02) | 38 |
| sops | 3.13.3, still the latest tag | main cbb1216 (2026-11-03) | 39 unreleased |
| agenix | b027ee2 (2026-02-03) | 5e5b88a (2026-10-20); tags 0.16.0, 0.17.0, 0.18.0 | 28 |
| agenix-rekey | never pinned | 87e5d5f (2026-10-09) | fresh survey |

## Corrections after verification

- sops-nix §5 question 3: the brief given to the surveyor was wrong, not the code. safix still carries an early-user unit, public templates and `identity.gnupgHome`; the survey's ranking stands.
- clan-core F3 and the agenix-rekey darwin remark: safix passes `SOPS_AGE_KEY_FILE` explicitly to every sops child (`crates/safix-core/src/ciphertext.rs:265-266`, `crates/safix-core/src/sops/mod.rs:330-331`), so a safix-launched sops finds the key `safix keygen` wrote. The darwin finding is an interoperability mismatch with standalone sops and clan, which default to `~/Library/Application Support/sops/age/keys.txt` when `XDG_CONFIG_HOME` is unset, not a safix decryption failure.
- agenix-rekey 3.1: confirmed. sops wraps to an `age1yubikey1…` recipient through `plugin.NewRecipient` (`age/keysource.go:498-503`), which runs `age-plugin-yubikey` from `PATH`; the CLI wrapper (`modules/flake/rust.nix:84-92`) does not put the plugin there.
- Additional finding not in any survey: sops 3.13.3 accepts hybrid post-quantum age recipients `age1pq1…` and identities `AGE-SECRET-KEY-PQ-1…` (`age/keysource.go:491-492`, `:566-567`); `crates/safix-core/src/adduser.rs:247-249` refuses them by shape.


## Survey: clan-core

**How I read the range.** There was no shell, so I could not run `git log` or `git diff`. The local clone is shallow and has no tags, and its reflog confirms the checkout moved from `56e35624d94e` to `dbae21013aca`. To see the changes I used Gitea's compare page and raw diff for exactly this range: `https://git.clan.lol/clan/clan-core/compare/56e35624d94e…dbae21013aca(.diff)`. Line numbers marked HEAD come from the local checkout at dbae21013.

### Summary

- **Generator contract: no change.** Nothing changed in the sandbox argument vector, the darwin profile, the `$in`/`$out`/`$prompts` environment and directories, the prompt types, or generator validation. safix's claim that its envelope matches clan's still holds.
- **One breaking change touches safix's recorded claims.** Every deployed vars path gained the generator's placement directory (`rel_dir`, PR #7298). A service secret moves from `/run/secrets/<gen>/<file>` to `/run/secrets/per-machine/<machine>/<gen>/<file>` or `/run/secrets/shared/<gen>/<file>`. clan's release notes don't mention it.
- **The `clan vars` output safix parses is unchanged.**
- **New age-backend features with smaller effects:** an external age store (`externalStore` + `CLAN_AGE_STORE_DIR`), `clan init` defaulting new clans to the age backend, a macOS identity-path fix, and a fix to `clan vars fix` for shared secrets.

### 1. Range surveyed
- Old: `56e35624d94e4f1ac55d36575ebab97cbd9b9cdd` (2026-08-03). New: `dbae21013aca57424620d06863cbfd2e4107352b` (2026-10-26, merge of #8285).
- Gitea counts **411 commits**; the net diff is 168 files, +2858/−3184.
- **Tags:** none in the local clone (`.git/refs/tags` is empty, and `packed-refs` holds only `origin/main`). Upstream has no CHANGELOG file; release notes live in `docs/src/releases/next.md` and `docs/release-notes.md`.
- Most commits are nixpkgs, sops-nix, nix-darwin and data-mesher bumps, the move to nixpkgs' built-in nspawn test driver, and service changes (yggdrasil, monitoring, data-mesher). I've left those out.

### 2. Breaking changes

#### B1. Deployed vars paths now include the placement (PR #7298)
Merge `728e598b7d`, core commit `4a00824f5d` "vars/directories: update all directories from all backends to use rel_dir".

- New internal option in `modules/clan/vars/settings-opts.nix` (hunk `@@ +23,24`): `rel_dir` is "One of: `per-machine/{machine}/{generator}`, `shared/{generator}`, `per-export/{export}/{generator}` … Must match `GeneratorId.rel_dir` in `clan_lib/vars/_types.py`."
- `nixosModules/clanCore/vars/interface.nix` (`@@ +72`) sets it: `if generator.config.share then "shared/${generator.config.name}" else "per-machine/${config.machineName}/${generator.config.name}"`.
- **age backend**, `nixosModules/clanCore/vars/secret/age.nix:244-256` (HEAD):
  ```
  -                "/run/secrets/${file.config.generatorName}/${file.config.name}"
  +                "/run/secrets/${file.config.rel_dir}/${file.config.name}"
  ```
  The same substitution applies to `/run/user-secrets/…`, `${secretLocation}/activation/…` and `/run/partitioning-secrets/…`. The decrypt destination moved the same way: `destFile = "${targetDir}/${f.rel_dir}/${f.fileName}"`.
- **sops backend**, `secret/sops/generators-to-sops.nix`:
  - The sops-nix secret name changes: `-        name = "vars/${secret.generator}/${secret.name}";` becomes `+        name = "vars/${secret.rel_dir}/${secret.name}";`.
  - The store name is now `lib.strings.sanitizeDerivationName "${secret.rel_dir}_${secret.name}"`.
- **password-store backend:**
  - Tar entries are now `f"{generator.key.rel_dir()}/{file.name}"` (`password_store.py` hunks `@@ +235`, `+253`).
  - The upload hash manifest now includes `rel_dir` (`da501aba47`), so every password-store machine sees a one-time hash change and re-uploads.
- **Plaintext activation/partitioning uploads:** `sops.py` and `age.py` `populate_dir` write to `output_dir / "activation" / generator.key.rel_dir() / file.name`.
- **Store names of public vars:** `in_repo.nix` builds `flakePath` as `"/vars/${file.config.rel_dir}/${file.config.name}/value"`, and `settings-opts.nix` names store paths `sanitizeDerivationName "${rel_dir}_${name}"`.
- **Not in release notes.** The `docs/src/releases/next.md` diff lists the `users` prompt change, the removed `clan state` command and the age template, but not this.

**Effect on safix:**
- Code: none. safix derives clan host keys from `services.openssh.hostKeys`, and clan's sshd module sets those from `config.clan.core.vars.generators.openssh.files."ssh.id_ed25519".path` (`clanServices/sshd/default.nix:229-233` HEAD), so the new path flows through on its own. The exclusion prefix (`/run/safix`) still holds.
- Recorded claims and fixtures that are now stale — see §4.

#### B2. `users` service: `prompt` now defaults to `false`
`docs/src/releases/next.md`: "The `users` service now generates a random password instead of prompting for one. Already generated passwords are unaffected. Set `roles.default.settings.prompt = true` to keep being prompted…" (`clanServices/users/default.nix @@ -38`).
- Effect on safix: only via the bridge. A safix mapping that expects a person-entered `user-password` var on a new machine will now find a generated value.
- **Recommendation: ignore**, unless a bridge mapping depends on it.

#### B3. New clans use the age vars backend
`next.md`: "`clan init` writes the public key of your selected or newly generated age key into `vars.settings.recipients.default`, instead of creating a sops user under `sops/users/`. … Existing clans are unaffected." (PR #8199, `e8a6e88ea1`, `a79aab0be5`; `clan_cli/vars/keygen.py` replaces `create_secrets_user_auto` with `select_admin_keys`/`register_admin_keys`.)
- Effect on safix: see F6.

### 3. Fixes, features, UX and docs (ranked by relevance)

#### Executor and generator contract — explicitly unchanged
- **Sandbox argument vector and profile.** `pkgs/clan-cli/clan_lib/sandbox_exec/__init__.py` is not in the diff. The only change under `sandbox_exec/` is `a7327c0ace`, which adds `@pytest.mark.impure` to `test_sandbox_works_returns_true_on_darwin`. Its reason: since 2026-10-20 the darwin builders cannot see `/usr/bin/sandbox-exec` inside the Nix build sandbox.
  - safix today: `crates/safix-core/src/sandbox.rs:1-41` records the three deviations; `crates/safix/tests/sandbox.rs:427-431` and `modules/flake/checks/envelope.nix:11-14` already decline to observe darwin inside a build sandbox.
  - **Recommendation: ignore.** Upstream reached the conclusion safix already records.
- **Generator env and directories.** `clan_lib/vars/generate.py` and `generic-generator.nix` are not in the diff. `generator.py` only adds the `vars_settings_age()` selector (`@@ +176`). `$in/<producing generator>/<file>` keeps the bare generator name; `rel_dir` affects deployment, not `$in`.
  - safix today: `crates/safix-core/src/inputs.rs:1-20`, `docs/guides/generators.md:31`.
  - **Recommendation: ignore.**
- **New prompt types, share semantics, generator validation.** None. `share` is still a boolean mapping to `Shared` or `PerMachine` (`interface.nix @@ +72`).

#### `clan vars` CLI that safix parses — unchanged
- `clan_cli/vars/{get,set,list}.py`, `clan_lib/vars/{get,set,var,_types}.py` and the `clan secrets users` CLI are not in the diff. `check.py` changes only a type hint (`None | str` to `str | None`).
- At HEAD the matched strings are still present:
  - `get.py:19` "has not been generated yet"
  - `clan_lib/vars/get.py:28` "Couldn't find var"
  - `check.py:134` "has outdated invalidation hash"
  - `var.py:86` `f"{self.id}: ********"`
- safix today: `crates/safix-core/src/clan.rs:52-80`.
- **Recommendation: ignore.** Re-anchor line citations whenever the pin moves.

#### F1. Opt-in external age store (PR #8017, commit `b9ca37a597`)
- **Upstream:**
  - `modules/clan/vars/settings-opts.nix @@ +240` adds `vars.settings.age.externalStore` (bool).
  - `clan_lib/vars/secret_modules/age.py` `secrets_dir()` (`@@ +100,95`) reads `CLAN_AGE_STORE_DIR`. It refuses when the flag is on without the variable, the variable is set without the flag, the directory is inside the flake, or `secrets/` is non-empty.
  - `_commit_store_files` and `_changed_files` commit into the external store's own git repo, or do nothing if it isn't one.
  - On the target, `age.nix` decrypts from `"${secretLocation}/vars/${rel_dir}/${fileName}/${fileName}.age"` (`secret/age-runtime-path.nix`). It prints `WARNING: encrypted secret … not found, skipping` instead of failing.
  - Documented in `docs/src/guides/vars/age/age-backend.md` (`@@ +191`, "External Store (Public Repositories)").
  - `nixosModules/clanCore/vars/default.nix @@ +38` asserts that a machine cannot override the setting.
- **safix today:**
  - safix inherits the environment when it runs clan (`clan.rs:173-184`, no `env_clear`), so `CLAN_AGE_STORE_DIR` reaches `clan vars get/set`, and clan's refusal is carried verbatim as `ClanCommandFailed`.
  - `bridge.rs:15-24` and `openspec/specs/bridge-sync/spec.md:154` say `clan vars set` "commits in the clan repository". Under an external store it commits in the store repo, or nowhere.
- **Recommendation: adapt (docs only).** Qualify the "commits in clan's own repository" wording to "in clan's store repository (the flake, or `CLAN_AGE_STORE_DIR` when `externalStore` is set)". Add an operator note in `docs/guides/syncing-password-managers.md` that the variable must be exported for `safix sync clan`.

#### F2. `clan vars fix` no longer drops recipients from shared deployed age secrets (PR #7978, `a55446be71`)
- **Upstream:** `clan_lib/vars/fix.py:10-17` (HEAD) resolves generators across all machines (`all_machines = list(machine.flake.list_machines())`) because "a shared generator's `machines` must hold all machines that declare it". The regression test is `test_age_fix_keeps_all_recipients_on_shared_deployed_secret`.
- **safix today:** never calls `vars fix`. A shared clan var that safix writes with `clan vars set` could have been re-keyed to one machine by an operator's `clan vars fix` at the old pin. safix's read-first compare (`bridge.rs:15-24`) would then report Unchanged while some machines can no longer decrypt.
- **Recommendation: ignore for code.** Bump the pin, and mention in the operator notes for shared mappings that clan < this range had the bug.

#### F3. macOS age identity path (PR #7999, `da3a8b468e`)
- **Upstream:** `age.py` `_identity_search_paths()` now uses `user_config_dir() / "sops" / "age" / "keys.txt"`. `clan_lib/dirs/__init__.py:90-98` resolves that to `$XDG_CONFIG_HOME`, else `~/Library/Application Support/` on darwin, else `~/.config`.
- **safix today:** always `$XDG_CONFIG_HOME` else `$HOME/.config`, at `crates/safix-core/src/identity.rs:114-118`, `keygen.rs:138-144` and `ciphertext.rs:95-103`.
- **Recommendation: investigate.** If sops on darwin defaults to `~/Library/Application Support/sops/age/keys.txt` when `XDG_CONFIG_HOME` is unset (`[INFERENCE]`, not checked against sops 3.13.3 source), then safix's keygen writes where sops does not look on darwin.

#### F4. Deterministic generator order (PR #8091, `96540836fd`)
- **Upstream:** `clan_lib/vars/graph.py:99-105` sorts predecessor sets by `k.key()` so the run and prompt order no longer depend on `PYTHONHASHSEED`.
- **safix today:** order comes from the Nix resolver (`generate.rs:3-6`), which is already deterministic.
- **Recommendation: ignore.**

#### F5. `clan vars cp <src> <dst> <gen/file>` (PR #8218, `294938fd4e`, `a8a8bd66e2`)
- **Upstream:**
  - New `clan_cli/vars/cp.py:1-73`. It refuses shared vars ("Var {var_id} is shared, … already use the same value") and refuses an ungenerated source.
  - It is built on `get_machine_var` and `set_var`, so it reaches only `PerMachine`/`Shared`, like safix.
  - Completion falls back to `source_machine` (`completions.py @@ -274`).
- **safix today:** nothing equivalent. The bridge moves values safix↔clan, not clan↔clan.
- **Recommendation: ignore.** It's out of safix's scope, and it confirms that `PerExport` stays unreachable through the per-machine API (safix `bridge.nix:35-39`).

#### F6. Card enrollment vs the age backend (made more visible by B3)
- **Upstream:** age-backend recipients come only from `vars.settings.recipients.{hosts.<m>,default}`; `age.py` `get_recipients` (HEAD `:211-247`) says "Please set vars.settings.recipients.hosts.{machine} or vars.settings.recipients.default in your clan.nix". `clan init` no longer creates sops users for age templates. This was already true for the age backend at the old pin; it's more visible now that age is the default.
- **safix today:** `enroll/mod.rs:515-531` registers a card through `clan secrets users add|add-key --age-key` (`clan.rs:250-279`). That adds a sops user, which gives no access to age-backend vars. CHANGELOG.md:979 records that this fleet runs `secretStore = "age"`.
- **Recommendation: investigate.** On an age-backend clan, the "registering the card with clan" step probably grants nothing for vars.

#### F7. Smaller fixes
- **`clan secrets` admin-key check** (PR #8092, `0356eb617c`). `clan_cli/secrets/secrets.py:221`: `if admin_keys not in recipient_keys:` became `if not admin_keys <= recipient_keys:`. The old check was always true, so admin keys were always re-added. **Ignore:** safix doesn't call `encrypt_secret`.
- **`age-keygen` via `nix_shell`** (PR #8003). `age.py`: `nix_shell(["age"], ["age-keygen"])` replaces a bare `age-keygen`. **Ignore:** it lowers clan's PATH requirement when safix's checks drive a real clan.
- **`clan-cli/default.nix` `makeWrapperArgs` split** (`83dacb6def`). New nixpkgs passes wrapper args as separate argv entries. **Ignore** unless safix wraps clan the same way.

### 4. Effects on safix's recorded compatibility claims and docs
- **Still true:**
  - "matches the one clan's default executor applies" (`openspec/specs/secret-generators/spec.md:230`, `sandbox.rs:3-8`, `docs/guides/generators.md:31`).
  - The `clan vars get/set/list/check` contracts in `clan.rs:10-80`.
- **Now stale (B1):**
  - `openspec/changes/archive/2026-08-22-own-secret-installer/design.md:13` and `proposal.md:23` say clan places vars at `/run/secrets/<generator>/<name>` (`age.nix:218-227`). At HEAD it is `/run/secrets/<rel_dir>/<name>`, at `age.nix:244-256`.
  - `design.md:141` says the derivation "on a clan machine … yields `/run/secrets/openssh/ssh.id_ed25519`". At HEAD that is `/run/secrets/per-machine/<machine>/openssh/ssh.id_ed25519`; the logic is unaffected.
  - `modules/flake/checks/installer.nix:94-98`, `:797-803` and `:1506` use the old shape in a synthetic fixture. The check stays valid because only the `/run/safix` prefix matters, but the fixture no longer looks like a real clan host. **Adapt:** update the fixture path and comment.
  - Anchors into `age.nix` (`:98-106`, `:141-154`, `:143-148`, `:204`, `:259-276`, `:296-308`) are off by about +22 lines at HEAD:
    - mount helper `:102-114`
    - `mountpoint -q` guards `:165-175`
    - `keyFile` `:226`
    - `setupSecrets` `:281-294`
    - `age-decrypt-secrets` unit `:318-330`
  - Archived changes can keep their pinned citations; any live doc or spec that repeats them needs re-anchoring.
- **Activation ordering and mount behaviour in `age.nix`: unchanged.** No new dependencies, same ramfs mount guarded by `mountpoint -q`, same entry name `setupSecrets`, same `RemainAfterExit` units. The one new runtime behaviour is the external-store branch: a missing source is a warning, not a failure. safix's coexistence design (D5/D6) still holds.
- **Clan's identity stays at `${secretLocation}/key.txt`** (`age.nix:226`). With `externalStore`, clan also uploads `vars/**.age` under `secretLocation` (`/etc/secret-vars` by default). safix doesn't read that directory.
- **May need rewording (F1):** "clan commits in its own repository" (`bridge-sync/spec.md:154`, `bridge.rs:18-23`).

### 5. Open questions
1. Is the fleet's clan pin moving to dbae21013? Only then do B1's deployed paths and the stale fixtures and claims matter in practice.
2. Does any consumer run `vars.settings.age.externalStore = true`? If so, F1's doc change should be prioritized and `CLAN_AGE_STORE_DIR` handling stated explicitly for `safix sync clan`.
3. F6: on an age-backend clan, what should "register the card with clan" mean? The only recipient surface is `vars.settings.recipients` in `clan.nix`, which clan's CLI writes only during `clan init`. safix's rule against editing clan's files would forbid writing it.
4. F3: what is sops 3.13.3's default `keys.txt` location on darwin when `XDG_CONFIG_HOME` is unset? That decides whether safix's identity and keygen paths are wrong on macOS.

## Survey: sops-nix

safix's installer copied three systemd bugs that sops-nix fixed in this range. All three are still in `modules/consume/installer.nix`:

- The main install unit is not ordered before `sysinit.target`, so ordinary services can start before their secrets exist.
- Neither unit stops cleanly at shutdown.
- The pre-user-creation unit uses `RequiresMountsFor=`, which can create a dependency loop at shutdown.

Separately, sops-nix now derives an identity from SSH host keys when only `services.openssh.generateHostKeys` is set; safix still requires `services.openssh.enable`. Nothing in the range invalidates `docs/guides/migrating-from-sops-nix-and-agenix.md`.

How I got the data: I had no shell, and the local clone is shallow, so I couldn't run `git log` or `git diff`. The commit list and diffs come from the GitHub compare API for f1406619…b18e01ce. Upstream line numbers are from the local checkout, which is at b18e01c.

### 1. Range surveyed

- **Old:** `f1406619a3884cd5c47992a70b8b35c9c0fcb4c9` (2026-07-04, a gitea-mq merge commit).
- **New:** `b18e01ce148eb3efff5de2f2d3181ca53b2b2a54` (2026-11-02).
- **Count:** GitHub reports 38 commits ahead and 0 behind. Most are dependabot bumps, "update vendorHash" commits and automated flake.lock updates.
- **Tags:** the local `.git/refs/tags/` is empty. I did not check upstream tags.
- **Files changed:**
  - `modules/sops/default.nix` (+10/−3)
  - `modules/sops/secrets-for-users/default.nix` (+1/−1)
  - `modules/home-manager/sops.nix` (+3/−3)
  - `modules/nix-darwin/default.nix` (+1/−1)
  - `pkgs/sops-install-secrets/main.go` (+13/−7) and `main_test.go` (+45)
  - `checks/nixos-test.nix` (+93/−6)
  - `flake.nix` (+4/−12), plus go.mod/go.sum and lock files
- **Not changed:** `README.md`, `modules/sops/manifest-for.nix` and `modules/sops/templates/`.

### 2. Breaking changes (verbatim)

1. **`x86_64-darwin` dropped** from `flake.nix` `systems`, and `darwinConfigurations.sops-x86_64` was deleted (it appears only in the compare diff for `flake.nix`, not in a single commit message):
   ```
   -        "x86_64-darwin"
   ...
   -        darwinConfigurations.sops-x86_64 = privateInputs.nix-darwin.lib.darwinSystem {
   ```
2. **Build changes** (`pkgs/sops-install-secrets/default.nix:1-11`; "Bump go to stable" 16954c1): `buildGo125Module` → `buildGoModule`, and `go.mod` `go 1.25.0` → `go 1.26.0`. Library pins moved: sops/v3 3.12.2 → 3.13.3 (4b62bf2), ProtonMail/go-crypto 1.4.1 → 1.5.2 (cda842f), moby/sys/mountinfo 0.7.2 → 0.7.3 (82c6758).
3. **The pre-user-creation unit is no longer hard-bound to its key mounts** (e80b8b7, `modules/sops/secrets-for-users/default.nix:49`):
   ```
   -        unitConfig.RequiresMountsFor = lib.concatLists [
   +        unitConfig.WantsMountsFor = lib.concatLists [
   ```
   Commit message: *"fix(systemd): use WantsMountsFor for the for-users unit — This avoids a potential dependency loop at shutdown. Fixes #962"*. This needs a systemd new enough to support `WantsMountsFor=` (an inference; I did not check the version).
4. **The main unit is now ordered before `sysinit.target` and stops at shutdown** (166fc0d and a8be3b1, `modules/sops/default.nix:477-484`):
   ```
   +            # WantedBy only pulls the unit in; without Before= normal services
   +            # (ordered after sysinit.target via DefaultDependencies) race with us.
   +            before = [
   +              "sysinit.target"
   +              "sysinit-reactivation.target"
   +              "shutdown.target"
   +            ];
   +            conflicts = [ "shutdown.target" ];
   ```
   Commit message (166fc0d): *"WantedBy= only pulls the unit in, it does not order it. Normal services (After=sysinit.target via DefaultDependencies) therefore raced secret installation, e.g. postfix-setup in NixOS/infra#1047."*

### 3. Fixes and features relevant to safix (most relevant first)

#### 3.1 Race with `sysinit.target`: safix has the bug
- **Upstream:** 166fc0d / a8be3b1, `modules/sops/default.nix:469-484`. A new VM test asserts `systemctl show -p After sysinit.target | grep -q sops-install-secrets.service`, and that a `secret-consumer` oneshot can see `/run/secrets/...` at start (`checks/nixos-test.nix`, hunk `@@ -13,6 +13,14`).
- **safix today:** `modules/consume/installer.nix:216-218`:
  ```nix
  wantedBy = [ "sysinit.target" ];
  requiredBy = [ "sysinit-reactivation.target" ];
  before = [ "sysinit-reactivation.target" ];
  ```
  - It has `DefaultDependencies = "no"` (`:75`), so nothing else orders the unit before `sysinit.target`.
  - `design.md:153` of the archived own-the-installer change says these edges were preserved "unchanged" from a8627b21, which is before the fix.
  - The check at `modules/flake/checks/installer.nix:673-677` only tests `wantedBySysinit` and `rerunsOnSwitch`.
  - The early unit (`:185-210`) is ordered transitively, because it is `before` `systemd-sysusers.service` and `userborn.service`. I inferred that those two are before `sysinit.target`; I did not check it.
  - The VM test `installer-vm.nix` does not assert that a consumer can see its secret at start.
- **Recommendation: adapt.** Add `before = [ "sysinit.target" ]` and a structural assertion next to `wantedBySysinit`.
  - Catch: safix's `installer.afterUnits` (`:224`, tested with `age-decrypt-secrets.service`) gets added to `after`. A unit named there with `DefaultDependencies=yes` would then form an ordering cycle. sops-nix has no such option on its unit form.
  - Either document that `afterUnits` must name early-boot units, or assert against it.

#### 3.2 Shutdown semantics: safix has the bug
- **Upstream:** same commits. It adds `before`/`conflicts` on `shutdown.target`, and the VM test asserts both (`assert "shutdown.target" in conflicts`).
- **safix today:** both units use `unitBase` (`installer.nix:59-78`) with `DefaultDependencies = "no"` and neither `conflicts` nor `before shutdown.target`. With `RemainAfterExit = true` they stay active until the final kill and never get the implicit `Conflicts=shutdown.target` (inference from systemd semantics).
- **Recommendation: adopt** for `safix-install-secrets`. Upstream did not add this to its for-users unit, so for `safix-install-users-secrets` decide deliberately (see Q1).

#### 3.3 `RequiresMountsFor` shutdown loop with userborn and an initrd key mount (#962): safix has the bug, and worse
- **Upstream:** e80b8b7, `secrets-for-users/default.nix:49`. New test `user-passwords-userborn-key-mount`: the key sits on a `neededForBoot` bind mount, the VM reboots, and the test asserts there are no `"ordering cycle"` lines in the previous boot's PID-1 journal. Upstream's main unit still uses `RequiresMountsFor` (`modules/sops/default.nix:496`).
- **safix today:** `installer.nix:76` sets `RequiresMountsFor = identityMounts` on both units through `unitBase`. The early unit is also `requiredBy = [ "systemd-sysusers.service" "userborn.service" … ]` (`:199-203`), whereas upstream's is only `wantedBy systemd-sysusers.service`. That is a tighter coupling than the configuration #962 was filed against.
- **Recommendation: adopt.** Give the early unit `WantsMountsFor` (split `unitBase` or override it) and port the userborn key-mount reboot test into `installer-vm.nix`, which already has a `userborn` node (`:160`).

#### 3.4 Host keys used without sshd enabled (`services.openssh.generateHostKeys`)
- **Upstream:** d855d66, `modules/sops/default.nix:184`:
  ```
  if config.services.openssh.enable || config.services.openssh.generateHostKeys then
  ```
  The assertion messages were updated at `:440`, `modules/home-manager/sops.nix:331` and `modules/nix-darwin/default.nix:366`.
- **safix today:** `installer.nix:122` still gates `derivedHostKeys` on `config.services.openssh.enable` alone. `docs/tutorials/first-secret.md:136-137` says it "needs `services.openssh.enable` for those keys to exist".
- **Recommendation: adapt.** Add `|| (config.services.openssh.generateHostKeys or false)`, guarded with `options.services.openssh ? generateHostKeys` so older nixpkgs still evaluate (upstream reads the option unguarded). The option exists in nixpkgs per mynixos.com; I did not check it against safix's pinned 26.11. Then update the tutorial and `docs/reference/profile-options.md:281-283`.

#### 3.5 Nested keys in JSON files (#604): safix does not have this bug
- **Upstream:** 311ef94, `pkgs/sops-install-secrets/main.go:326-336`. A type switch now accepts both `map[string]interface{}` (JSON) and `map[interface{}]interface{}` (YAML). Test: `TestRecurseSecretKeyNested`.
- **safix today:** `ciphertext.rs:557-576` (`extract_json`) walks a `serde_json::Value` after `sops decrypt --output-type json`, so nested JSON already works. The test is `install.rs:1983` `a_nested_key_resolves_through_every_segment`.
- **Recommendation: ignore for the installer.** It does matter for `safix migrate` with `deploymentTarget = "sops-nix"`; see §4.

#### 3.6 Top-level key named in type errors: safix does not have this bug, but its error text differs
- **Upstream:** b8dc4ec and 4493170, `main.go:305-313`. Before the fix it printed `the value of key '' is not a string` (`:341`). Test: `TestRecurseSecretKeyErrorNamesTopLevelKey`.
- **safix today:** `install.rs:1227-1234` always names `entry.key`. But `extract_key` maps every failure through `missing_key` (`:1218-1224`), so a key that exists but holds an object or array is reported as `"{document} holds no value at key '{key}'"` (`error/mod.rs:1916`). safix also accepts numbers and booleans (`secret.rs:262-268`), which upstream rejects.
- **Recommendation: investigate (low priority).** Consider a separate "value at key is not a scalar" error. Keep accepting numbers and booleans as a deliberate divergence, and record it.

#### 3.7 Test VMs moved off `boot.initrd.postDeviceCommands`
- **Upstream:** 03b17c8: *"boot.initrd.postDeviceCommands does not exists on the new systemd initrd, which is now the default."* It now uses `system.activationScripts.earlySecrets` with `setupSecrets.deps`.
- **safix today:** no use of `postDeviceCommands` in `modules/`.
- **Recommendation: ignore.** Do note that the systemd initrd is now the default when writing VM fixtures.

#### 3.8 `stdenv.isLinux` → `stdenv.hostPlatform.isLinux`
- **Upstream:** 2b735bd.
- **safix today:** already uses `hostPlatform` throughout (for example `common.nix:517`, `home.nix:193`).
- **Recommendation: ignore.**

#### 3.9 Things that did not change
Home-manager restart semantics, `%r` expansion, templates, tmpfs/ramfs handling, generations and gnupg had no upstream changes in this range. The home-manager diff is cosmetic.

### 4. Effects on safix's compatibility claims and docs

- **`docs/guides/migrating-from-sops-nix-and-agenix.md`:** nothing in it is invalidated. Its sops-nix claims (no raw age files; keyed binary or empty values need a whole binary destination, `:131`) still hold, since upstream still needs a `string` leaf (`main.go:339-341`).
  - Version caveat: `migrate.rs:541-547` does not refuse a JSON destination with a `/`-nested key for the sops-nix target. sops-nix older than 311ef94 fails on such a module at activation with "does not refer to a dictionary".
  - Either state "requires sops-nix ≥ 311ef94 for nested keys in JSON destinations" in the guide and in `docs/reference/migration-plan.md:155-158`, or refuse that combination.
- **`openspec/changes/archive/2026-09-15-own-the-installer/design.md`:**
  - `:5` and `:153` say the unit edges were preserved verbatim from sops-nix a8627b21. Those edges are the buggy ones fixed in §3.1–3.3. The archived design can stay as history, but any live spec or doc claiming parity with sops-nix ordering is now out of date.
  - `:161` says `derivedHostKeys` is "unchanged in behaviour" compared with the provisioner. That is no longer true after d855d66 (§3.4).
- **`docs/tutorials/first-secret.md:136-137`:** needs updating if §3.4 is adopted.
- **`docs/notes/research/zero-knowledge-vault.md:9,108`:** the citation is pinned to a8627b21, so `main.go:340-346` is still correct for that revision. At HEAD the same `decrypt.File` call is at `main.go:349`. No change needed.
- **`flake.nix:87-93`:** safix already excludes `x86_64-darwin`, and upstream now agrees. No change.
- **sops version:** safix's research note already pins sops 3.13.3, the same version upstream moved its Go library to. No change.

### 5. Open questions

1. **Shutdown edges on the early unit.** Should `safix-install-users-secrets` also get `conflicts`/`before shutdown.target`? Upstream added these only to the main unit. safix's early unit is also `wantedBy sysinit.target` and `requiredBy userborn.service`, which upstream's is not.
2. **`afterUnits` and the cycle risk.** If `before = sysinit.target` is added (§3.1), should `installer.afterUnits` be restricted, either by an assertion or in its option description, to units with `DefaultDependencies=no`? The current fixture uses `age-decrypt-secrets.service`, and I have not checked that unit's dependencies.
3. **The brief's "dropped" features are present in the code.** It said safix dropped templates, `neededForUsers` and gnupg identities. The current code has all three: an early-user unit (`installer.nix:17-18`, `:185-210`), templates (`installer-vm.nix:89`; unit description "render public templates"), and `identity.gnupgHome` (`installer.nix:57`). §3.3 assumes the early-user path is live, as it is in the code. If something else is meant, the priorities above should be re-ranked.

### Files
- `modules/consume/installer.nix:54-78` (`identityMounts`, `unitBase`), `:114-136` (`derivedHostKeys`), `:185-229` (both units)
- `modules/flake/checks/installer.nix:673-677` (unit ordering assertions)
- `modules/flake/checks/installer-vm.nix:157-160` (VM nodes, including `userborn`)
- `crates/safix-core/src/install.rs:1207-1234` and `ciphertext.rs:557-576` (key extraction)
- `crates/safix-core/src/secret.rs:251-283` (scalars that are accepted)
- `crates/safix-core/src/migrate.rs:541-547`, `:1587-1594` (sops-nix target)
- `docs/tutorials/first-secret.md:134-138`, `docs/reference/profile-options.md:281-283`, `docs/reference/migration-plan.md:155-158`
- Upstream: `modules/sops/default.nix:184,440,469-496`, `modules/sops/secrets-for-users/default.nix:38-54`, `pkgs/sops-install-secrets/main.go:291-344`, `checks/nixos-test.nix` (new `user-passwords-userborn-key-mount` test)

## Survey: sops

I couldn't run `git` because this session has no shell. The commit list, file list and patches come from the GitHub compare API for `30332a959…cbb121664` and `v3.13.3…30332a959`. I checked the content against the local checkout, and the only files with fresh mtimes are the 9 the API lists.

**Short answer:** nothing in this range changes an argument, a stderr string or the metadata format that safix relies on. There are no new key sources, no age/ssh/pgp changes and no new security fix. One upstream security advisory still has no fix, and safix can protect itself against it today (item 3.1).

### 1. Range surveyed

- **Range:** `30332a959` (2026-08-10, merge of #2273) → `cbb121664` (2026-11-03, merge of #2339).
- **Size:** 39 commits, 2 contributors, 9 files.
- **Tags:** none. `.git/FETCH_HEAD` and `.git/packed-refs` end at `v3.13.3` (→ `26e2f4784`, 2026-07-23).
- **nixpkgs:** 3.13.3 is still the newest tag, so nixpkgs is not behind a release. It is 51 commits behind `main`: the 12 in `v3.13.3…30332a959`, which are all dependency bumps, plus these 39.
- **CHANGELOG:** `CHANGELOG.rst` is an 83-byte pointer. In `CHANGELOG.md`, line 3 is `## 3.13.3`, and there is no "Unreleased" or later section. Everything below is unreleased on `main`.
- **Files changed:** `cmd/sops/main.go` (+8), `sops.go` (+8), `functional-tests/src/lib.rs` (+43), `functional-tests/Cargo.lock`, `go.mod`, `go.sum`, and three CI workflows (`cli.yml`, `codeql.yml`, `release.yml`).
- **Untouched:** `age/`, `pgp/`, `stores/`, `config/`, `aes/`, `shamir/`, `cmd/sops/subcommand/*` (updatekeys, exec, filestatus), and `cmd/sops/{decrypt,set,edit}.go`.

### 2. Breaking changes

None. Both code changes only add things:

- **`cmd/sops/main.go`:** the hunk `@@ -994,6 +994,14 @@` adds two `cli.StringFlag` entries to the `encrypt` subcommand.
- **`sops.go`:** the hunk `@@ -380,6 +380,14 @@` adds a check inside `walkSlice`.

**Arguments safix passes:** none were removed or renamed. Each one below is unchanged upstream:

| Where in safix | sops call |
|---|---|
| `crates/safix-core/src/sops/mod.rs:144-147` | `decrypt --output-type json` |
| `mod.rs:219-230` | `--config X encrypt --filename-override R --input-type json --output-type yaml /dev/stdin` |
| `mod.rs:303-313` | `updatekeys --input-type F [--yes]` |
| `crates/safix-core/src/ciphertext.rs:414-420` | `decrypt --input-type F --output-type F -- path` |
| `ciphertext.rs:958-963` | `set --value-stdin --idempotent --input-type F --output-type F` |
| `ciphertext.rs:1021-1041` | legacy top-level `--encrypt --config /dev/null … --age … --pgp … --encrypted-regex .*` |

**stderr wording:** safix matches `no matching creation rules found` (`mod.rs:32`, used at `mod.rs:258`). Upstream still produces it at `config/config.go:602`: `fmt.Errorf("error loading config: no matching creation rules found")`. That file did not change in the range.

**Metadata format:** `stores/` did not change. The fields `document.rs` reads are still serialised as before, for example `stores/stores.go:54-56` (`encrypted_comment_regex`, `mac_only_encrypted`, `version`). That covers `age`, `pgp`, `key_groups`, `shamir_threshold`, the `*_regex`/`*_suffix` fields, and the flattened `key_groups__list_N__map_…` form at `document.rs:125-160`.

### 3. Fixes, features, UX and docs changes, ranked by relevance

**3.1 No fix yet for GHSA-jgf3-f6rg-8x3h (high, CVSS 7.4): HC Vault / OpenBao token exfiltration**
- **Upstream:** the advisory was published 2026-08-14 and lists `vulnerable_version_range: "*"` with no patched version. sops ≥ 3.13.0 offers an opt-in variable, `SOPS_HC_VAULT_ALLOWLIST` (`hcvault/keysource.go:31`, CHANGELOG.md:128-131), but its default is `all`. The advisory says: "Future versions of SOPS will eventually change the default to `none`, **without waiting for a new major release 4.0.0**." The attack: decrypting a file whose metadata names an attacker's `vault_address` sends `~/.vault-token` / `VAULT_TOKEN` to that server.
- **safix today:** decrypt runs through `Sops::command()` (`mod.rs:318-334`) and `sops_command()` (`ciphertext.rs:241-274`). Neither sets `SOPS_HC_VAULT_ALLOWLIST`, and nothing in the repo mentions it. `document.rs:153-160` already marks `hc_vault` as `<unsupported sops hc_vault recipients>`. However, `decrypt` still runs on whatever a teammate committed, and inherited-environment runs pass `VAULT_TOKEN` through.
- **Recommendation: adopt.** Set `SOPS_HC_VAULT_ALLOWLIST=none` in both sops builders, unless the operator has set it themselves (the same pattern as `SOPS_GPG_EXEC`). safix never supports Vault, so this costs nothing. It also matches what happens when upstream flips the default. Document it under "Upstream variables safix honours" in `docs/reference/environment.md:125`.

**3.2 Feature: `--unencrypted-comment-regex` / `--encrypted-comment-regex` on `sops encrypt`**
- **Upstream:** commit `74469c8aa` (PR #2283, merge `73416337f`) adds them at `cmd/sops/main.go:997-1004`. Before this, they existed only as top-level legacy flags (`main.go:1845-1852`) or as `.sops.yaml` creation-rule keys. They feed `main.go:2124-2213`, which allows only one crypt rule per file.
- **safix today:** it never emits comment regexes. `create_empty_document` uses the `encrypt` subcommand with no regex flags (`mod.rs:223-230`). The direct path uses the legacy `--encrypt` with `--encrypted-regex .*` (`ciphertext.rs:1040`). `document.rs:324-334` declines the in-place update index when either comment regex is set.
- **Recommendation: ignore.** safix's policy is encrypt-everything, and the flags add nothing for it. If safix ever moves `ciphertext.rs:1021` from legacy `--encrypt` to the `encrypt` subcommand, nothing breaks, because the flag sets now match.

**3.3 Fix: comment handling in `walkSlice` during encrypt/decrypt round-trips**
- **Upstream:** commits `5f3bdb4ed` and `3957bba9b` (tests), merged as `7b6eaa311` (PR #2284). At `sops.go:383-390`, once `encrypted_comment_regex` has encrypted a comment inside a YAML list, the comment becomes a non-comment, so the comment stack was cleared wrongly. That produced a wrong round-trip, and MAC/decrypt ran over a different tree than encrypt. The regression test is `functional-tests/src/lib.rs:1743` (`test_slice_comment_round_trip`). This is the same area as the 3.13.2→3.13.3 fix #2245 ("wrong MAC during decryption for sequences (YAML lists) that contain a comment").
- **safix today:** it only matters for files encrypted with `encrypted_comment_regex`. safix never writes such files, and `document.rs:324-334` treats them as foreign.
- **Recommendation: ignore** for safix-written files. Note it as a reason to take the next sops tag if consumers hand-edit governed files with comment regexes.

**3.4 Dependencies that touch crypto or parsing (no sops code change)**
- **Upstream:** `golang.org/x/crypto` 0.54.0→0.55.0 (`0be6a50bc`, PR #2279; covers SSH key parsing for `age/ssh_parse.go`). `go-jose/v4` 4.1.4→4.1.6, indirect (`57a5ad740`, PR #2339). Before the range, `go.yaml.in/yaml/v3` 3.0.4→3.0.5 (`003b1dc25`, PR #2264). `filippo.io/age` stays at v1.3.1 and `ProtonMail/go-crypto` at v1.4.1 (`go.mod:8,12`). Go stays at 1.25.8 (`go.mod:3`).
- **safix today:** it uses whatever nixpkgs builds. nixpkgs normally builds from the tag's `go.sum`, so these bumps arrive only with the next tag.
- **Recommendation: ignore** for now. Neither commit message says it fixes a security issue.

**3.5 CI and supply chain:** SLSA generator workflows pinned (`63a7476b0`, PR #2291), Go 1.27 added to CI (`417dc6e9b`, PR #2319), plus action bumps. **Ignore**: they change no binary behaviour.

**Not in range:** no `--idempotent` change (safix already uses it, `ciphertext.rs:958`). No `mac_only_encrypted` change. No age plugin, ssh key or pgp handling change. No new key source. No `updatekeys`, `exec-*` or `filestatus` change.

### 4. Effect on safix's compatibility claims and docs

- `docs/notes/research/zero-knowledge-vault.md:9` says sources were read at "sops 3.13.3". That is still the latest tag, so the claim is accurate.
- safix records no minimum or maximum sops version in `docs/`, `README.md` or `openspec/specs/`. It does rely on features that need sops ≥ 3.10: `set --value-stdin --idempotent` and the `encrypt`/`decrypt`/`updatekeys` subcommands. None of these moved.
- `docs/reference/environment.md:125-157` ("Upstream variables safix honours") lists `SOPS_AGE_*` and `SOPS_GPG_EXEC` but not `SOPS_HC_VAULT_ALLOWLIST`. If 3.1 is adopted, add it there. Also consider a line saying safix does not rely on, or break when, sops' announced default flip to `none`.
- The three other advisories, also published 2026-08-14, are all fixed by 3.13.3: GHSA-hqjv-fvgv-rh27 (INI/DotEnv memory exhaustion, fixed in 3.13.0), GHSA-x4cm-pcq4-39j4 (`exec-file --filename` path traversal, 3.13.0) and GHSA-pvcx-747w-54qw (`exec-* --user` GID, 3.13.2). Any doc that names the nixpkgs pin can state the pin is clear of them. safix does not use `exec-*`, and `document.rs` parses flattened INI/DotEnv metadata with its own code, not sops'.

### 5. Open questions

1. Should safix set `SOPS_HC_VAULT_ALLOWLIST=none` unconditionally, or only when it isn't already set? An operator who also uses Vault-backed sops files outside safix might expect their value to win. Your existing `SOPS_GPG_EXEC` handling (`mod.rs:321-325`) suggests "only when unset".
2. When `inherit_environment` is false, `sops_command()` scrubs `HOME` and the identity variables (`ciphertext.rs:244-260`). I didn't confirm whether that scrub also drops `VAULT_TOKEN`; the visible lines only set specific variables. If it doesn't, the inherited-environment exposure in 3.1 also applies to the "isolated" runs.

## Survey: agenix

### Summary

28 commits landed between 2026-02-03 and 2026-10-20. They add three release tags (0.16.0, 0.17.0, 0.18.0), two module options (`age.enable`, `age.verbosity`), a per-secret `enable` option, a new CLI verb (`agenix --check`), a rename of the rules file (`secrets.nix` → `agenix-rules.nix`), and a new secret ordering under sysusers/userborn. After 0.18.0, `main` also started using tmpfs with `noswap` instead of ramfs for the secrets mount.

For safix, four findings matter most:
1. **One safix claim is now false.** The migration guide and refusal say agenix cannot express early-user secrets. Agenix installs every secret before user creation under both activation paths.
2. **A safix doc claim contradicts agenix's new check.** `agenix --check` reads raw-age SSH recipient stanzas, which safix's guide says carry nothing a reader can attribute.
3. **safix's tmpfs fallback is weaker than agenix's.** When `noswap` is rejected (for example in containers), safix mounts a swappable tmpfs; agenix falls back to ramfs.
4. **Owner-without-group output can fail on darwin.** Under agenix 0.16.0–0.17.0, safix's agenix output with an owner but no group fails to evaluate on nix-darwin.

**How the evidence was gathered.** This session had no shell, so I could not run `git log --stat` or `git diff`. The local clone is a shallow, blobless partial clone, so its pack can't be read directly either. Instead:
- The working tree, which is at 5e5b88a (`.git/logs/HEAD`: "pull --ff-only: Fast-forward"), was read locally.
- Commit lists came from GitHub compare pages.
- Diffs came from `https://github.com/ryantm/agenix/commit/<sha>.diff`.
- Baseline files came from `raw.githubusercontent.com/.../b027ee2/...`.
- Release notes came from `releases/tag/*`.

All of this was read-only network access, not a clone. Line anchors below are at HEAD.

### 1. Range surveyed

- **Range:** `b027ee29` (2026-02-03, shallow boundary in `.git/shallow`) → `5e5b88a0` (2026-10-20).
- **Commits:** 28 in total.
  - 11: b027ee2 → 0.16.0
  - 3: 0.16.0 → 0.17.0
  - 6: 0.17.0 → 0.18.0
  - 8: 0.18.0 → HEAD
- **Tags:**
  - `0.16.0` = f4fcfdd (2026-09-26 16:35Z)
  - `0.17.0` = 07a3331 (2026-09-26 17:55Z)
  - `0.18.0` = 9353cb2 (2026-09-27 17:43Z)
- **Release-note caveat:** the 0.16.0 notes cover everything since 0.15.0 (2023-12). Items such as armor (#276), sysusers (#255), rage→age (#230), decrypt-parallel and its revert, and the darwin `identityPaths` fix (#307) were merged before b027ee2, so they are **not** in this range.
- **Version string:** `pkgs/agenix.nix:21` is still `version = "0.15.0"` at HEAD, so `agenix -h` reports 0.15.0. That this was also true at the 0.18.0 tag is [INFERENCE].

### 2. Breaking changes (verbatim)

| # | Commit / release | Verbatim | Impact |
|---|---|---|---|
| B1 | f4fcfdd (#210, 0.16.0) | `doc/reference.md`: "The legacy `RULES` environment variable and automatic discovery of `secrets.nix` still work, but agenix warns when either is used. Both will be removed in a future version. Explicitly selecting `secrets.nix` with `AGENIX_RULES` does not warn." `pkgs/agenix.sh:174`: `warning: RULES and automatic discovery of secrets.nix are deprecated and will be removed in a future version of agenix; use AGENIX_RULES and agenix-rules.nix instead.` | The CLI now `cd`s into the rules directory (`agenix.sh:176`) and "Resolves relative secret paths from the selected rules file's directory." `-i` paths are made absolute against the original `$PWD` (`agenix.sh:88-91`). Only `agenix-rules.nix` is searched for in parent directorie…
| B2 | 7133e84 (#384, 0.16.0), fixed by 9353cb2 (#399, 0.18.0) | 9353cb2 message: "#384 dropped the `or "0"` fallback, so an owner matching a user without a `group` attribute fails evaluation. nix-darwin users never have one." | In 0.16.0 and 0.17.0, `age.secrets.<n>.owner = "x"` with no `group` fails evaluation on nix-darwin. The default group is now looked up by the user's `.name`, not the attribute name (`modules/age.nix:222`). |
| B3 | de100bf (#353, 0.16.0) | "fix: create secrets before sysusers, chown after … Fixes #345". The unit changes from `after = [ "systemd-sysusers.service" ]` to `before = [ "systemd-sysusers.service" ]` (`age.nix:346`), and a new `agenix-chown.service` is added (`age.nix:362-385`). | On sysusers/userborn hosts, secrets are now root-owned until `agenix-chown` runs. Anything ordered after `agenix-install-secrets` expecting final ownership must now order after `agenix-chown`. |
| B4 | 07a3331 (#317, 0.17.0) | `config = mkIf (cfg.secrets != { })` → `mkIf cfg.enable`, with `age.enable` defaulting to `enabledSecrets != [ ]` (`age.nix:243-245`). Doc: "Switching an existing system to `false` does not remove secrets already decrypted in `/run`; reboot to clear them." | Mostly additive. A config that declares only disabled secrets now installs nothing. |
| B5 | 33bcf55 (#381, 0.16.0) | Removes `systems.url = "github:nix-systems/default";`. | `agenix.inputs.systems.follows` overrides now point at an input that no longer exists. |
| B6 | 94657d1 (#407, post-0.18.0) | `doc/contributing.md`: "The integration `checks`, `darwinConfigurations`, and Home Manager test configuration previously exported by the root flake are now under `./test#`." The root `flake.nix` now has only a `nixpkgs` input. | `inputs.darwin` and `inputs.home-manager` are gone from the root flake, so `follows` overrides on them no longer apply. |
| B7 | 488dfad (#410, post-0.18.0) | "The former `packages.<system>.doc` flake output is retired" (`doc/contributing.md`). | Flake output removed. |
| B8 | 847720f (#454) + 5e5b88a (#456), `main` only, no tag | `mount -t tmpfs none … -o nodev,nosuid,mode=0751,noswap` when the kernel is ≥ 6.4 and the host is not `boot.isContainer`; ramfs otherwise (`age.nix:44-60`). | The secret store's filesystem type changes. Mounts are detected by `grep "<mp> tmpfs" /proc/mounts`. [INFERENCE] On a live switch from a ramfs-mounted host, a tmpfs gets mounted on top of the existing ramfs and the old generations stay hidden in RAM until reboot. |

**Why three releases in two days.**
- **0.16.0** was the first tag since 2023 and carried the rules-file rename (B1), the sysusers reordering (B3) and the group-lookup change (B2).
- **0.17.0**, 80 minutes later, added user-facing features: one-way secrets (6dc0d4c, #157), `enable` toggles (07a3331, #317) and `./` path normalisation (c34d0c9, #392).
- **0.18.0**, the next day, fixed the 0.16.0 darwin regression (9353cb2). It also:
  - guarded `config.services.userborn.enable` behind `options.services ? userborn` (c37a8b4, #394);
  - added `age.verbosity` (231eb17, #393);
  - added `agenix --check` (654f731, #397).

### 3. Fixes, features, UX and docs relevant to safix (ranked)

1. **Early-user secrets under sysusers/userborn** (de100bf; `age.nix:340-385`, `test/userborn.nix`).
   - **Upstream:** decrypts every secret before `systemd-sysusers`/userborn, then chowns afterwards. The activation-script path already did this at the baseline: `users.deps = [ "agenixInstall" ]` (`age.nix:406`), which was present at b027ee2. The new test checks that `hashedPasswordFile = config.age.secrets.password.path` works under userborn.
   - **safix today:** the migration emitter refuses `neededForUsers` for the agenix target (`crates/safix-core/src/migrate.rs:526-538`), and the docs say agenix can't express it (see §4).
   - **Recommendation: adapt.** For agenix, `neededForUsers: true` with root ownership (which `migrate.rs:557-570` already enforces) can be emitted as a plain `age.secrets` entry, because agenix installs everything before users exist. The `neededForUsers` flag itself would be dropped from the output, but its guarantee holds. Pin agenix ≥ 0.16.0 for userborn hosts.
   - **Validation:** safix's own installer already orders before `systemd-sysusers.service` and `userborn.service` (`modules/consume/installer.nix:199-208`) and guards `options.services ? userborn` (`installer.nix:90-91`). Upstream has converged on safix's design here.

2. **`agenix --check`** (654f731; `pkgs/agenix.sh:294-300` `ssh_tag`, `:302-403` `check_file`, `:406-414` `check`; `doc/reference.md` CLI section).
   - **Upstream:**
     - recomputes age's 4-byte SHA-256 tag for each `ssh-ed25519`/`ssh-rsa` rule key and compares it with the `-> ssh-… <tag>` stanzas;
     - decodes armor first;
     - skips `*-grease` stanzas;
     - flags missing, extra and duplicate recipients;
     - recovers the full extra key when the same key appears literally in the rules file;
     - refuses non-SSH recipients ("cannot check non-SSH recipient").
   - **safix today:** `check.rs:348-353` reports every raw-age file as `RecipientRosterUnverifiable`. `docs/guides/inspectable-recipients.md` says "A raw age file records nothing a reader can attribute". safix's own note `docs/notes/research/zero-knowledge-vault.md:107` already concedes that SSH stanzas carry a fingerprint.
   - **Recommendation: adapt.** Add a partial SSH-tag roster check for raw-age files. Report it as "SSH recipients verified, X25519 unverifiable" rather than a pass, because the tags are only 32 bits and X25519 recipients are hidden. At minimum, correct the guide sentence.

3. **tmpfs with `noswap`, and the container fallback** (847720f, 5e5b88a; `age.nix:44-60`).
   - **Upstream:** uses tmpfs+`noswap` on kernels ≥ 6.4. Otherwise, and always inside containers, it uses ramfs, citing issue #455: "Turning off swap in unprivileged tmpfs mounts unsupported".
   - **safix today:** ramfs by default, tmpfs only when `safix.installer.useTmpfs` is set (`installer.nix:82-86`). On `EINVAL` it retries tmpfs **without** `noswap` (`crates/safix-core/src/install.rs:1309-1324`), which gives a swappable store. [INFERENCE] The kernel's unprivileged-userns rejection of `noswap` returns `EINVAL`, so a safix container with `useTmpfs = true` silently gets a swappable store.
   - **Recommendation: adapt.** On `EINVAL`, fall back to ramfs (agenix's choice) or refuse; don't fall back to swappable tmpfs. Keep safix's default as is. Note the stacked-mount hazard when toggling filesystem type on a live host (B8): safix's `filesystem_magic != magic → mount` at `install.rs:1272-1274` has the same shape.

4. **Default group lookup and the darwin regression** (7133e84, 9353cb2; `age.nix:222`).
   - **safix today:** the agenix emitter writes `owner` and `group` only when the plan supplies them (`migrate.rs:670-691`). A numeric `uid` with no group gets agenix's default group.
   - **Recommendation: adapt.** Either always emit `group` for the agenix target, or document "agenix ≥ 0.18.0 required on nix-darwin when `owner` is set without `group`". Under 0.16.0–0.17.0, the generated module fails evaluation on darwin.

5. **One-way secrets** (6dc0d4c, then 4a99c13's darwin fix that replaced `cp /dev/stdin` with `cat >`; `agenix.sh:233-249`). Piped input replaces a secret without decrypting it first, while rekeying still needs an identity.
   - **safix today:** `safix set` reads piped stdin whole (`docs/reference/cli.md:72-87`). The guide says raw-age writes go to the declared audience.
   - **Recommendation: investigate** whether `safix set` on raw-age or binary entries works with no identity. If it does, document it as the equivalent; if not, consider adopting it for whole-document formats only.

6. **`age.verbosity` and warnings moved to stderr** (231eb17; `age.nix:257-269`; `>&2` at `age.nix:113,119,120,134` and `age-home.nix:62,69,70,85`). At the baseline these warnings went to stdout.
   - **Recommendation: ignore** for code; nothing in safix parses agenix output. It is a possible UX idea for safix's installer.

7. **`age.enable` and per-secret `enable`** (07a3331).
   - **Recommendation: ignore.** The safix emitter needs no change; the new options default to true.

8. **CLI `./` normalisation and rules-relative paths** (c34d0c9, f4fcfdd; `agenix.sh:53-58`, `:176`).
   - **Recommendation: ignore** for code. For coexistence: safix's quickstart declarations file is also called `secrets.nix` (`examples/quickstart/secrets.nix`, `docs/tutorials/first-secret.md:54`). Running `agenix` in that directory auto-discovers it as legacy rules and fails confusingly. Worth one line in the migration guide (see §4).

9. **Identity and decryption behaviour (no change).**
   - The default NixOS `identityPaths` are still the rsa+ed25519 host keys, with ed25519/rsa fixed paths on darwin (`age.nix:301-326`).
   - `test -r` and `test -s` skipping is unchanged on NixOS (`age.nix:107-108`, present at the baseline). Home-manager still lacks `test -s`; only the loop was refactored to an array (`age-home.nix:55-60`).
   - Still no age-plugin support in the modules (`ageBin` only) and no plugin `PATH`. safix's `safix.installer.agePlugins` (`installer.nix:62`) remains a differentiator.
   - Armor encryption predates the range; it is now also understood by `--check`. safix's installer accepts armored raw age (`install.rs:1113-1118`).
   - **Recommendation: ignore.**

10. **Home-manager and darwin changes.**
    - Home-manager gained `age.enable`, `age.verbosity`, per-secret `enable` and stderr warnings. There are no path or launchd/systemd unit changes (`age-home.nix:268-296` matches the baseline).
    - Darwin:
      - The CI test notes "nix-darwin 26.05 runs activation as root" (3683254, `test/install_ssh_host_keys_darwin.nix`).
      - The group-fallback fix (B2).
      - Piped CLI input now works on darwin (4a99c13).
      - The hdiutil HFS RAM disk is unchanged.
    - **Recommendation: ignore**, apart from item 4.

11. **Docs reorganisation** (488dfad, 3daa710, b274cb2). The README is now a stub pointing to `ryantm.github.io/agenix`, with the reference in `doc/reference.md`. The README quick start uses `agenix-rules.nix` and `nixos-26.05`.
    - **Recommendation: adapt** safix links: link to the published site or `doc/reference.md`, not README anchors. safix currently links nothing upstream.

### 4. Items affecting safix's recorded compatibility claims and docs

**Stale or wrong after this range:**
- `docs/guides/migrating-from-sops-nix-and-agenix.md:132`: "Agenix as the target for a template, a service hook or early-user metadata. Agenix requires whole raw-age files and expresses none of the three." The early-user part is wrong: agenix installs all secrets before user creation (`age.nix:346`, `:406`; tested by `test/userborn.nix`). Templates and hooks remain correct.
- `docs/reference/migration-plan.md:150-153`: "It supports no … `neededForUsers`". Same correction.
- `crates/safix/src/usage.rs:862-863`: "Agenix cannot retain templates, service hooks or early-user metadata." Same correction.
- `crates/safix-core/src/migrate.rs:533-538`: the refusal "agenix deployment does not support restartUnits, reloadUnits, or neededForUsers". Same.
- `docs/guides/inspectable-recipients.md` ("A raw age file records nothing a reader can attribute…" and "the file discloses no roster"): contradicted for SSH recipients by agenix 0.18.0's `--check` (item 3.2).
- `docs/guides/migrating-from-sops-nix-and-agenix.md:84`: "It carries the units to restart or reload, and whether the value is needed before user creation." This is not true of the agenix output (`migrate.rs:693-707` omits both). The inaccuracy predates this range, but it sits beside the line above.
- `openspec/specs/secret-migration/spec.md:29` ("SHALL publish usable declarations for the chosen … agenix consumer"): "usable" depends on agenix version for owner-without-group on darwin (B2). Either record a minimum agenix version or always emit `group`.

**Missing from the migration guide:**
- Agenix users now keep `agenix-rules.nix`, not `secrets.nix`. The rename needs no action from `safix migrate`, which takes explicit paths, but a mention would help. Also note the `secrets.nix` name clash with safix's quickstart.
- No minimum agenix version is stated for the agenix target. A floor of ≥ 0.18.0 is recommended (darwin group fix, userborn guard).

**Not stale:**
- `README.md:266-272`: no agenix-specific claim.
- `docs/notes/research/zero-knowledge-vault.md:107-108`: already acknowledges SSH stanza fingerprints.

**Tests:** no safix check evaluates the generated agenix module against real agenix options (grepping `modules/flake` and `crates/safix/tests` for agenix found only identity-path fixture strings). Every claim above is therefore unchecked by CI.

### 5. Open questions

1. When `deploymentTarget = "agenix"` and `neededForUsers: true`, should the emitter emit the entry (agenix's ordering satisfies it) or keep refusing (the flag can't be written into the agenix output)? This is a policy call.
2. On `noswap` `EINVAL`, should safix fall back to ramfs (agenix's choice), refuse, or keep swappable tmpfs? The kernel errno in an unprivileged userns is [INFERENCE] and was not tested here.
3. Should safix pin a minimum agenix version (≥ 0.18.0), or always emit `group` to stay compatible with 0.16.0–0.17.0 on darwin?
4. Should safix adopt partial SSH-tag roster checking for raw-age files, with a finding distinct from `RecipientRosterUnverifiable`, or only correct the guide?
5. Should a CI check be added that evaluates the generated `age.secrets` module against a pinned agenix?
6. To confirm the diffs locally, run `git -C /home/sernl/ghq/github.com/ryantm/agenix fetch --deepen=40` (or `ghq-sync --deepen 40`), then `git log --stat b027ee2..5e5b88a`. The clone is shallow and blobless, so this needs a network fetch, which this read-only session could not do.

[Some lines truncated to 768 chars. Use agent://AgenixDelta:raw:41-41 to read line 41 whole]

## Survey: agenix-rekey

Nothing upstream breaks safix: safix has never pinned or imported agenix-rekey, and no safix doc claims compatibility with it. The highest-value findings are four safix issues that mirror upstream bugs:

- **Card recipients may not be encryptable from the shipped CLI.** safix's CLI wrapper doesn't put `age-plugin-yubikey` on its PATH. Upstream had to fix the same thing in 8dbbdd9: encrypting to a plugin recipient where the plugin isn't available.
- **Editor history can leak plaintext.** `safix edit` does nothing to stop vim or nvim writing swap, undo, backup or register history to disk. With default nvim settings that lands outside safix's memory-backed staging. Upstream passes hardening flags to both editors.
- **`check` misses stale derived values.** It doesn't flag a derived value whose input was later changed with `set`, `edit` or `sync`. Upstream catches this case, though crudely, by comparing file mtimes.
- **`fix --yes` rewrites every raw-age file on every run.** Upstream's `update-masterkeys` has the same flaw: a "skip if unchanged" check that can never fire, because age output is randomised.

Most of upstream's other features (rekey caching, storage modes, a stand-in key for unknown hosts, intermediary secrets) answer problems safix's design doesn't have.

### 1. Tree and commit range surveyed

- **Tree:** `/home/sernl/ghq/github.com/oddlama/agenix-rekey` at HEAD `87e5d5f` ("Merge pull request #161 … fix-cross-platform-age-plugins", 2026-10-09). I read `README.md`, `apps/{rekey,generate,edit-view,update-masterkeys}.nix`, `nix/{lib,output-derivation,select-nodes}.nix` and parts of `modules/agenix-rekey.nix`.
- **How I got the history:** I couldn't run `git log` here (no shell, and the pack files can't be read). `.git/shallow` lists 7 boundary commits (`0086353 26f8ba2 3dd6239 4e66c4b a05e170 aa08dce f5d277d`), which fits the 25 local commits you mentioned. For the history itself I used GitHub's commit list for `main` and the `.patch` of each relevant commit.
- **Range:** 35 commits listed, from `4ca4022` (2025-09-11) to `87e5d5f` (2026-10-09). I found no release tags. Relevant commits:

| Commit | What it does |
|---|---|
| `f5d277d` | Stop rekeying intermediary secrets for hosts |
| `26f8ba2` | Hide the stand-in key warning when there are no secrets |
| `aa08dce`, `0086353`, `60c1fca`, `7de21fb` | darwin modules, home-manager class, darwin/nixos name collisions |
| `3dd6239` / `8b9c179` | Generator `settings` |
| `f5e3a32` → `d672994` | `requiredSystemFeatures`, then a fix so it doesn't change store paths |
| `8dbbdd9` | Plugins available when encrypting stand-in secrets |
| `f6c79f2` | flake-compat |
| `4e66c4b` (PR #107) | `extraDecryptionArgs` |
| `4425a20` → `8beed85` | Filter age plugins by platform |
| `6e58a07` | Keep `rekeyFile`'s store reference |

### 2. Breaking changes, quoted

1. **`extraDecryptionArgs` changed from a string to a list** (`4e66c4b`, `flake.nix:94`):
   ```
   -              # Arguments to append to the age decryption command
   -              extraDecryptionArgs ? "",
   +              # Additional decryption arguments, escaped individually (no shell expansion).
   +              extraDecryptionArgs ? [ ],
   ```
2. **Generator `deps.host` and host names are now prefixed** (`7de21fb`, `nix/select-nodes.nix`). README:
   > "`host` is always namespaced as `nixos:<host>` or `darwin:<host>`"
   
   Code: `effectiveHostConfigurations = prefixedNixosConfigurations // prefixedDarwinConfigurations;`. Any generator script that matched on a bare host name now breaks.
3. **Intermediary secrets are no longer rekeyed** (`f5d277d`, `apps/rekey.nix:80`, `nix/output-derivation.nix`): `secret.rekeyFile != null && !secret.intermediary`. Rekeyed files for those secrets become orphans and are deleted.
4. **The master-identity assertion was loosened** (`4e66c4b`, `modules/agenix-rekey.nix:215-217`). It went from `"rekey.masterIdentities must be set."` to `"age.rekey.masterIdentities or age.rekey.extraEncryptionPubkeys must be set."`.
5. **`requiredSystemFeatures` changed derivation paths for everyone** (`f5e3a32`). `d672994` undid that: *"Even an empty list would change the existing derivation and output paths."* (`nix/output-derivation.nix`, the `optionalAttrs (… != [ ])` at the end).

None of these touch safix.

### 3. Fixes, features and UX, ranked by relevance to safix

**3.1 Encrypting to a plugin recipient without the plugin — Investigate (high)**
- **Upstream:** `8dbbdd9` added `nativeBuildInputs = [ pkgs.age ] ++ config.age.rekey.agePlugins;` (`modules/agenix-rekey.nix:57-79`), because encrypting to a plugin recipient needs the plugin. It also ships `age-plugin-yubikey` by default and appends plugins to the *end* of PATH (`modules/agenix-rekey.nix:789`, `nix/lib.nix:48`: `PATH="$PATH":<plugins>`).
- **safix today:** the CLI wrapper adds age, sops, gnupg, openssh, ssh-to-age, git and nix to PATH, but no plugin (`modules/flake/rust.nix:80-99`). Only the installer gets plugins (`modules/consume/common.nix:491-495,508-512`; `modules/consume/installer.nix:62`). `enroll` finds the plugin through `SAFIX_AGE_PLUGIN_YUBIKEY` or PATH, but only to generate the identity (`crates/safix-core/src/enroll/identity.rs:48-51`). safix's own checks say *"wrapping a data key to a card's recipient runs the plugin"* (`modules/flake/checks/cli.nix:733-734`).
- **Why it matters [inference]:** after `enroll`, every `set`, `edit`, `fix` or `generate` that writes a file whose audience includes an `age1yubikey1…` recipient needs `age-plugin-yubikey` on PATH. If the plugin was given only through `SAFIX_AGE_PLUGIN_YUBIKEY`, or comes from the bare `nix run` package, sops would fail to wrap.
- **Recommendation:** check whether sops 3.13.3 needs the plugin binary to encrypt to these recipients. If it does, append the plugin to the wrapper's PATH, after anything already on PATH as upstream does.

**3.2 Editor history leaks — Adopt (high)**
- **Upstream:** `apps/edit-view.nix:152-156` sets, for nvim, `--cmd 'au BufRead * setlocal nobackup nomodeline noshelltemp noswapfile noundofile nowritebackup shadafile=NONE'`, and the same for vim with `viminfo=""`.
- **safix today:** it states the limit but doesn't reduce it (`crates/safix-core/src/edit.rs:24-26`; `openspec/specs/editor-input/spec.md:93-101`). That spec assumes swap files sit "beside the buffer", but nvim's default swap directory is `$XDG_STATE_HOME/nvim/swap//`, and its shada and vim's viminfo store register contents. So a default nvim already escapes the guarantee.
- **Recommendation:** when `$VISUAL`/`$EDITOR`'s first word is `vim` or `nvim`, pass these options as argv; that fits safix's existing whitespace-split exec. Keep the stated limit for all other editors.

**3.3 Derived values going stale after an upstream write — Adapt (high)**
- **Upstream:** `apps/generate.nix:120-140` regenerates a secret when any dependency's file mtime is newer (`dep_mtimes`, `mtime_newest_dep -gt mtime_this`). It's crude: git checkouts reset mtimes.
- **safix today:** it only cascades on `--regenerate` (`crates/safix-core/src/generate.rs:180-182`). `set`, and so `edit` and `sync`, update the upstream value's stamp (`crates/safix-core/src/set.rs:217`) but know nothing about generators. `check` reports only `DefinitionDrift` and `RotationDue` (`crates/safix-core/src/check.rs:177,195`). A hash of a password changed by hand therefore goes unreported, which is exactly what safix's own cascade text warns against (`generate.rs:282-286,316-318`).
- **Recommendation:** add a `check` finding for when a dependency's UPDATED stamp is later than the dependent's. It needs no decryption, and the remedy is `generate --regenerate <dep>`. This is better than upstream's mtime approach.

**3.4 Rewrites that never skip — Investigate (medium)**
- **Upstream:** `apps/update-masterkeys.nix:34-46` compares the sha512 of the old ciphertext with the freshly re-encrypted one. age output is randomised, so `Skipping [already rekeyed]` can never be reached and every file is rewritten.
- **safix today:** `rewrap_age` always decrypts, re-encrypts and renames each raw-age file under `--yes` (`crates/safix-core/src/fix.rs:167-232`, rename at `:220`). So every `safix fix --yes` changes every `.age` file in git even when the audience hasn't changed.
- **Recommendation:** skip when nothing could have changed. One option is to record a digest of the declared audience per raw-age file next to the definition records. Separately, it's worth confirming that sops `updatekeys` is a no-op on files that are already current [unverified].

**3.5 Changing what a digest covers — Adapt (medium)**
- **Upstream:** `d672994` adds `requiredSystemFeatures` only when non-empty, so existing derivation hashes stay the same (`nix/output-derivation.nix`, last `optionalAttrs`).
- **safix today:** adding a field to the generator digest bumps the format tag (`crates/safix-core/src/definition.rs:89-94`, v1 → v2 for `network`). Every older record then reads as "unknown version" and gives no finding, so drift detection goes quiet for every value minted before the bump.
- **Recommendation:** next time, leave a new field out of the canonical text while it holds its default. Untouched declarations keep their digest, and old records keep being checked.

**3.6 Choosing which identity to try first — Investigate (medium)**
- **Upstream:** `AGENIX_REKEY_PRIMARY_IDENTITY` and `…_ONLY` (`nix/lib.nix:155-190`; README "Environment variables"). It also pulls the public key out of `Recipient: age1yubikey1…` comments in identity files (`nix/lib.nix:106-111`), and on a failed decrypt asks retry / abort / use a stand-in value / use stand-ins from now on (`apps/rekey.nix:396-401`).
- **safix today:** one identity file, and the order of identities in it decides what is tried (`docs/reference/environment.md`, "Which identity decrypts"). `enroll` appends the card's identity block after the software one (`docs/guides/hardware-keys.md`, "What the run does"). The design allows the card to become the only master later (archived enroll design, line 87).
- **Recommendation:** for card-only operators, look at `safix fix --yes` running 4 card unlocks at once (`fix.rs:60,590`). That may contend on pcscd and look like `plugin_stalled`. Consider a rule that runs one at a time when the only identity is a plugin stub, or document `SAFIX_FIX_CONCURRENCY=1`.

**3.7 FIDO2 and TPM keys — Investigate (low–medium)**
- **Upstream:** README section "Using a FIDO2 key instead of a YubiKey" (`age-plugin-fido2-hmac`, with its `public key:` comment extracted at `nix/lib.nix:112-115`). TPM is mentioned as a master key type.
- **safix today:** `enroll` handles YubiKey PIV only, through ykman and `age-plugin-yubikey` (`docs/guides/hardware-keys.md`).
- **Recommendation:** decide whether "a hardware recovery identity" should include FIDO2 hmac-secret keys. If it shouldn't, say so in the hardware keys guide.

**3.8 View/edit preview defaults — Ignore (deliberate), document**
- **Upstream:** the fzf preview is off by default; `-p` turns it on with the warning "Can be insecure, as the secrets are shown in plain-text." (`apps/edit-view.nix:36-37`). `edit -i INFILE` refuses to overwrite an existing file (`:135`). An unchanged buffer is detected with sha512 (`:145-167`).
- **safix today:** the preview is on by default, with `--no-preview` to turn it off (`docs/reference/cli.md:89-115,129-174`). An unchanged buffer is already a no-op, and piped `set` covers `-i`.
- **Recommendation:** keep the default if it was chosen on purpose. The archived view-picker proposal doesn't say why the default differs from upstream, so record the reason there.

**3.9 Generator model differences — Ignore**
- **Upstream:**
  - Generator scripts run unsandboxed and print the value to stdout (README, the `generator.script` option: "runs outside of any sandbox").
  - It has no declared prompts.
  - Dependencies can cross hosts.
  - It adds `settings` (`3dd6239`) and `tags` with `generate -t` (`apps/generate.nix:189-214`).
  - Orphaned files in `generatedSecretsDir` are deleted automatically (`apps/generate.nix:270-285`).
- **safix today:** confined sandbox, declared prompts, dependencies limited to the same person, cascade confirmation, and orphans reported rather than deleted (`docs/guides/generators.md`; `docs/reference/cli.md` "check").
- **Recommendation:** safix is ahead on every one of these. `settings` adds nothing to string-based Nix scripts, and tags are a nice-to-have at most.

**3.10 Bootstrapping a new host — Ignore**
- **Upstream:** a stand-in public key (`modules/agenix-rekey.nix:42,255-260,432-434`) produces secrets that fail on first activation. `26f8ba2` shows the warning only when there are secrets.
- **safix today:** `upload` writes the host's private key before its first activation, and a machine with no recipient is a refusal (`docs/guides/unattended-hosts.md:19-22,81-84,101`).
- **Recommendation:** safix's approach is stricter. Nothing to take.

**3.11 Keeping the store reference for a path — Ignore (safix is already safe)**
- **Upstream:** `6e58a07` changed `secret.rekeyFile` to `"${secret.rekeyFile}"` because `toString` dropped the store dependency.
- **safix today:** the manifest serialises the `sopsFile` path value with `builtins.toJSON` (`modules/consume/common.nix:133-150,571-600`), which copies it into the store. Migration output uses `builtins.path` (`crates/safix-core/src/migrate.rs:617-621`).

**3.12 Plugins built for another platform — Ignore**
- **Upstream:** `4425a20`/`8beed85` (`nix/lib.nix:30-35`) stop plugins merged from all hosts' configs being run on the wrong architecture.
- **safix today:** each host's plugins reach only that host's installer, and nothing merges them into the operator's CLI. This becomes relevant only if you take 3.1 and source plugins from host configs.

**3.13 Secrets the host should never decrypt — Ignore (covered)**
- **Upstream:** `f5d277d` fixed a secret meant to stay off the host still being encrypted to the host's key.
- **safix today:** the audience is the encryption set, and `check` reports extra recipients (`RecipientDrift`, `check.rs`). The one exception is raw age, which `check` flags as unverifiable (`RecipientRosterUnverifiable`).

### 4. Compatibility claims and docs

- safix makes no agenix-rekey compatibility claim anywhere; the only mention of `rekeyFile` is its removal (`openspec/changes/archive/2026-08-17-extract-safix-from-dotfiles/design.md:94`). Nothing above invalidates a recorded safix claim.
- **Migration docs gap:** "Migrating from sops-nix and agenix" covers ryantm/agenix only. agenix-rekey users have:
  - master-encrypted `rekeyFile`s, which are the real sources;
  - per-host `rekeyed/` folders, which should not be migrated;
  - YubiKey identity files (`AGE-PLUGIN-YUBIKEY-…`) as source identities.
  
  A migration plan's `ageKeyFile` can point at such a file, but decrypting then needs the plugin on PATH, which ties back to 3.1.
- **Recommendation:** add an agenix-rekey subsection to `docs/guides/migrating-from-sops-nix-and-agenix.md`.
- The `"agenix"` migration target (`migrate.rs:605-621`) writes `age.secrets.<n>.file`, which agenix-rekey still accepts (`rekeyFile` defaults to null). The namespacing change in §2 item 2 doesn't affect it.

### 5. Open questions

1. Does sops 3.13.3 run `age-plugin-yubikey` from PATH when *encrypting* to `age1yubikey1…`? That settles whether 3.1 is a real bug or only a hardening.
2. Does sops `updatekeys` decrypt the data key before deciding nothing changed? That decides whether `fix` on a card-only identity costs one unlock per file.
3. Was the preview-on-by-default choice in `safix view` and `safix edit` made consciously against upstream's opt-in, or should it be written down?
4. Should FIDO2 hmac-secret keys be enrollable at all, or excluded on purpose?
