# Holds ../../../examples/quickstart/: the fleet the README is written around,
# consumed by one NixOS profile and one home-manager profile.
#
# ── what it reads ──
# `examples/quickstart/hosts/web.nix` through a real `nixosSystem` and
# `examples/quickstart/home/alice.nix` through home-manager's own library, the
# way `./examples-profiles.nix` does. Both files are read whole, including
# their own `imports` line: each reaches the consumption module through
# `inputs.safix.nixosModules.safix` or `inputs.safix.homeModules.safix`, so the
# module arguments below stand in for that input and point those two names at
# the bare consumption modules. `safix` itself reaches the package set through
# one overlay, because a bare consumption module imports nothing and its own
# default for `safix.installer.package` is `pkgs.safix`.
#
# ── why the binding is forced ──
# The committed profiles bind through `safix.flake`, which reads the flake
# output `safix.lib` — and that output is `mkVault` over `./secrets.nix` with
# `root = ./.`. A resolved `sopsFile` is `lib.types.path`, so forcing one under
# that root resolves a ciphertext file this repository does not commit. The
# stand-in flake below therefore binds the same declarations with an empty
# root, which is what makes the compared paths the repository-relative strings
# `./examples-profiles.nix` also compares.
#
# ── why `flake.nix` is read as text ──
# It names `inputs.safix.url = "path:../.."`, so evaluating it inside the
# sandbox would resolve this repository's whole input closure a second time
# with no network. `examples/dendritic/flake.nix` is read the same way and for
# the same reason. What is asserted of it is the wiring the two evaluated
# profiles cannot show: that this one file is where the declarations and both
# profiles are named.
#
# ── severity: four drills, all observed ──
# Dropping `sharedWith.web."grafana-secret-key"` from `secrets.nix` empties the
# machine's second entry and reddens `systemSecrets`, and the template row goes
# with it: `config.safix.placeholder."grafana-secret-key"` no longer exists, so
# the host profile fails to evaluate rather than rendering a template with one
# placeholder. That is the pair this example exists to show — a template
# renders what the machine resolves and nothing else. Observed.
# Changing `flake.safix.rotation.quarterly.every` to `"30d"` reddens
# `rotationOnGenerated.everySeconds` alone and leaves every other row green,
# which is what says the deadline is read off the placement rather than off the
# declaration. Observed.
# Setting `safix.rotation.enable = true` in `home/alice.nix` reddens
# `homeRotation.unitDefined` and `.timerDefined` together, which is the row
# pair holding "the option is shown and the unit is not installed". Observed.
# Renaming `hosts/web.nix` in `flake.nix` alone reddens `flakeReferences.host`
# while both profile evaluations stay green, because each is evaluated by path
# here rather than through that file. Observed.
{
  inputs,
  lib,
  ...
}:
let
  quickstartRoot = ../../../examples/quickstart;
  libRoot = ../../../lib;

  mkVault = (import libRoot { inherit lib; }).mkVault;

  projection = mkVault {
    modules = [ (quickstartRoot + "/secrets.nix") ];
    root = "";
  };

  flakeText = builtins.readFile (quickstartRoot + "/flake.nix");

  # The nine fields of a resolved system-scope entry that safix decides, as
  # `./examples-profiles.nix` lists them. `uid` and `gid` are the provisioner's
  # numeric echo of `owner` and `group`, which are already here.
  systemFields = [
    "key"
    "path"
    "mode"
    "owner"
    "group"
    "sopsFile"
    "format"
    "restartUnits"
    "reloadUnits"
  ];
