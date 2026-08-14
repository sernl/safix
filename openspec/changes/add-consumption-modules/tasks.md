# Tasks: add-consumption-modules

`<dotfiles>` names the originating repository, and every path under it is relative to that repository's root.
It is read-only for the whole of this change.

Two disciplines hold throughout.
No real recipient, no real hostname, and no real user name from any fleet enters this repository; fixtures use `ana`, `bo`, `cy` and synthetic `age1` strings.
Nothing here deploys, switches, or activates anything; every verification builds or evaluates.

A third discipline is specific to this change.
No sentence describing a guarantee is written before the code enforcing it exists in the same commit — the activation ordering in particular, whose whole value is that the message is true.

## 1. The module-collision fact the export shape rests on

- [ ] 1.1 Add a check that evaluates two byte-identical copies of one option-declaring module at two distinct paths and asserts the evaluation fails, and that the same module imported twice at one path succeeds
- [ ] 1.2 Assert the same for the secret provisioner's own module rather than a synthetic one, by importing the provisioner's home-manager module both directly and through a second copy of its directory in the store
- [ ] 1.3 Severity drill: reducing the check to the same-path case alone turns it green against a module system that had started rejecting the idempotent import, which is the failure it exists to catch; assert both halves
- [ ] 1.4 Verify: the check is red when its expectation is inverted

## 2. The shared consumption vocabulary

- [ ] 2.1 Write `modules/consume/common.nix` declaring the options both scopes share — the binding to the consumer's declarations, the person, the host, the tags, the enable flag, and the read-only resolved set
- [ ] 2.2 Make the binding option's default throw a message naming the option and the likely cause when handed a flake that carries no resolver projection
- [ ] 2.3 Make the resolved set empty whenever the binding, the person, or the host is unset, so the assertions in 2.4 are reachable rather than pre-empted by a resolution that throws
- [ ] 2.4 Add assertions for each of those three unset states, naming the option of safix's namespace that is wrong
- [ ] 2.5 Check the resolver's violation list before materializing, and throw safix's own message carrying every violation rather than letting the resolver's first one surface from inside the provisioner
- [ ] 2.6 Default the enable flag to whether the resolved set is non-empty, and gate the whole of each module's configuration on it
- [ ] 2.7 Verify: `nix eval` of a fixture profile that sets nothing but the import produces an empty resolved set and no error

## 3. The home-manager module

- [ ] 3.1 Write `modules/consume/home.nix`: the person defaulting to the profile's own username, the host defaulting from `osConfig` where home-manager is evaluated as a NixOS module and null otherwise, and the resolved set materialized at user scope
- [ ] 3.2 Declare the identity options, with the key file defaulting to null and the option description carrying the provisioner's fatality asymmetry as fact — set-but-unreadable key file aborts, missing ssh key path is skipped with a warning
- [ ] 3.3 Define the identity onto the provisioner's options at normal priority, so a `mkDefault` elsewhere in a consumer's tree loses and a plain definition conflicts loudly
- [ ] 3.4 Port the identity preflight from `<dotfiles>/modules/home/base/sops.nix`, keeping the required/sufficient partition, the gnupg-source condition that decides when the ssh key paths stop being load-bearing, and the remediation text
- [ ] 3.5 Sort the preflight `entryBefore [ "checkLinkTargets" ]` and gate it on a non-empty resolved set and a non-empty identity
- [ ] 3.6 Rewrite the preflight's guarantee paragraph so every sentence in it is one this repository's checks hold, and state its limit: presence and readability were checked, decryption was not
- [ ] 3.7 Verify: a fixture home-manager configuration evaluates, and its resolved set matches the fixture fleet's expectation for that person on that host

## 4. The NixOS module

