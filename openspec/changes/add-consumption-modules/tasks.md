# Tasks: add-consumption-modules

`<dotfiles>` names the originating repository, and every path under it is relative to that repository's root.
It is read-only for the whole of this change.

Two disciplines hold throughout.
No real recipient, no real hostname, and no real user name from any fleet enters this repository; fixtures use `ana`, `bo`, `cy` and synthetic `age1` strings.
Nothing here deploys, switches, or activates anything; every verification builds or evaluates.

A third discipline is specific to this change.
No sentence describing a guarantee is written before the code enforcing it exists in the same commit — the activation ordering in particular, whose whole value is that the message is true.

## 1. The module-collision fact the export shape rests on

- [x] 1.1 Add a check that evaluates two byte-identical copies of one option-declaring module at two distinct paths and asserts the evaluation fails, and that the same module imported twice at one path succeeds
- [x] 1.2 Assert the same for the secret provisioner's own module rather than a synthetic one, by importing the provisioner's home-manager module both directly and through a second copy of its directory in the store
- [x] 1.3 Severity drill: reducing the check to the same-path case alone turns it green against a module system that had started rejecting the idempotent import, which is the failure it exists to catch; assert both halves
- [x] 1.4 Verify: the check is red when its expectation is inverted

## 2. The shared consumption vocabulary

- [x] 2.1 Write `modules/consume/common.nix` declaring the options both scopes share — the binding to the consumer's declarations, the person, the host, the tags, the enable flag, and the read-only resolved set
- [x] 2.2 Make the binding option's default throw a message naming the option and the likely cause when handed a flake that carries no resolver projection
- [x] 2.3 Make the resolved set empty whenever the binding, the person, or the host is unset, so the assertions in 2.4 are reachable rather than pre-empted by a resolution that throws
- [x] 2.4 Add assertions for each of those three unset states, naming the option of safix's namespace that is wrong
- [x] 2.5 Check the resolver's violation list before materializing, and throw safix's own message carrying every violation rather than letting the resolver's first one surface from inside the provisioner
- [x] 2.6 Default the enable flag to whether the resolved set is non-empty, and gate the whole of each module's configuration on it
- [x] 2.7 Verify: `nix eval` of a fixture profile that sets nothing but the import produces an empty resolved set and no error

## 3. The home-manager module

- [x] 3.1 Write `modules/consume/home.nix`: the person defaulting to the profile's own username, the host defaulting from `osConfig` where home-manager is evaluated as a NixOS module and null otherwise, and the resolved set materialized at user scope
- [x] 3.2 Declare the identity options, with the key file defaulting to null and the option description carrying the provisioner's fatality asymmetry as fact — set-but-unreadable key file aborts, missing ssh key path is skipped with a warning
- [x] 3.3 Define the identity onto the provisioner's options at normal priority, so a `mkDefault` elsewhere in a consumer's tree loses and a plain definition conflicts loudly
- [x] 3.4 Port the identity preflight from `<dotfiles>/modules/home/base/sops.nix`, keeping the required/sufficient partition, the gnupg-source condition that decides when the ssh key paths stop being load-bearing, and the remediation text
- [x] 3.5 Sort the preflight `entryBefore [ "checkLinkTargets" ]` and gate it on a non-empty resolved set and a non-empty identity
- [x] 3.6 Rewrite the preflight's guarantee paragraph so every sentence in it is one this repository's checks hold, and state its limit: presence and readability were checked, decryption was not
- [x] 3.7 Verify: a fixture home-manager configuration evaluates, and its resolved set matches the fixture fleet's expectation for that person on that host

## 4. The NixOS module

- [x] 4.1 Write `modules/consume/nixos.nix`: the host defaulting to the configuration's own hostname, the person required, and the resolved set materialized at system scope
- [x] 4.2 Declare the same identity options, defining them onto the provisioner's system-scope equivalents only where the consumer set them, so the provisioner's own host-key default survives untouched
- [x] 4.3 Install no activation guard, and record in the module header that no atomic refusal point at system activation has been demonstrated
- [x] 4.4 Verify: a fixture NixOS configuration evaluates and its resolved set carries the ownership fields the same declarations refuse at user scope

## 5. Exports

