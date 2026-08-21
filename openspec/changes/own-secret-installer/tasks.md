# Tasks: own-secret-installer

Revisions are as named in `proposal.md`, and every line anchor below was read at one of them.
Where a task says "hold", it means add a check that fails when the claim stops being true, not add a sentence asserting it.

Three disciplines hold throughout.
No real recipient, no real hostname, and no real user name from any fleet enters this repository; fixtures use the `alice`, `bob` and `carol` names the existing consumption fixtures already use, together with synthetic `age1` strings.
Nothing here deploys, switches, or activates anything: every verification builds or evaluates, and the one runtime claim is driven inside a build sandbox against the installer binary.
No sentence describing a guarantee is written before the code enforcing it exists in the same commit.

A note on where the regression lives, because the obvious place is wrong.
`crates/safix/tests/support/clan-stub.rs` and `nix-stub.rs` stand in for subprocess boundaries the command crosses — `clan vars get`, `nix eval` — and the rust suite drives the command.
Nothing in this defect is reachable from the command: it is a NixOS module composing with another NixOS module, and it fails during activation.
So the regression is in the nix check suite, and the single claim an evaluation cannot hold is driven against the installer binary in group 8.
`modules/flake/checks/real-clan.nix` is the other place a reader will look, because it already solves running clan inside a build sandbox with clan's own `setupNixInNix`, and it is wrong for this for the same reason: it drives the clan CLI across the bridge, and no CLI invocation can hold a claim about how two NixOS modules compose during activation.

## 1. The four facts the remedy rests on, held before anything is built

- [x] 1.1 Add `modules/flake/checks/installer.nix` with a `safix-installer-mechanism` check, and assert in it that a manifest built by the provisioner's own `manifest-for.nix` with an `extraJson` naming two other roots carries those roots rather than the hardcoded ones, which is the whole mechanism (`${inputs.sops-nix}/modules/sops/manifest-for.nix:36-38` and the `// extraJson` at `:52`)
- [x] 1.2 Assert that the provisioner already does exactly this to itself, by reading the two roots out of the manifest its `secrets-for-users` submodule builds (`modules/sops/secrets-for-users/default.nix:24-27`), so the mechanism is held against the provisioner's own use of it and not only against a synthetic call
- [x] 1.3 Assert that the provisioner's NixOS option surface offers neither root: enumerate `options.sops` of an evaluated system configuration and assert no option path sets a secrets mount point or a symlink path, while the home-manager scope's `sops.defaultSymlinkPath` and `sops.defaultSecretsMountPoint` both exist (`modules/home-manager/sops.nix:184`, `:193`)
- [x] 1.4 Evaluate a fixture system configuration that defines `system.activationScripts.setupSecrets` twice — once in the shape the provisioner uses (`modules/sops/default.nix:497-515`) and once in clan's (`nixosModules/clanCore/vars/secret/age.nix:259-276`) — and assert the result is one activation step whose text contains both bodies and whose dependency list is the union, because that is why no ordering between them is expressible (`nixos/modules/system/activation/activation-script.nix:101-107`)
- [x] 1.5 Assert in the same check that the activation wrapper carries an `ERR` trap and no `set -e` (`activation-script.nix:62-63`), so the record that a failed half does not stop the other half is held rather than asserted in prose
- [x] 1.6 Severity drill: replacing the two `setupSecrets` definitions in 1.4 with two differently-named steps turns the merge assertion red; removing the `extraJson` argument in 1.1 turns the root assertion red
- [x] 1.7 Verify: `nix build .#checks.x86_64-linux.safix-installer-mechanism` green, and each drill in 1.6 observed red before its expectation is restored

## 2. The manifest safix writes

