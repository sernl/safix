# Design: safix owns its own installer

## Context

Citations are as read while designing this change, against this worktree and against the local sops-nix checkout at the revision safix pins (`a8627b21b9107c5711c96b84f32a9a4b3d45295f`, `flake.lock:274`).
Line numbers drift; the symbol name is the anchor that survives.

Three facts fix the shape of this change.

The hardest coupling is a type, not a binary.
`modules/consume/common.nix:210` reads `type = options.sops.secrets.type`, so safix's resolved set is typed by the provisioner's own option declaration inside the same evaluation.
That single line is why the provisioner's module is a required import even for `nixosModules.safix`, whose contract is that it imports nothing, and it is why deleting the input breaks every system-scope evaluation before it breaks any activation.

Everything else safix takes from sops-nix at system scope is a read of a declared option, enumerated exhaustively in decision I4, plus one `exec` of one binary (`installer.nix:141`) and one `checkPhase` invocation of the same binary built for the other platform (`:226-229`).
The manifest safix already builds is its own JSON, assembled at `installer.nix:186-222`; the activation registration, the unit, the ordering options and the identity preflight are already safix's own (`:343-398`).
So at system scope this change is a type, an option table, and a program — not a re-architecture.

At user scope it is genuinely new work.
`modules/consume/home.nix` owns exactly one thing today, `home.activation.safixIdentityPreflight` (`:246-249`), and hands installation entirely to the provisioner's home-manager module: it writes `sops.secrets` (`:232`), `sops.age.keyFile` (`:236`) and `sops.age.sshKeyPaths` (`:240`) and lets that module's own `systemd.user.services.sops-nix` and activation entry do the work.
Owning the user scope means adding a manifest, a unit, an activation entry, and the `%r` expansion the provisioner performs on linux and darwin.

## Goals / Non-Goals

**Goals:**

Delete the `sops-nix` flake input, so no safix entrypoint reaches any flake input and no `safix.*` declaration is governed by a revision the consumer happens to pin.
Declare, in safix's own words, the entry type and the manifest schema that the resolved set passes through.
Install at both scopes through one program safix ships, tests, and versions.
Preserve, verbatim, every runtime behaviour a check or a piece of prose in this repository already asserts.
State every capability reduction — templates, `neededForUsers`, gnupg, the user-scope path move — rather than let it be discovered.

**Non-Goals:**

Replacing the `sops` binary with an in-process implementation of the document format.
The decision is already recorded and enforced (`crates/safix-core/src/sops/mod.rs:1-14`, `modules/flake/rust.nix:56-60`, `modules/flake/checks/cli.nix:18-20`), the surface is `decrypt --extract`, `encrypt --filename-override`, `set --value-stdin --idempotent` and `updatekeys` rather than "decrypt", and a divergence in creation-rule interpretation or MAC handling is a silently re-encrypted secret.
This change removes a nix dependency and a Go activation binary; it does not require owning the format.

Removing sops-nix from `flake.lock`.
`clan-core` is a check-only input (`flake.nix:53-57`) carrying its own sops-nix (`flake.lock:237-262`), so the honest claim is that safix declares no sops-nix input.

Templates and `neededForUsers`/`secrets-for-users` relocation, permanently rather than deferred — see I10.

Any change to `secret-custody`, `recipient-policy`, `secret-catalogue`, `public-outputs`, `plaintext-staging` or `secrets-vault`.
Each was checked against this change's touch points: none of them names the installer binary, the entry type, or either module's import list, and the `sopsFile` values the resolver mints (`resolve.nix:2139`, `:2218`) are consumed identically by safix's own type as by the provisioner's, since both are `lib.types.path`.

## Decisions

### I1. safix declares the secret-entry type, and `sopsFileHash` becomes an option rather than a default

`modules/consume/common.nix` gains `secretEntryType`, a `lib.types.submodule` safix declares, and `modules/consume/nixos.nix` passes it as `common.sharedOptions`' `secretsType` argument in place of today's `options.sops.secrets.type`.
Home scope's argument is unchanged: it stays `lib.types.attrsOf lib.types.raw`, because the user-scope manifest is built from the same `materializeFor` output and needs no second typing pass.

The type carries, and carries only, what the manifest needs:

| field | type | default |
|---|---|---|
| `name` | `str` | `config._module.args.name` |
| `key` | `str` | `name` |
| `path` | `str` | `"${cfg.installer.symlinkPath}/${name}"` |
| `mode` | `str` | `"0400"` |
| `owner` | `nullOr str` | `null` |
| `group` | `nullOr str` | `null` |
| `uid` | `int` | `0` |
| `gid` | `int` | `0` |
| `sopsFile` | `path` | none; required |
| `format` | `enum [ "yaml" ]` | `"yaml"` |
| `restartUnits` | `listOf str` | `[ ]` |
| `reloadUnits` | `listOf str` | `[ ]` |

Two consequences are decided here rather than discovered later.

