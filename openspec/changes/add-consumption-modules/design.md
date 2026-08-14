# Design: the consumption modules

## Context

`flake.safix.lib.materialize { user; hostname; tags; scope; } cfg` already returns the attrset sops-nix's `sops.secrets` takes, in either scope, and `safix-materialization` holds that claim against sops-nix's own option types.
What is missing is everything between that function and a running profile: who calls it, with which arguments, where the result is assigned, and what happens when the machine cannot decrypt what arrives.

The originating repository answers all four by hand in `<dotfiles>/modules/home/users/sernl/sops/default.nix`, and reading it is the whole brief.
Four assignments — `defaultSopsFile`, `age.keyFile = null`, `age.sshKeyPaths`, `secrets = (resolver …).sops` — carry 67 lines of rationale, and the rationale divides cleanly.
One part is that consumer's: which file its secrets live in, which key its people hold.
The other part is about sops-nix, and is true for every consumer that ever writes this file: that `sops-install-secrets` treats a set-but-unreadable `age.keyFile` as fatal while it writes a missing `sshKeyPath` to stderr and continues; that sops-nix registers its activation entry as a bare string, which home-manager treats as `entryAnywhere` and sorts 21st of 22 on the fleet it was measured on, after `linkGeneration` and `reloadSystemd`; that pinning that entry earlier is not available as a fix, because the unit it restarts is materialized by `linkGeneration` and made visible by `reloadSystemd`, so an early restart aborts on the first switch and thereafter silently restarts the previous generation's unit.

The second part is what this change moves into safix, along with the guard `<dotfiles>/modules/home/base/sops.nix` built from it: a read-only preflight, `entryBefore [ "checkLinkTargets" ]`, that checks the configured identity for presence and readability and refuses the switch before anything is linked.

## Goals / Non-Goals

Goals.
Make arrival declarative in the module system the profile is written in, so a secret is established from anywhere in NixOS or home-manager by naming which resolved set this profile serves.
Move every sops-nix fact that is not consumer-specific out of consumers and into safix, together with the code enforcing it.
Keep the consumption surface free of anything that declares custody.
Make a wiring mistake an evaluation error that names safix, not a stack trace from inside someone else's manifest generation.

Non-Goals.
Any system-scope activation guard: no atomic refusal point at NixOS activation has been demonstrated, and a guarantee is not documented before the code enforcing it exists.
Any migration of `<dotfiles>`, which is a separate change.
Replacing sops-nix's own options: `sops.*` stays exactly where it is and remains directly settable; `safix.*` sits above it and defines a subset of it.

## Decisions

### D1. Consumption options live in the machine's module system; custody stays at flake level

`safix.user`, `safix.hostname`, `safix.tags`, `safix.identity.*` and `safix.enable` answer one question: which already-resolved set arrives in this profile.
None of them can declare a secret, a recipient, a grant, or an audience.

The split is forced, not stylistic.
An audience is a function of every user's declarations at once — `sharedWith` on one profile widens the file another profile reads — so a single machine's module system is structurally incapable of computing one.
`.sops.yaml` compounds it: the recipient policy is one repository-global file that the sops CLI reads off disk, and a per-machine declaration could not contribute a rule to it without every machine being evaluated to render it.
So custody declarations stay at `flake.safix.*`, where every user is visible simultaneously, and the modules consume the resolved sets and never declare into them.

### D2. The consumer's own flake is the module's input, named once

A home-manager module receives `config`, `lib`, `pkgs` and whatever the consumer put in `extraSpecialArgs`.
It does not receive the flake that evaluated it, and safix cannot require a particular special arg without making every consumer's evaluation seam part of safix's interface — the same refusal `consumer-integration` already makes about user registries.

