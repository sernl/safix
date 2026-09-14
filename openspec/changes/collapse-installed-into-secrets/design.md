# Design: one option for what arrived at system scope

## Context

See `proposal.md` — Why, for the motivation.
Four facts fix the shape of this change; each was read against the checked-out worktree on 2026-09-15, and the symbol name rather than the line number is the anchor that survives drift.

`safix.secrets` is declared exactly once, in `common.sharedOptions` (`modules/consume/common.nix:379-392`), shared verbatim by both scopes, with `readOnly = true`, no `default`, and type `types.attrsOf types.raw`.
Each scope defines it exactly once, unconditionally, outside the enable gate (`modules/consume/home.nix:200-214`, `modules/consume/nixos.nix:84-107`), which is what makes `readOnly` satisfiable today.

`safix.installed` is declared only at system scope (`modules/consume/nixos.nix:72-74`, from `common.installedOptionFor` at `common.nix:207-218`), has no `readOnly` and a `default = { }`, and is defined inside `lib.mkIf cfg.enable` with the `<symlinkPath>/<name>` mint mapped over `cfg.secrets` (`nixos.nix:140-143`).
Its only production consumer is `installer.nix:144-149`, feeding `failedAssertions`, `manifest.secrets`, and `preflightRemediation`'s count.

`enable`'s default is `cfg.secrets != { }` (`common.nix:245-248`), and the system scope's own `safix.secrets` definition is `if resolved != { } && !hasIdentity then throw (common.noSystemIdentityMessage …) else resolved`, where `hasIdentity` deliberately excludes `sops.age.*` because safix defines those under the enable gate (`nixos.nix:93-97`).
That dependency chain — `secrets` → `enable` → the enable-gated block — is what constrains where the mint can live.

`common.resolvedFor` (`common.nix:449-468`) reaches the declarations through `cfg.lib.violations` and `cfg.lib.materialize args target`, and `materialize` is published at `flake.safix.lib.materialize` (`modules/flake/safix/default.nix:64,175`) as `args: resolve.materializeFor (bound args)`, a function of `args` then of the target configuration.
Both of this change's hardest problems — a read-only option a test fixture must nevertheless perturb, and an expected side that must not be the option under test — are solved by entering through that seam rather than through the option.

## Goals / Non-Goals

**Goals:**

Make one option, at each scope, the whole of what safix reports arrived, and make the manifest read that same option.
Keep every typing consequence the system scope gets today from the provisioner's entry type: the mode, owner, group, uid and gid coercions, the `sopsFile` default, the `sopsFileHash` default.
Keep `readOnly = true` at both scopes, and move the one fixture that depended on its absence to the resolver seam.
Keep `safix-installer-store` falsifiable — the collapse's own severity risk — by re-deriving its expected side from the declarations rather than from the option under test.
Keep the `<symlinkPath>/<name>` mint coupled to the store root, in one place, without introducing an evaluation cycle.

**Non-Goals:**

Changing the moment at which the no-system-identity `throw` fires, or the shape of any refusal. The throw fires when `safix.secrets` is forced to an attrset, before any element submodule is entered, so `fires = e: !(builtins.tryEval (builtins.deepSeq e e)).success` keeps working unchanged at its five existing sites.
Typing the user scope. `lib.types.attrsOf lib.types.raw` is what it passes, and `home.nix`'s `sops.secrets = cfg.secrets` is still where the provisioner's home-manager `secretType` applies.
Any rust change. Nothing under `crates/**` reads either option; every `installed` hit there is the English word.
Any new option, alias, or deprecation path. See C6.

## Decisions

### C1. One option per scope, typed by a `secretsType` argument, and `installedOptionFor` deleted

`common.sharedOptions`'s argument set gains `secretsType` beside `cfg`, `userDefault`, `userDefaultText`, `hostnameDefault`, `hostnameDefaultText`, and `secrets` becomes `mkOption { type = secretsType; readOnly = true; description = …; }`.
`nixos.nix` passes `secretsType = options.sops.secrets.type;` and its `options.safix` becomes one plain `sharedOptions` call again, with no `// { installed = …; }` suffix.
`home.nix` passes `secretsType = lib.types.attrsOf lib.types.raw;`.
`installedOptionFor` and its `inherit` entry are deleted.