- [x] 2.1 Write `modules/consume/installer.nix` building a manifest with `pkgs.writeTextFile`, carrying every field the provisioner's builder emits (`manifest-for.nix:33-51`) with safix's own values for the two roots
- [x] 2.2 Give the derivation the provisioner's own `checkPhase` from `manifest-for.nix:54-58`, run with `config.sops.validationPackage`, mirroring its conditional rather than picking a branch of it: `-check-mode=${if config.sops.validateSopsFiles then "sopsfile" else "manifest"}`
- [x] 2.3 Record beside it why the branch is load-bearing, because `manifest` is the tempting one and the weaker one: `validateSopsFiles` defaults true (`modules/sops/default.nix:228-230`), so `sopsfile` is what runs over safix's entries today, and `manifest` mode returns a stub from `loadSopsFile` without reading the file (`main.go:503-505`) and skips the key-presence check (`:558`), leaving the JSON schema and nothing else
- [x] 2.4 Copy the nix-level assertion block at `manifest-for.nix:11-28` into safix's builder — both the named file-existence message and the "is not in the Nix store" refusal — under the same `config.sops.validateSopsFiles` gate the original carries at `:27`, because that block lives in the builder safix does not call and does not travel with the option type (group 4 records why)
- [x] 2.5 Read `package`, `validationPackage`, `validateSopsFiles`, `log`, `keepGenerations`, `useTmpfs`, `placeholder`, `environment`, `gnupg.home`, `gnupg.sshKeyPaths` and `age.plugins` from the `sops` namespace rather than minting a second option surface, and record at the top of the file that safix continues to couple to those settings while no longer using the provisioner's installer
- [x] 2.6 Note at `placeholder` why it is read and why it will be empty: it is defined only under `mkIf (config.sops.templates != { })` and maps over `config.sops.secrets` (`modules/sops/templates/default.nix:116`, `:130-134`), and safix now leaves both empty, so the field is carried for schema parity rather than for content
- [x] 2.7 Expose the built manifest as `system.build.safix-manifest`, mirroring the provisioner's `system.build.sops-nix-manifest` (`modules/sops/default.nix:534`), so the checks in groups 2, 3 and 6 read one concrete attribute rather than rebuilding the derivation each time
- [x] 2.8 Add `safix-installer-manifest` asserting the built manifest parses, that its two roots are safix's, and that its `userMode` is false
- [x] 2.9 In the same check, build the provisioner's own manifest from `inputs.sops-nix` over the same fixture and assert the two JSON key sets are equal, so a field the provisioner adds reddens this check on the commit that moves the pin rather than reaching a host
- [x] 2.10 Severity drill: dropping one field from safix's manifest turns 2.9 red; giving the derivation a manifest the binary rejects turns 2.2 into a build failure, observed; and pinning the mode to `manifest` over a fixture whose declared `key` is absent from its ciphertext turns that same build green, which is the evidence that the two modes are not interchangeable
- [x] 2.11 Verify: `nix build .#checks.x86_64-linux.safix-installer-manifest` green, and every drill in 2.10 observed

## 3. The store roots and the per-entry path default, which move together

- [x] 3.1 Set the manifest's `secretsMountPoint` to `/run/safix.d` and its `symlinkPath` to `/run/safix`, both settable through options of the consumption namespace so a consumer with a reason can move them
- [x] 3.2 Default every resolved entry that declares no path to `<symlinkPath>/<name>`, replacing reliance on the provisioner's `/run/secrets/<name>` default (`modules/sops/default.nix:73-80`), and update the comment at `modules/flake/safix/resolve.nix:2130-2131` that records where a path-less entry parks
- [x] 3.3 Add `safix-installer-store` asserting, off `system.build.safix-manifest`, that the two roots are safix's and that every entry's path is inside the symlink path
- [x] 3.4 Assert in the same check that an entry declaring a path of its own keeps it, and that the path-collision refusal at `resolve.nix:2142-2164` is unchanged, by resolving a fixture whose two entries declare one path and asserting it still throws
- [x] 3.5 Severity drill, and this is the one that matters most in this group: move the root without moving the entry default and assert the check goes red, because that combination is what makes `symlinkSecretsAndTemplates` (`main.go:254-268`) write a symlink into the other store's directory instead of colliding with it
- [x] 3.6 Verify: `nix build .#checks.x86_64-linux.safix-installer-store` green, and the drill in 3.5 observed red

## 4. One installer, and the validation that has to survive leaving the provisioner's option

