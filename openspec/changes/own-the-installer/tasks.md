# Tasks: own-the-installer

Citations are as read while designing this change; re-read the named lines before editing, since this change lands after `collapse-installed-into-secrets` and line numbers drift.
This change assumes that one applied: one `safix.secrets` option per scope, read-only, typed through `common.sharedOptions`' `secretsType` argument, and no `safix.installed` anywhere.
No real fleet identifier, hostname, or recipient enters this repository; fixtures use `alice`, `bob` and `carol`, and synthetic `age1` strings, matching the existing consumption fixtures.
Where a task says "hold", add a check that fails when the claim stops being true, not a sentence asserting it.

Group order is the landing order, and it is forced: the entry type must exist before the system module can stop reading `options.sops.secrets.type`, the manifest schema must exist before either module can emit it, the program must exist before either module can invoke it, and the `sops-nix` input can only be deleted once no check references it.
The input deletion is task 9.6 and nothing before it can be validated with the input already gone.

## 1. The entry type safix declares

- [ ] 1.1 Add `secretEntryType` to `modules/consume/common.nix`, a `lib.types.submodule` taking the system-scope `cfg` it needs for the `path` default, declaring exactly: `name` (`str`, default `config._module.args.name`), `key` (`str`, default `name`), `path` (`str`, default `"${cfg.installer.symlinkPath}/${name}"`), `mode` (`str`, default `"0400"`), `owner` (`nullOr str`, default `null`), `group` (`nullOr str`, default `null`), `uid` (`int`, default `0`), `gid` (`int`, default `0`), `sopsFile` (`path`, no default), `format` (`enum [ "yaml" ]`, default `"yaml"`), `restartUnits` (`listOf str`, default `[ ]`), `reloadUnits` (`listOf str`, default `[ ]`)
- [ ] 1.2 Write each option's `description` so a newcomer needs no other document: what the field is for, who reads it, and for `path` that its default is what keeps the store root and the per-entry default inseparable
- [ ] 1.3 Change `modules/consume/nixos.nix:60`'s `common.sharedOptions` call to pass `secretsType = common.secretEntryType { inherit cfg; }` in place of `options.sops.secrets.type`; leave `modules/consume/home.nix:154`'s `lib.types.attrsOf lib.types.raw` unchanged
- [ ] 1.4 Delete the `<symlinkPath>/<name>` path mint from `modules/consume/nixos.nix:141-144` (the `lib.mapAttrs` over `cfg.secrets`), which the type's `path` default now performs; the definition site itself is retired, not merely emptied
- [ ] 1.5 Rewrite `modules/consume/common.nix:192-206`'s comment block: it currently records what the provisioner's type carries and what it does not. It becomes a record of what safix's own type carries, why `sopsFileHash` is absent (task 2.4), and that the two store/existence refusals still live in `installer.nix` rather than in the type
- [ ] 1.6 Rewrite `modules/consume/common.nix:184-190`'s `sopsFileMissingMessage` and `sopsFileOutsideStoreMessage` provenance comments: both messages stay byte-identical, and the comment stops calling them copies of `manifest-for.nix:11-28` and states them as safix's own
- [ ] 1.7 Add to `modules/flake/checks/installer.nix` a `typeContract` assertion reading `common.secretEntryType`'s `getSubOptions [ ]` and comparing the option name set and each default against an accepted literal, so a field added, removed or re-defaulted is a failing check
- [ ] 1.8 Add a fixture check that an entry declaring no `path` resolves to `<symlinkPath>/<name>` and one declaring a `path` keeps it, with the expectation derived from `config.flake.safix.lib.materialize …` as an independent oracle rather than from the type's own default
- [ ] 1.9 Severity drill: change the type's `path` default to a literal `/run/secrets/<name>` and confirm 1.8's assertion and `safix-installer-store`'s `entryPathContract` both redden, which is the evidence the store root and the entry default are held as a pair rather than separately
- [ ] 1.10 Verify: `nix build .#checks.x86_64-linux.safix-installer-store` and the check carrying 1.7-1.8 green, drill in 1.9 observed

## 2. The manifest schema in `safix-core`

