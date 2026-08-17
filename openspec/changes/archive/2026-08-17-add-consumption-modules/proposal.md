# Consumption modules: a `safix.*` namespace inside NixOS and home-manager

Throughout this change `<dotfiles>` names the originating repository, the private nix-config flake safix was extracted from.
It is read-only here: nothing in this change edits it, and the swap that replaces its hand-written wiring is a separate change.

## Why

safix resolves.
It does not yet arrive.

`flake.safix.lib.materialize` produces the exact attrset the secret provisioner's option tree takes, and `extract-safix-from-dotfiles` proves it against sops-nix's own option types in both scopes.
What no one has is a way to say *put mine here*.
A consumer today writes the arrival themselves: import sops-nix, work out which host scope this profile is on, call the resolver with the right user and hostname, assign the result into `sops.secrets`, and — if they read the source closely enough — reproduce the identity preflight that makes a missing decryption key refuse the switch before anything is linked rather than halfway through it.

`<dotfiles>/modules/home/users/sernl/sops/default.nix` is that arrival written by hand, and it is 120 lines of which 67 are the rationale for four assignments.
Most of that rationale is not this consumer's: that a set-but-missing `age.keyFile` is fatal where a missing `sshKeyPath` is a warning, that sops-nix's own activation entry sorts after `linkGeneration`, that pinning it earlier restarts the previous generation's unit with no signal — these are facts about sops-nix, discovered once, at some cost, and true for everyone.
A package that resolves custody and then hands the consumer a bare attrset makes every consumer rediscover them.

The operator's requirement is that users can establish their secrets from anywhere in NixOS and home-manager.
The acceptance test is that the file above becomes roughly four lines with behaviour unchanged.

## What Changes

- Two new module outputs, each declaring a `safix.*` namespace inside the module system it serves, exactly as sops-nix declares `sops.*`: `homeModules.default` for a home-manager profile and `nixosModules.default` for a system configuration.
- The namespace is consumption only.
  `safix.user`, `safix.hostname`, `safix.tags`, `safix.identity.*` and `safix.enable` all name *which already-declared secrets arrive here*; none of them declares a secret, a recipient, or an audience.
  Custody stays at `flake.safix.*`, where the declarations of every user are visible at once, because a single profile cannot compute an audience and `.sops.yaml` is repository-global.
- The identity semantics the source repository paid for become the module's, with the guard in the same commit as the sentence describing it.
  `safix.identity.keyFile` defaults to null because sops-nix treats a set-but-missing key file as fatal, and `home.activation.safixIdentityPreflight` sorts before `checkLinkTargets` so a machine without a usable identity refuses the switch while refusing is still atomic.
- Both modules ship in two forms, and the reason is a hard fact about the nix module system rather than a taste: importing two distinct copies of one declaring module is an evaluation error, not a no-op.
  `homeModules.default` imports sops-nix for a consumer who has not; `homeModules.safix` declares the same namespace and imports nothing, for a consumer who already has sops-nix in their tree at a revision of their own choosing.
  `nixosModules` mirrors both.
- Resolver refusals surface as safix's own evaluation errors.
  A custody violation is reported by this module naming the declarations that broke, not as a stack trace from inside the provisioner's manifest generation.
- An empty resolved set is a no-op module: nothing defined, no activation entry, no unit.

Not in scope: any edit to `<dotfiles>`; any change to the custody namespace, the resolver, the policy renderer, or the CLI; darwin-specific arrival beyond what sops-nix already handles; and a system-scope preflight guard, because no atomic refusal point has been demonstrated at system activation and this change does not describe guarantees it has not built.

## Capabilities

### New Capabilities

- `secret-consumption`: the module-system-side namespace — what a profile names to receive its resolved secrets, what defaults from where, which wiring mistakes are refused at evaluation, the identity contract and its activation-ordering guard, and the no-op property of an empty resolution.

### Modified Capabilities

None.
`consumer-integration` states what safix refuses to assume about a consumer's tree; this change adds what safix does offer once a consumer has wired it, and the two do not overlap.

## Impact

Affected code:

- New: `modules/consume/common.nix`, `modules/consume/home.nix`, `modules/consume/nixos.nix` — the shared option declarations and the two scope-specific modules.
- Modified: `flake.nix` — four new module outputs, and `home-manager` joins `sops-nix` as a check-only input, since proving an activation entry sorts ahead of `checkLinkTargets` requires evaluating a real home-manager configuration.
- Modified: `README.md` — the quick start gains the import, and the consumption section becomes the option surface rather than a sketch of one.

Affected checks: `modules/flake/checks/consumption.nix` is new and carries the equivalence proof, the ordering proof, the scope asymmetry, the no-op property, and the module-collision fact the two-form export rests on.
Every claim gets a severity drill in `tasks.md`.
