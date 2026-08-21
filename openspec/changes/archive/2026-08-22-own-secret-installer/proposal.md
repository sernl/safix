# safix installs its own secrets, into its own store, after its own identity exists

Throughout this change, revisions are named because every claim below was read at one.
`sops-nix` is `github:Mic92/sops-nix` at `a8627b21b9107c5711c96b84f32a9a4b3d45295f`, this flake's `inputs.sops-nix`.
clan-core pins `f1406619a3884cd5c47992a70b8b35c9c0fcb4c9`; of the five sops-nix files this change reasons about, four are byte-identical between the two revisions, and the fifth, `modules/home-manager/sops.nix`, differs only at `:371` and `:438` in two `stdenv.isLinux`-versus-`stdenv.hostPlatform.isLinux` lines that nothing here cites, so no claim depends on which one a consumer's tree resolves to.
`clan-core` is `56e35624d94e4f1ac55d36575ebab97cbd9b9cdd`, this flake's `inputs.clan-core`.
`nixpkgs` is `0e251e24a4f24e036a084b6b4b2d2491af4167f4`, this flake's root `inputs.nixpkgs`; clan-core pins a different one, `59ea0b1c043c463e39fcb3cfb9a5c8bcf0777c72`, and the `activation-script.nix` anchors below are byte-identical at both.

## Why

`nixosModules.default` cannot deliver a secret to a machine that runs clan, and cannot at any point in its lifetime.
That is not a bug in one of the two packages.
It is what happens when safix delegates arrival to sops-nix's NixOS module, because that module hardcodes where secrets live and names its activation entry the same thing clan names its own.

Three facts compose into it, and each was read rather than inferred.

The store is hardcoded.
`modules/sops/manifest-for.nix:36-38` emits `secretsMountPoint = "/run/secrets.d"` and `symlinkPath = "/run/secrets"` under sops-nix's own comment asking whether this needs to be configurable, and the NixOS option surface has no answer: a grep of `modules/` for either field finds them in the two manifest builders, in the `secrets-for-users` overrides, and in the home-manager module's `defaultSymlinkPath` and `defaultSecretsMountPoint` options at `modules/home-manager/sops.nix:184` and `:193`.
The home-manager scope has the options and the NixOS scope does not.

The installer removes what it finds there.
`pkgs/sops-install-secrets/main.go:404` defines `prepareSecretsDir`, and at `:415-423` it calls `os.RemoveAll(linkName)` whenever the symlink path exists and is not a symlink, under the comment "if `/run/secrets` exists, but is not a symlink, we need to remove it".
On a clan machine `/run/secrets` is a ramfs mount that clan established: `nixosModules/clanCore/vars/secret/age.nix:141-154` mounts a secret filesystem there, and `:218-227` places every service-phase var at `/run/secrets/<generator>/<name>`.
Unlinking a mountpoint is `EBUSY`, which is exactly what the pilot host reported, twice: `failed to prepare new secrets directory: cannot remove /run/secrets: unlinkat /run/secrets: device or resource busy`.

The activation entry is not safix's to name.
sops-nix defines `system.activationScripts.setupSecrets` at `modules/sops/default.nix:497-515` with `lib.stringAfter [ "specialfs" "users" "groups" ]`, and clan defines `system.activationScripts.setupSecrets` at `age.nix:259-276` with the same three dependencies.
`text` is `types.lines` (`nixos/modules/system/activation/activation-script.nix:106-107`), so the two do not conflict and do not order: they concatenate into one snippet whose halves run in definition order, which neither module declares.

The composition is what makes it total rather than intermittent.
Whichever half runs first, safix's secrets do not land.
Cold boot with sops-nix first: `/run/secrets` does not exist yet, so `prepareSecretsDir` is fine, but the identity is not there either — `importAgeSSHKeys` warns to stderr and continues on a missing path (`main.go:886-911`), the age keyfile is written with only its header comment, and `decryptSecrets` at `main.go:1441` fails before `prepareSecretsDir` at `:1448` is ever reached.
Every other combination — cold boot with clan first, or any later switch — finds `/run/secrets` mounted, decrypts successfully, and then fails at `:1448`.
So the two failure modes are distinct, they are reached under complementary conditions, and their union is every activation.

Two things follow that are worth stating plainly rather than leaving to be discovered.

The failure is loud but not destructive, and overstating it would be wrong.
Activation snippets run under `trap "_status=1 _localstatus=\$?" ERR` with no `set -e` (`activation-script.nix:62-63`), so the failed half records a status and the merged snippet continues into clan's decrypter.
The clan secrets survive every time.
Nothing is lost; nothing safix declared is delivered.

Repairing the identity alone would make things worse.
If safix's identity were fixed while `symlinkPath` stayed at `/run/secrets`, then on a cold boot with sops-nix first the installer would succeed, symlink `/run/secrets` to `/run/secrets.d/1`, and hand clan's decrypter a symlink where it expects a directory — at which point `mountpoint -q "$target"` reads false and clan mounts a fresh ramfs over safix's generation directory (`age.nix:143-148`).
The two halves are one repair.

There is a second defect underneath the first, and it is the one that would survive a narrow fix.
`sops.age.sshKeyPaths` defaults to the ed25519 entries of `services.openssh.hostKeys`, filtered by `defaultImportKeys` at `modules/sops/default.nix:181-191`, which drops every key whose path begins `/run/secrets` under the comment "Skip ssh keys deployed with sops to avoid a catch 22".
On a clan machine every host key is at `/run/secrets/openssh/...`, so the filter empties the list, and `sops.age.keyFile` is null, and `sops.gnupg.*` is unset.
sops-nix's key-source assertion at `:432-441` then fires — its condition tests four options and does not include `services.openssh.enable`, which only its message at `:440` mentions — so evaluation refuses naming sops-nix's options and none of safix's.
`modules/consume/nixos.nix:19-23`, `modules/consume/common.nix:92-94` and `README.md:193`, `:805` and `:828` all state the opposite: that the system scope usually has an identity without naming one.
On the fleet this package was extracted from, that sentence is false on every machine.
The pilot got past it by setting `safix.identity.sshKeyPaths` by hand, which is a consumer working around a claim safix made.

