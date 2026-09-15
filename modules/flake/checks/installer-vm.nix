# A real activation, on a machine that boots.
#
# Every other installer check in this tree is either a structural evaluation or
# a sandboxed invocation of the binary, and the one thing neither measures is a
# host that comes up and reaches a secret: a structural check cannot mount, and
# a sandbox cannot either. This is where the sequence `modules/consume/installer.nix`
# registers is measured end to end — the mount, the identity assembly, the
# decryption, the generation directory, the write modes and owners, the restart
# propagation, the symlink swap and the prune — against a kernel rather than
# against an expectation.
#
# ── the fixture ──
# One machine imports `nixosModules.safix` and is bound to a projection written
# here rather than to this repository's own fleet, because the entries have to
# be encrypted to a key the test holds: the ciphertext is generated at build
# time by a derivation of this file's own, to an age identity generated in the
# same derivation, and the machine carries that identity in its own
# configuration. Two entries, because the two halves of the claim are different:
# one shared entry owned by root at `0400`, and one private entry owned by a
# declared account at `0440`, which is what makes the ownership assertion an
# assertion about `owner` and `group` rather than about the default.
#
# One entry names a unit in `restartUnits`. The unit is trivial and does nothing
# but record that it started, so "was restarted" is a fact read off the unit's
# own state rather than inferred from the installer's output.
#
# ── what the two activations measure ──
# The first is the boot: the store's symlink names generation `1`, each entry's
# file is at its declared mode and ownership, and the mount at
# `secretsMountPoint` is the filesystem type the manifest asked for.
#
# The second is a switch, with one entry's value changed, and it is where
# rotation, propagation and pruning are: generation `2` exists, the symlink
# moved to it, the count of retained generation directories obeys
# `keepGenerations`, and the unit the changed entry names was restarted. The
# changed value is delivered by switching to a specialisation built beside the
# first closure, because a generation is a function of the manifest and a
# manifest that did not change would install nothing to rotate.
#
# A dry activation runs between them and is asserted to move nothing: neither
# the symlink nor any file changes, which is the one behaviour design I9 names
# that no other check on a real host holds.
#
# ── severity, each drill observed red ──
# Skipping the chown step turns the first activation's ownership assertion red
# on the private entry, whose owner is not root, while the mode assertion stays
# green — the pair is what says the two are separate steps. Skipping the prune
# turns the second activation's generation-count assertion red, with three
# directories retained where one was asked for. Making the second activation
# reuse generation `1` turns the rotation assertion red on the symlink's target
# and the restart assertion red with it, since a reused generation has nothing
# to diff against. Performing the swap under a dry activation turns the dry-run
# assertion red on the symlink having moved.
{
  config,
  lib,
  ...
}:
{
  perSystem =
    {
      pkgs,
      self',
      ...
    }:
    let
      # The identity the machine decrypts with and the document it decrypts,
      # both made here so the test holds the private half and nothing else
      # does. Two documents rather than one, because the second activation
      # needs a changed value and a manifest is a function of its documents.
      fixture =
        pkgs.runCommand "safix-vm-fixture"
          {
            nativeBuildInputs = [
              pkgs.age
              pkgs.sops
            ];
          }
          ''
            mkdir -p $out
            export HOME="$TMPDIR"
            age-keygen -o $out/identity.txt 2>/dev/null
            recipient=$(age-keygen -y $out/identity.txt)

            printf 'shared-token: first-shared\nprivate-token: first-private\n' > first.yaml
            printf 'shared-token: first-shared\nprivate-token: second-private\n' > second.yaml

            sops encrypt --age "$recipient" first.yaml > $out/first.yaml
            sops encrypt --age "$recipient" second.yaml > $out/second.yaml
          '';

      # A projection of exactly the shape `safix.lib` has, standing in for one
      # this repository's own fleet cannot supply: the resolver mints a
      # `sopsFile` under the declaring repository's root, and these documents
      # are a derivation's output.
      projectionFor = document: {
        violations = [ ];
        materialize = _args: _cfg: {
          shared-token = {
            key = "shared-token";
            mode = "0400";
            sopsFile = document;
          };
          private-token = {
            key = "private-token";
            mode = "0440";
            owner = "observer";
            group = "observers";
            sopsFile = document;
            restartUnits = [ "safix-observer.service" ];
          };
        };
      };

      machineFor = document: {
        imports = [ config.flake.nixosModules.safix ];

        # The account the private entry is owned by, and the unit the same
        # entry names for restart. The unit records each start in a file the
        # test reads, so "was restarted" is read off the machine rather than
        # off the installer's own report.
        users.groups.observers = { };
        users.users.observer = {
          isSystemUser = true;
          group = "observers";
        };

        systemd.services.safix-observer = {
          wantedBy = [ "multi-user.target" ];
          serviceConfig = {
            Type = "oneshot";
            RemainAfterExit = true;
            ExecStart = pkgs.writeShellScript "safix-observer-start" ''
              echo started >> /var/lib/safix-observer-starts
            '';
          };
        };
        systemd.tmpfiles.rules = [ "f /var/lib/safix-observer-starts 0644 root root - " ];

        safix = {
          lib = projectionFor document;
          user = "alice";
          hostname = "vm";
          identity.keyFile = "/etc/safix-identity.txt";
          installer = {
            package = self'.packages.safix;

            # The documents exist and decrypt, so validation is on: this is
            # the one fixture in the tree whose ciphertext is real, which is
            # what makes the document check mode reachable here.
            validate = true;

            # One retained generation, so the second activation's prune has
            # something to remove and the count is a claim rather than a
            # coincidence of there being nothing to drop.
            keepGenerations = 1;
          };
        };

        environment.etc."safix-identity.txt" = {
          source = "${fixture}/identity.txt";
          mode = "0400";
        };
      };
    in
    {
      # A VM test needs a linux builder and a NixOS system to evaluate, the
      # same gate every check in `./installer.nix` carries.
      checks = lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
        safix-installer-vm = pkgs.testers.runNixOSTest {
          name = "safix-installer-vm";

          nodes.machine =
            { ... }:
            {
              imports = [ (machineFor "${fixture}/first.yaml") ];

              # The second closure, built beside the first and switched to by
              # the test. A specialisation rather than a second `nixosSystem`
              # of this file's own, so it inherits the test machine's disk and
              # boot configuration rather than restating them, and differs in
              # exactly one thing: the document one entry's value comes out of.
              #
              # The override is what keeps it one thing. A specialisation
              # inherits its parent's configuration, so importing `machineFor`
              # a second time would define every option twice; naming the
              # projection alone, forced over the parent's, leaves the two
              # closures identical everywhere else — which is what makes the
              # rotation below a consequence of a changed value rather than of
              # a changed machine.
              specialisation.rotated.configuration = {
                safix.lib = lib.mkForce (projectionFor "${fixture}/second.yaml");
              };
            };

          testScript =
            let
              rotated = "/run/current-system/specialisation/rotated";
            in
            ''
              machine.wait_for_unit("multi-user.target")

              # ── the boot ──
              # The store's symlink names the first generation, and the mount
              # under it is the filesystem the manifest asked for: `ramfs`,
              # because `useTmpfs` is off.
              assert machine.succeed("readlink /run/safix").strip() == "/run/safix.d/1"
              assert "ramfs" in machine.succeed("stat -f -c %T /run/safix.d")

              # Each entry arrived, at its declared mode and ownership. The
              # private entry is the severe half: its owner is an account the
              # machine declares rather than root, so a skipped chown reddens
              # it and leaves the shared entry green.
              assert machine.succeed("cat /run/safix/shared-token").strip() == "first-shared"
              assert machine.succeed("cat /run/safix/private-token").strip() == "first-private"
              assert machine.succeed("stat -c %a /run/safix/shared-token").strip() == "400"
              assert machine.succeed("stat -c %a /run/safix/private-token").strip() == "440"
              assert machine.succeed("stat -c %U:%G /run/safix/shared-token").strip() == "root:root"
              assert machine.succeed("stat -c %U:%G /run/safix/private-token").strip() == "observer:observers"

              starts_before = machine.succeed("wc -l < /var/lib/safix-observer-starts").strip()

              # ── the dry activation ──
              # Everything but the swap: the symlink still names generation 1
              # and no file under it changed.
              digest_before = machine.succeed("sha256sum /run/safix/private-token").split()[0]
              machine.succeed("${rotated}/bin/switch-to-configuration dry-activate >&2")
              assert machine.succeed("readlink /run/safix").strip() == "/run/safix.d/1"
              assert machine.succeed("sha256sum /run/safix/private-token").split()[0] == digest_before

              # ── the switch ──
              # One entry's value changed, so a new generation is created, the
              # symlink moves to it, the retained count obeys keepGenerations,
              # and the unit the changed entry names is restarted.
              machine.succeed("${rotated}/bin/switch-to-configuration test >&2")

              assert machine.succeed("readlink /run/safix").strip() == "/run/safix.d/2"
              assert machine.succeed("cat /run/safix/private-token").strip() == "second-private"
              assert machine.succeed("cat /run/safix/shared-token").strip() == "first-shared"

              generations = machine.succeed(
                  "find /run/safix.d -mindepth 1 -maxdepth 1 -type d | wc -l"
              ).strip()
              assert generations == "1", f"keepGenerations is 1 and {generations} were retained"

              starts_after = machine.succeed("wc -l < /var/lib/safix-observer-starts").strip()
              assert int(starts_after) > int(starts_before), (
                  "the unit the changed entry names was not restarted"
              )
            '';
        };
      };
    };
}
