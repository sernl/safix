# Design: the installer safix owns

Revisions are as named in `proposal.md`.
Every line anchor below was read at one of them.

## Context

safix's system-scope arrival is currently one assignment.
`modules/consume/nixos.nix:77-87` sets `sops.secrets = cfg.secrets` and lets sops-nix do the rest: build a manifest, pick a store, register an activation entry, derive an identity, run the binary.
That is the right shape when sops-nix is the only thing on the host that installs secrets.
It is the wrong shape when something else already is, because every one of those five decisions is sops-nix's and none of them is negotiable from a NixOS option.

clan is such a thing, and it is not an unusual one: it owns `/run/secrets` as a real ramfs mount (`age.nix:141-154`), it places its own vars beneath it as `/run/secrets/<generator>/<name>` (`age.nix:218-227`), it names its activation entry `setupSecrets` (`age.nix:259-276`), and it holds the machine's age identity at `/etc/secret-vars/key.txt` (`age.nix:204`, `:210`).
sops-nix's NixOS module makes the mirror-image choices, and the two collide in all three places at once: the store, the entry name, and — because clan's own host keys live under `/run/secrets` and sops-nix's default identity filter drops that prefix (`default.nix:181-191`) — the identity.

The measurements that establish the collision, the two complementary failure modes, and the reason the failure is loud rather than destructive are in `proposal.md` and are not repeated here.
What matters for the design is the shape of the escape, and it is already in sops-nix's own tree.

## Goals / Non-Goals

Goals.
Make a resolved entry arrive on a machine that runs another secret store, without safix removing, mounting over, or writing into anything that store owns.
Make the ordering against that store expressible, which requires an activation entry safix can name.
Derive a system-scope identity that is correct on such a machine, or refuse in safix's own words naming safix's own options.
Change no line of sops-nix, vendored, patched, or overlaid.

Non-Goals.
The home-manager scope.
Its store is already relocatable through `sops.defaultSymlinkPath` and `sops.defaultSecretsMountPoint` (`modules/home-manager/sops.nix:184`, `:193`), its identity has no host-key default to be wrong about, and its activation guard is untouched by any of this.
Making sops-nix itself coexist with clan.
A consumer who writes `sops.secrets` directly, beside safix, still gets the original collision, and this change neither fixes that nor pretends to.
darwin, where clan mounts an HFS RAM disk by another route (`age.nix:98-106`) and where nothing has been observed.

## Decisions

### D1. The two store roots are manifest fields, not options, and safix writes the manifest

The whole remedy rests on one fact and it was read rather than assumed.
`sops-install-secrets` takes `secretsMountPoint` and `symlinkPath` from the manifest JSON — `main.go:1381` mounts at `manifest.SecretsMountPoint`, `:1448` prepares at `manifest.SymlinkPath`, `:1469` symlinks to it — and never consults a NixOS option, because the binary has no idea one exists.
`manifest-for.nix` hardcodes both at `:36-38` and then merges its `extraJson` argument over them at `:52`, so the hardcoding is a default rather than a floor.

sops-nix already exploits this on itself.
`modules/sops/secrets-for-users/default.nix:24-27` calls the same `manifestFor` with `{ secretsMountPoint = "/run/secrets-for-users.d"; symlinkPath = "/run/secrets-for-users"; }` and gets a second, disjoint store out of the same binary.
That submodule is the existence proof, and it is why this change needs no upstream cooperation at all.

safix does not call `manifestFor` itself, because that would make the sops-nix source tree a dependency of a module that must be importable without one — `nixosModules.safix` imports nothing by contract, and there is no honest way for it to reach `${inputs.sops-nix}/modules/sops/manifest-for.nix`.
So safix writes the JSON with `pkgs.writeTextFile` and validates it with the binary that will read it, using the `checkPhase` `manifest-for.nix:54-58` already uses: `sops-install-secrets -check-mode=manifest "$out"`.
The field set is copied from `manifest-for.nix:33-51` at the pinned revision.
Copying a schema is a drift risk and it is answered rather than accepted: a check builds sops-nix's own manifest from `inputs.sops-nix`, which the check has and the module does not, and compares the two JSON key sets.
A field sops-nix adds reddens that check on the commit that bumps the input, which is the only place a fresh required field could otherwise reach a machine unannounced.