- [x] 4.1 Declare `safix.installed` in `modules/consume/common.nix` with `type = options.sops.secrets.type`, read off the provisioner's own declaration in the same evaluation, and define it from `safix.secrets` at system scope
- [x] 4.2 Remove the `sops.secrets = cfg.secrets` definition from `modules/consume/nixos.nix:79`, keeping the identity definitions at `:81-85` where the installer reads them back
- [x] 4.3 Build the manifest from `config.safix.installed` rather than from the raw resolution, so every entry has passed the provisioner's `secretType` (`modules/sops/default.nix:46`): its mode, owner, group, uid and gid coercions, its `sopsFile` default, and the `sopsFileHash` default that forces `builtins.hashFile "sha256"` under `validateSopsFiles` (`:51-53`)
- [x] 4.4 Do not expect the type to carry more than that, which is the correction this group exists to record and which was settled by evaluating it rather than by reading it: an option declared `type = options.sops.secrets.type` accepts an entry whose `sopsFile` is outside the store and returns it unchanged, because `pathNotInStore` is declared at `:19-25` and used at exactly one site, `sops.age.keyFile` at `:338`, never on a secret entry, whose `sopsFile` is plain `lib.types.path` at `:137`, and because the store-membership refusal is `manifest-for.nix:19-23`, inside the builder safix does not call — what refuses instead is the block copied in 2.4
- [x] 4.5 Add `safix-installer-sole` asserting that a fixture system configuration with a non-empty resolution has an empty `config.sops.secrets`, no provisioner activation step, no provisioner unit, and exactly one installer invocation, which is safix's
- [x] 4.6 Assert in the same check that an entry whose `sopsFile` is outside the nix store is still refused and that the refusal is safix's own copied block, by matching its message text; assert separately that a `sopsFile` that does not exist also refuses, and record that this second one is incidental — the forced `hashFile` throws a bare `path '...' does not exist`, which is not safix's message and which `tryEval` does not catch
- [x] 4.7 Severity drill: restoring the `sops.secrets` definition turns 4.5 red on the provisioner-step assertion; removing the block copied in 2.4 turns the outside-store half of 4.6 red, which is the evidence the option type never carried it
- [x] 4.8 Verify: `nix build .#checks.x86_64-linux.safix-installer-sole` green, and both drills observed

## 5. The named entry, both mechanisms, and the ordering options

- [x] 5.1 Register the installer as `system.activationScripts.safixInstallSecrets`, and record in the file why the name is load-bearing: two definitions of one name are one node with no edge to state
- [x] 5.2 Add the systemd form as `systemd.services.safix-install-secrets`, mirroring the provisioner's unit wiring at `modules/sops/default.nix:467-495` including its `sysinit-reactivation.target` relationship, so the unit re-runs on a switch
- [x] 5.3 Select between the two from `config.systemd.sysusers.enable` and `config.services.userborn.enable`, the same condition the provisioner and clan both compute (`default.nix:321-323`, `secrets-for-users/default.nix:28-30`), with `safix.installer.useSystemdActivation` as the override, and record why the selection is not read from `sops.useSystemdActivation`
- [x] 5.4 Add `safix.installer.afterActivation` and `safix.installer.afterUnits`, both `listOf str` defaulting to `[ ]`, and wire each into the mechanism selected in 5.3
- [x] 5.5 Add `safix-installer-ordering` asserting, off two evaluated fixture configurations, that the activation form's step is its own node and carries the named dependency in its `deps`, and that the unit form carries the named unit in its `after`
- [x] 5.6 Assert in the same check that a fixture naming neither option still evaluates and registers the installer with no foreign dependency, so an unordered host is a supported configuration rather than an omission
- [x] 5.7 Severity drill: naming the step `setupSecrets` again turns 5.5 red on the own-node assertion; dropping the `deps` wiring turns it red on the dependency assertion
- [x] 5.8 Verify: `nix build .#checks.x86_64-linux.safix-installer-ordering` green, and both drills observed

## 6. The identity the system scope derives

