# Holds one declaration to what it becomes in each of the two scopes it can be
# served in, judged by handing the result to the provisioner that will read it.
#
# The claim safix makes is that scope is not a property of a declaration. A
# secret is declared once — a mode, a key, a path, an audience — and the same
# record materializes into a person's home-manager profile and into a system
# configuration, with nothing in the declaration naming which. That claim is only
# worth anything if what comes out is a shape the provisioner accepts, so this
# does not compare safix's output with a literal alone: it evaluates the real
# sops-nix modules over it and reads the entry back through their own option
# types. A field safix emits that sops-nix does not declare fails here, and so
# does a field it declares that safix stopped emitting.
#
# The two modules are evaluated on their own rather than inside a full NixOS
# system or home-manager configuration, because what is under test is the option
# surface a materialized entry has to satisfy. `_module.check = false` is what
# admits the module's own definitions of options its host would have declared —
# activation scripts, units — none of which are forced here.
#
# ── the ownership asymmetry ──
# The user-scope provisioner has no ownership axis: it runs as the user, and its
# secret type has no `owner` and no `group`. That is the fact safix's user-scope
# refusal rests on, and it is asserted here against the provisioner's own option
# names rather than restated as prose, so the refusal stops being justified the
# moment its justification stops being true.
#
# Severity: proven by perturbation, one drill per claim.
# Dropping the `scope == "system"` guard from either ownership field in
# `materializeFor` fails `userScopeRefusesOwnership`, and — because the field
# then reaches a provisioner that never declared it — fails the user-scope
# readback as well.
# Emitting `key` where the entry names no `sopsKey` fails `sameInBothScopes` and
# the two readbacks, since the provisioner's own default for the omitted field is
# what the check reads.
# Having `selectFor` derive the file from anything but the audience fails
# `fileFromAudience`, whose expectation is the path the recipient policy writes a
# rule for and is written independently of the resolver.
# Making the user scope drop rather than refuse an ownership field fails
# `userScopeRefusesOwnership` alone, which is the whole distinction: a dropped
# field reads afterwards as an ownership claim that was honoured.
{
  config,
  inputs,
  lib,
  ...
}:
{
  perSystem =
    { pkgs, ... }:
    let
      safix = config.flake.safix.lib;
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      # The configuration an entry's `path` is a function of, standing in for the
      # consumer's own. Both scopes are handed the same one, so a difference
      # between them is a difference safix made. Its shape is the one a real
      # home-manager configuration has, so the fixture's `path` function is the
      # expression a consumer would actually write.
      fixtureCfg.home.homeDirectory = "/home/ana";

      materializedFor =
        user: hostname: scope:
        safix.materialize {
          inherit user hostname scope;
          tags = [ ];
        } fixtureCfg;

      anaUser = materializedFor "ana" "workstation" "user";
      anaSystem = materializedFor "ana" "workstation" "system";
      boSystem = materializedFor "bo" "server" "system";
      boUser = materializedFor "bo" "server" "user";

      # The provisioner's own module, evaluated over what safix produced.
      provisioner =
        module: secrets:
        lib.evalModules {
          modules = [
            module
            { _module.args.pkgs = pkgs; }
            { _module.args.utils = { }; }
            { _module.check = false; }
            { sops.secrets = secrets; }
          ];
        };

      userProvisioner = provisioner inputs.sops-nix.homeManagerModules.sops anaUser;
      systemProvisioner = provisioner inputs.sops-nix.nixosModules.sops anaSystem;
      ownedProvisioner = provisioner inputs.sops-nix.nixosModules.sops boSystem;

      axisOf = evaluated: builtins.attrNames (evaluated.options.sops.secrets.type.getSubOptions [ ]);

      readback =
        evaluated: name: fields:
        lib.getAttrs fields evaluated.config.sops.secrets.${name};

      fires = e: !(builtins.tryEval (builtins.deepSeq e e)).success;

      placedIn = entry: file: lib.hasSuffix "/${file}" (toString entry.sopsFile);

      sortNames = lib.sort (a: b: a < b);
    in
    {
      checks.safix-materialization = mkStructuralCheck {
        name = "safix-materialization";
        actual = {
          # Both scopes resolve the same names, because selection is custody and
          # custody has no scope.
          names = {
            user = sortNames (builtins.attrNames anaUser);
            system = sortNames (builtins.attrNames anaSystem);
          };

          # No field of a materialized entry names a scope, and for a
          # declaration that sets no ownership the two scopes produce the same
          # record outright.
          fields = sortNames (builtins.attrNames anaUser.ana-alone);
          sameInBothScopes = anaUser == anaSystem;

          # The entry as each provisioner sees it, read back through its own
          # option type rather than off the attrset safix handed it.
          userReadback = readback userProvisioner "ana-alone" [
            "mode"
            "path"
            "key"
          ];
          systemReadback = readback systemProvisioner "ana-alone" [
            "mode"
            "path"
            "key"
          ];

          # The file is derived from the audience, so it is the one the
          # recipient policy writes a rule for. Asserted as a suffix because the
          # prefix is whatever store path this flake's source is at.
          fileFromAudience = {
            own = placedIn anaUser.ana-alone "secrets/safix/users/ana/secrets.yaml";
            shared = placedIn anaUser.ops-handover "secrets/safix/shared/ana,bo/secrets.yaml";
            carried = placedIn anaUser.ops-tooling "secrets/safix/users/ana/secrets.yaml";
          };

          # The asymmetry the user-scope refusal rests on, read off the two
          # provisioners rather than asserted about them.
          ownershipAxis = {
            user = builtins.filter (f: f == "owner" || f == "group") (axisOf userProvisioner);
            system = builtins.filter (f: f == "owner" || f == "group") (axisOf systemProvisioner);
          };

          # An entry that sets ownership reaches the system scope carrying it,
          # and the user scope refuses rather than dropping it.
          ownedReadback = readback ownedProvisioner "bo-service" [
            "mode"
            "path"
            "owner"
            "group"
          ];
          userScopeRefusesOwnership = fires boUser;
        };

        expected = {
          # `wg-public` is absent from both scopes and present in the
          # selection, which is the claim: the provisioner is handed a sopsFile
          # and a key and decrypts at activation, and a public output has
          # neither, so an entry for one would fail to extract a key that will
          # never exist.
          names = {
            user = [
              "ana-alone"
              "api-token"
              "ops-handover"
              "ops-tooling"
              "team-vault"
              "web-token"
              "wg-private"
            ];
            system = [
              "ana-alone"
              "api-token"
              "ops-handover"
              "ops-tooling"
              "team-vault"
              "web-token"
              "wg-private"
            ];
          };

          fields = [
            "key"
            "mode"
            "path"
            "sopsFile"
          ];
          sameInBothScopes = true;

          userReadback = {
            mode = "0440";
            path = "/home/ana/.config/safix-fixture/ana-alone";
            key = "ana_alone";
          };
          systemReadback = {
            mode = "0440";
            path = "/home/ana/.config/safix-fixture/ana-alone";
            key = "ana_alone";
          };

          fileFromAudience = {
            own = true;
            shared = true;
            carried = true;
          };

          ownershipAxis = {
            user = [ ];
            system = [
              "group"
              "owner"
            ];
          };

          ownedReadback = {
            mode = "0400";
            path = "/var/lib/safix-fixture/bo-service";
            owner = "bo";
            group = "staff";
          };
          userScopeRefusesOwnership = true;
        };
      };
    };
}