### D2. The store roots and the per-entry path default move together, because one without the other is worse

Moving `symlinkPath` alone does not relocate safix's secrets.
`symlinkSecretsAndTemplates` (`main.go:254-268`) walks every secret and, whenever `secret.Path` differs from `filepath.Join(symlinkPath, secret.Name)`, calls `os.MkdirAll` on that path's parent and creates a symlink there.
sops-nix defaults `path` to `/run/secrets/<name>` (`default.nix:73-80`), and `modules/flake/safix/resolve.nix:2130-2131` deliberately relies on that default for every entry that declares no path — "Entries handed no path park at the provisioner's own default".

So a `symlinkPath` of `/run/safix` with the default path untouched would stop unlinking clan's directory and start writing symlinks into it, one per resolved entry, under root.
That is a worse failure than the current one because it is silent.
`safix.installer.defaultPath` therefore defaults every path-less entry to `/run/safix/<name>`, in the same commit, and a check asserts the two agree by building a manifest and reading both the root and every entry's path out of it.

The path-collision refusal at `resolve.nix:2125-2164` is unaffected and its comment stays true: a minted default is a function of the name alone and so still cannot collide, and only a declared path can.

### D3. safix stops defining `sops.secrets`, and recovers the typing it was getting for free

Two installers cannot both be right about one resolved set.
sops-nix gates its entire installer on `regularSecrets != { }` (`default.nix:27`, `:432`, `:468`, `:498`), so the way to have exactly one is for safix's system module to leave `sops.secrets` empty and carry its resolution elsewhere.
Doing so also disarms sops-nix's key-source assertion (`:432-441`), which is correct: it is an assertion about sops-nix's installer, and on this path sops-nix has none.

What that gives up is the option-type validation safix currently gets by assigning into a typed option.
It is recovered rather than dropped: `safix.installed` is declared with `type = options.sops.secrets.type`, read off sops-nix's own declaration in the same evaluation, so every entry still passes through `secretType` (`default.nix:46`) with its `pathNotInStore` check, its `sopsFileHash`, its mode and owner and group coercions, and its file-existence assertions in `manifest-for.nix:11-28`.
The submodule's `config` block defaults `sopsFile` from `sops.defaultSopsFile`, which is harmless because safix sets `sopsFile` on every entry explicitly (`resolve.nix:2244`).

safix keeps reading and defining the rest of the `sops.*` namespace — `package`, `validationPackage`, `keepGenerations`, `useTmpfs`, `placeholder`, `environment`, `age.plugins`, and its own `age.keyFile` and `age.sshKeyPaths` — so a consumer who tunes sops-nix still tunes safix, and safix does not mint a second copy of an option surface that already exists.

### D4. The entry is named, because a merged entry cannot depend on its other half

The ordering half of the defect is not that safix's entry sorts wrongly.
It is that safix has no entry.
Both packages define `system.activationScripts.setupSecrets`; `deps` is `listOf str` and `text` is `types.lines` (`activation-script.nix:101-107`), so the definitions union their dependencies and concatenate their texts into one snippet.
There is exactly one node in the activation DAG, so there is no edge to state, and the relative order of the two halves is definition order — a function of module import order that neither module declares and neither can rely on.

Naming safix's entry `safixInstallSecrets` creates the second node, and with it the ability to write the edge.
That is the whole of why the rename is load-bearing, and it costs nothing: nothing outside safix names this entry.

### D5. The ordering itself is the consumer's to name, in both of the forms NixOS offers