- [ ] 2.1 Add `crates/safix-core/src/install.rs` with `pub struct Manifest` and `pub struct ManifestSecret`, both `#[serde(deny_unknown_fields)]` and `#[serde(rename_all = "camelCase")]` where the JSON spelling requires it, carrying exactly the fields design I2 lists, and register the module in `crates/safix-core/src/lib.rs`
- [ ] 2.2 Add `pub const MANIFEST_VERSION: u32 = 1;` and a `Manifest::validate_version` that refuses a version it does not know, naming the version read and the version supported
- [ ] 2.3 Add the refusal variants this group needs to `crates/safix-core/src/error/` with `refusal_codes!` entries in `error/code.rs`: `ManifestUnreadable { path, cause }`, `ManifestUnparsable { path, cause }`, `ManifestVersionUnknown { found, supported }`, `ManifestModeUnparsable { name, mode }`, `ManifestOwnerUnknown { name, owner }`, `ManifestGroupUnknown { name, group }`, `ManifestKeyMissing { name, document, key }`
- [ ] 2.4 Emit `manifestInputHash` from nix rather than from rust: in `modules/consume/installer.nix`, when `cfg.installer.validate`, set it to a `builtins.hashString "sha256"` over the concatenated `builtins.hashFile "sha256" sopsFile` of each distinct `sopsFile`, and `null` otherwise. `Manifest` deserializes it into an `Option<String>` the installer never reads
- [ ] 2.5 Add a unit test per refusal in 2.3 over a hand-written JSON literal, plus one asserting the real manifest shape round-trips through `serde_json` unchanged
- [ ] 2.6 Add a reporter sample arm in `crates/safix/src/reporter.rs`'s `sample` for each new `Code`, and accept the two insta snapshots (plain and graphical) each produces
- [ ] 2.7 Severity drill: remove `deny_unknown_fields` from `ManifestSecret` and confirm the unknown-field test in 2.5 turns green when it should refuse, which is the evidence the attribute is what refuses rather than a coincidence of field ordering
- [ ] 2.8 Severity drill: drop the `manifestInputHash` emission in 2.4 and confirm that editing a fixture ciphertext file no longer changes `system.build.safix-manifest`'s derivation path, which is the evidence the field is what makes the derivation a function of the ciphertext
- [ ] 2.9 Verify: `cargo test -p safix-core install::` green, `cargo insta test -p safix` accepted with the new snapshots, drills in 2.7 and 2.8 observed

## 3. `safix install`: the verb, the flags, the check modes

- [ ] 3.1 Add `usage::INSTALL` to `crates/safix/src/usage.rs`, describing the manifest argument, the three check modes, `--ignore-passwd`, `--dry-run`, and stating that this is the verb an activation runs rather than one an operator types
- [ ] 3.2 Add `Verb { name: "install", help: usage::INSTALL, run: install_command }` to `crates/safix/src/main.rs`'s `VERBS`, last, after `upload`, with a comment stating why the position is last rather than interleaved
- [ ] 3.3 Add the `install` line to `usage::SCAFFOLD`'s verb list, in the same position, and rewrite the `upload` entry's closing sentence at `usage.rs:840-843` so that ongoing delivery is attributed to `safix install` during activation rather than to "sops-nix reading the committed file"
- [ ] 3.4 Add `install_command` to `crates/safix/src/main.rs`: parse the manifest path and the three flags, refuse an unknown `--check-mode` value naming the three it accepts, read `NIXOS_ACTION` and treat `dry-activate` as implying `--dry-run`, and call into `install::run`
- [ ] 3.5 Implement `install::run`'s two checking tiers: `CheckMode::Manifest` performs validation (version, mode octal parse, owner/group resolution with numeric fallback, `format`) and returns; `CheckMode::Document` additionally decrypts each distinct document and resolves each declared key; `CheckMode::Off` proceeds to the filesystem sequence
- [ ] 3.6 Implement `--ignore-passwd`: skip every user, group and keys-group lookup and force owner and group to `0`, and apply the same forcing when `NIXOS_ACTION=dry-activate`
- [ ] 3.7 Regenerate `crates/safix/tests/snapshots/upload__safix_help.snap` and both `crates/safix/src/snapshots/safix__reporter__tests__{plain,graphical}-unknown_subcommand.snap`, all three derived from the verb table and the scaffold
- [ ] 3.8 Add `crates/safix/tests/install.rs` driving the verb through the existing harness: a manifest accepted under each check mode, a manifest rejected under `document` but accepted under `manifest` (documents absent), and an unknown `--check-mode` value refused naming the three
- [ ] 3.9 Register `crates/safix/tests/install.rs` as a whole-target claim in `modules/flake/checks/single-runtime.nix` as `safix-install`, with a doc comment naming its drill, the way `safix-bridge-sync-converge` is registered
- [ ] 3.10 Severity drill: make `CheckMode::Manifest` also decrypt, and confirm the "accepted under `manifest`, rejected under `document`" test in 3.8 turns red, which is the evidence the two tiers are distinguished rather than one mode with two spellings
- [ ] 3.11 Severity drill: stop reading `NIXOS_ACTION` and confirm a test asserting a dry activation leaves the store's symlink unmoved turns red, which is the evidence `supportsDryActivation`'s claim is held by the program rather than by the script
- [ ] 3.12 Verify: `cargo test -p safix --test install` green, `nix build .#checks.x86_64-linux.safix-install` green, drills in 3.10 and 3.11 observed

## 4. Identity assembly, `ssh-to-age`, and the two environment overrides

