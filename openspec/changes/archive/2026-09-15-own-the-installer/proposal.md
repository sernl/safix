# safix owns its own installer and declares no sops-nix input

## Dependency

This change lands after `collapse-installed-into-secrets` and assumes that change applied.
That is, there is exactly one resolved-set option per scope — `safix.secrets`, read-only, typed per scope through `common.sharedOptions`' `secretsType` argument — and `safix.installed` no longer exists anywhere in the tree.
This change's central edit is what that argument receives at system scope: today's `options.sops.secrets.type`, the provisioner's own option declaration read out of the same evaluation, becomes a submodule safix declares itself.
Nothing here depends on `configurable-storage-roots` or `add-view-picker`, and the three are independent of each other.

## Why

safix builds its own installer manifest and registers its own activation step, but the binary that reads that manifest, the type that validates every entry in it, and the defaults that fill it in all belong to sops-nix.
The coupling is not a matter of convenience: `modules/consume/common.nix:210` types safix's resolved set as `options.sops.secrets.type`, which makes the provisioner's module a required import even for `nixosModules.safix`, the entrypoint whose whole contract is that it imports nothing (`openspec/specs/consumer-integration/spec.md:152-155`).
Twelve further options are read out of the `sops.*` namespace rather than declared — `package`, `validationPackage`, `validateSopsFiles`, `log`, `keepGenerations`, `useTmpfs`, `placeholder`, `environment`, `gnupg.home`, `gnupg.sshKeyPaths`, `age.plugins`, `age.keyFile`, `age.sshKeyPaths` — and at user scope safix does not install at all: it contributes a preflight and hands the whole installation to sops-nix's home-manager module.
So a consumer's `safix.*` declarations are silently governed by whichever sops-nix revision their tree happens to pin, safix writes `sops.secrets`, `sops.age.keyFile` and `sops.age.sshKeyPaths` into that tree at normal priority where a consumer's own sops-nix use conflicts loudly (`README.md:839`), and `consumer-integration`'s "safix reads no option outside its own namespace" requirement is honoured everywhere except the one place it matters most.

Owning the installer closes all of that at once: one module per scope instead of a `.default`/`.safix` split whose only reason to exist is whether the consumer already pins the provisioner, one namespace, one binary that safix ships and tests, and a user scope that installs rather than delegates.

## What Changes

- Delete the `sops-nix` flake input (`flake.nix:32-33`), its top-level destructuring (`:61`), and both `.default` imports (`:72`, `:140`).
  `sops` the binary stays exactly where it is: the document format, its MAC, its IV-reuse rule and its key wrapping remain upstream's, driven as a subprocess, per `rust-runtime`'s "The cryptographic backend stays the authority".