in
{
  perSystem =
    {
      pkgs,
      self',
      system,
      ...
    }:
    let
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      safixOverlay = _: _: { safix = self'.packages.safix; };

      # What `inputs` is inside both example profiles. `self.safix.lib` is the
      # output `../flake.nix` publishes, rebound against an empty root here.
      exampleInputs = {
        self.safix.lib = projection;
        safix = {
          nixosModules.safix = ../../consume/nixos.nix;
          homeModules.safix = ../../consume/home.nix;
        };
      };

      systemProfile =
        (inputs.nixpkgs.lib.nixosSystem {
          specialArgs.inputs = exampleInputs;
          modules = [
            (quickstartRoot + "/hosts/web.nix")
            {
              nixpkgs.hostPlatform = system;
              nixpkgs.overlays = [ safixOverlay ];
            }
          ];
        }).config;

      homeProfile =
        (inputs.home-manager.lib.homeManagerConfiguration {
          pkgs = pkgs.extend safixOverlay;
          extraSpecialArgs.inputs = exampleInputs;
          modules = [ (quickstartRoot + "/home/alice.nix") ];
        }).config;

      rows = {
        # Both entries at the machine's scope: one a person types, one a
        # generator mints, both granted onward to `web` by alice.
        systemSecrets = lib.mapAttrs (_name: lib.getAttrs systemFields) systemProfile.safix.secrets;

        # The rendered template is public text with two runtime placeholders in
        # it. No plaintext is here, which is the property: substitution happens
        # in a private runtime generation and never in nix.
        template = lib.getAttrs [
          "content"
          "path"
          "mode"
          "owner"
          "group"
          "restartUnits"
        ] systemProfile.safix.templates."grafana.env";

        # The deadline, read off the placement the resolver emitted rather than
        # off the declaration that set it.
        rotationOnGenerated = projection.placements.alice."grafana-secret-key".rotation;
        rotationAbsent = projection.placements.alice."grafana-admin-password".rotation;

        # The option is shown in the example and left off, so the profile must
        # carry the values and install no unit.
        homeRotation = {
          inherit (homeProfile.safix.rotation) enable repository onCalendar;
          unitDefined = homeProfile.systemd.user.services ? safix-rotate;
          timerDefined = homeProfile.systemd.user.timers ? safix-rotate;
        };

        # What alice resolves on her own account, which is both entries again:
        # a grant to a machine takes nothing away from the person granting it.
        homeSecrets = builtins.attrNames homeProfile.safix.secrets;

        flakeReferences = {
          declarations = lib.hasInfix "./secrets.nix" flakeText;
          host = lib.hasInfix "./hosts/web.nix" flakeText;
          home = lib.hasInfix "./home/alice.nix" flakeText;
        };
      };

      expected = {
        systemSecrets = {
          "grafana-admin-password" = {
            format = "yaml";
            group = null;
            key = "grafana-admin-password";
            mode = "0400";
            owner = null;
            path = "/run/safix/grafana-admin-password";
            reloadUnits = [ ];
            restartUnits = [ "grafana.service" ];
            sopsFile = "/secrets/safix/shared/alice,web/secrets.yaml";
          };
          "grafana-secret-key" = {
            format = "yaml";
            group = null;
            key = "grafana-secret-key";
            mode = "0400";
            owner = null;
            path = "/run/safix/grafana-secret-key";
            reloadUnits = [ ];
            restartUnits = [ "grafana.service" ];
            sopsFile = "/secrets/safix/shared/alice,web/secrets.yaml";
          };
        };
        template = {
          content = ''
            GF_SECURITY_ADMIN_PASSWORD=<safix:grafana-admin-password>
            GF_SECURITY_SECRET_KEY=<safix:grafana-secret-key>
          '';
          path = "/run/safix/grafana.env";
          mode = "0400";
          owner = null;
          group = null;
          restartUnits = [ "grafana.service" ];
        };
        rotationOnGenerated = {
          policy = "quarterly";
          everySeconds = 7776000;
        };
        rotationAbsent = null;
        homeRotation = {
          enable = false;
          repository = "/home/alice/secrets";
          onCalendar = "daily";
          unitDefined = false;
          timerDefined = false;
        };
        homeSecrets = [
          "grafana-admin-password"
          "grafana-secret-key"
        ];
        flakeReferences = {
          declarations = true;
          host = true;
          home = true;
        };
      };
    in
    {
      checks.safix-examples-quickstart = mkStructuralCheck {
        name = "examples-quickstart";
        actual = rows;
        expected = expected;
      };
    };
}