- [ ] 4.1 Replace `Command::new("age-keygen")` at `crates/safix-core/src/keygen.rs:226` with a lookup of `SAFIX_AGE_KEYGEN` falling back to `age-keygen`, following `crates/safix-core/src/sops/mod.rs:68-75`'s shape exactly
- [ ] 4.2 Add `install::ssh_to_age`, a subprocess driver looking up `SAFIX_SSH_TO_AGE` and falling back to `ssh-to-age`, reading one ssh private key path and returning its age form as a `Secret`
- [ ] 4.3 Add `ssh-to-age` to the package's runtime closure wherever `sops` already is, and to `modules/flake/devshell.nix` beside `pkgs.sops`
- [ ] 4.4 Implement `install::assemble_identity`: create `<secretsMountPoint>/age-keys.txt` at mode `0600`, append the age form of each `ageSshKeyPaths` entry, append the contents of `ageKeyFile`, and export `SOPS_AGE_KEY_FILE` for the decrypt step
- [ ] 4.5 Preserve the asymmetry verbatim: an unreadable `ageKeyFile` is a refusal naming the path (new variant `IdentityKeyFileUnreadable { path, cause }`, with a code, a sample arm and two snapshots); an absent or unconvertible ssh key prints one line to stderr and is skipped
- [ ] 4.6 Update `modules/consume/common.nix:402-406` and `:422-426` — the `identity.keyFile` and `identity.sshKeyPaths` descriptions — so each cites safix's own installer for the behaviour it describes rather than `sops-install-secrets`, keeping the asymmetry itself stated
- [ ] 4.7 Add unit tests over a temporary directory: a key file that is present, one that is absent (refusal), three ssh keys of which one does not convert (two lines in the assembled file, one line on stderr), and the assembled file's mode
- [ ] 4.8 Add a hermetic check exercising both overrides by pointing `SAFIX_AGE_KEYGEN` and `SAFIX_SSH_TO_AGE` at scripts of the check's own, asserting each is invoked
- [ ] 4.9 Severity drill: make an unreadable `ageKeyFile` a skip rather than a refusal and confirm 4.7's refusal test reddens; separately make an unconvertible ssh key fatal and confirm 4.7's three-key test reddens. The pair is the evidence the asymmetry is deliberate in both directions
- [ ] 4.10 Severity drill: ignore `SAFIX_AGE_KEYGEN` and confirm 4.8 reddens, which is the evidence the override is read rather than merely declared
- [ ] 4.11 Verify: `cargo test -p safix-core install::identity keygen::` green, the check in 4.8 green, drills in 4.9 and 4.10 observed

## 5. Decryption

- [ ] 5.1 Add `install::decrypt_documents`: group the manifest's secrets by `sopsFile`, run `sops decrypt --output-type json <file>` once per distinct document through the existing `SAFIX_SOPS` lookup, and hold each result as a `Secret`
- [ ] 5.2 Add `install::extract_key`: resolve a `/`-nested key path inside a decrypted JSON document, returning `ManifestKeyMissing` when any segment is absent, and add unit tests for a flat key, a nested key, a missing leaf, and a segment that is not an object
- [ ] 5.3 Hold the value discipline: no decrypted value reaches an argument vector or the environment, and no `String` or `Vec<u8>` of a decrypted value outlives the write. Add a unit test asserting the extraction returns the crate's own value type rather than a `String`
- [ ] 5.4 Reference the `sops` binary from the activation script by store path, never through `PATH`, in both `modules/consume/installer.nix`'s `installerScript` and the home-scope equivalent; keep `safix.installer.agePlugins` on `PATH`, which is the one thing that must be
- [ ] 5.5 Add a check asserting the activation script names `sops` by store path and that `PATH` in `activationEnvironment` is composed of the age plugins alone
- [ ] 5.6 Severity drill: replace the store-path reference with a bare `sops` and confirm 5.5 reddens, which is the evidence risk 6 of design I14 is closed by a check rather than by the prose
- [ ] 5.7 Severity drill: decrypt per entry rather than per document and confirm a test counting `sops` invocations over a two-entry, one-document fixture reddens, which is the evidence the granularity is held
- [ ] 5.8 Verify: `cargo test -p safix-core install::decrypt install::extract` green, the check in 5.5 green, drills in 5.6 and 5.7 observed

## 6. The filesystem sequence and the four preserved behaviours

- [ ] 6.1 Implement `%r` expansion (`install::expand_runtime_dir`): `$XDG_RUNTIME_DIR` on linux, the output of `getconf DARWIN_USER_TEMP_DIR` on darwin, `%%` splitting to a literal `%`, applied to `secretsMountPoint`, `symlinkPath` and every secret `path`, and only when `userMode`
- [ ] 6.2 Implement the mount step: `mkdir -p` at `0751`; a no-op under `userMode`; otherwise `ramfs`, or `tmpfs` with `noswap` when `useTmpfs` with an `EINVAL` fallback dropping `noswap`, with `MS_NODEV|MS_NOSUID|MS_NOEXEC` and `mode=0751`, skipped when already mounted with that filesystem's magic, then chown `0:keys`
- [ ] 6.3 Add the mount and filesystem-magic syscalls through a crate exposing a safe interface (design I15), extending `crates/safix-core/Cargo.toml`'s existing `statfs` dependency rather than introducing an `unsafe` block, and confirm `#![forbid(unsafe_code)]` is untouched
- [ ] 6.4 Implement the generation directory: read the symlink at `symlinkPath`, parse its basename as an unsigned integer, increment, remove a same-named leftover, `mkdir` at `0751`, chown `0:keys` outside user mode
- [ ] 6.5 Implement the write step: parent directories at `0751` chowned `0:keys`, each file at its declared mode, chowned `owner:group` outside user mode
- [ ] 6.6 Implement the restart/reload diff: byte-compare each old `<symlinkPath>/<name>` against the new one, collect `restartUnits`/`reloadUnits` for new and changed entries, skipped wholesale under `userMode`
- [ ] 6.7 Implement propagation: `systemctl --no-block restart/reload` when `SOPS_RESTART_UNITS_VIA_SYSTEMCTL` is set, otherwise write the unit names to `/run/nixos/activation-restart-list` and `-reload-list`
- [ ] 6.8 Implement the atomic symlink swap, skipped under `--dry-run`, followed by the per-entry symlinks for every entry whose `path` differs from `<symlinkPath>/<name>`, creating parent directories and replacing whatever is there until the link matches
- [ ] 6.9 Implement `keepGenerations` pruning, keeping the last N generation directories
- [ ] 6.10 Preserve, and test individually, the four behaviours design I9 names: a non-symlink at `symlinkPath` is removed; the diff returns early when `symlinkPath` does not exist; the environment variable selects between `systemctl` and the activation lists; dry-activate skips the swap and only the swap
- [ ] 6.11 Severity drill: make the non-symlink case a refusal rather than a removal and confirm `safix-installer-coexistence` (task 10.5) reddens, which is the evidence that check measures a live hazard rather than a hazard the rewrite quietly removed
- [ ] 6.12 Severity drill: remove the early return when `symlinkPath` is absent and confirm a first-install test reddens on a missing previous generation, which is the evidence the stage-2-init case is held
- [ ] 6.13 Severity drill: perform the swap under `--dry-run` and confirm the dry-activate test in 3.11 reddens
- [ ] 6.14 Verify: `cargo test -p safix-core install::` green, drills in 6.11-6.13 observed