- [x] 6.1 Add `safix.identity.deriveHostKeys`, default true at system scope, producing the ed25519 entries of `config.services.openssh.hostKeys` whose paths do not begin with safix's own symlink path
- [x] 6.2 Record at the option why the exclusion prefix is safix's store rather than `/run/secrets`: the provisioner's filter at `modules/sops/default.nix:181-191` excludes the store its own installer writes, and safix's store is now a different one
- [x] 6.3 Make a consumer-named identity win over the derived one, and make the derivation contribute nothing when the switch is off
- [x] 6.4 Add `safix-installer-identity` asserting that a fixture whose host keys sit under `/run/secrets` derives them, that a fixture whose host keys sit under `/run/safix` does not, and that a named identity is unchanged by either
- [x] 6.5 Assert in the same check that the derived identity reaches `system.build.safix-manifest`'s `ageSshKeyPaths`, so the claim is about what the binary will read rather than about an intermediate option
- [x] 6.6 Severity drill: restoring the `/run/secrets` prefix turns 6.4 red on the first fixture; dropping the exclusion entirely turns it red on the second
- [x] 6.7 Verify: `nix build .#checks.x86_64-linux.safix-installer-identity` green, and both drills observed

## 7. The two refusals

- [x] 7.1 Add the evaluation-time refusal to `modules/consume/common.nix` as a named message function beside `noIdentityMessage`, so a check can read the string without evaluating a module, and fire it at system scope when the resolution is non-empty and no identity is configured or derivable
- [x] 7.2 Have the message name `safix.identity.sshKeyPaths`, `safix.identity.keyFile` and `safix.identity.deriveHostKeys`, and record why the provisioner cannot be left to refuse: its key-source assertion sits inside `mkIf (cfg.secrets != { })` (`modules/sops/default.nix:432-441`), which safix now leaves empty, so nothing refuses at all
- [x] 7.3 Add the pre-decryption check to the installer script: for each configured identity path, test presence and readability, and exit non-zero naming each path, how it failed, both ordering options from 5.4, and that a foreign store which has not yet run is the usual cause
- [x] 7.4 State the limit in that message the way the user-scope preflight states its own (`modules/consume/home.nix`, the `remediation` binding): presence and readability were checked, decryption was not
- [x] 7.5 Add `safix-installer-refusals` holding both — the evaluation refusal read off a configuration evaluated without the module system's assertion collection, as `safix-consumption-refusals` already does, and the installer script's text asserted to name every configured path and to exit non-zero, without running it
- [x] 7.6 Severity drill: removing the evaluation refusal turns 7.5 red and nothing else goes red, which is the evidence that no other refusal covers it; dropping one identity path from the script turns the script assertion red
- [x] 7.7 Verify: `nix build .#checks.x86_64-linux.safix-installer-refusals` green, and both drills observed

## 8. Coexistence, held against the binary rather than against an evaluation

This is the only claim in the change that an evaluation cannot hold, because the hazard is a runtime branch: `prepareSecretsDir` calls `os.RemoveAll` on the symlink path whenever it exists and is not a symlink (`main.go:404`, `:415-423`).

- [ ] 8.1 Add `safix-installer-coexistence` as a `runCommand` that runs the real `sops-install-secrets` twice inside the build sandbox, over fixture ciphertext and a fixture age identity, in user mode so no privileged mount is needed
- [ ] 8.2 First run: point the manifest's symlink path at a pre-created ordinary directory holding a sentinel file, and assert the sentinel is gone afterwards, which demonstrates the destructive branch is real rather than assumed
- [ ] 8.3 Second run: point the manifest at safix's own roots, with the same foreign directory and sentinel present elsewhere, and assert the sentinel survives and the foreign directory is byte-identical afterwards
- [ ] 8.4 Assert the second run's own outputs landed inside safix's store and that no path outside it was created, by walking the sandbox tree before and after and diffing the two listings
- [ ] 8.5 Record in the check header what this stands in for and what it does not: the observed failure was `EBUSY` on a mountpoint, a sandbox cannot mount, and the branch being demonstrated is the removal itself, which is what a mountpoint turns into an error
- [ ] 8.6 Gate the check on Linux, following `safix-syscall-proof` and `safix-consumption-system`, and make its absence explicit rather than trivially green
- [ ] 8.7 Severity drill: pointing the second run's manifest back at the foreign directory turns 8.3 red
- [ ] 8.8 Verify: `nix build .#checks.x86_64-linux.safix-installer-coexistence` green, and the drill observed red