`safix.installer.afterActivation` is a list of activation-script names and `safix.installer.afterUnits` a list of unit names; both default to the empty list, and both are set by the consumer.
On the fleet this was measured on, that is `[ "setupSecrets" ]` and `[ "age-decrypt-secrets.service" ]`.

safix does not sniff for clan, and the reason is the one `consumer-integration` already gives about user registries: reading `config.clan.core.vars.settings.secretStore` to decide safix's ordering would make clan's option tree part of safix's interface, and would answer for exactly one foreign store out of the many a host might run.
The cost is that safix cannot guarantee the ordering by construction, and that cost is paid explicitly in D7 rather than papered over: the installer verifies its identity before it decrypts and refuses with a message that names these two options as the remedy.

Whether the ordering is expressed as an activation dependency or as a systemd `After=` is not a choice safix makes either, and this is the part of the question worth answering carefully, because flipping sops-nix's `useSystemdActivation` is tempting and is not the fix.

That option (`default.nix:319-334`) defaults to whether `systemd.sysusers` or `services.userborn` is enabled, and it is a global sops-nix switch — setting it changes behaviour for every sops-nix consumer in the tree, not just for safix.
It also does nothing whatever about the store, since `symlinkPath` is a manifest field and the unit reads the same manifest.
And on the ordering half it is actively worse than the status quo: sops-nix's unit is `wantedBy = [ "sysinit.target" ]` with `after = [ "local-fs.target" "systemd-sysusers.service" "userborn.service" ]` (`default.nix:467-495`), clan's `age-decrypt-secrets` is `wantedBy = [ "sysinit.target" ]` with `after = [ "systemd-sysusers.service" ]` (`age.nix:296-308`), and neither names the other.
Two units in one target with no relation between them is a race, where the merged activation snippet is at least deterministic per evaluation.
So the systemd path does not solve the ordering more cleanly by itself.
It solves it only once safix owns the unit and can write `After=` on it, which is this change.

Both forms therefore ship, and safix selects between them the way sops-nix and clan both do — from `config.systemd.sysusers.enable` and `config.services.userborn.enable` (`default.nix:321-323`, `secrets-for-users/default.nix:28-30`) — with `safix.installer.useSystemdActivation` as the override.
Selecting from those two NixOS options rather than from `sops.useSystemdActivation` keeps safix's path independent of a switch that now governs an installer safix no longer uses.

The boot-versus-switch asymmetry is real and is benign in the direction that matters.
clan's unit is `RemainAfterExit = true` and declares nothing about `sysinit-reactivation.target`, so it runs at boot and not again on a `nixos-rebuild switch`; safix's unit mirrors sops-nix's `requiredBy`/`before` wiring for that target and so does re-run.
`After=` on an already-active `RemainAfterExit` unit is satisfied immediately, which is correct, because what safix is ordering against is the existence of the identity and the identity persists in `/run/secrets` across the switch.
On the activation-script path both entries run at boot and on every switch, and the dependency is a plain edge.

### D6. The identity is derived with the exclusion prefix that is actually safix's

sops-nix's `defaultImportKeys` (`default.nix:181-191`) maps `services.openssh.hostKeys` to their paths and drops any whose path begins `/run/secrets`, under the comment "Skip ssh keys deployed with sops to avoid a catch 22".
The exclusion is right and the prefix is the installer's own store, hardcoded because sops-nix's store is.

Once safix's store is `/run/safix`, the correct prefix for safix is `/run/safix`.
A host key under `/run/secrets` is not something safix deployed, so using it creates no catch-22 — it is a key another store placed there before safix ran, which is exactly the ordering D5 exists to guarantee.
So `safix.identity.deriveHostKeys` defaults true at system scope and produces the ed25519 entries of `services.openssh.hostKeys` whose paths do not begin with safix's own `symlinkPath`, and on a clan machine that yields `/run/secrets/openssh/ssh.id_ed25519` without the consumer naming it.