## What Changes

- safix builds its own installer manifest and invokes `sops-install-secrets` itself at system scope, rather than routing its resolved set through `sops.secrets` and inheriting sops-nix's activation.
  The mechanism needs no patch to sops-nix and is not novel: `manifest-for.nix` reads both roots out of the manifest JSON rather than out of a NixOS option, and merges its `extraJson` argument over the hardcoded defaults at `:52`, which is how `modules/sops/secrets-for-users/default.nix:24-27` already relocates the same two fields to `/run/secrets-for-users.d` and `/run/secrets-for-users`.
- safix's store becomes `/run/safix.d` and `/run/safix`, and every resolved entry that declares no path of its own parks at `/run/safix/<name>` rather than at sops-nix's `/run/secrets/<name>` default (`modules/sops/default.nix:73-80`).
  Both halves are load-bearing together: `symlinkSecretsAndTemplates` (`main.go:254-268`) creates a symlink at each secret's own `path` whenever that path is not `<symlinkPath>/<name>`, so moving the store without moving the default path would stop colliding with clan's directory and start writing into it.
- The installer entry is safix's own, named `safixInstallSecrets` rather than `setupSecrets`, which is what makes ordering expressible at all: an entry that merges into a shared name cannot depend on the other half of itself, and an entry with its own name can.
- The ordering against a foreign secret store is named by the consumer, in the two forms NixOS offers, and defaults to naming nothing.
  safix reads no option of clan's to discover it, for the reason `consumer-integration` already gives about user registries.
- The system-scope identity is derived by safix rather than inherited from sops-nix's default, using the same rule with the correct prefix: exclude the host keys this installer itself deploys, which are now under `/run/safix`, and not the ones a different secret store deploys under `/run/secrets`.
- Where nothing is derivable, the system scope refuses at evaluation with safix's own message naming safix's options, in the shape `modules/consume/common.nix:95-129` already established for the user scope.
- The manifest safix writes is checked at build time by the same binary that will read it, through the `checkPhase` at `manifest-for.nix:54-58`, so a drift between safix's hand-written JSON and the installer's schema is a build failure rather than an activation failure.
  That `checkPhase` is conditional and safix mirrors the conditional rather than picking a branch of it.
  `validateSopsFiles` defaults true (`modules/sops/default.nix:228-230`), so the mode that runs over safix's entries today is `sopsfile`, which reads the ciphertext (`main.go:507`), parses it by format, and verifies each declared `key` resolves (`:559`), where `manifest` mode returns a stub from `loadSopsFile` without reading the file at all (`:503-505`) and skips that check (`:558`).
  Both catch schema drift, which is what this bullet is for, so naming `manifest` alone would have read as correct and would have silently dropped the ciphertext half of what the package validates today.
- The nix-level refusals safix currently gets are copied into safix's builder rather than assumed to travel with the entry type.
  `manifest-for.nix:11-28` refuses a `sopsFile` that does not exist and one that lies outside the nix store, and it does so inside the builder safix no longer calls, so safix carries the same block under the same `validateSopsFiles` gate.

Not in scope: any change to the home-manager scope, whose store is already relocatable through sops-nix's own options and whose activation guard is unaffected; any change to custody, the resolver, the policy renderer, the bridge, or the CLI; any patch to sops-nix, vendored or otherwise; and darwin, where clan mounts an HFS RAM disk by a different route (`age.nix:98-106`) and no failure has been observed.

## Capabilities

### New Capabilities

- `secret-installation`: the installer safix owns — that it invokes the provisioner's binary against a manifest of its own, that its store is its own and disjoint from any other on the host, how it is ordered against a store it did not create, how the system-scope identity is derived and what happens when none is, and what it leaves untouched.

### Modified Capabilities

- `secret-consumption`: the system module's activation contribution is no longer empty, so the requirement that records its asymmetry with the user scope must say what it does contribute.
  The namespace also gains the two ordering options and the identity-derivation switch.
- `consumer-integration`: ordering against a foreign secret store joins host attachment as a thing safix takes from the consumer rather than discovers, and for the same stated reason.

## Impact

Affected code:

- New: `modules/consume/installer.nix` — the manifest builder, the unit and activation-entry forms, and the identity derivation.
- Modified: `modules/consume/nixos.nix` — stops defining `sops.secrets`, imports the installer, and carries the system-scope identity refusal.
- Modified: `modules/consume/common.nix` — the new options and their messages.
- Modified: `README.md:193`, `:805` and `:828` — the three sentences that state the system scope inherits a usable identity from sops-nix.

Affected checks: `modules/flake/checks/installer.nix` is new and carries the manifest roots, the per-entry path default, the ordering, the identity derivation, the refusal, and the coexistence claim.
`modules/flake/checks/consumption.nix` changes where it reads the system-scope arrival, because that arrival is no longer `config.sops.secrets`, and gains the system-scope half of the inert claim, whose existing probes are home-manager only and so would not notice an ungated installer.
Every claim gets a severity drill in `tasks.md`, and the one claim an evaluation cannot hold — that the installer's destructive branch is real — is drilled against the binary itself.