- [x] 5.1 Export `homeModules.safix` and `nixosModules.safix` as the two modules from sections 3 and 4, importing nothing
- [x] 5.2 Export `homeModules.default` and `nixosModules.default` as those modules plus the provisioner's own module for their scope
- [x] 5.3 Add `home-manager` as a check-only input, following this flake's nixpkgs, with the reason recorded in `flake.nix` beside the same note `sops-nix` carries
- [x] 5.4 Verify: all four module outputs are listed — `nix eval .#homeModules` and `.#nixosModules` each name `default` and `safix`

  `nix flake show` could not be the instrument, for a reason that predates this change and is not about it.
  It evaluates every system in `systems`, which includes `x86_64-darwin`, and the pinned nixpkgs has dropped that platform: "Nixpkgs 26.11 has dropped support for x86_64-darwin".
  `packages.x86_64-darwin.safix`, an output older than any consumption module, fails identically, and `nix flake check` never sees it because it checks the current system only.
  Carried out of this change as a question about the declared platform matrix.

## 6. The equivalence proof

- [x] 6.1 Write `modules/flake/checks/consumption.nix` evaluating a fixture home-manager configuration in the four-line consumer form against the same fixture fleet the materialization check uses
- [x] 6.2 Beside it, evaluate a second configuration wiring the resolver by hand in the shape of `<dotfiles>/modules/home/users/sernl/sops/default.nix` — the resolver call assigned into `sops.secrets`, the identity set directly on the provisioner
- [x] 6.3 Assert the two configurations' `sops.secrets` are equal as read back through the provisioner's own option types, entry by entry, not as the attrsets safix handed them
- [x] 6.4 Severity drill: changing the module's materialization scope, dropping a field from what it emits, or resolving for a different host each turns the equivalence red
- [x] 6.5 Verify: the check is red when the module form is pointed at a different person

## 7. The ordering proof

- [x] 7.1 Topologically sort the fixture profile's activation DAG with the module's entry present, and assert this package's entry precedes `checkLinkTargets`
- [x] 7.2 Assert the provisioner's own entry does not, so the check records the asymmetry the guard exists for rather than only the guard's own position
- [x] 7.3 Extract the preflight script from the evaluated profile and assert it names every configured identity path and exits non-zero, without running it
- [x] 7.4 Severity drill: replacing `entryBefore [ "checkLinkTargets" ]` with a bare string turns 7.1 red; dropping one identity path from the script turns 7.3 red
- [x] 7.5 Verify: both drills observed red before the expectations are restored

## 8. The scope and no-op proofs

- [x] 8.1 Evaluate a fixture NixOS configuration and a fixture home-manager configuration over one fleet, and assert each establishes exactly what the scope-free resolver selected for its person

  Written as "the same names" for both, which is not the claim that survived contact with the fixture.
  ana's `ana-alone` declares its `path` as a function of `home.homeDirectory`, so her set is materializable in a profile and not in a system configuration — a property of that declaration and of the `path`-is-a-function-of-the-consuming-configuration contract, not of safix.
  So the system side resolves bo, and `selectionIsScopeFree` carries the claim that does hold on each side independently: what arrives at a scope is exactly what the scope-free resolver selected.
  The unmaterializable case is not asserted, because it surfaces as a missing attribute rather than a throw and `builtins.tryEval` catches neither.
- [x] 8.2 Assert an entry carrying ownership reaches the system configuration with it, and that the same entry at user scope fails evaluation
- [x] 8.3 Evaluate a fixture profile for a person who resolves nothing on that host, and assert its activation entries carry no entry from this package and no provisioning unit exists
- [x] 8.4 Assert the same profile still evaluates green rather than failing, since inert and broken must not look alike
- [x] 8.5 Severity drill: removing the enable gate from the module turns 8.3 red; dropping the user-scope ownership refusal turns 8.2 red
- [x] 8.6 Verify: `nix build` of each new check green

## 9. Documentation

- [x] 9.1 Rewrite the README's consumption section as the option surface of both modules, with the four-line form and the fifth line the standalone case needs
- [x] 9.2 Add the quick start's import, saying which of the two forms to pick and why, with the collision error quoted from the check that holds it
- [x] 9.3 Document the identity contract, the activation guard, exactly what it guarantees, and the system-scope asymmetry with its reason
- [x] 9.4 Verify: every guarantee stated in the README names a check in this repository that holds it

## 10. Verification

- [x] 10.1 `openspec validate add-consumption-modules --strict`
- [x] 10.2 `nix flake check` green
- [x] 10.3 `rg` the whole tree for any real fleet identifier and confirm none
- [x] 10.4 Confirm no file under `<dotfiles>` was modified