The half of the comment at `common.nix:192-206` that remains true — that the provisioner's type carries the coercions and the `sopsFileHash` forcing but *not* the two store-and-existence refusals, which live in `manifest-for.nix:11-28` and are copied into `installer.nix` because safix does not call that builder — is load-bearing and moves to the `nixos.nix` call site, beside the argument that introduces the type.

**Alternative rejected**: branching on scope inside `sharedOptions`, e.g. a `scope` string and an `if`.
`sharedOptions`'s own comment (`common.nix:233-235`) states the rule the file already follows: what the scopes disagree about is passed in rather than branched on, so a scope that cannot derive a value says so in its own file.
A `secretsType` argument is that rule applied once more; an internal branch would be the first exception to it.

**Alternative rejected**: keeping `secrets` as `attrsOf raw` at both scopes and applying the provisioner's type inside `installer.nix`, where the manifest is built.
That reintroduces exactly the split this change removes — a consumer would read the untyped set while the installer read a typed one — and it moves the typing out of the option tree, so `options.safix.secrets.type` would stop being the honest answer to what the option holds.

**Alternative rejected**: `types.attrsOf (types.submodule …)` restated by safix at system scope instead of `options.sops.secrets.type`.
Restating is what the archived `own-secret-installer` change already rejected (`archive/2026-08-22-own-secret-installer/design.md:77-81`): a field the provisioner adds becomes a silent divergence rather than a failing check, and `safix-installer-manifest`'s parity comparison against the provisioner's own builder exists precisely to catch that.

### C2. The mint lives in the unconditional definition's `else` branch, over `resolved`

The `else resolved;` branch of the system scope's `safix.secrets` definition becomes:

```nix
else
  lib.mapAttrs (
    name: entry:
    entry // lib.optionalAttrs (!(entry ? path)) { path = "${cfg.installer.symlinkPath}/${name}"; }
  ) resolved;
```

and the comment at `nixos.nix:133-139` — the store-root-and-entry-default coupling, with its `main.go:254-268` citation — moves up to sit beside it.

Two properties make this the only correct placement, and both are cycles rather than preferences.
Mapping over `cfg.secrets` inside the definition of `safix.secrets` is a direct self-reference: forcing the option forces the definition, which forces the option.
Wrapping the definition in `lib.mkIf cfg.enable` is the same cycle one hop longer, because `enable`'s default reads `cfg.secrets`.
Mapping over the local `resolved` binding, outside any gate, closes over `common.resolvedFor` and `cfg.installer.symlinkPath` alone — the latter a `types.str` with a literal `/run/safix` default and no dependency on `safix.secrets`, `safix.enable`, or `sops.*` — so there is no cycle.
`entry ? path` is still tested on the pre-type value, because the test happens inside the definition, before the submodule's own `path` default could apply; that is unchanged from today.

Behavioural delta, accepted: with `safix.enable = false` over a non-empty resolution, `config.safix.installed` reads `{ }` today, whereas the collapsed `config.safix.secrets` reads the resolution with minted paths.
Nothing depends on the empty reading. `installer.nix:298` wraps the whole installer `config` in `lib.mkIf cfg.enable`, so `manifest`, `system.build.safix-manifest`, `system.activationScripts.safixInstallSecrets` and `systemd.services.safix-install-secrets` are all still absent, and the inert probes in `consumption.nix` read those surfaces rather than the option.
Task 5.3 holds that as a check rather than leaving it as this paragraph's assertion.

**Alternative rejected**: minting the path in `installer.nix`, where the manifest is built, and leaving `safix.secrets` as the bare resolution.
Then the option a consumer reads would not carry the path the entry actually arrives at, which is half of what the collapsed option is for, and the root-and-default coupling would live in the builder rather than beside the option — the arrangement whose failure mode `main.go:254-268` makes a silent write into another store's directory.

### C3. `readOnly = true` is kept, and the one fixture that needed its absence enters through `safix.lib`

nixpkgs refuses a read-only option set more than once after `mkIf`/`mkMerge` resolution (`lib/modules.nix:1157-1172`), which is exactly what lets `checks/installer.nix:725-736`'s `refusalFixtureWith` write `safix.installed = entry;` today: that option carries no `readOnly`.
Collapsing onto `safix.secrets` makes that fixture throw, and the fix is not to drop `readOnly` — the option is a projection of `flake.safix.*` and saying so is the honest statement — but to inject one layer lower, at the seam the module itself reads.