This is the sentence `README.md:193` was reaching for and getting wrong.
The recipient a machine declares is still "the age form of the host identity its system scope already decrypts with" — `custody-subjects` states it at that level and stays true — but which key that is, and whether sops-nix's default finds it, are two different questions and only the first was answered.

`/etc/secret-vars/key.txt` (`age.nix:204`) is the other candidate and is deliberately not taken.
It is clan's machine age identity, available at cold boot with no ordering constraint at all, which is attractive.
It is also a different key from the one `flake.safix.machines.<m>.recipient` is the age form of, so adopting it would change what every machine-granted file is encrypted to.
That is a custody decision, not a mechanical one, and it is left as an open question rather than folded into a defect fix.

### D7. Nothing derivable is a refusal in safix's words, at evaluation and again before decryption

Two refusals, because they catch different things.

At evaluation, where the resolved set is non-empty and no identity is configured or derivable, the system module throws safix's own message naming `safix.identity.sshKeyPaths`, `safix.identity.keyFile` and `safix.identity.deriveHostKeys`, in the shape `modules/consume/common.nix:95-129` established for the user scope and for the same reason: the next thing to refuse is sops-nix's key-source assertion, which names five sops-nix options and none of safix's — and on this path it will not even fire, because `sops.secrets` is empty and the assertion sits inside `mkIf (cfg.secrets != { })`.
So without safix's own refusal there is no refusal at all, and the module would evaluate green and install nothing decryptable.

At activation, before invoking the binary, the installer checks each configured identity path for presence and readability and refuses naming the paths, the ordering options from D5, and the fact that a store which has not run yet is the usual cause.
This is what turns the cold-boot ordering failure from `importAgeSSHKeys`'s two stderr lines and a downstream decryption error (`main.go:886-911`, `:1441`) into a message that says which activation step has not run.
Its limit is stated the way the user-scope preflight's is: presence and readability were checked, decryption was not.

### D8. One change rather than two

The store defect and the identity defect are separable to describe and are not separable to fix.
Landing the identity fix alone would make the system strictly worse, and this is mechanical rather than a matter of taste: with a working identity and `symlinkPath` still at `/run/secrets`, a cold boot whose merged snippet runs sops-nix's half first now succeeds, replacing `/run/secrets` with a symlink to `/run/secrets.d/1` (`main.go:1469`).
clan's decrypter then runs `mountpoint -q "$target"`, reads false because a symlink to a generation directory is not a mountpoint, and mounts a fresh ramfs over safix's secrets (`age.nix:143-148`).
Landing the store fix alone leaves the machine unable to decrypt at all, which is the status quo plus a rename.

They also share one cause — safix borrowing an installer whose store, entry name and identity rule are all hardcoded to being the only one — and one remedy, which is a single new file.
Two changes would land half an installer.

## Alternatives considered

### Patch or bump sops-nix upstream to expose the two roots

The most honest-looking route, because sops-nix's own source asks the question: `manifest-for.nix:36` carries the comment "Does this need to be configurable?" directly above the two hardcoded fields, and the home-manager scope already answers yes for itself with `defaultSymlinkPath` and `defaultSecretsMountPoint` (`modules/home-manager/sops.nix:184`, `:193`).
Two NixOS options mirroring those would be a small, well-motivated patch, and it should be sent.

It is not what this change waits on, for two reasons.
It fixes one third of the defect: the entry name and the identity derivation are both still hardcoded to sops-nix being the only installer, so a machine with relocated roots would still have one merged `setupSecrets` node with no expressible ordering and still derive an empty identity from `defaultImportKeys`.
And it puts safix's ability to install a secret behind someone else's review queue and release cadence, for a capability safix can have today with no patch at all.
The right sequencing is to ship this, then send the upstream patch, and simplify D1 to use the options if and when they land.

### A fleet-local overlay carrying the patch