First, the `path` default folds `nixos.nix:138-146`'s path mint into the type.
That mint exists because the installer creates a symlink at every path that is not `<symlinkPath>/<name>` (`sops-nix pkgs/sops-install-secrets/main.go:260-290`), so the store root and the per-entry default must move together or a moved root writes symlinks into another store's directory.
Folding it into the type makes the two structurally inseparable instead of separately maintained, and the `safix-installer-store` check keeps holding the pair against an independent oracle.

Second, sops-nix's type defaults `sopsFileHash` to `builtins.hashFile "sha256" sopsFile` under `validateSopsFiles`, which is what makes editing a ciphertext file change the manifest derivation and so cause a rebuild.
safix's type does not carry that field, because the manifest schema does not carry it either (I2).
The behaviour is not lost: `safix.installer.validate` (I4) governs both the store/existence refusals and a `sopsFileHash`-equivalent — a `builtins.hashFile "sha256"` of each distinct `sopsFile`, concatenated into a single `manifestInputHash` field of the manifest — so a ciphertext edit still changes the derivation, through one field for the whole manifest rather than one per entry.
The installer ignores that field entirely; it exists to make the derivation a function of the ciphertext, and the spec says so.

**Alternative rejected**: keeping `format` a wider enum (`yaml`, `json`, `dotenv`, `ini`, `binary`) matching sops-nix's.
safix's resolver mints only yaml documents (`resolve.nix`'s `audienceFileOf` produces a `.yaml` name, and the policy file's `path_regex` matches `\\.yaml$`, `policy.nix:209`), so a second format is a value no safix declaration can produce and no check can exercise without inventing a document the rest of the tree cannot govern.
A single-member enum is the honest spelling and widens compatibly if a second format is ever resolvable.

**Alternative rejected**: keeping `sopsFileHash` as a per-entry field for parity with the shape sops-nix's manifest has.
Per-entry hashes mean N `hashFile` calls for N entries over a document set that is small by construction and typically shared, and the field's only consumer was nix's own derivation hashing — the Go decoder ignored it. One manifest-level hash is the same guarantee for less evaluation.

### I2. The manifest is safix's own schema, versioned and narrower

One `serde` struct in `crates/safix-core/src/install.rs`, `#[serde(deny_unknown_fields)]`, is the single definition; `modules/consume/installer.nix` and `modules/consume/home.nix` both emit exactly it, and the schema snapshot check (I12) is what holds the nix side to the rust side.

```
{ version: u32,
  secrets: [ { name, key, path, owner: Option<String>, group: Option<String>,
               uid: u32, gid: u32, sopsFile, format, mode,
               restartUnits: [String], reloadUnits: [String] } ],
  secretsMountPoint, symlinkPath, keepGenerations: usize,
  ageKeyFile: Option<String>, ageSshKeyPaths: [String],
  useTmpfs: bool, userMode: bool,
  logging: { keyImport: bool, secretChanges: bool },
  manifestInputHash: Option<String> }
```

`version` is `1` and is validated: a manifest naming a version this binary does not know is refused naming both numbers, which is the whole reason the field exists.
sops-nix's manifest has no version field and relies on Go's decoder silently ignoring unknown keys — which is exactly how safix's over-wide entries work today, and exactly the failure mode a version field turns into a diagnosis.

Dropped relative to sops-nix's schema: `templates` and `placeholderBySecretName`, emitted empty for parity alone (`installer.nix:189-192`, `:206-213`); `gnupgHome` and `sshKeyPaths`, which served an identity safix cannot declare (I10).

**Alternative rejected**: keeping sops-nix's exact schema so that either binary could read either manifest.
Interchangeability is a property nothing needs once safix owns both ends, and it costs four dead fields plus the standing obligation to track a schema safix no longer controls — which is precisely the obligation the `safix-installer-manifest` parity check existed to discharge and which this change is removing.

### I3. `safix install` — the verb, the flags, the two check modes

A `Verb { name: "install", help: usage::INSTALL, run: install_command }` is added to `crates/safix/src/main.rs`'s `VERBS`, placed last, after `upload`.
The position is deliberate: `VERBS`' order is the operator-facing lifecycle order and is the order the unknown-subcommand refusal names, and `install` is the only machine-facing verb in the table — a consumer never types it, an activation script does.
Placing it last states that rather than interleaving it with verbs an operator runs.

Flags:

- `--check-mode=off|manifest|document`, default `off`.
  `manifest` validates the schema, the version, every mode's octal parse, every owner/group resolution, and nothing else; it reads no ciphertext.
  `document` additionally decrypts each distinct `sopsFile` and verifies each declared `key` resolves through the `/`-nested lookup.
  The two tiers mirror sops-nix's `-check-mode=manifest|sopsfile` exactly, so the build-time `checkPhase` keeps its shape and `secret-installation`'s existing "the mode is not fixed to the weaker of the two" scenario stays meaningful.
- `--ignore-passwd`, which skips uid/gid resolution and the `keys` group lookup and forces owner and group to `0`.
  The sandboxed coexistence check needs it, and so does `-check-mode` inside a nix build where no such users exist.