- [ ] 4.1 Write `modules/consume/nixos.nix`: the host defaulting to the configuration's own hostname, the person required, and the resolved set materialized at system scope
- [ ] 4.2 Declare the same identity options, defining them onto the provisioner's system-scope equivalents only where the consumer set them, so the provisioner's own host-key default survives untouched
- [ ] 4.3 Install no activation guard, and record in the module header that no atomic refusal point at system activation has been demonstrated
- [ ] 4.4 Verify: a fixture NixOS configuration evaluates and its resolved set carries the ownership fields the same declarations refuse at user scope

## 5. Exports

- [ ] 5.1 Export `homeModules.safix` and `nixosModules.safix` as the two modules from sections 3 and 4, importing nothing
- [ ] 5.2 Export `homeModules.default` and `nixosModules.default` as those modules plus the provisioner's own module for their scope
- [ ] 5.3 Add `home-manager` as a check-only input, following this flake's nixpkgs, with the reason recorded in `flake.nix` beside the same note `sops-nix` carries
- [ ] 5.4 Verify: `nix flake show` lists all four module outputs

## 6. The equivalence proof

- [ ] 6.1 Write `modules/flake/checks/consumption.nix` evaluating a fixture home-manager configuration in the four-line consumer form against the same fixture fleet the materialization check uses
- [ ] 6.2 Beside it, evaluate a second configuration wiring the resolver by hand in the shape of `<dotfiles>/modules/home/users/sernl/sops/default.nix` — the resolver call assigned into `sops.secrets`, the identity set directly on the provisioner
- [ ] 6.3 Assert the two configurations' `sops.secrets` are equal as read back through the provisioner's own option types, entry by entry, not as the attrsets safix handed them
- [ ] 6.4 Severity drill: changing the module's materialization scope, dropping a field from what it emits, or resolving for a different host each turns the equivalence red
- [ ] 6.5 Verify: the check is red when the module form is pointed at a different person

## 7. The ordering proof

- [ ] 7.1 Topologically sort the fixture profile's activation DAG with the module's entry present, and assert this package's entry precedes `checkLinkTargets`
- [ ] 7.2 Assert the provisioner's own entry does not, so the check records the asymmetry the guard exists for rather than only the guard's own position
- [ ] 7.3 Extract the preflight script from the evaluated profile and assert it names every configured identity path and exits non-zero, without running it
- [ ] 7.4 Severity drill: replacing `entryBefore [ "checkLinkTargets" ]` with a bare string turns 7.1 red; dropping one identity path from the script turns 7.3 red
- [ ] 7.5 Verify: both drills observed red before the expectations are restored

## 8. The scope and no-op proofs

- [ ] 8.1 Evaluate a fixture NixOS configuration and a fixture home-manager configuration over one fleet, and assert they establish the same names
- [ ] 8.2 Assert an entry carrying ownership reaches the system configuration with it, and that the same entry at user scope fails evaluation
- [ ] 8.3 Evaluate a fixture profile for a person who resolves nothing on that host, and assert its activation entries carry no entry from this package and no provisioning unit exists
- [ ] 8.4 Assert the same profile still evaluates green rather than failing, since inert and broken must not look alike
- [ ] 8.5 Severity drill: removing the enable gate from the module turns 8.3 red; dropping the user-scope ownership refusal turns 8.2 red
- [ ] 8.6 Verify: `nix build` of each new check green

## 9. Documentation

- [ ] 9.1 Rewrite the README's consumption section as the option surface of both modules, with the four-line form and the fifth line the standalone case needs
- [ ] 9.2 Add the quick start's import, saying which of the two forms to pick and why, with the collision error quoted from the check that holds it
- [ ] 9.3 Document the identity contract, the activation guard, exactly what it guarantees, and the system-scope asymmetry with its reason
- [ ] 9.4 Verify: every guarantee stated in the README names a check in this repository that holds it

## 10. Verification

- [ ] 10.1 `openspec validate add-consumption-modules --strict`
- [ ] 10.2 `nix flake check` green
- [ ] 10.3 `rg` the whole tree for any real fleet identifier and confirm none
- [ ] 10.4 Confirm no file under `<dotfiles>` was modified