## 7. System scope: the option table replacing every `sops.*` read

- [ ] 7.1 Declare, in `modules/consume/installer.nix`'s `options.safix.installer`: `package` (`package`, default the flake's own `packages.safix`), `validationPackage` (`package`, default the build-platform build), `validate` (`bool`, default `true`), `keepGenerations` (`int`, default `1`), `useTmpfs` (`bool`, default `false`), `log` (`listOf (enum [ "keyImport" "secretChanges" ])`, default `[ ]`), `environment` (`attrsOf str`, default `{ }`), `agePlugins` (`listOf package`, default `[ ]`), each with a description a newcomer can act on
- [ ] 7.2 Replace `activationEnvironment` (`installer.nix:41-44`): `sopsCfg.environment` → `cfg.installer.environment`, `sopsCfg.age.plugins` → `cfg.installer.agePlugins`; `HOME=/var/empty` and the `lib.makeBinPath` construction unchanged
- [ ] 7.3 Replace `requiredIdentities` (`:62-65`): `sopsCfg.age.keyFile` → `cfg.identity.keyFile`, and drop the `generateKey` conjunct, which has no system-scope counterpart
- [ ] 7.4 Delete `hasGnupgSource` (`:67`) and every use of it, and delete the gnupg conjunct from `sufficientIdentities`' guard (`:76`)
- [ ] 7.5 Replace `sufficientIdentities`' sources (`:76-81`): `sopsCfg.age.keyFile` → `cfg.identity.keyFile`, `sopsCfg.age.sshKeyPaths` → `cfg.identity.sshKeyPaths`; update each identity's `origin` string from `sops.age.*` to the safix option it now names, since those strings appear in the refusal text
- [ ] 7.6 Replace the invocation (`:141`): `exec ${sopsCfg.package}/bin/sops-install-secrets ${manifest}` → `exec ${cfg.installer.package}/bin/safix install ${manifest}`
- [ ] 7.7 Replace the resolved-set read (`:149`): `config.safix.installed` → `config.safix.secrets`
- [ ] 7.8 Re-gate the assertion block (`:155-180`) on `cfg.installer.validate` in place of `sopsCfg.validateSopsFiles`, and re-gate `manifest`'s own guard (`:180-181`) the same way; both messages stay byte-identical
- [ ] 7.9 Rewrite the manifest JSON (`:186-222`): add `version = 1`; keep `secrets`, `secretsMountPoint`, `symlinkPath`; `keepGenerations` ← `cfg.installer.keepGenerations`; `ageKeyFile` ← `cfg.identity.keyFile`; `ageSshKeyPaths` ← `cfg.identity.sshKeyPaths`; `useTmpfs` ← `cfg.installer.useTmpfs`; `userMode = false`; `logging.{keyImport,secretChanges}` ← `cfg.installer.log`; add `manifestInputHash` per 2.4; delete `templates`, `placeholderBySecretName`, `gnupgHome`, `sshKeyPaths`
- [ ] 7.10 Replace the `checkPhase` (`:226-229`): `${sopsCfg.validationPackage}/bin/sops-install-secrets -check-mode=${if sopsCfg.validateSopsFiles then "sopsfile" else "manifest"}` → `${cfg.installer.validationPackage}/bin/safix install --check-mode=${if cfg.installer.validate then "document" else "manifest"} --ignore-passwd`
- [ ] 7.11 Replace the unit's reads (`:372-398`): `environment = cfg.installer.environment // { SOPS_RESTART_UNITS_VIA_SYSTEMCTL = "1"; }`, `path = cfg.installer.agePlugins`, and `RequiresMountsFor` built from `cfg.identity.keyFile` and `cfg.identity.sshKeyPaths` alone, with the two gnupg contributions (`:390-391`) deleted; `wantedBy sysinit.target`, the `sysinit-reactivation.target` edges and `DefaultDependencies=no` unchanged
- [ ] 7.12 Delete `modules/consume/nixos.nix`'s whole `sops = { … }` definition block (`:139-152`), and rewrite the comment at `:130-137` — which explains why `sops.secrets` is left empty — into a statement that safix defines no option outside its own namespace
- [ ] 7.13 Rewrite `derivedHostKeys`' comment (`nixos.nix:39-46`) so the rule is safix's own rather than a re-implementation of the provisioner's, keeping the catch-22 reason verbatim, since `secret-installation`'s "A host key another store deployed is usable" scenario rests on it; the expression itself is unchanged
- [ ] 7.14 Delete `modules/consume/nixos.nix:93`'s read of `config.sops.gnupg.*` and the identity-sufficiency branch it fed
- [ ] 7.15 Rewrite `modules/consume/common.nix:113-131`'s prose block, which explains the provisioner's host-key default and its keyfile fatality, into a statement of safix's own defaults and its own fatality
- [ ] 7.16 Add a check asserting no file under `modules/consume/` contains the string `sops.` as an option read or definition, with the `sops` binary reference and the `sopsFile` field name excluded by exact spelling rather than by a loose pattern
- [ ] 7.17 Severity drill: reintroduce one `sopsCfg` read and confirm 7.16 reddens, which is the evidence the namespace claim is held mechanically rather than by review
- [ ] 7.18 Severity drill: collapse `validationPackage` into `package` and confirm a cross-build evaluation of the manifest derivation fails, which is the evidence risk 5 of design I14 is closed
- [ ] 7.19 Verify: `nix build .#checks.x86_64-linux.safix-installer-store .#checks.x86_64-linux.safix-installer-ordering .#checks.x86_64-linux.safix-installer-sole .#checks.x86_64-linux.safix-installer-refusals .#checks.x86_64-linux.safix-installer-identity` green, the check in 7.16 green, drills in 7.17 and 7.18 observed

