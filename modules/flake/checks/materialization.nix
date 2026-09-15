# Holds one declaration to what it becomes in each of the two scopes it can be
# served in, judged by handing the result to the type that will read it.
#
# The claim safix makes is that scope is not a property of a declaration. A
# secret is declared once — a mode, a key, a path, an audience — and the same
# record materializes into a person's home-manager profile and into a system
# configuration, with nothing in the declaration naming which. That claim is only
# worth anything if what comes out is a shape the installer accepts, so this
# does not compare safix's output with a literal alone: it hands the result to
# safix's own declared entry type and reads the entry back through it. A field
# safix emits that the type does not declare fails here, and so does a field the
# type declares that safix stopped emitting.
#
# The type is evaluated on its own rather than inside a full NixOS system or
# home-manager configuration, because what is under test is the option surface a
# materialized entry has to satisfy. `common.secretEntryType`'s `path` default
# reads `cfg.installer.symlinkPath` and nothing else, so the stub cfg below is
# the whole of what it needs.
#
# ── the ownership asymmetry ──
# The user scope has no ownership axis: its resolved set is typed `raw`, because
# the user-mode manifest is built from the same materialization and needs no
# second typing pass, and safix's own entry type — the one the system scope is
# typed by — is where `owner` and `group` are declared at all. That is the fact
# safix's user-scope refusal rests on, and it is asserted here against the two
# scopes' own declared types rather than restated as prose, so the refusal stops
# being justified the moment its justification stops being true.
#
# Severity: proven by perturbation, one drill per claim.
# Dropping the `scope == "system"` guard from either ownership field in
# `materializeFor` fails `userScopeRefusesOwnership`, and — because the field
# then reaches a type that never declared it — fails the system-scope readback
# as well.
# Emitting `key` where the entry names no `sopsKey` fails `sameInBothScopes` and
# the two readbacks, since the type's own default for the omitted field is
# what the check reads.
# Having `selectFor` derive the file from anything but the audience fails
# `fileFromAudience`, whose expectation is the path the recipient policy writes a
# rule for and is written independently of the resolver.
# Making the user scope drop rather than refuse an ownership field fails
# `userScopeRefusesOwnership` alone, which is the whole distinction: a dropped
# field reads afterwards as an ownership claim that was honoured.
{
  config,
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
      fixtureCfg.home.homeDirectory = "/home/alice";

      materializedFor =
        user: hostname: scope:
        safix.materialize {
          inherit user hostname scope;
          tags = [ ];
        } fixtureCfg;

      aliceUser = materializedFor "alice" "workstation" "user";
      aliceSystem = materializedFor "alice" "workstation" "system";
      bobSystem = materializedFor "bob" "server" "system";
      bobUser = materializedFor "bob" "server" "user";

      systemCommon = import ../../consume/common.nix {
        inherit lib;
        scope = "system";
      };

      # The configuration the entry type's `path` default is a function of.
      # It is the only thing the type reads outside its own fields, which is
      # what makes a stub sufficient here.
      entryCfg.installer.symlinkPath = "/run/safix";

      # The type each scope's module passes to `common.sharedOptions` as its
      # `secretsType` argument: safix's own submodule at system scope
      # (`modules/consume/nixos.nix`), and `raw` at user scope
      # (`modules/consume/home.nix`), where the user-mode manifest is built
      # from this same materialization and needs no second typing pass.
      systemType = lib.types.attrsOf (systemCommon.secretEntryType { cfg = entryCfg; });
      userType = lib.types.attrsOf lib.types.raw;

      # safix's own declared type, evaluated over what safix produced. Nothing
      # of the module system's host surface is involved, so no
      # `_module.check = false` is needed: the fixture declares exactly the one
      # option under test.
      typedThrough =
        type: secrets:
        lib.evalModules {
          modules = [
            { options.safix.secrets = lib.mkOption { inherit type; }; }
            { safix.secrets = secrets; }
          ];
        };

      userTyped = typedThrough userType aliceUser;
      systemTyped = typedThrough systemType aliceSystem;
      ownedTyped = typedThrough systemType bobSystem;

      axisOf = type: builtins.attrNames (type.getSubOptions [ ]);

      readback =
        evaluated: name: fields:
        lib.getAttrs fields evaluated.config.safix.secrets.${name};

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
            user = sortNames (builtins.attrNames aliceUser);
            system = sortNames (builtins.attrNames aliceSystem);
          };

          # No field of a materialized entry names a scope, and for a
          # declaration that sets no ownership the two scopes produce the same
          # record outright.
          fields = sortNames (builtins.attrNames aliceUser.alice-alone);
          sameInBothScopes = aliceUser == aliceSystem;

          # The entry as each scope's own declared type resolved it, read back
          # through that type rather than off the attrset safix handed it.
          userReadback = readback userTyped "alice-alone" [
            "mode"
            "path"
            "key"
          ];
          systemReadback = readback systemTyped "alice-alone" [
            "mode"
            "path"
            "key"
          ];

          # The file is derived from the audience, so it is the one the
          # recipient policy writes a rule for. Asserted as a suffix because the
          # prefix is whatever store path this flake's source is at.
          fileFromAudience = {
            own = placedIn aliceUser.alice-alone "secrets/safix/users/alice/secrets.yaml";
            shared = placedIn aliceUser.ops-handover "secrets/safix/shared/alice,bob/secrets.yaml";
            carried = placedIn aliceUser.ops-tooling "secrets/safix/users/alice/secrets.yaml";
          };

          # The asymmetry the user-scope refusal rests on, read off the two
          # scopes' own declared types rather than asserted about them.
          ownershipAxis = {
            user = builtins.filter (f: f == "owner" || f == "group") (axisOf userType);
            system = builtins.filter (f: f == "owner" || f == "group") (axisOf systemType);
          };

          # An entry that sets ownership reaches the system scope carrying it,
          # and the user scope refuses rather than dropping it.
          ownedReadback = readback ownedTyped "bob-service" [
            "mode"
            "path"
            "owner"
            "group"
          ];
          userScopeRefusesOwnership = fires bobUser;
        };

        expected = {
          # `wg-public` is absent from both scopes and present in the
          # selection, which is the claim: the installer is handed a sopsFile
          # and a key and decrypts at activation, and a public output has
          # neither, so an entry for one would fail to extract a key that will
          # never exist.
          names = {
            user = [
              "alice-alone"
              "api-token"
              "corp-handover"
              "corp-token"
              "ops-handover"
              "ops-tooling"
              "team-vault"
              "web-token"
              "wg-private"
            ];
            system = [
              "alice-alone"
              "api-token"
              "corp-handover"
              "corp-token"
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
            path = "/home/alice/.config/safix-fixture/alice-alone";
            key = "alice_alone";
          };
          systemReadback = {
            mode = "0440";
            path = "/home/alice/.config/safix-fixture/alice-alone";
            key = "alice_alone";
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
            path = "/var/lib/safix-fixture/bob-service";
            owner = "bob";
            group = "staff";
          };
          userScopeRefusesOwnership = true;
        };
      };
    };
}