So the wiring is one option.
`safix.flake` takes the consumer's own `self`; `safix.lib` defaults to `safix.flake.safix.lib`, which exists because flake-parts publishes `config.flake.safix` as the flake output `safix`, and is settable directly for a consumer whose seam differs.
That is the one line the acceptance test's four does not already contain, and it is stated rather than sniffed out of the module arguments, because a fallback chain across `self`, `flake` and `inputs` would guess at three conventions and be wrong for the fourth silently.

If `safix.flake` is set to something without a `safix.lib` attribute, the default throws naming the option and the likely cause — a flake that has not imported `inputs.safix.flakeModules.default`.

### D3. Each module ships in two forms, because duplicate module imports are an error

The obvious friendly choice is for `homeModules.default` to import sops-nix so a consumer needs one import instead of two.
The obvious hazard is a consumer who already imports sops-nix, at their own revision.

Nix module imports deduplicate by key, and for a module given as a path the key is that path.
The same store path imported twice is therefore idempotent.
Two *different* store paths declaring the same options are not idempotent and not a warning; they are a hard evaluation error, measured directly:

```
lib.evalModules { modules = [ ./a.nix ./b.nix ]; }   # b.nix a byte-identical copy of a.nix
error: The option `demo.thing' in `…/a.nix' is already declared in `…/b.nix'.
```

`sops-nix.homeManagerModules.sops` and `sops-nix.nixosModules.sops` are both paths, so a consumer whose `sops-nix` input `follows` safix's — or is coincidentally the same revision — resolves to one store path and is safe.
A consumer pinning their own revision is not, and there is no configuration that can repair it after the fact: `imports` cannot depend on an option, so a `safix.importProvisioner` flag is not expressible.

The decision is therefore to make it an import-time choice by shipping both module values.
`homeModules.default` and `nixosModules.default` import sops-nix; `homeModules.safix` and `nixosModules.safix` are the same modules with no imports at all, for a tree that already has sops-nix in it.
The README says which to pick and why, and the collision fact is held by a check rather than asserted in prose, so the day the module system starts merging identical declarations the documentation stops being true out loud.

### D4. `safix.enable` defaults to whether anything resolved, and everything is gated on it

An empty resolved set produces no `sops.secrets`, no identity configuration, no activation entry and no unit.
That is not an optimization; sops-nix gates its entire config block on `lib.mkIf (cfg.secrets != { })` (`modules/home-manager/sops.nix:322` at the locked revision), so a module that unconditionally set `sops.age.*` would define options into a block that never materializes, and would still register safix's own activation entry on a profile with nothing to decrypt.

`safix.enable` therefore defaults to `safix.secrets != { }`, and the module's whole `config` is `mkIf cfg.enable`.
`safix.secrets` is read-only and is the materialization, or `{ }` when the module is unwired — which is what keeps the default non-circular.

### D5. Wiring mistakes are refused before the resolution is forced

A resolution that throws inside sops-nix's manifest generation produces a trace whose top frame belongs to sops-nix.
Three mistakes are cheap to catch earlier and are caught by assertions instead: `safix.lib` set with no `safix.user`, `safix.lib` set with no `safix.hostname`, and a `safix.flake` that carries no `safix.lib`.

For those assertions to be reachable, the resolution must not throw first.
`safix.secrets` is therefore `{ }` whenever `lib`, `user` or `hostname` is unset, which makes `enable` false, which makes the module inert, which lets the assertion be the thing that speaks.

Custody violations are handled one layer in.
`flake.safix.lib.violations` is the full refusal list and is computable without resolving anything, so the module checks it before calling `materialize` and throws its own message carrying every violation.
The resolver would have thrown the first one from inside the provisioner's evaluation of `sops.secrets`; this reports all of them, from safix.

### D6. The identity contract is safix's, and the guard ships in the same commit as the sentence

`safix.identity.keyFile` is `nullOr path`, default null.
The default is not a preference: `sops-install-secrets` aborts on a set-but-unreadable key file, and skips a missing ssh key path with a line to stderr, so a non-null default would abort activation on every machine lacking the path while an empty `sshKeyPaths` costs nothing.
`safix.identity.sshKeyPaths` is a list, default empty.