## 8. User scope installs

- [ ] 8.1 Declare `options.safix.installer.{secretsMountPoint,symlinkPath}` at home scope, defaulting to `"%r/safix.d"` and `"%r/safix"`, with descriptions stating that `%r` is expanded by the installer against the platform's runtime directory and that the store therefore does not survive a reboot without a login
- [ ] 8.2 Declare `options.safix.identity.generateKey` at home scope (`bool`, default `false`), described as minting the key file at activation when it is absent, and naming `safix keygen` as the operator-run alternative
- [ ] 8.3 Declare the home-scope subset of `safix.installer.*` the user-mode manifest needs — `package`, `validate`, `keepGenerations`, `log` — and state at each that `useTmpfs`, `environment` and `agePlugins` are system-scope only because user mode mounts nothing and runs no unit environment of its own
- [ ] 8.4 Build the user-mode manifest in `modules/consume/home.nix`: the same `serde` shape as system scope, `userMode = true`, entries from `cfg.secrets` through `materializeFor`'s user-scope output, `ageKeyFile` ← `cfg.identity.keyFile`, `ageSshKeyPaths` ← `cfg.identity.sshKeyPaths`, `version = 1`
- [ ] 8.5 Add the build-time check phase to the home-scope manifest derivation, mirroring 7.10, using the build-platform package
- [ ] 8.6 Add `systemd.user.services.safix`, linux only, invoking `safix install <manifest>`, gated on `cfg.enable` and a non-empty resolved set
- [ ] 8.7 Add `home.activation.safixInstall` on both platforms, registered with `lib.hm.dag.entryAfter [ "writeBoundary" ]`, with a comment stating why a bare string (which becomes `entryAnywhere`) is refused
- [ ] 8.8 Add the key-generation step to the activation entry, running `age-keygen -o <keyFile>` through `SAFIX_AGE_KEYGEN` when `cfg.identity.generateKey` and the file is absent, before the install step
- [ ] 8.9 Delete `modules/consume/home.nix`'s three `sops.*` definitions (`:232`, `:236`, `:240`) and every `sopsCfg` read (`:52-53`, `:58-61`, `:68-70`, `:106`); the refusal-text secret count at `:106` reads `cfg.secrets` instead
- [ ] 8.10 Re-gate `home.activation.safixIdentityPreflight` (`:246-249`) from `sopsCfg.secrets != { }` to `cfg.secrets != { }`, keeping its `entryBefore [ "checkLinkTargets" ]` placement
- [ ] 8.11 Rewrite the header prose (`home.nix:4-32`), the ordering description (`:176-183`) and the key-source comment (`:190-198`), all three of which describe the provisioner's activation entry, its user unit, and its assertion placement; each becomes a statement about safix's own
- [ ] 8.12 Rewrite the remediation text (`:104-115`), which currently tells the reader that sops-nix has no per-person key default, into safix's own equivalent statement
- [ ] 8.13 Add a check evaluating a home-manager fixture on `x86_64-linux` and on `aarch64-darwin`, asserting the linux one carries the user unit and the darwin one does not, and that both carry the activation entry and a manifest with `userMode = true`
- [ ] 8.14 Add integration coverage in `crates/safix/tests/install.rs` for `%r` expansion: a user-mode manifest installed with `XDG_RUNTIME_DIR` set, asserting the store lands under it, and a test asserting `%%` yields a literal `%`
- [ ] 8.15 Add a test asserting a user-mode install mounts nothing, chowns nothing, and restarts nothing: observed by running it as an unprivileged user with a `restartUnits` entry declared and asserting no `systemctl` invocation and no ownership change
- [ ] 8.16 Severity drill: set `userMode = false` in the home-scope manifest and confirm 8.15 reddens on an attempted mount or chown, which is the evidence the three omissions are driven by the field rather than by the scope happening to lack privileges
- [ ] 8.17 Severity drill: register the activation entry as a bare string and confirm 8.13's ordering assertion reddens, which is the evidence the DAG placement is held
- [ ] 8.18 Verify: `nix build .#checks.x86_64-linux.safix-portability-home` and the check in 8.13 green, `cargo test -p safix --test install` green, drills in 8.16 and 8.17 observed