`refusalFixtureWith` becomes a fixture that sets `safix.lib = { violations = [ ]; materialize = _args: _cfg: entry; }`, `safix.user`, `safix.hostname`, and `safix.identity.sshKeyPaths`, so `common.resolvedFor` returns `entry` and the no-system-identity `throw` does not pre-empt the refusal under test.
`cfg.lib.subjects` is not needed: `tags`' default reads it only when `safix.machine` is set (`common.nix:361-364`), and this fixture is person-shaped.
The fixture's own comment records that it enters through the resolver seam *because* the option is read-only, which is the point of the collapse rather than a workaround for it.

**Alternative rejected**: dropping `readOnly` from `secrets` so the existing fixture keeps working.
That weakens a real guarantee at both scopes — a consumer could then define entries into a projection and have them silently merge with the resolution — to accommodate one test fixture, which is the tail wagging the dog.

**Alternative rejected**: keeping a separate writable option purely as the refusal fixture's entry point.
That is `safix.installed` under another name, and the surface would not shrink at all.

### C4. `safix-installer-store`'s expected side is re-derived from `materialize`

`entryPathContract` (`checks/installer.nix:429-438`) computes, per name, `entry.path or "${symlinkPath}/${name}"` over `fixture.config.safix.secrets`, and `safix-installer-store` (`:1051-1103`) diffs it against `jq '.secrets | map({(.name): .path}) | add'` of the built manifest — which is built from `safix.installed`.
The check is two independent computations of one contract, which is what its own comment at `:429-433` claims ("one claim rather than two").
Collapse the options and both sides become the same value: `entry.path or …` always takes the `entry.path` branch, and the diff compares the manifest with itself.
It stays green and becomes unfalsifiable.

The expected side therefore moves one layer below the module, to the same call `resolvedFor` makes:

```nix
entryPathContract =
  fixture:
  lib.mapAttrs (
    name: entry: entry.path or "${fixture.config.safix.installer.symlinkPath}/${name}"
  ) (
    config.flake.safix.lib.materialize {
      user = "bob";
      machine = null;
      hostname = "server";
      tags = [ ];
      scope = "system";
    } fixture.config
  );
```

with `symlinkPath` still read off the fixture's own option, so the moved-roots fixture still measures the coupling at the option rather than at a literal.
This is an oracle, not a re-implementation: it reads the resolution before any module minted anything, which is what the check's comment already says it does.

**Alternative rejected**: a literal expected map written into the check.
A literal cannot follow `movedFixture`'s moved `symlinkPath`, so the root-and-default coupling — the one thing this check exists for — would stop being measured, and the literal would have to be hand-updated whenever the fixture fleet changes.

**Alternative rejected**: accepting the tautology and relying on `safix-installer-manifest`'s parity comparison for coverage.
Parity compares field *sets* against the provisioner's builder, never path values, so nothing would hold the mint at all; removing it from `nixos.nix` would leave every check green.
Task 1.9's drill is the evidence that this is repaired rather than assumed.

### C5. The deep-force hazard the submodule type introduces, and its disposition per fixture

`fires` uses `builtins.deepSeq`. Under `attrsOf raw` a deep force walks the resolver's own values; under the provisioner's entry type it additionally forces each entry's `sopsFileHash`, whose default is `builtins.hashFile "sha256" config.sopsFile` under `sops.validateSopsFiles`.
For a fixture where the resolver does *not* throw and `validateSopsFiles` is left at its `true` default, `fires` could then become true for a `hashFile` reason rather than for safix's reason.

Disposition, per site, all five enumerated:

| Site | Today | After |
|---|---|---|
| `checks/installer.nix:452`, `:777` (`fires identityFreeSystem.config.safix.secrets`) | fixture sets `sops.validateSopsFiles = false` (`:764`) | unaffected; no edit |
| `checks/portability.nix:243`, `:417` (`fires (nixosFor …).safix.secrets`) | `nixosFor` does not set `validateSopsFiles`; probes run only over `broken` fleets, where the resolver throws first | add `sops.validateSopsFiles = false;` to `nixosFor`, so the probe's anti-vacuity margin does not depend on the resolver winning a race with a file hash |
| `checks/consumption.nix:243`, `:782`, `:808` | user scope, `attrsOf raw` either way | unaffected; no edit |