- **BREAKING** (nix surface): safix declares its own secret-entry submodule type, replacing `options.sops.secrets.type` as the `secretsType` argument `modules/consume/nixos.nix` passes to `common.sharedOptions`.
  It carries `name`, `key`, `path` (defaulting to `<symlinkPath>/<name>`, which folds `nixos.nix:138-146`'s path mint into the type), `mode` (default `"0400"`), `owner`/`group` (`nullOr str`), `uid`/`gid` (int, default `0`), `sopsFile` (path), `format` (enum, default `"yaml"`), and `restartUnits`/`reloadUnits` (`listOf str`, default `[]`).
  It deliberately drops `sopsFileHash`, and the ciphertext-edit-causes-a-rebuild behaviour that default carried is replaced by a named, documented option rather than lost silently.
- **BREAKING** (nix surface): a versioned manifest schema of safix's own, narrower than sops-nix's, defined once as a `serde` struct in `safix-core` and emitted by `modules/consume/installer.nix`.
  It drops `templates`, `placeholderBySecretName`, `gnupgHome` and `sshKeyPaths` — the first two were emitted empty for schema parity alone (`installer.nix:189-192`, `:206-213`), the last two served a gnupg identity safix has never been able to declare — and adds a `version` field so a future migration is diagnosable rather than a silent field-drop.
- **BREAKING** (nix surface): new `safix.installer.{package,validationPackage,keepGenerations,useTmpfs,log,environment,agePlugins,validate}` options replace every `sops.*` read in `installer.nix`.
  The migration note is a rename table; the new namespace is a superset of what was read, so no configuration becomes inexpressible.
- New verb `safix install <manifest> [--check-mode=off|manifest|document] [--ignore-passwd] [--dry-run]`, which also honours `NIXOS_ACTION=dry-activate`.
  `--check-mode=manifest` validates the schema without reading any ciphertext; `--check-mode=document` additionally decrypts each named document and verifies each declared key resolves.
  `userMode` stays a manifest field rather than a flag, as it is today.
- **BREAKING** (nix surface): user scope stops delegating.
  `modules/consume/home.nix` gains a manifest with `userMode = true`, `secretsMountPoint = "%r/safix.d"` and `symlinkPath = "%r/safix"`, `%r` expanded against `$XDG_RUNTIME_DIR` on linux and `getconf DARWIN_USER_TEMP_DIR` on darwin, a `systemd.user` service on linux, and a `home.activation` entry on both.
  A user-scope install mounts nothing, chowns nothing, and propagates no restart or reload, matching every `!userMode` branch of the behaviour it replaces.
  A person's secrets move from `~/.config/sops-nix/secrets/<name>` to `%r/safix/<name>`, a runtime directory rather than a home directory.
- **BREAKING** (nix surface): the `.default`/`.safix` module split collapses to one module per scope.
  `homeModules.default` and `homeModules.safix` become the same value, as do `nixosModules.default` and `nixosModules.safix`; both names stay published so existing `imports` lines keep resolving, and `homeManagerModules` stays an alias of `homeModules`.
  Every published entrypoint now imports nothing outside its own file.
- safix stops writing `sops.secrets`, `sops.age.keyFile` and `sops.age.sshKeyPaths` into a consumer's tree.
  A consumer who uses sops-nix for their own non-safix secrets keeps it, now genuinely independent of safix, and the normal-priority conflict hazard `README.md:839` documents is gone.
- Identity assembly moves into the installer: it writes `<secretsMountPoint>/age-keys.txt` at mode `0600`, appends the contents of the configured age key file (unreadable is fatal, naming the path), appends the age form of each configured ssh key (absent or unconvertible prints one line to stderr and is skipped), and exports `SOPS_AGE_KEY_FILE`.
  ssh-to-age conversion is a subprocess behind a new `SAFIX_SSH_TO_AGE` override, consistent with every other external tool the runtime drives.
- `age-keygen` gains a `SAFIX_AGE_KEYGEN` override, the one tool in the runtime spawned today by a hardcoded program name (`crates/safix-core/src/keygen.rs:226`).
- Templates, `neededForUsers`/`secrets-for-users` relocation, and any gnupg identity are stated as unsupported in the spec and the changelog rather than left to be inferred from a missing manifest field.
  safix has never supported any of the three; what changes is that a consumer can no longer reach for `sops.*` alongside to get them for a safix-resolved entry.
- `safix-installer-mechanism` and `safix-installer-manifest` are replaced rather than ported: their subject is parity against a second builder, which no longer exists.
  In their place, a manifest schema snapshot and a round-trip check that `safix install --check-mode=…` accepts the built manifest and rejects a mutated one.
  `safix-installer-coexistence` ports by swapping the package; `safix-materialization`, `safix-consumption*`, `safix-module-entrypoints` and `safix-portability-*` are re-pointed at safix's own type; `safix-module-collision` loses its subject and is replaced by a collision check over safix's own declaring module.
- A NixOS VM test exercising a real activation is added, because every installer check that exists today is either a structural evaluation or a sandboxed binary invocation, and the one thing neither measures is a boot that reaches a secret.

Not in scope: replacing the `sops` binary with an in-process implementation of the document format.
That decision is already recorded and enforced in this repository (`crates/safix-core/src/sops/mod.rs:1-14`, `modules/flake/rust.nix:56-60`, `modules/flake/checks/cli.nix:18-20`), the surface is not "decrypt" but `decrypt --extract`, `encrypt --filename-override` honouring creation rules, `set --value-stdin --idempotent` reproducing sops's own MAC and IV-reuse semantics byte for byte, and `updatekeys`; any divergence there is a silently re-encrypted secret.
Owning the installer removes a nix input and a Go activation binary and does not require owning the format.

Also not in scope: removing sops-nix from `flake.lock`.
`clan-core`, a check-only input, carries its own copy (`flake.lock:237-262`), so the claim this change may make is that safix declares no sops-nix input, never that sops-nix is gone.

Also not in scope, permanently rather than deferred: templates and `neededForUsers`.
Both are named above as capability reductions to be stated, not as work deferred to a later change.

## Capabilities

### New Capabilities

None.
Every requirement this change introduces belongs to a capability that already exists.

### Modified Capabilities

- `secret-installation`: rewritten end to end.
  Every requirement in the file is phrased around "the secret provisioner's installer binary", "the provisioner's own secret type" and "the provisioner's own manifest builder"; each becomes a statement about safix's own binary, type and manifest.
  "safix installs its resolved set itself at system scope" is retired rather than amended, because two of its three scenarios assert that the resolved set is typed by the provisioner's own option declaration, and a requirement naming whose program installs replaces it.
  The user scope — which this capability has never covered, because safix did not install there — gains its own requirements for the user-mode manifest, the two runtime-directory expansions, and the three behaviours user mode omits, alongside new requirements for the versioned schema, the verb, the three named absences, and a real activation on a booted host.
- `secret-consumption`: loses the requirement "Each consumption module ships in a form that imports the provisioner and a form that does not" and its three scenarios, and gains a requirement that one module per scope is published under both names.
- `consumer-integration`: "Module entrypoints follow the secrets provisioner's own naming and import without a flake" is retired and replaced by "Module entrypoints keep both published names and import nothing", because two of its scenarios justify the doubled naming by the provisioner's own flake and assert an import asymmetry that no longer has a second side.
  "safix reads no option outside its own namespace" gains a scenario for the installer, which is where it was previously untrue, and the scope-parity requirement's ownership-axis refusal is re-based on safix's own type.
- `safix-cli`: the lifecycle requirement gains `install` and its three flags, the retired-and-reserved-verbs requirement records that `install` is a machine-facing verb rather than an operator-facing one, and the entry-file requirement's count moves from fourteen of fifteen to fifteen of sixteen, with `install` unaffected because it evaluates no nix.
- `rust-runtime`: the cryptographic-authority requirement gains the installer's decrypt path — one `sops decrypt --output-type json` per distinct document, values through `Secret`, never argv or the environment — the unsafe-code requirement gains the mount syscall and states that it goes through a safe wrapper rather than an exception, and two requirements are added: the manifest is one schema definition in the library, and every external program the runtime invokes is selectable by a named variable, which `ssh-to-age` and `age-keygen` bring to completion.

## Impact

Affected nix:

- `flake.nix` — the input, its destructuring, both `.default` imports, and the comments justifying all three.
- `modules/consume/common.nix` — `secretsType`'s system-scope argument, `sopsFileMissingMessage`/`sopsFileOutsideStoreMessage`'s provenance comments, the key-source prose at `:113-131`, and the `identity.keyFile`/`identity.sshKeyPaths` descriptions citing `sops-install-secrets`.
- `modules/consume/installer.nix` — the full option-read surface, the binary invocation, the `checkPhase`, the manifest JSON, and the assertion block copied from the provisioner's builder.
- `modules/consume/nixos.nix` — `derivedHostKeys`' provenance, the `sops` definition block, and the path mint that moves into the new type.
- `modules/consume/home.nix` — every `sopsCfg` read, the three `sops.*` definitions, the manifest, the unit, the activation entry, and the preflight's gate.
- `modules/flake/checks/{installer,consumption,entrypoints,materialization,portability}.nix` — every `inputs.sops-nix` reference and every expectation encoding a sops-nix default.
- `modules/flake/checks/single-runtime.nix` — a new whole-target claim for the install integration suite.
- A new VM test, and its registration among the flake's checks.

Affected rust:

- `crates/safix-core/src/install.rs` — new: the manifest schema, validation, identity assembly, decryption, and the filesystem sequence.
- `crates/safix-core/src/keygen.rs` — `SAFIX_AGE_KEYGEN`.
- `crates/safix/src/main.rs` — the verb table.
- `crates/safix/src/usage.rs` — the `install` help text, and the `upload` entry's claim that delivery happens "through sops-nix reading the committed file".
- `crates/safix/src/reporter.rs` and `crates/safix-core/src/error/`— the new refusals, their codes, and their snapshots.
- `crates/safix/tests/read_path.rs` — the doc comment referencing `sops-install-secrets` and `SOPS_AGE_KEY_FILE`.
- `crates/safix/tests/snapshots/upload__safix_help.snap` and both `unknown_subcommand` snapshots — regenerated from the verb table.

Affected documentation: `README.md:6-7, 56-58, 761, 804-822, 833, 838-846, 854-859, 868-886`; `CHANGELOG.md` under `[Unreleased]`; `docs/notes/research/zero-knowledge-vault.md:9, 21, 92, 98, 108, 132`.

Every guarantee this change states gets a severity drill in `tasks.md`.