## 9. One module per scope, and the input deletion

- [ ] 9.1 Change `flake.nix:63-72`'s `homeModules` binding so `default` and `safix` are the same value, `./modules/consume/home.nix`, and rewrite the comment above it: the dual naming is retained for compatibility, and it is no longer copied from another flake's convention
- [ ] 9.2 Change `flake.nix:138-143`'s `nixosModules` the same way
- [ ] 9.3 Delete `flake.nix:126-131`'s comment explaining the `.default`/`.safix` split, replacing it with one sentence stating that both names are one value and why both are retained
- [ ] 9.4 Delete `sops-nix` from `flake.nix:61`'s `outputs = inputs@{ … }` destructuring
- [ ] 9.5 Delete `flake.nix:26-31`'s comment justifying the input
- [ ] 9.6 Delete `flake.nix:32-33` — `sops-nix.url` and `sops-nix.inputs.nixpkgs.follows` — and regenerate `flake.lock`, confirming the root `sops-nix` input edge (`flake.lock:26-27`) and the `sops-nix_2` node (`:263-282`) are gone while clan-core's own node (`:237-262`) remains
- [ ] 9.7 Add a check asserting that no attribute under `flake.nixosModules` or `flake.homeModules` has an import closure naming any flake input, over all four published names
- [ ] 9.8 Severity drill: add a fixture module importing a flake input and confirm 9.7 fires on it, which is the anti-vacuity probe the retired `namesSopsNix` predicate carried (`entrypoints.nix:58-71`) and which must survive the predicate's generalization
- [ ] 9.9 Severity drill: point `homeModules.default` back at a two-element `imports` list naming any input and confirm 9.7 reddens
- [ ] 9.10 Verify: `nix flake metadata --json | jq '.locks.nodes | keys'` shows sops-nix present only as clan-core's transitive node, the check in 9.7 green, drills in 9.8 and 9.9 observed

## 10. Check suite: `modules/flake/checks/installer.nix`

- [ ] 10.1 Delete `manifestFor` (`:195-211`) and `parityManifest` (`:394-398`), the two bindings that call the provisioner's own manifest builder
- [ ] 10.2 Delete `providerFixture` (`:226-239`), including the `neededForUsers` entry (`:231-234`) and the read of `system.build.sops-nix-users-manifest` (`:239`), and delete the standalone evaluation of the provisioner's home-manager module (`:245`)
- [ ] 10.3 Delete `safix-installer-mechanism` (`:953`) and every assertion feeding it; its subject is the provisioner's `extraJson` merge and its `secrets-for-users` relocation, and there is no second builder to measure
- [ ] 10.4 Delete `safix-installer-manifest` (`:1008`) and the field-set parity diff (`:1037-1046`); replace it with two checks: `safix-installer-schema`, an accepted snapshot of the built manifest's full structure over the existing manifest fixture, and `safix-installer-roundtrip`, running `safix install --check-mode=document --ignore-passwd` over the built manifest and over four single-field mutations (unknown version, non-octal mode, a `key` absent from its document, an unknown top-level field), asserting acceptance once and refusal four times, each naming its field
- [ ] 10.5 Port `safix-installer-coexistence` (`:1166`): swap `inputs.sops-nix.packages.${system}.sops-install-secrets` (`:1173`) for the flake's own `packages.safix` invoked as `safix install`, leaving the sandbox, the fixture store and the assertions unchanged
- [ ] 10.6 Confirm `safix-installer-store` (`:1051`), `-ordering` (`:1102`), `-sole` (`:1108`), `-refusals` (`:1114`) and `-identity` (`:1123`) evaluate unchanged now that `config.flake.nixosModules.default` needs no sops option tree, and update each fixture's expectation only where a default this change moved is asserted
- [ ] 10.7 Confirm `safix-installer-store`'s `entryPathContract` still derives its expectation from `config.flake.safix.lib.materialize …` rather than from the type default now carrying the mint, so the check does not go vacuous when the mint moves (task 1.4)
- [ ] 10.8 Severity drill: mutate one field of the built manifest and confirm `safix-installer-schema` reddens; mutate the `serde` struct instead and confirm the same check reddens, which is the evidence the snapshot holds both sides of the boundary
- [ ] 10.9 Severity drill: make `--check-mode=document` accept a `key` absent from its document and confirm `safix-installer-roundtrip`'s third mutation turns green when it should refuse
- [ ] 10.10 Severity drill: remove the mint from the entry type and confirm `entryPathContract` reddens, repeating 1.9's drill from the check's own side, which is what holds 10.7's non-vacuity claim
- [ ] 10.11 Verify: every check in `modules/flake/checks/installer.nix` builds green, `nix eval .#checks.x86_64-linux --apply builtins.attrNames` shows `safix-installer-mechanism` and `safix-installer-manifest` gone and `safix-installer-schema` and `safix-installer-roundtrip` present, drills in 10.8-10.10 observed