**Alternative rejected**: narrowing the portability probes to `builtins.attrNames (…).safix.secrets` under `tryEval` instead of setting `validateSopsFiles`.
That would work — the throw fires before any element is entered — but it changes what the probe measures from "nothing in this resolution evaluates" to "the attribute names do not evaluate", weakening a check to work around a fixture default when the fixture default is the thing that is wrong.

### C6. No alias, no `visible = false`, no deprecation warning

`safix.installed` is deleted. A consumer reading it afterwards gets the module system's own error naming the option.

**Alternative rejected**: retaining it with `visible = false` and a definition equal to `secrets`.
The point of the change is that the surface shrinks; a hidden option is still an option, and `options.safix ? installed` would still be true, so every check that reads it would keep passing and the collapse would be undemonstrated.
A loud evaluation error naming the option is also a better migration than a silently-still-working read: the consumer learns the name changed at the moment they evaluate, not later.

**Alternative rejected**: `mkRemovedOptionModule`.
It buys a nicer message for one option in a package whose entire nix surface is documented in one table and one changelog, at the cost of a permanent module in the tree; the `[Unreleased]` note carries the same information where a consumer already looks for breaking nix changes (`CHANGELOG.md:17-19`).

### C7. What the documentation must now say, and the one comment that becomes false

`README.md:783`'s option-table row can be accurate at both scopes for the first time, and no row is added — the visible win.
`README.md:869-895`, the system-scope installer narrative, says `sops.secrets` stays empty and never says where to look instead; it gains one sentence naming `config.safix.secrets` as both the readable surface and the manifest's own source, beside the `safix-installer-sole` sentence.

`checks/entrypoints.nix:21-23` records *which* option's unforced `type` thunk makes bare evaluation work under `_module.check = false`, and it names `installed`.
The mechanism is unchanged — it is still one unforced `type` field on one option in the same tree — so the sentence is rewritten to name `secrets` and the claim is kept.
`checks/consumption.nix:415-419`'s comment explains the choice of `installed` over `sops.secrets`; after the collapse there is one safix option and it must say so.

## Capabilities considered and not touched

`secret-consumption` — no requirement names either option; its `:178`/`:190` hits are the English word.
Its "Selection only" requirement, that each consumer option names which resolved set arrives or how the machine decrypts it, is better satisfied afterwards, since the surface loses an option that is neither selection nor decryption.

`consumer-integration` — names neither `safix.secrets` nor `safix.installed`, and the module entrypoints, the `.default`/`.safix` split, and the zero-flake claim are all untouched by this change.

`secret-catalogue`, `secret-custody`, `recipient-policy`, `public-outputs`, `secrets-vault`, `generated-secrets` — nothing in any of them reads a consumption-scope option.

## Migration Plan

One rename, no data and no state: a consumer reading `config.safix.installed` reads `config.safix.secrets` instead, and gets the same attribute set with the same typing and the same minted paths.
A consumer reading `config.safix.secrets` for the untyped resolution at system scope now gets it typed and path-carrying, which is additive per entry — every field that was there is still there — unless they were relying on an entry *not* having a `path` attribute, which the `[Unreleased]` note states.
Nothing to migrate at user scope.

## Risks / Trade-offs

The collapse's own severity risk is C4's tautology, and the mitigation is a drill rather than a claim: task 1.9 removes the mint from `nixos.nix` and requires `safix-installer-store` to redden.
If that drill does not reproduce, the check is vacuous and the change is not done.

The system scope's `safix.secrets` now forces `sopsFileHash` under a deep force, so a fixture that neither throws nor disables `validateSopsFiles` pays a `hashFile` per entry and can report a `fires` for the wrong reason.
C5 disposes of every existing site; a *new* fixture added later carries the same obligation, which is why the `validateSopsFiles = false` line added to `nixosFor` carries a comment saying why rather than standing bare.

The archived `own-secret-installer` change's design and tasks describe the two-option arrangement and remain in the tree.
They are accurate history as of their own change and are not edited; the `[Unreleased]` note is where a reader learns the arrangement changed, and `CHANGELOG.md:101-102` is deliberately left as written.
