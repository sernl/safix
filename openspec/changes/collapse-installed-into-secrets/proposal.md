# One option for what arrived at system scope

## Why

A system configuration today reports what safix resolved twice: `config.safix.secrets` holds the resolution untyped (`modules/consume/common.nix:379-392`), and `config.safix.installed` holds the same set typed by the secret provisioner's own entry type with safix's `<symlinkPath>/<name>` path minted over it (`modules/consume/nixos.nix:72-74`, `:127-143`).
Only the second is what safix's installer manifest is built from (`modules/consume/installer.nix:144-149` is its single production consumer), and it is the one the README never names — `safix.installed` appears nowhere in `README.md`, whose option table points a consumer at `safix.secrets` (`README.md:783`).
So the option a consumer is told to read is not the option the installer uses, and nothing holds the two together: they are one set read twice, and the surface carries the cost of both.

## What Changes

- **BREAKING** (nix surface): `config.safix.installed` is deleted outright — no alias, no `visible = false` shim.
  A consumer reading it afterwards gets an evaluation error naming the option rather than a silent empty set, which is the failure mode worth having.
- `config.safix.secrets` becomes the whole of what a scope reports: still `readOnly = true`, still declared exactly once per scope in `sharedOptions`, but now typed per scope through a new `secretsType` argument to `common.sharedOptions`.
  The system scope passes `options.sops.secrets.type`, so the resolution arrives carrying the provisioner's mode, owner, group, uid and gid coercions, its `sopsFile` default and its `sopsFileHash` default — exactly what `safix.installed` carried.
  The user scope passes `lib.types.attrsOf lib.types.raw`, which is what it has today, and its typing still happens downstream where `home.nix` assigns `sops.secrets = cfg.secrets`.
- The `<symlinkPath>/<name>` path mint for an entry that declares no path of its own moves into the existing unconditional `safix.secrets` definition at `modules/consume/nixos.nix:84-107`, over the local `resolved` binding rather than over `cfg.secrets`, and stays outside `lib.mkIf cfg.enable`.
  Mapping over `cfg.secrets` inside its own definition, or gating that definition on `cfg.enable` whose default reads `cfg.secrets`, are both infinite recursion; the `else` branch over `resolved` is neither.
- `installer.nix` reads `config.safix.secrets`, so the option a consumer reads and the set the manifest is built from are provably the same value rather than two computations of one contract.
- `safix-installer-store`'s expected side is re-derived from an independent oracle — `config.flake.safix.lib.materialize`, the same call `common.resolvedFor` makes, read before any module minted anything — because the collapse otherwise makes that check diff the manifest against itself and go silently vacuous while staying green.
- `checks/installer.nix`'s `refusalFixtureWith` re-enters through `safix.lib` (a settable `types.raw` seam) instead of defining `safix.installed`, since a second definition of a `readOnly` option is a module-system throw.
- The four check files that read `.safix.installed` (`consumption.nix`, `installer.nix`, `portability.nix`) follow the rename; `entrypoints.nix`'s load-bearing comment, which records *which* option's unforced `type` thunk makes bare evaluation work, is rewritten to name `secrets`.
- `README.md`'s option-table row becomes accurate at both scopes for the first time, and the system-scope installer narrative gains the sentence it is missing: which option to read instead of the provisioner's empty `sops.secrets`.

Non-goals:

- No change to the user scope's observable behaviour. Its `safix.secrets` stays the untyped projection and `sops.secrets` stays where it lands.
- No compatibility alias, deprecation warning, or `visible = false` retention of `safix.installed`. The breaking note in `CHANGELOG.md` is the migration.
- No rewrite of `CHANGELOG.md:101-102`, the released `own-secret-installer` entry that names both options. It was accurate when written; the new `[Unreleased]` note supersedes it.
- No new `Finding`, refusal, or rust change of any kind: no code under `crates/**` reads either nix option.
- No new capability spec, and no edit to `secret-consumption` or `consumer-integration`, neither of which names either option.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `secret-installation`: the requirement "safix installs its resolved set itself at system scope", scenario "What a consumer reads to see what arrived", asserts today that two options are readable in this package's namespace; it becomes one option, typed and path-carrying, stated to be the same one the manifest is built from.
  The requirement "The secret store is this package's own and is named nowhere else", scenario "An entry that declares no path parks in this package's store", keeps its one-check coupling of the store root and the entry default, and gains the statement that the check's expected side is derived independently of the option under test — without which the collapse makes that scenario unfalsifiable.

## Impact

Affected production nix (4 files):

- `modules/consume/common.nix` — `installedOptionFor` (`:207-218`) and its `inherit` entry (`:229`) are deleted; the comment block at `:192-206` that records what the provisioner's type does and does not carry is folded into the surviving description and the nixos call site; `sharedOptions` (`:236-243`) gains `secretsType`; `secrets` (`:379-392`) uses it.
- `modules/consume/nixos.nix` — `:60-74` passes `secretsType = options.sops.secrets.type;` and drops the `// { installed = …; }` suffix; `:84-107` folds the mint into the `else` branch; `:127-143` loses the `safix.installed` definition, leaving the enable-gated block as the `sops.age.*` half alone.
- `modules/consume/home.nix` — passes `secretsType = lib.types.attrsOf lib.types.raw;` and nothing else.
- `modules/consume/installer.nix` — `:144-149` reads `config.safix.secrets`; its comment, which distinguishes the typed set from the raw resolution, no longer has two sets to distinguish.

Affected checks (4 files):

- `modules/flake/checks/installer.nix` — the parity-manifest argument (`:399`), `soleFacts.established` (`:861`), `refusalFixtureWith` (`:725-736`, rebuilt through `safix.lib`), and `entryPathContract` (`:429-438`, re-derived from `materialize`).
- `modules/flake/checks/consumption.nix` — `systemView` (`:415-419`) and its comment.
- `modules/flake/checks/portability.nix` — `person` (`:350`), `machine` (`:361`), `machineRaw` (`:377`), `serviceOwnership.system` (`:543`).
- `modules/flake/checks/entrypoints.nix` — the comment at `:21-23`.

Affected docs: `README.md:783` and `README.md:869-895`; `CHANGELOG.md`'s `[Unreleased]` section.

Not touched: `openspec/changes/archive/2026-08-22-own-secret-installer/{design.md,tasks.md}`, which record this arrangement accurately as of their own change; `crates/**`; `examples/**`; every other check file, none of which names either option.