## 11. Check suite: consumption, entrypoints, materialization, portability

- [ ] 11.1 Delete `sopsHomeCopy` (`consumption.nix:112`) and every use of it
- [ ] 11.2 Re-point `handForm` (`consumption.nix:187`): the hand-wired comparison arm stops importing the provisioner's module, and the `equivalence` claim (`:575`) becomes "the module form equals the resolver output read back through safix's own entry type"; update the check's own header prose to state the new premise
- [ ] 11.3 Re-point `viewOf` (`consumption.nix:204-207`) to read each entry field back through `common.secretEntryType` rather than through the provisioner's option types
- [ ] 11.4 Re-point the refusal fixture at `consumption.nix:316` to import safix's own module without home-manager's assertion wrapper, unchanged in intent
- [ ] 11.5 Re-establish `safix-module-collision` (`consumption.nix:517`, fixtures `:547-548`, `:557-558`) over two distinct copies of `modules/consume/nixos.nix` reached by two paths that are not the same store path, so the fact `consumer-integration` needs held — importing one declaring module twice is an evaluation error rather than a merge — keeps a subject
- [ ] 11.6 Re-base the literal expectations at `consumption.nix:627-631` (`format = "yaml"` and its neighbours) onto safix's own declared defaults, and add a comment stating that the values are unchanged but their source is not, so a literal that silently agrees with a deleted dependency's default cannot go unnoticed
- [ ] 11.7 Generalize `namesSopsNix` (`entrypoints.nix:118`) into `namesAnyFlakeInput`, a predicate over every declared input
- [ ] 11.8 Reduce `importAsymmetry` (`entrypoints.nix:159-164`) from four expectations to one symmetric assertion over all four published names, and rebuild the anti-vacuity probe (`:166`) on the new predicate, keeping the reasoning at `:58-71` up to date with what the probe now measures
- [ ] 11.9 Replace `materialization.nix:86-88`'s three `provisioner` evaluations with evaluations of safix's own modules, and re-point `axisOf` (`:90`) from `options.sops.secrets.type.getSubOptions [ ]` to `common.secretEntryType`'s own sub-options, leaving the ownership-asymmetry assertion (`:141-146`) claiming the same thing
- [ ] 11.10 Rewrite `materialization.nix:8-12`'s header prose: the premise changes from "hand safix's output to the real provisioner and read it back" to "hand safix's output to safix's own declared type and read it back"
- [ ] 11.11 Re-base `portability.nix`'s `format = "yaml"` literals (`:588`, `:598`, `:608`, `:628`, `:782`, `:790`, `:798`, `:811`) onto safix's own default, and rewrite `:257-264`'s justification for excluding `path`, `owner`, `group` and `sopsFileHash` from `decided`: `sopsFileHash` is dropped from the list entirely (it no longer exists) and the other three are justified as safix's own defaults
- [ ] 11.12 Re-base `portability.nix:819-820`'s expected user-scope path from `"/home/alice/.config/sops-nix/secrets/nginx/service-token"` to the `%r/safix/…` path the new default produces, and verify the new literal against a real home-manager profile rather than against the fixture alone, since the fixture is what produced the old path
- [ ] 11.13 Re-point `portability.nix:500-506`'s `systemIdentity` at `derivedHostKeys`, which is now where the host-key default comes from where safix names no identity
- [ ] 11.14 Confirm by grep that `exported`, `namespace`, `examples`, `cli`, `single-runtime`, `policy`, `bridge`, `bridge-sync`, `custody`, `envelope`, `vault`, `vault-projection`, `subjects`, `generators`, `gate-guard`, `keepassxc`, `real-clan`, `fixture-fleet`, `integration` and `collision-fixture` name no `sops-nix` reference, and leave every one of them unedited
- [ ] 11.15 Severity drill: make `handForm` re-derive its expectation through the same `materializeFor` call the module form uses and confirm `safix-consumption`'s equivalence assertion goes vacuous — observable as the check staying green under a deliberate `materializeFor` mutation — which is the evidence the two sides stay independent after the re-point
- [ ] 11.16 Severity drill: make the two collision fixtures in 11.5 the same store path and confirm `safix-module-collision` turns green when it should fail, which is the evidence the check measures two distinct copies rather than one imported twice by name
- [ ] 11.17 Verify: `nix build .#checks.x86_64-linux.{safix-consumption,safix-consumption-system,safix-consumption-refusals,safix-consumption-ordering,safix-module-collision,safix-module-entrypoints,safix-materialization,safix-portability-system,safix-portability-home}` green, drills in 11.15 and 11.16 observed

## 12. A NixOS VM test