- `--dry-run`, which performs every step except the atomic symlink swap.
  `NIXOS_ACTION=dry-activate` in the environment implies it, so `system.activationScripts.safixInstallSecrets.supportsDryActivation` (`installer.nix:365-367`) keeps its meaning without the activation script having to translate.

`--user` is deliberately not a flag.
`userMode` is a manifest field today and stays one: the manifest is the whole input, and a mode that changes mounting, chowning and restart propagation belongs in the artifact the check mode validates rather than in an argv the check mode never sees.

### I4. Every `sops.*` read, replaced by name

Exhaustive against `modules/consume/installer.nix`'s own header (`:4-9`) and the read surface below it.
Each row is the disposition; nothing in this table is left undecided.

| site | read today | replaced by |
|---|---|---|
| `:41-44` `activationEnvironment` | `sops.environment`, `sops.age.plugins` | `safix.installer.environment` (`attrsOf str`, default `{ }`), `safix.installer.agePlugins` (`listOf package`, default `[ ]`); `HOME=/var/empty` and the `PATH` construction are unchanged |
| `:62-65` `requiredIdentities` | `sops.age.keyFile`, `sops.age.generateKey` | `safix.identity.keyFile`, which safix already declares (`common.nix:390-440`); `generateKey` has no system-scope counterpart (I6) |
| `:67` `hasGnupgSource` | `sops.gnupg.home`, `sops.gnupg.sshKeyPaths` | deleted (I10) |
| `:76-81` `sufficientIdentities` | `sops.age.keyFile`, `sops.age.sshKeyPaths` | `safix.identity.keyFile`, `safix.identity.sshKeyPaths` |
| `:141` the `exec` | `sops.package` → `sops-install-secrets` | `safix.installer.package` (default the flake's own `packages.safix`) → `exec ${cfg.installer.package}/bin/safix install ${manifest}` |
| `:149` the resolved set | `config.safix.installed`, typed by `options.sops.secrets.type` | `config.safix.secrets`, typed by `common.secretEntryType` (I1) |
| `:155-180` the assertion block | gated on `sops.validateSopsFiles` | gated on `safix.installer.validate` (`bool`, default `true`); the two messages, `sopsFileMissingMessage` and `sopsFileOutsideStoreMessage`, stay verbatim, with their provenance comments rewritten to say safix declares them rather than that they are copies |
| `:196` `keepGenerations` | `sops.keepGenerations` | `safix.installer.keepGenerations` (`int`, default `1`) |
| `:197-198` `gnupgHome`, `sshKeyPaths` | `sops.gnupg.*` | deleted from the manifest (I2, I10) |
| `:199-200` `ageKeyFile`, `ageSshKeyPaths` | `sops.age.keyFile`, `sops.age.sshKeyPaths` | `safix.identity.keyFile`, `safix.identity.sshKeyPaths` |
| `:201` `useTmpfs` | `sops.useTmpfs` | `safix.installer.useTmpfs` (`bool`, default `false`) |
| `:208` `placeholderBySecretName` | `sops.placeholder` | deleted from the manifest (I2) |
| `:212-213` `logging` | `sops.log` | `safix.installer.log` (`listOf (enum [ "keyImport" "secretChanges" ])`, default `[ ]`); the two `builtins.elem` reductions are unchanged |
| `:226-229` `checkPhase` | `sops.validationPackage` → `-check-mode=${if validateSopsFiles then "sopsfile" else "manifest"}` | `safix.installer.validationPackage` → `--check-mode=${if cfg.installer.validate then "document" else "manifest"}` (I5) |
| `:343-398` registration | reads `sops.environment`, `sops.age.plugins`, `sops.gnupg.*`, `sops.age.*` for `RequiresMountsFor` | reads the `safix.installer.*` and `safix.identity.*` equivalents; `SOPS_RESTART_UNITS_VIA_SYSTEMCTL = "1"`, `wantedBy sysinit.target`, the `sysinit-reactivation.target` edges and `DefaultDependencies=no` are unchanged |

Three options sops-nix declares are deliberately not reproduced, each for a reason already established in this tree rather than a new one.
`sops.defaultSopsFile` is never read today — every resolved entry carries an explicit `sopsFile` — so there is nothing to default.
`sops.templates` is never read (I10).
`sops.useSystemdActivation` was already refused as an input: `safix.installer.useSystemdActivation` computes its own default from `systemd.sysusers.enable` and `services.userborn.enable` (`installer.nix:244-283`), which is a fact about the host rather than a switch belonging to an installer safix no longer uses.

`modules/consume/nixos.nix` loses its whole `sops = { … }` definition block (`:139-152`) and `modules/consume/home.nix` loses its three (`:232`, `:236`, `:240`).
`derivedHostKeys` (`nixos.nix:39-55`) is unchanged in behaviour and keeps its exclusion prefix of `cfg.installer.symlinkPath`; only the comment naming it a re-implementation of the provisioner's rule is rewritten to state it as safix's own rule, with the reason — avoiding decryption with a key this installer itself deploys — preserved, since that reason is what `secret-installation`'s "A host key another store deployed is usable" scenario rests on.

### I5. Cross-compilation: `package` and `validationPackage` are two options

sops-nix splits these because the `checkPhase` binary must run on the *build* platform while the activation binary must run on the *host* platform.
safix reproduces the split rather than collapsing it: `safix.installer.package` defaults to the host-platform `packages.safix`, `safix.installer.validationPackage` defaults to the build-platform one, and `installer.nix`'s `checkPhase` uses the latter.
Collapsing them would make `nix flake check` fail on any cross build, which is a failure mode the current tree does not have and this change must not introduce.

**Alternative rejected**: `pkgs.pkgsBuildBuild.safix` inline at the `checkPhase`, with no option.
An inline expression cannot be overridden by a consumer who cross-builds through a non-default path, and the current surface already exposes both as options — collapsing an existing pair of options into one inline expression is a narrowing this change has no reason to make.

### I6. The user scope installs, and what it does not do

`modules/consume/home.nix` gains the same manifest-and-invocation shape `installer.nix` has, with `userMode = true`.

Roots: `secretsMountPoint = "%r/safix.d"`, `symlinkPath = "%r/safix"`, each exposed as `safix.installer.secretsMountPoint`/`symlinkPath` at home scope exactly as at system scope, so a person who needs a different root has the same option they have at system scope.
`%r` is expanded by the installer, not by nix: `$XDG_RUNTIME_DIR` on linux, and the output of `getconf DARWIN_USER_TEMP_DIR` on darwin.
safix supports `aarch64-darwin` (`flake.nix:79-83`), so the darwin arm is not optional, and `%%` splits to a literal `%` in both, matching the substitution the provisioner performs.

Registration: `systemd.user.services.safix` on linux, and a `home.activation.safixInstall` entry on both platforms.
The activation entry is registered with `lib.hm.dag.entryAfter [ "writeBoundary" ]` rather than as a bare string, because a bare string becomes `entryAnywhere`, which is what the provisioner's home-manager module does (`modules/home-manager/sops.nix:408-409`) and which gives home-manager no ordering to reason about.
`home.activation.safixIdentityPreflight` (`:246-249`) keeps its `entryBefore [ "checkLinkTargets" ]` placement and its gate moves from `sopsCfg.secrets != { }` to `cfg.secrets != { }`.

Three things a user-mode install does not do, each because the behaviour being replaced does not do them either: no ramfs or tmpfs mount, no chown of any directory or file, and no restart or reload propagation.
All three are `!userMode` branches in the program (mount at `linux.go:22-67`, chown inside `writeSecrets`, propagation skipped wholesale at `main.go:1454`), and reproducing them at user scope would mean a user-scope install asking for privileges the scope does not have.

`sops.age.generateKey`'s equivalent is declared rather than inherited: `safix.identity.generateKey` (`bool`, default `false`) at home scope, which runs `age-keygen -o <keyFile>` when the key file is absent, through the `SAFIX_AGE_KEYGEN` override (I7).
The default is `false` where the provisioner's is also `false`, so no existing profile's behaviour changes by adoption, and the option exists because `keygen` — safix's own verb for the same act — is an operator-run command, not an activation step, and a person whose profile has never been activated has no other way to get a key file in place before the first install runs.

### I7. Identity assembly, and the two new environment overrides

Before invoking `sops`, the installer writes `<secretsMountPoint>/age-keys.txt` at mode `0600` and exports `SOPS_AGE_KEY_FILE` pointing at it.
Into that file it appends, in order: the age form of each configured ssh key, then the contents of the configured age key file.

The asymmetry between the two sources is preserved exactly, because two option descriptions already rest on it (`common.nix:402-406`, `:422-426`).
An unreadable age key file is fatal, naming the path.
An ssh key that is absent or that does not convert prints one line to stderr and is skipped, so a host with three ssh keys of which one is unconvertible still decrypts.

ssh-to-age conversion is a subprocess: `ssh-to-age`, from the same upstream sops-nix links as a Go library, behind a new `SAFIX_SSH_TO_AGE` override.
This is the one genuinely cryptography-adjacent component the change adds, and a subprocess is what `modules/flake/rust.nix:56-60` states as policy — "the runtime drives sops, git and nix as subprocesses and links none of them, which is what keeps the cryptographic surface an upstream one".
Doing it in process means an ed25519-to-X25519 birational map plus an OpenSSH private-key parser, which is a crypto dependency class this crate does not have (`crates/safix-core/Cargo.toml:13-46` carries no crypto crate at all).

`age-keygen` gains `SAFIX_AGE_KEYGEN`, replacing today's hardcoded `Command::new("age-keygen")` (`crates/safix-core/src/keygen.rs:226`).
It is the only external tool in the runtime with no override, beside `nix-instantiate` (`nix.rs:405`), and a hermetic check of `safix.identity.generateKey` cannot exist without one.

**Alternative rejected**: converting ssh keys in process with an age crate.
It adds the first crypto dependency to a crate whose stated design is to have none, and it duplicates a conversion whose upstream is the same project whose Go implementation safix is replacing — so a divergence would be a silently wrong identity, the same class of failure that keeps `sops` a subprocess.

### I8. Decryption: one subprocess per document, not per entry

`sops decrypt --output-type json <file>` once per distinct `sopsFile`, into a `Secret`, then `/`-nested key extraction in memory mirroring the recursion sops-nix performs.
Per-entry decryption would multiply subprocesses by entry count where the provisioner decrypts once per file and extracts in memory; the document set is small by construction, since one audience gets one file.

Every value stays inside `crates/safix-core/src/secret.rs`'s zeroizing type and reaches a file only through a write, never argv and never the environment, which is what the existing `safix-value-pipe` claim (`modules/flake/checks/single-runtime.nix:59`) measures and what `rust-runtime`'s "No plaintext value reaches a child process except through a pipe" requires.
The `sops` binary is referenced by store path from the activation script, never taken from `PATH`; age plugins stay on `PATH` through `safix.installer.agePlugins`, which is the one thing that must be on it.

### I9. The filesystem sequence, and the four behaviours preserved verbatim

In order, matching the program being replaced step for step:

1. Expand `%r` in `secretsMountPoint`, `symlinkPath` and every secret `path` when `userMode`.
2. Validate the manifest (version, modes as octal, owner/group resolution with numeric `uid`/`gid` fallback, `format`), and under `--check-mode=document` load and parse each document and resolve each key.
3. Resolve the `keys` group's gid, falling back to `nogroup`, and to `0` under `--ignore-passwd`.
4. Mount: `mkdir -p` the mount point at `0751`; a no-op under `userMode`; otherwise mount `ramfs`, or `tmpfs` with `noswap` when `useTmpfs` with an `EINVAL` fallback that drops `noswap`, with `MS_NODEV|MS_NOSUID|MS_NOEXEC` and `mode=0751`, only when not already mounted with that filesystem's magic; then chown `0:keys`.
5. Assemble the identity file (I7).
6. Decrypt (I8).
7. Generation directory: read the symlink at `symlinkPath`, parse its basename as an unsigned integer, increment, `mkdir` the new one at `0751`, chown `0:keys` outside user mode.
8. Write each secret: parent directories at `0751` chowned `0:keys`, the file at its declared mode, chowned `owner:group` outside user mode.
9. Diff against the previous generation and collect `restartUnits`/`reloadUnits` for new and changed entries; skipped wholesale under `userMode`.
10. Atomically swap `symlinkPath` to the new generation directory.
11. Symlink every entry whose `path` differs from `<symlinkPath>/<name>`, creating parent directories and replacing whatever is there until the link matches.
12. Prune to `keepGenerations`.

Four behaviours are preserved verbatim, because a check or a piece of published prose already asserts each:

A non-symlink at `symlinkPath` is removed.
This is the destructive branch `safix-installer-coexistence` measures and the one `secret-installation`'s "The destructive branch is real and is what is being avoided" scenario names; it must stay real, or the coexistence claim becomes a claim about a hazard that no longer exists.

The restart/reload diff returns early when `symlinkPath` does not exist.
That is the stage-2-init case, where there is no previous generation to compare against and nothing to restart.

`SOPS_RESTART_UNITS_VIA_SYSTEMCTL=1` selects `systemctl --no-block restart/reload`; without it, the unit names are written to `/run/nixos/activation-restart-list` and `-reload-list` for `switch-to-configuration`.
The unit sets the variable (`installer.nix:377-379`) and the activation script does not, which is the whole mechanism by which the two registration paths propagate differently.

Dry-activate skips step 10 and only step 10.
Everything before it is performed, which is what makes a dry activation informative rather than a no-op.

### I10. gnupg is dropped; templates and `neededForUsers` are stated unsupported

safix has never been able to declare a gnupg identity: `safix.identity` carries `keyFile` and `sshKeyPaths` and nothing else (`common.nix:390-440`).
What the tree does today is *tolerate* a consumer's gnupg configuration as evidence that something else can decrypt — `hasGnupgSource` at `home.nix:59-62`, `installer.nix:67`, `nixos.nix:93` — which suppresses safix's own no-identity refusal on the strength of a configuration safix neither wrote nor can use.
That is dropped: identity sufficiency is `safix.identity.keyFile` or `safix.identity.sshKeyPaths`, and nothing else counts.
Keeping it would mean reimplementing the temporary-GNUPGHOME construction and the RSA-ssh-to-gpg conversion for a path no safix declaration can produce.

Templates were never read; the manifest emitted `templates = [ ]` for schema parity alone.
`neededForUsers`/`secrets-for-users` was never set; its single occurrence in the tree is a fixture inside the check that measures the provisioner's own relocation mechanism (`modules/flake/checks/installer.nix:231-234`, reading `system.build.sops-nix-users-manifest` at `:239`).

What changes for a consumer is not that safix loses a feature but that the workaround closes: today a consumer wanting a template or a user-relocated secret could write `sops.*` alongside their `safix.*` declarations and get it for a hand-written entry.
After this change the two installers still coexist, but a safix-resolved entry can never feed a template.
Stated in `secret-installation`'s spec and in the changelog, not inferred from an absent field.

### I11. One module per scope

`flake.nix` publishes `homeModules.safix` and `homeModules.default` as the same value, `nixosModules.safix` and `nixosModules.default` as the same value, and `homeManagerModules` as an alias of `homeModules`.
Both names stay so that every existing `imports` line keeps resolving; this is a collapse, not a rename, and there is no alias layer, no `visible = false` shim, and no deprecation path.

The `.default`/`.safix` split existed for exactly one reason, recorded at `flake.nix:126-131`: `imports` cannot depend on an option, so a tree without the provisioner needed a form that imported it and a tree pinning its own needed a form that did not.
With no provisioner to import, the distinction has no content: both forms import nothing outside their own file, which is strictly the stronger of the two properties the split used to offer separately.

Two consequences follow and are handled in I12: `safix-module-collision`'s subject — that importing two distinct copies of one declaring module is an evaluation error — must be re-established over safix's own declaring module, and `entrypoints.nix`'s `importAsymmetry`, whose four expectations include two `true`s asserting that the `.default` forms name a sops-nix path, becomes an assertion that no published entrypoint names any flake input.

### I12. The check suite, check by check

Exhaustive over every site referencing `inputs.sops-nix` and every check transitively dependent on the sops option tree.

**`modules/flake/checks/installer.nix`.**
`manifestFor` (`:195-211`), the callPackage of the provisioner's `manifest-for.nix`, and `parityManifest` (`:394-398`) are deleted.
`safix-installer-mechanism` (`:953`) and `safix-installer-manifest` (`:1008`) are deleted and replaced, not ported: the first measures the provisioner's `extraJson` merge and its `secrets-for-users` relocation, the second diffs safix's field set against the provisioner's builder's (`:1037-1046`), and neither subject exists once there is one builder.
Their replacements are two checks of strictly stronger evidence, since safix now owns both ends:

- `safix-installer-schema`, an accepted snapshot of the built manifest's full structure — every key, every type, every default — over the existing manifest fixture, so a nix-side field added, removed or renamed without the `serde` struct moving is a failing check.
- `safix-installer-roundtrip`, which runs `safix install --check-mode=document --ignore-passwd` over the built manifest and asserts it is accepted, then over each of a set of single-field mutations (unknown version, non-octal mode, a `key` absent from the document, an unknown top-level field) and asserts each is refused naming the field.

`providerFixture` (`:226-239`), the `neededForUsers` fixture and its read of `system.build.sops-nix-users-manifest`, is deleted with the mechanism check it served.
`:245`'s standalone evaluation of the provisioner's home-manager module is deleted.
`safix-installer-coexistence` (`:1166`) ports by swapping `inputs.sops-nix.packages.${system}.sops-install-secrets` (`:1173`) for the flake's own `packages.safix`; its subject — the destructive branch, measured against a real binary in a sandbox — is unchanged and is the check that makes I9's first preserved behaviour load-bearing.
`safix-installer-store` (`:1051`), `-ordering` (`:1102`), `-sole` (`:1108`), `-refusals` (`:1114`) and `-identity` (`:1123`) keep their subjects; each evaluates `config.flake.nixosModules.default` (`:374`, `:408`, `:494`, `:525`, `:646`, `:729`, `:756`) and so simply stops needing the sops option tree.
`safix-installer-store`'s `entryPathContract` continues to derive its expectation from an independent oracle rather than from the mint under test, which is what `collapse-installed-into-secrets` established and what this change must not undo when the mint moves into the type (I1).

**`modules/flake/checks/consumption.nix`.**
`sopsHomeCopy` (`:112`) is deleted.
`handForm` (`:187`) is re-pointed: the equivalence claim (`:575`) becomes "the module form equals the resolver output read back through safix's own entry type" rather than "module form equals the sops-nix wiring", which is what the hand form was by definition.
`viewOf` (`:204-207`) reads entry fields back through `common.secretEntryType` instead of the provisioner's types.
The refusal fixture at `:316` imports safix's own module without home-manager's assertion wrapper, unchanged in intent.
`safix-module-collision` (`:517`, fixtures at `:547-548`, `:557-558`) is re-established over two distinct copies of `modules/consume/nixos.nix` — reached by two paths that are not the same store path — since that is the fact `consumer-integration` still needs held and the reason it cannot be repaired by configuration is unchanged.
The literal expectations at `:627-631` (`format = "yaml"` and its neighbours) become expectations about safix's own declared defaults, which happen to be the same values; the check gains a comment saying so, because a literal that silently agrees with a deleted dependency's default is the kind of expectation that goes vacuous without anyone noticing.

**`modules/flake/checks/entrypoints.nix`.**
`namesSopsNix` (`:118`) becomes `namesAnyFlakeInput`, a predicate over every declared input rather than over one.
`importAsymmetry` (`:159-164`) reduces from four expectations, two of them `true`, to a single symmetric one: no published entrypoint — `.safix` or `.default`, home or nixos — names any flake input.
The anti-vacuity probe (`:166`, reasoned at `:58-71`) is rebuilt on the new predicate: it must still be the case that the probe fires on a deliberately-input-naming fixture, or the check asserts nothing.

**`modules/flake/checks/materialization.nix`.**
The three `provisioner` evaluations (`:86-88`) are replaced by evaluations of safix's own modules.
`axisOf` (`:90`), which reads the ownership axis off `options.sops.secrets.type.getSubOptions [ ]`, reads it off `common.secretEntryType`'s own sub-options instead, and the ownership-asymmetry assertion (`:141-146`) is unchanged in what it claims.
The check's premise changes from "hand safix's output to the real provisioner and read it back" (`:8-12`) to "hand safix's output to safix's own declared type and read it back"; the header prose is rewritten to say so, because leaving the old premise in place would describe a round trip through a component that is gone.

**`modules/flake/checks/portability.nix`.**
No `inputs.sops-nix` reference, but `:280` and `:317` evaluate the `.default` forms, which now import nothing.
Three sets of expected literals encode sops-nix defaults and each is re-based: `format = "yaml"` at `:588`, `:598`, `:608`, `:628`, `:782`, `:790`, `:798`, `:811` becomes safix's own default of the same value; `:257-264`'s exclusion of `path`, `owner`, `group` and `sopsFileHash` from `decided` — justified there by their being the provisioner's defaults — is rewritten with `sopsFileHash` dropped entirely (I1) and the other three justified as safix's own defaults; and `:819-820`'s `"/home/alice/.config/sops-nix/secrets/nginx/service-token"` becomes the `%r/safix/…` path the new user-scope default produces.
`systemIdentity` (`:500-506`), which asserts the provisioner's host-key default stands where safix names no identity, is re-pointed at `derivedHostKeys`, which is where that default now comes from.

**Unaffected, verified by grep rather than assumed.**
`exported`, `namespace`, `examples`, `cli`, `single-runtime`, `policy`, `bridge`, `bridge-sync`, `custody`, `envelope`, `vault`, `vault-projection`, `subjects`, `generators`, `gate-guard`, `keepassxc`, `real-clan`, `fixture-fleet`, `integration` and the `collision-fixture` directory name no sops-nix reference.
`cli.nix` and `single-runtime.nix` drive the `sops` binary and `pkgs.age`, not sops-nix, and `cli.nix:18-20` explicitly forbids stubbing sops — a prohibition this change leaves in force.
`modules/flake/devshell.nix:10` keeps shipping `pkgs.sops`.

**New: a VM test.**
Every installer check today is either a structural evaluation or a sandboxed invocation of the binary, and neither measures a host that boots and reaches a secret.
`modules/flake/checks/installer-vm.nix` adds a `pkgs.testers.runNixOSTest` (linux systems only) that imports `nixosModules.safix`, resolves one shared and one private entry, provides an age key file through the test machine's own configuration, boots, and asserts: the symlink at `symlinkPath` points at generation `1`; each entry's file exists at its declared mode and ownership; a second activation produces generation `2` and prunes to `keepGenerations`; and a unit named in `restartUnits` is observed restarted.
This is the check that makes the "loss of upstream review on a privileged path" risk (I14, risk 2) something the tree measures rather than something the prose concedes.

**New: an integration claim.**
`crates/safix/tests/install.rs` drives `safix install` through the existing test harness, and is registered as a whole-target claim in `modules/flake/checks/single-runtime.nix` under `safix-install`, the way every other integration target there is.

### I13. The migration is a rename table

A consumer's whole migration is: change nothing about `.sops.yaml`, the file layout, `safix fix`, or any custody declaration; rename any tuned option; and, at user scope, update anything that referenced the old secret path.

| was | becomes |
|---|---|
| `sops.package` | `safix.installer.package` |
| `sops.validationPackage` | `safix.installer.validationPackage` |
| `sops.validateSopsFiles` | `safix.installer.validate` |
| `sops.keepGenerations` | `safix.installer.keepGenerations` |
| `sops.useTmpfs` | `safix.installer.useTmpfs` |
| `sops.log` | `safix.installer.log` |
| `sops.environment` | `safix.installer.environment` |
| `sops.age.plugins` | `safix.installer.agePlugins` |
| `sops.age.keyFile` | `safix.identity.keyFile` (already existed; safix stops defining the `sops` one from it) |
| `sops.age.sshKeyPaths` | `safix.identity.sshKeyPaths` (already existed; likewise) |
| `sops.age.generateKey` | `safix.identity.generateKey`, home scope only |
| `sops.gnupg.home`, `sops.gnupg.sshKeyPaths`, `sops.gnupg.qubes-split-gpg.enable` | no equivalent; gnupg is not an identity safix accepts (I10) |
| `sops.defaultSopsFile`, `sops.templates`, `sops.placeholder`, `sops.useSystemdActivation` | never read by safix; a consumer setting them was configuring their own sops-nix, which is unaffected |
| `imports = [ safix.nixosModules.default ]` | unchanged; `.default` and `.safix` are now one value |
| `~/.config/sops-nix/secrets/<name>` | `$XDG_RUNTIME_DIR/safix/<name>` on linux, `$(getconf DARWIN_USER_TEMP_DIR)/safix/<name>` on darwin |

The whole table is `safix.*` on the right, which is the point: a consumer who also runs sops-nix for their own secrets keeps every `sops.*` value they set, and it now means only what they meant by it.

### I14. Risks, each with its disposition

1. **The user-scope path moves.**
   `~/.config/sops-nix/secrets/<name>` becomes `%r/safix/<name>`, a runtime directory, so anything referencing the old path breaks and a person's secrets no longer survive a reboot without a login.
   The provisioner's own home-scope default is itself runtime-dir based (`%r/secrets`), so this is a change of owner and name rather than a change of kind; the `.config` path in `portability.nix:819` is what that fixture's configuration produced, not a default.
   Disposition: stated in the rename table, in the changelog as **BREAKING**, and held by `safix-portability-home`'s re-based literal.
   The migration note must be verified against a real profile before it is published, not derived from the fixture.
2. **Loss of upstream review on a privileged path.**
   `sops-install-secrets` is widely deployed; a fresh reimplementation has no such exposure.
   Disposition: the coexistence check against the real binary in a sandbox, the VM test exercising a real activation, and I9's step-for-step ordering — the mitigation is that the sequence is a transcription rather than a redesign, and that the four behaviours checks already assert stay asserted.
3. **Templates and `neededForUsers` become permanently unavailable for a safix entry.**
   Disposition: I10, stated in the spec and the changelog rather than left to be discovered.
4. **gnupg removal is a capability reduction.**
   For a path no safix declaration can produce, so the real risk is low; the observable change is that a consumer's gnupg configuration stops suppressing safix's own no-identity refusal.
   Disposition: I10, and a scenario in `secret-installation` stating that identity sufficiency counts only safix's own two sources.
5. **Cross-compilation of the `checkPhase`.**
   Disposition: I5 reproduces the `package`/`validationPackage` split rather than collapsing it.
6. **`sops` must now be on the activation path.**
   Today it need not be, because the provisioner links the Go library.
   Disposition: I8 — the activation script references the `sops` store path explicitly and never `PATH`; age plugins stay on `PATH`, which is the one thing that must be.
7. **Per-document decrypt subprocess cost at activation**, where the provisioner did it in process.
   Bounded by the number of distinct audience documents, which is small by construction — one audience, one file.
   Disposition: accepted, and I8 fixes the granularity at per-document rather than per-entry so the bound holds.
8. **`clan-core` still drags sops-nix into the lock.**
   Disposition: the claim is "safix declares no sops-nix input", in the changelog and the README, and a task checks the phrasing against `nix flake metadata`'s actual output.
9. **Spec churn.**
   `secret-installation` needs a full rewrite rather than an amendment, and `secret-consumption` loses a whole requirement.
   Disposition: the delta specs do exactly that, and the rewrite is what the proposal's Capabilities section commits to.

### I15. The privileged filesystem work keeps `#![forbid(unsafe_code)]`

`mount(2)` and reading a mounted filesystem's magic are the first syscalls the runtime makes that the standard library does not expose, and `rust-runtime`'s "Every crate forbids unsafe code" applies to every crate in the workspace with no exception.
`safix-core` already depends on a `statfs` wrapper (`crates/safix-core/Cargo.toml:32-35`), so the mount call is added through the same class of dependency — a crate exposing a safe interface over the syscall — rather than through an `unsafe` block behind a local wrapper.
The forbid declaration stands unweakened, and the spec says so, because a new syscall is exactly the occasion on which such a rule quietly acquires its first exception.

**Alternative rejected**: a small `unsafe` wrapper in `safix-core` with an `#![allow(unsafe_code)]` on one module.
The requirement is stated over crates, not modules, so a module-level allow is a change to the requirement rather than a use of it, and the syscall in question has safe bindings in a crate the workspace is already one dependency away from.

## Migration Plan

For this repository: the change lands as one sequence, because the input cannot be deleted before the type that depends on it is replaced and the checks that reference it are re-pointed.
Task group order in `tasks.md` is that sequence — type, manifest, program, system scope, user scope, entrypoints, checks, documentation — and the input deletion is the last step of the entrypoint group, at which point `nix flake check` is the gate.

For a consumer: the rename table in I13, and nothing else.
No ciphertext is re-encrypted, no path in the encrypted tree moves, and no custody declaration changes.

Rollback is a revert: the input, the type, and the two modules return together, and no on-disk state produced by this change survives a reboot — the secret store is a runtime directory at user scope and a fresh mount at system scope, so there is nothing to undo outside the configuration itself.

## Open Questions

None outstanding.
The three the inquiry left open are settled above: gnupg is dropped (I10), `sopsFileHash` is replaced by a single manifest-level hash governed by `safix.installer.validate` (I1), and `generateKey` becomes `safix.identity.generateKey` at home scope with a `false` default (I6).