## 9. The consumption checks that read the old arrival

- [x] 9.1 Move `modules/flake/checks/consumption.nix:388`, which reads `(nixosFor "bob").config.sops.secrets`, to read `config.safix.installed`, and record beside it that the system scope no longer delivers through the provisioner's option
- [ ] 9.2 Hold `secret-consumption`'s unmodified requirement that a profile resolving nothing is inert — "no secrets, no identity configuration, no activation entry, and no unit" — at the system scope, which this change gives two new surfaces to and where nothing holds it today, the existing inert probes being built from `inertProfile = moduleForm "carol"` (`modules/flake/checks/consumption.nix:215`) and reading `home.activation` and `systemd.user.services` (`:527-535`, expected at `:567-576`), so an ungated `system.activationScripts.safixInstallSecrets` or `systemd.services.safix-install-secrets` would pass every check in the suite
- [ ] 9.3 Add the system-scope half to `safix-consumption-system`: evaluate `nixosFor "carol"`, the same user who resolves nothing on that host, and assert `system.activationScripts ? safixInstallSecrets` and `systemd.services ? safix-install-secrets` are both false, alongside the identity fields the user-scope probe already reads
- [ ] 9.4 Severity drill: dropping the `enable` gate from either installer mechanism turns 9.3 red on that mechanism, and neither drill reddens anything else, which is the evidence that the gate is what holds the requirement rather than an accident of the fixture
- [ ] 9.5 Re-check every other assertion in that file that reads `sops.*` at system scope, and leave the home-manager half untouched
- [ ] 9.6 Confirm `safix-consumption-ordering` is unaffected, since it holds a home-manager activation DAG and this change touches no home-manager path
- [ ] 9.7 Verify: `nix build .#checks.x86_64-linux.safix-consumption-system` and `.#checks.x86_64-linux.safix-consumption` and `.#checks.x86_64-linux.safix-consumption-ordering` and `.#checks.x86_64-linux.safix-consumption-refusals` all green, and both drills in 9.4 observed

## 10. Documentation, and the three sentences that are currently false

- [ ] 10.1 Rewrite `README.md:193` so it says the recipient is the age form of the host identity and that safix derives which key that is, rather than that the provisioner's default finds it
- [ ] 10.2 Rewrite `README.md:805` and `README.md:828`, both of which state the system scope keeps the provisioner's host-key default, and say instead what safix derives and what it excludes
- [ ] 10.3 Rewrite the header of `modules/consume/nixos.nix:19-23` and the note at `modules/consume/common.nix:92-94`, which make the same claim at the option, and while in that region correct `common.nix:126-128`, which says the provisioner's key-source assertion "names its five": the message at `modules/sops/default.nix:440` names three, its condition tests four, and the two assertions of that block together name four distinct options
- [ ] 10.4 Document the installer: the two roots, the ordering options with the values a clan host needs, the derivation switch, and both refusals
- [ ] 10.5 State the coexistence limit plainly — that this covers safix's installer, and that a consumer writing `sops.secrets` directly on such a host still collides
- [ ] 10.6 Verify: every guarantee stated in the README names a check in this repository that holds it, and `rg -n '/run/secrets' README.md modules/` returns only the sentences that are about another store

## 11. Verification

- [ ] 11.1 `openspec validate own-secret-installer --strict`
- [ ] 11.2 `openspec validate --all --strict`, compared against the baseline recorded when this change was proposed: 19 specs, plus this change, reported as 20 passed and 0 failed over 20 items
- [ ] 11.3 `nix eval .#checks.x86_64-linux --apply builtins.attrNames` lists every check named in groups 1 through 8
- [ ] 11.4 `nix flake check` green
- [ ] 11.5 `cargo test` green, confirming the rust suite is untouched by a change that has no command surface
- [ ] 11.6 `rg` the whole tree for any real fleet identifier and confirm none