Cheaper than upstreaming and available immediately, and it is rejected on a stronger ground than cost.
safix is an extracted, publishable package whose whole premise is that a consumer imports it and it works; an overlay is a consumer-side obligation that no `flakeModules` or `nixosModules` export can carry, so the package would ship a module that only functions in a tree that had separately been told to rewrite one of its inputs.
It also inherits the first alternative's coverage problem — one third of the defect — while adding a patch that must be rebased on every `sops-nix` bump, and a divergence between what this repository's checks build and what a consumer's flake evaluates.

### Require safix consumers to be non-clan machines

This is the current behaviour with a sentence added, and the sentence would be a large one.
The fleet safix was extracted from runs clan on every machine, so the restriction excludes the only consumer the package has.
It also mis-states the constraint: nothing about clan in particular is the problem, and any component that mounts a store at `/run/secrets` or defines `system.activationScripts.setupSecrets` collides identically.
A package whose stated scope is "custody-first secrets management for nix" cannot reasonably carve out hosts that already manage secrets.

### The recommendation

Own the installer, as D1 through D8 describe.
It is the only one of the four that reaches all three hardcoded decisions, it needs no cooperation from anyone, the mechanism it depends on is one sops-nix already uses on itself and therefore has reason to keep, and it leaves the upstream patch available as a later simplification rather than a prerequisite.

## Risks / Trade-offs

Copying sops-nix's manifest schema is the largest one.
It is bounded by the `-check-mode=manifest` validation at build time and by the key-set comparison against `inputs.sops-nix` in the check suite, and both fail on the commit that bumps the input rather than on a machine.
It is not eliminated: a field whose *meaning* changes without its name changing passes both.

Owning the installer means owning its failure modes.
safix now has a unit and an activation entry that can fail on their own, where previously any failure was sops-nix's and looked like it.
That is the point, but it does mean safix's name appears in activation output where it did not before.

A second store costs a second ramfs mount and a second generation directory per host.
`keepGenerations` applies to safix's store independently (`main.go:1475`), so the cost is bounded by the same option that bounds sops-nix's.

The ordering options default to naming nothing, so a consumer who does not set them on a clan machine gets D7's activation refusal rather than a working machine.
That is deliberate — a wrong guess about which foreign store to order against is worse than a refusal that names the option — but it does mean adopting this change on such a fleet is two settings rather than none, and `tasks.md` carries them.

Coexistence is proven for safix and not for sops-nix.
A tree that writes `sops.secrets` directly, beside safix, on a clan machine still has the original collision, and this change does not detect or report that.

## Migration Plan

Within this repository the change is not additive and that is the one thing an implementer must not get wrong: `nixosModules.default` and `nixosModules.safix` stop delivering through `sops.secrets`, so any consumer reading `config.sops.secrets` to see what safix established reads `{ }` afterwards.
`config.safix.secrets` is unchanged and remains the resolved set; `config.safix.installed` is the typed set the manifest is built from.
`modules/flake/checks/consumption.nix:388` reads `config.sops.secrets` for the system fixture and moves to the latter.

For the fleet this was extracted from, adoption is the two ordering options and nothing else, because D6 derives the identity that the pilot set by hand.
The `safix.identity.sshKeyPaths` workaround can be removed at the same time or left, since a set value still wins.

## Open Questions

Whether a machine's identity should eventually be clan's `/etc/secret-vars/key.txt` rather than its ssh host key.
It is strictly better mechanically — on disk, present at cold boot, no ordering constraint — and it is a different key, so the question is whether `flake.safix.machines.<m>.recipient` should be able to name a machine identity that is not the host's ssh key.
That is a custody question and belongs to `custody-subjects`, not here.

Whether the same treatment is owed to the home-manager scope for symmetry.
It is not needed — sops-nix exposes both roots there as options — but safix currently sets neither, so a user profile and a system configuration on one host would use stores chosen by different mechanisms.
Left alone until a collision is observed at that scope.