Both are defined at normal priority onto `sops.age.*`, which means a consumer's `mkDefault` elsewhere loses to safix and a consumer's plain definition conflicts loudly.
That reproduces exactly what `<dotfiles>` gets today, where the per-user file's plain `keyFile = null` overrides the base module's `mkDefault`.

The guard is `home.activation.safixIdentityPreflight`, ported from `<dotfiles>/modules/home/base/sops.nix` and unchanged in substance.
It partitions the configured identity into the *required* one — a non-null `keyFile` that `generateKey` will not create, whose absence is individually fatal — and the *sufficient* ones — the ssh key paths, which are load-bearing only collectively and only while they are the sole identity source, since a gnupg source decrypts on its own.
It reads, it does not decrypt, and it is absent entirely when there is no identity to check.
It sorts `entryBefore [ "checkLinkTargets" ]`, and that ordering is what the guarantee in its own failure message rests on, so a check reads the sorted activation DAG off a real home-manager evaluation and holds the index.

The system scope gets no such guard, and the README says so plainly.
sops-nix's NixOS module already defaults `age.sshKeyPaths` to the host key, so the common failure does not arise; and no atomic refusal point at system activation has been demonstrated, so claiming one would be documenting a guarantee that no code enforces.

### D7. Scope is a property of the module, never of a declaration

`homeModules` materialize at `scope = "user"` and `nixosModules` at `scope = "system"`.
Nothing a consumer writes names a scope, which is the `one declaration serves both scopes` requirement holding at the arrival end too.
The asymmetry the resolver already enforces — ownership fields carried at system scope, refused rather than dropped at user scope — is inherited unchanged, and the check evaluates both modules over one fixture fleet to show that the same declaration reaches both.

### D8. `safix.hostname` defaults from the host where a host is knowable

The resolver is host-scoped: `perHost` and `perTag` adjustments are the reason it takes a hostname at all.
A NixOS module knows its hostname, so `safix.hostname` defaults to `config.networking.hostName`.
A home-manager module knows it only when home-manager is being evaluated as a NixOS module, where `osConfig` is in scope; standalone, it does not, and there is no honest default, so it defaults to null and the assertion in D5 asks for it.

`<dotfiles>` derives it from its own `configName` through a repository-local helper, which is exactly the kind of thing safix must not read — so the tandem swap sets `safix.hostname` from that helper explicitly, and that is the fifth line of the four-line form.

## Risks / Trade-offs

The `safix.flake` line is a real cost, and it is the price of not depending on a special argument.
It is also the only place a consumer can get the wiring wrong, and D5 makes that failure name the option.

Shipping two forms of each module doubles the import surface and asks the consumer a question they would rather not be asked.
The alternative is a module that works until the consumer bumps their own sops-nix input, then fails with an error naming two store paths and neither of them safix.

Defining `sops.age.*` at normal priority will conflict with a consumer who also defines them at normal priority.
That is deliberate — the alternative is `mkDefault`, under which safix's null `keyFile` would silently lose to an existing base module's XDG default and re-arm the abort the null exists to prevent.

The preflight is a presence-and-readability check, not a decryption.
A key that exists, is readable, and is not a recipient of the file still fails later, in `sops-install-secrets`, and the check's own message says so rather than implying more.

## Migration Plan

None required in this repository: the module outputs are additive and no existing output changes.
For `<dotfiles>`, the swap is a separate change and this one is written so that it is an import, `safix.flake`, `safix.user`, `safix.hostname`, and `safix.identity.sshKeyPaths`, with `modules/home/base/sops.nix` losing its preflight to safix's.

## Open Questions

Whether `safix.identity` should eventually carry the gnupg sources too.
The guard already reasons about them, because it must in order to know when the ssh key paths stop being load-bearing, but it reasons about `sops.gnupg.*` as set by the consumer rather than about a `safix.identity.gnupg` it owns.
Left alone until a consumer wants it.
