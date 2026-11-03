# Real activation under legacy users, systemd-sysusers and userborn. The identity
# is provisioned on the first boot; the rotated profile must install a password
# before creating a new account. A fourth node keeps its identity on a mount
# needed for boot and reboots, so shutdown ordering is observed as well. All keys
# and values below are disposable fixtures.
{ config, lib, ... }:
{
  perSystem =
    { pkgs, self', ... }:
    let
      passwordHash = "$6$safix-vm$VHlET0mnTccqPLIVZZEB4kFdjuWnAqGqoeradXAWoDVEPJikXUJW43ob9lq3F/.yB.DHSn2eyt/dcWSY1Zsep.";
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
            printf 'shared-token: first-shared\nprivate-token: second-private\naccount-password: %s\n' ${lib.escapeShellArg passwordHash} > second.yaml
            sops encrypt --age "$recipient" first.yaml > $out/first.yaml
            sops encrypt --age "$recipient" second.yaml > $out/second.yaml
          '';
      keyDirectory = pkgs.runCommand "safix-vm-key-dir" { } ''
        install -D -m 444 ${fixture}/identity.txt $out/identity.txt
      '';
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
      recordUserTemplate =
        name:
        pkgs.writeShellScript "safix-user-${name}" ''
          ${pkgs.coreutils}/bin/cat "$XDG_RUNTIME_DIR/safix-hook/config" >> "$XDG_RUNTIME_DIR/${name}"
        '';
      machineFor = document: {
        imports = [ config.flake.nixosModules.safix ];
        users.groups.observers = { };
        users.users.observer = {
          isSystemUser = true;
          group = "observers";
          home = "/var/lib/safix-observer";
          createHome = true;
          linger = true;
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
        systemd.user.services.safix-user-observer.serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          ExecStart = recordUserTemplate "restart-observed";
          ExecReload = recordUserTemplate "unexpected-reload";
        };
        systemd.user.services.safix-user-reload.serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          ExecStart = "${pkgs.coreutils}/bin/true";
          ExecReload = recordUserTemplate "reload-observed";
        };
        systemd.tmpfiles.rules = [ "f /var/lib/safix-observer-starts 0644 root root - " ];
        # Default dependencies and no edge to safix: the secret is there at
        # start only because the installer runs before sysinit.target.
        systemd.services.safix-consumer = {
          wantedBy = [ "multi-user.target" ];
          serviceConfig = {
            Type = "oneshot";
            RemainAfterExit = true;
          };
          script = "test -e /run/safix/shared-token";
        };
        safix = {
          lib = projectionFor document;
          user = "alice";
          hostname = "vm";
          identity.keyFile = "/etc/safix-identity.txt";
          templates.application = {
            content = "token=<safix:private-token>\n";
            mode = "0400";
            owner = "observer";
            group = "observers";
          };
          installer = {
            package = self'.packages.safix;
            validate = true;
            keepGenerations = 1;
          };
        };
        environment.etc."safix-identity.txt" = {
          source = "${fixture}/identity.txt";
          mode = "0400";
        };
      };
      userManifest =
        document:
        pkgs.writeText "safix-user-hooks.json" (
          builtins.toJSON {
            version = 1;
            userMode = true;
            secretsMountPoint = "%r/safix-hook.d";
            symlinkPath = "%r/safix-hook";
            keepGenerations = 1;
            ageKeyFile = "/var/lib/safix-observer/user-identity";
            ageSshKeyPaths = [ ];
            gnupgHome = null;
            useTmpfs = true;
            logging = {
              keyImport = false;
              secretChanges = true;
            };
            manifestInputHash = null;
            secrets = [
              {
                name = "token";
                key = "private-token";
                sopsFile = document;
                format = "yaml";
                path = "%r/safix-hook/token";
                mode = "0400";
                uid = 0;
                gid = 0;
                reloadUnits = [ ];
                restartUnits = [ "safix-user-observer.service" ];
              }
            ];
            templates = [
              {
                name = "config";
                content = "token=<safix:token>\n";
                path = "%r/safix-hook/config";
                mode = "0400";
                uid = 0;
                gid = 0;
                restartUnits = [ ];
                reloadUnits = [
                  "safix-user-observer.service"
                  "safix-user-reload.service"
                ];
              }
            ];
          }
        );
    in
    {
      checks = lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
        safix-installer-vm = pkgs.testers.runNixOSTest {
          name = "safix-installer-vm";
          nodes =
            lib.genAttrs [ "machine" "systemdMachine" "userborn" ] (name: {
              imports = [ (machineFor "${fixture}/first.yaml") ];
              systemd.sysusers.enable = name == "systemdMachine";
              services.userborn.enable = name == "userborn";
              specialisation.rotated.configuration = { config, ... }: {
                safix.lib = lib.mkForce (projectionFor "${fixture}/second.yaml");
                safix.importedSecrets.account-password = {
                  sopsFile = "${fixture}/second.yaml";
                  key = "account-password";
                  neededForUsers = true;
                };
                users.users.late-user = {
                  isNormalUser = name != "systemdMachine";
                  isSystemUser = name == "systemdMachine";
                  group = "observers";
                  hashedPasswordFile = config.safix.secrets.account-password.path;
                };
              };
            })
            // {
              # The shape of sops-nix #962: userborn, an early entry, and the
              # identity on a mount needed for boot, which is stopped at
              # shutdown while the units that want it are still being stopped.
              # Drilled: RequiresMountsFor on the early unit, or a conflict
              # with shutdown.target on it, each turns the reboot red with a
              # stop-job ordering cycle through the key mount.
              userbornKeyMount =
                { config, ... }:
                {
                  imports = [ (machineFor "${fixture}/first.yaml") ];
                  boot.initrd.systemd.enable = true;
                  services.userborn.enable = true;
                  users.mutableUsers = false;
                  virtualisation.fileSystems."/var/lib/safix-key" = {
                    device = "${keyDirectory}";
                    fsType = "none";
                    options = [ "bind" ];
                    neededForBoot = true;
                  };
                  safix.identity.keyFile = lib.mkForce "/var/lib/safix-key/identity.txt";
                  safix.lib = lib.mkForce (projectionFor "${fixture}/second.yaml");
                  safix.importedSecrets.account-password = {
                    sopsFile = "${fixture}/second.yaml";
                    key = "account-password";
                    neededForUsers = true;
                  };
                  users.users.mounted-user = {
                    isNormalUser = true;
                    hashedPasswordFile = config.safix.secrets.account-password.path;
                  };
                };
            };
          testScript = ''
            start_all()
            for node in [machine, systemdMachine, userborn]:
                node.wait_for_unit("multi-user.target")
                node.succeed("systemctl is-active safix-consumer.service")
                if node in [systemdMachine, userborn]:
                    after = node.succeed("systemctl show -p After --value sysinit.target")
                    assert "safix-install-secrets.service" in after.split(), f"sysinit.target is not ordered after the installer: {after}"
                    before = node.succeed("systemctl show -p Before --value safix-install-secrets.service")
                    assert "shutdown.target" in before.split(), f"the installer is not ordered before shutdown.target: {before}"
                    conflicts = node.succeed("systemctl show -p Conflicts --value safix-install-secrets.service")
                    assert "shutdown.target" in conflicts.split(), f"the installer does not conflict with shutdown.target: {conflicts}"
                assert node.succeed("readlink /run/safix").strip() == "/run/safix.d/1"
                assert "ramfs" in node.succeed("stat -f -c %T /run/safix.d")
                assert node.succeed("cat /run/safix/shared-token") == "first-shared"
                assert node.succeed("cat /run/safix/private-token") == "first-private"
                assert node.succeed("cat /run/safix/application") == "token=first-private\n"
                assert node.succeed("stat -c %a /run/safix/shared-token").strip() == "400"
                assert node.succeed("stat -c %a /run/safix/private-token").strip() == "440"
                assert node.succeed("stat -c %U:%G /run/safix/shared-token").strip() == "root:root"
                assert node.succeed("stat -c %U:%G /run/safix/private-token").strip() == "observer:observers"
                starts_before = node.succeed("wc -l < /var/lib/safix-observer-starts").strip()
                node.fail("getent passwd late-user")
                node.succeed("/run/current-system/specialisation/rotated/bin/switch-to-configuration dry-activate >&2")
                assert node.succeed("readlink /run/safix").strip() == "/run/safix.d/1"
                assert node.succeed("cat /run/safix/private-token") == "first-private"
                assert node.succeed("cat /run/safix/application") == "token=first-private\n"
                node.fail("getent passwd late-user")
                node.succeed("/run/current-system/specialisation/rotated/bin/switch-to-configuration test >&2")
                assert node.succeed("readlink /run/safix").strip() == "/run/safix.d/2"
                assert node.succeed("cat /run/safix/private-token") == "second-private"
                assert node.succeed("cat /run/safix/shared-token") == "first-shared"
                assert node.succeed("cat /run/safix/application") == "token=second-private\n"
                assert node.succeed("getent shadow late-user").split(":")[1] == ${builtins.toJSON passwordHash}
                assert node.succeed("stat -c %U:%G:%a /run/safix-for-users/account-password").strip() == "root:root:400"
                node.fail("test -e /run/safix/account-password")
                assert node.succeed("find /run/safix.d -mindepth 1 -maxdepth 1 -type d | wc -l").strip() == "1"
                starts_after = node.succeed("wc -l < /var/lib/safix-observer-starts").strip()
                assert int(starts_after) > int(starts_before)

            # Run the actual user service manager, not a systemctl stand-in.
            node = systemdMachine
            uid = node.succeed("id -u observer").strip()
            runtime = f"/run/user/{uid}"
            node.wait_for_unit(f"user@{uid}.service")
            node.succeed("install -m600 -o observer -g observers /etc/safix-identity.txt /var/lib/safix-observer/user-identity")
            user = f"runuser -u observer -- env XDG_RUNTIME_DIR={runtime} DBUS_SESSION_BUS_ADDRESS=unix:path={runtime}/bus"
            install = user + " ${self'.packages.safix}/bin/safix install"
            node.succeed(install + " ${userManifest "${fixture}/first.yaml"}")
            node.succeed(user + " systemctl --user start safix-user-observer.service safix-user-reload.service")
            assert node.succeed(f"cat {runtime}/restart-observed") == "token=first-private\n"
            node.succeed(install + " --dry-run ${userManifest "${fixture}/second.yaml"}")
            assert node.succeed(f"cat {runtime}/safix-hook/config") == "token=first-private\n"
            assert node.succeed(f"cat {runtime}/restart-observed") == "token=first-private\n"
            node.fail(f"test -e {runtime}/reload-observed")
            node.succeed(install + " ${userManifest "${fixture}/second.yaml"}")
            node.wait_until_succeeds(f"test $(wc -l < {runtime}/restart-observed) -eq 2")
            node.wait_until_succeeds(f"test -e {runtime}/reload-observed")
            assert node.succeed(f"cat {runtime}/restart-observed") == "token=first-private\ntoken=second-private\n"
            assert node.succeed(f"cat {runtime}/reload-observed") == "token=second-private\n"
            node.fail(f"test -e {runtime}/unexpected-reload")

            # A reboot with the identity on a mount needed for boot: the
            # previous boot must have reached shutdown without an ordering cycle.
            # The shadow entry persists across boots, so the unit's own state
            # is what shows the early secret was installed on each boot.
            node = userbornKeyMount
            def mounted_user_password_set():
                node.wait_for_unit("multi-user.target")
                node.succeed("systemctl is-active safix-install-users-secrets.service")
                node.succeed("test -e /run/safix-for-users/account-password")
                assert node.succeed("getent shadow mounted-user").split(":")[1] == ${builtins.toJSON passwordHash}

            mounted_user_password_set()
            after = node.succeed("systemctl show -p After --value safix-install-users-secrets.service")
            assert "var-lib-safix" in after, f"the early installer is not ordered after the key mount: {after}"
            node.shutdown()
            node.start()
            mounted_user_password_set()
            journal = node.succeed("journalctl -b -1 --no-pager -o cat _PID=1")
            assert "Reached target System Shutdown" in journal, "no clean shutdown in the previous boot"
            cycles = [line for line in journal.splitlines() if "ordering cycle" in line]
            assert not cycles, "\n".join(cycles)
          '';
        };
      };
    };
}