- [ ] 12.1 Add `modules/flake/checks/installer-vm.nix` with a `pkgs.testers.runNixOSTest`, registered under `checks.safix-installer-vm` on linux systems only, and add the file to `flake.nix`'s `imports` list
- [ ] 12.2 Build the test machine: import `config.flake.nixosModules.safix`, bind it to a fixture declaring one shared and one private entry, place an age key file on the machine through its own configuration, and declare one entry's `restartUnits` naming a trivial unit the test can observe
- [ ] 12.3 Assert, after boot: the symlink at `symlinkPath` names generation `1`; each entry's file exists at its declared mode and ownership; the mount at `secretsMountPoint` is the expected filesystem type
- [ ] 12.4 Assert, after a second activation with one entry's value changed: generation `2` exists, the symlink moved to it, the retained generation count obeys `keepGenerations`, and the unit named in `restartUnits` was restarted
- [ ] 12.5 Assert a dry activation changes neither the symlink nor any file
- [ ] 12.6 Severity drill: skip the chown step and confirm 12.3's ownership assertion reddens; skip the prune and confirm 12.4's generation-count assertion reddens. The pair is the evidence the VM test measures the sequence rather than only that the machine booted
- [ ] 12.7 Severity drill: make the second activation reuse generation `1` and confirm 12.4 reddens, which is the evidence generation rotation is held on a real host rather than only in a unit test
- [ ] 12.8 Verify: `nix build .#checks.x86_64-linux.safix-installer-vm` green, drills in 12.6 and 12.7 observed

## 13. Documentation, changelog, and the migration table

- [ ] 13.1 Rewrite `README.md:6-7` and `:56-58`, which name sops-nix as a dependency and as what a consumer's profile reads
- [ ] 13.2 Rewrite `README.md:804-822`, `:833`, `:838-846`, `:854-859` and `:868-886` — the consumption sections describing the `.default`/`.safix` split, the `sops.*` options a consumer tunes, and the normal-priority conflict hazard at `:839` — into the single-module surface, the `safix.installer.*` namespace, and a statement that safix now writes no `sops.*` option at all
- [ ] 13.3 Rewrite `README.md:761`, and add the migration rename table from design I13 verbatim to the README, including the user-scope path row and the three-absence row (templates, user-relocated secrets, gnupg)
- [ ] 13.4 Verify the user-scope path row against a real home-manager profile before publishing it, per design I14 risk 1, and record what was observed beside the table
- [ ] 13.5 Rewrite `docs/notes/research/zero-knowledge-vault.md:9`, `:21`, `:92`, `:98`, `:108` and `:132`, each of which reasons about what sops-nix can consume; the reasoning about the sops document format is unchanged, and only the attribution of who reads it moves
- [ ] 13.6 Rewrite `crates/safix/tests/read_path.rs:176-186`'s doc comment, which references `sops-install-secrets` and `SOPS_AGE_KEY_FILE`, to reference safix's own installer and its own identity assembly
- [ ] 13.7 Add a `[Unreleased]` CHANGELOG entry marked breaking for the nix surface: the entry type is safix's own, the manifest schema is safix's own and versioned, the `safix.installer.*` namespace replaces every `sops.*` read, the two published module names per scope are one value, the user-scope secret path moves to a runtime directory, templates and user-relocated secrets and gnupg identities are unsupported, and `safix install` is a new verb. The sops-nix claim is phrased exactly as "safix declares no sops-nix input", never as "sops-nix is gone"
- [ ] 13.8 Check the changelog's sops-nix phrasing against `nix flake metadata --json`'s actual node list, so the claim is verified rather than asserted (design I14 risk 8)
- [ ] 13.9 Audit every guarantee the rewritten README sections state, sentence by sentence, against the check or test in this repository that holds it, and record the mapping in this task
- [ ] 13.10 Severity drill: for each of the three absences in 13.3, confirm that a fixture attempting it fails — a manifest carrying a `templates` field is refused by `deny_unknown_fields`, a `neededForUsers` field likewise, and a gnupg-only identity fails evaluation per task 7.4 — so the documented absences are refusals rather than omissions
- [ ] 13.11 Verify: 13.9's audit complete with no unheld sentence, drill in 13.10 observed

## 14. Verification

- [ ] 14.1 `openspec validate own-the-installer --strict` green
- [ ] 14.2 `openspec validate --all --strict`, compared against the total recorded when this change was proposed
- [ ] 14.3 `nix eval .#checks.x86_64-linux --apply builtins.attrNames` lists every check named in groups 1, 7-12, and does not list `safix-installer-mechanism` or `safix-installer-manifest`
- [ ] 14.4 `nix flake check --keep-going --print-build-logs` green on `x86_64-linux`
- [ ] 14.5 `nix eval .#checks.aarch64-darwin --apply builtins.attrNames` and a darwin evaluation of the home-scope fixture from 8.13, confirming the darwin arm of `%r` expansion and the absence of the user unit are both reachable on that platform
- [ ] 14.6 `cargo test --locked --workspace` green
- [ ] 14.7 `rg "sops-nix"` over the whole tree and confirm every remaining match is either a historical reference in `openspec/changes/archive/`, a released CHANGELOG entry, or a statement that safix declares no such input
- [ ] 14.8 `rg "sops\."` over `modules/` and `crates/` and confirm every remaining match is the `sops` binary, the `sopsFile` field name, or a `.sops.yaml` path, and none is an option read or definition
- [ ] 14.9 `rg "unsafe"` over `crates/` and confirm `#![forbid(unsafe_code)]` is present in every crate root and no `allow` weakens it, holding design I15 as a fact
- [ ] 14.10 Confirm every severity drill in groups 1-13 was observed and recorded beside the task that names it
