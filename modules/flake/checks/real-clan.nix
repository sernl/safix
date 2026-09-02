# The one check that drives the real clan, over a clan built for it.
#
# Every other bridge check drives the stub in `crates/safix/tests/support/`, and
# that file states what a stub can establish. What it cannot establish is that
# the argument vectors safix sends mean to clan what safix thinks they mean: a
# stub answers them because it was written to, and would go on answering them
# after clan changed its command line, its output convention or its wording.
#
# So this check exists, and it is absent rather than trivially green where it
# cannot be made. Two conditions gate it, and both are real:
#
#   - linux, because generating a var runs the generator under bubblewrap and
#     clan's own `age` vars tests are marked broken on darwin. This is the same
#     shape and the same reasoning as `safix-syscall-proof`'s.
#   - clan-cli in the closure, which here means the `clan-core` input being
#     present. clan-cli is not packaged in nixpkgs, so there is no attribute to
#     fall back to and no third state to worry about.
#
# `crates/safix/tests/real_clan.rs` carries the other half of the absence: the
# suite compiled without a clan in its environment says what it did not do, so a
# `cargo test` in a devshell neither fails for want of a clan nor claims to have
# driven one.
#
# # Running clan inside a build sandbox
#
# clan evaluates its own flake and builds its own derivations, and a build
# sandbox has no daemon, no network and a read-only store. clan solves this for
# its own test suite and the solution is used here rather than reinvented; the
# pieces are all clan's, and `pkgs/testing/flake-module.nix` in clan-core is
# where its `setupNixInNix` lives.
#
#   - a private chroot store under the build directory, seeded by copying a
#     `closureInfo` into it and loading that closure's registration, so the
#     nested nix has a store it may write to;
#   - `CLAN_TEST_STORE`, which clan-cli reads in `clan_lib/nix/__init__.py` and
#     turns into `--store` on every nix invocation it makes;
#   - `IN_NIX_SANDBOX`, which makes `clan_lib/nix/shell.py` take its runtime
#     tools off `PATH` instead of resolving them with `nix build`, which is the
#     one thing that genuinely cannot work against a read-only store;
#   - `clan-core.packages.clan-core-flake`, a copy of clan-core whose lock names
#     store paths rather than URLs, which is what lets the throwaway clan lock
#     with no network at all.
#
# The throwaway clan's nixpkgs is this flake's, injected over clan-core's own.
# clan-core supports the injection deliberately — it sets
# `clan.checks.minNixpkgsVersion.ignore` for exactly this — and the alternative
# is a second nixpkgs in the closure, instantiated a second time, for a `stdenv`
# the check would then have to build twice.
#
# # The clan the check builds
#
# Two machines, six `age`-backed generators between them, an identity minted
# here and a recipient derived from it. Three generators on `meridian` differ
# in the two axes that turn out to matter, and the real clan is what
# established that the second axis exists; a fourth on `meridian`, `orphan`,
# is `enumerate-clan-namespace`'s own addition, a var no bridge mapping this
# check declares ever names; the fifth, `bothways`, and the sixth,
# `everywhere`, are both `share = true`, the one declared on `meridian`
# alone and the other on both machines:
#
#   - `ntfy` declares `validation` and is generated. `validationHash` is null
#     unless a generator declares `validation`, and `hash_is_valid` calls a
#     null-in-nix, null-on-disk pair valid for backwards compatibility, so a
#     generator without it is one clan will never call stale and the drift
#     refusal would have nothing to refuse.
#   - `handover` declares no `validation` and is not generated. This is the
#     ordinary export target: a var clan declares, holds nothing for, and has no
#     recorded validation to be out of step with.
#   - `scheduled` declares `validation` and is not generated. clan reports this
#     one as having an outdated invalidation hash — a declared validation with
#     nothing recorded beside it does not match — so it is the state safix's
#     drift refusal fires on at a first export, and it is correct that it does:
#     such a generator has not run and will, and the run would overwrite
#     whatever the export wrote.
#   - `orphan` declares no `validation` and is not generated, the same shape as
#     `handover`. No mapping in this check's fleet ever names it, so `clan vars
#     list meridian` reports it and `audit clan`'s lingering section is what is
#     expected to name it — real evidence that `Clan::list`'s parsing and the
#     claimed-set computation hold against the real command's own output shape,
#     not only against `tests/support/clan-stub.rs`'s.
#
# The third was not anticipated. It is what a real clan establishes and a stub
# cannot: the stub's staleness is a switch a test throws, so no fixture of it
# would have produced a generator that is stale for having never run.
#
# `bothways` is the fifth, `share = true`, declared on `meridian` alone and
# minted there; `aurora` declares no generator named `bothways`. This is
# group 8's own addressing-search fixture, and it exists for the same reason
# the third generator does: `crates/safix/tests/bridge_sync.rs`'s stubbed
# clan can only ever answer "Couldn't find var" because it was told to, for
# any machine name a test hands it, so it cannot establish that a real,
# unrelated second machine — one that genuinely does not declare the
# generator — is what makes `clan vars get` say that. Only a real clan can be
# asked and answer honestly, which is what makes this the one place the
# addressing search's skip-a-real-candidate property is established rather
# than merely modelled.
#
# `everywhere` is the sixth, declared identically on `meridian` and `aurora`
# and never generated on either — `enumerate-clan-namespace`'s own group-1
# fixture, holding design.md's Context finding that a `Shared`-placed
# generator's var is present in `clan vars list <machine>` on every machine
# whose own configuration declares it, real evidence a stub cannot give
# because the stub answers each `list` call in isolation rather than from one
# shared registry two machine listings both read.
#
# # The drills, and what they were observed to do
#
# Withholding `SAFIX_TEST_REAL_CLAN_SEED` makes every test in the target report
# the absence and return, and libtest then says eighteen passed. Observed: the
# result-line guard below caught it and the check failed. That is the failure
# this whole shape exists to prevent — an attribute that is present, green, and
# asserting nothing.
#
# Putting the stub in `SAFIX_TEST_REAL_CLAN`'s place, which is the strongest
# available statement that the real command is what is under test. Observed:
# every test but the ungenerated-var outcome failed. That one passes because
# the stub's answer for that state was written against this
# clan's — which is the stub being right rather than the drill being weak.
{ inputs, ... }:
{
  perSystem =
    {
      config,
      lib,
      pkgs,
      system,
      ...
    }:
    let
      integration = import ./integration.nix { inherit pkgs; };

      clanCli = inputs.clan-core.packages.${system}.clan-cli;
      clanCoreFlake = inputs.clan-core.packages.${system}.clan-core-flake;

      machine = "meridian";

      # A real second machine, declaring no generator at all. Group 8's
      # addressing-search test needs one — see "The clan the check builds"
      # above.
      secondMachine = "aurora";

      generator =
        { value, validation, share ? false }:
        {
          files.token.secret = true;
          script = ''echo -n ${value} > "$out"/token'';
        }
        // lib.optionalAttrs (validation != null) { validation.revision = validation; }
        // lib.optionalAttrs share { share = true; };

      machineConfiguration = builtins.toJSON {
        nixpkgs.hostPlatform = system;
        clan.core = {
          # Every state-version read is a var operation, and this clan has no
          # host to have a state version of.
          settings.state-version.enable = false;
          vars = {
            settings.secretStore = "age";
            generators = {
              ntfy = generator {
                value = "CANARY-minted-by-clan";
                validation = 1;
              };
              handover = generator {
                value = "CANARY-never-minted";
                validation = null;
              };
              scheduled = generator {
                value = "CANARY-would-be-minted";
                validation = 1;
              };
              orphan = generator {
                value = "CANARY-never-claimed";
                validation = null;
              };
              # `bothways` is `share = true`, so it is stored once under
              # `vars/shared/` rather than per-machine — declared here, on
              # `meridian`, and not on `aurora` at all.
              bothways = generator {
                value = "CANARY-shared-and-real";
                validation = null;
                share = true;
              };
              # `everywhere` is declared identically on both machines below,
              # which is `enumerate-clan-namespace`'s own group-1 fixture: a
              # shared generator's var must appear with the identical id in
              # `clan vars list` on every machine that declares it
              # (design.md's Context, citing `get_machine_generators`,
              # `clan_lib/vars/generator.py:229-382`). It is never generated
              # on either machine, so both listings show it `<not set>`.
              everywhere = generator {
                value = "CANARY-declared-on-both-machines";
                validation = null;
                share = true;
              };
            };
          };
        };
      };

      # `aurora` declares no generator `bothways` names, which is what makes
      # it a real candidate `bothways`'s addressing search must skip; it does
      # declare `everywhere`, identically to `meridian`, for the group-1
      # shared-listing fixture above.
      secondMachineConfiguration = builtins.toJSON {
        nixpkgs.hostPlatform = system;
        clan.core = {
          settings.state-version.enable = false;
          vars = {
            settings.secretStore = "age";
            generators.everywhere = generator {
              value = "CANARY-declared-on-both-machines";
              validation = null;
              share = true;
            };
          };
        };
      };

      # Everything the nested nix has to find in the private store: the tools
      # clan runs, and the build closure of the generator script it builds there.
      # The three derivation paths and `lndir` are what a nested build needs to
      # run at all rather than what it produces.
      nested = pkgs.closureInfo {
        rootPaths = [
          clanCli
          clanCoreFlake
          pkgs.age
          pkgs.bash
          pkgs.bubblewrap
          pkgs.coreutils
          pkgs.git
          pkgs.gnutar
          pkgs.gzip
          pkgs.jq.dev
          pkgs.nix
          pkgs.stdenv
          pkgs.stdenvNoCC
          pkgs.stdenv.drvPath
          pkgs.stdenvNoCC.drvPath
          pkgs.bash.drvPath
          pkgs.buildPackages.lndir
          (pkgs.closureInfo { rootPaths = [ ]; }).drvPath
        ];
      };
    in
    {
      checks = lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
        safix-bridge-real-clan =
          pkgs.runCommand "safix-bridge-real-clan"
            {
              nativeBuildInputs = integration.backends ++ [
                clanCli
                pkgs.bubblewrap
                pkgs.findutils
              ];
              closureInfo = nested;
              env = {
                SAFIX_TEST_BINARY = "${config.checks.safix-integration}/libexec/safix";
                SAFIX_TEST_NIX_STUB = "${config.checks.safix-integration}/libexec/safix-nix-stub";
                SAFIX_TEST_SHIM = "${config.checks.safix-integration}/libexec/safix-test-shim";
                SAFIX_TEST_CLAN_STUB = "${config.checks.safix-integration}/libexec/safix-clan-stub";
                SAFIX_TEST_REAL_CLAN = "${clanCli}/bin/clan";
                inherit machineConfiguration secondMachineConfiguration;
              };
            }
            ''
              set -euo pipefail

              export HOME="$TMPDIR"
              export NIX_STATE_DIR="$TMPDIR/nix"
              export NIX_CONF_DIR="$TMPDIR/etc"
              mkdir -p "$NIX_CONF_DIR"
              # The nested store is thrown away with the build, so its metadata
              # need not survive a crash it will not outlive.
              echo "fsync-metadata = false" > "$NIX_CONF_DIR/nix.conf"

              export IN_NIX_SANDBOX=1
              export CLAN_TEST_STORE="$TMPDIR/store"
              export LOCK_NIX="$TMPDIR/nix-lock"
              mkdir -p "$CLAN_TEST_STORE/nix/store" "$CLAN_TEST_STORE/nix/var/nix/gcroots"
              xargs -r -P"$(nproc)" cp --recursive --no-dereference --reflink=auto \
                --target-directory "$CLAN_TEST_STORE/nix/store" < "$closureInfo/store-paths"
              nix-store --load-db --store "$CLAN_TEST_STORE" < "$closureInfo/registration"

              clan="$TMPDIR/seed-clan"
              mkdir -p "$clan/machines/${machine}" "$clan/machines/${secondMachine}" "$clan/.age"

              # Minted here, per run, and never leaving this build: the recipient
              # the throwaway clan encrypts to is derived from it and nothing else
              # is ever encrypted to either half.
              age-keygen -o "$clan/.age/key.txt" 2>/dev/null
              recipient="$(age-keygen -y "$clan/.age/key.txt")"
              export AGE_KEYFILE="$clan/.age/key.txt"

              cat > "$clan/flake.nix" <<EOF
              {
                inputs.clan-core.url = "path://${clanCoreFlake}";
                inputs.nixpkgs.url = "path://${pkgs.path}";
                inputs.clan-core.inputs.nixpkgs.follows = "nixpkgs";

                outputs =
                  { self, clan-core, ... }:
                  let
                    clan = clan-core.lib.clan {
                      imports = [ ./clan.nix ];
                      inherit self;
                      meta.name = "safix-throwaway";
                    };
                  in
                  {
                    inherit (clan.config) nixosConfigurations nixosModules clanInternals;
                    clan = clan.config;
                  };
              }
              EOF

              cat > "$clan/clan.nix" <<EOF
              {
                vars.settings.recipients.hosts.${machine} = [ "$recipient" ];
                vars.settings.recipients.hosts.${secondMachine} = [ "$recipient" ];
              }
              EOF

              printf '%s' "$machineConfiguration" \
                > "$clan/machines/${machine}/configuration.json"
              cat > "$clan/machines/${machine}/configuration.nix" <<'EOF'
              { ... }:
              {
                imports = [ (builtins.fromJSON (builtins.readFile ./configuration.json)) ];
              }
              EOF

              printf '%s' "$secondMachineConfiguration" \
                > "$clan/machines/${secondMachine}/configuration.json"
              cat > "$clan/machines/${secondMachine}/configuration.nix" <<'EOF'
              { ... }:
              {
                imports = [ (builtins.fromJSON (builtins.readFile ./configuration.json)) ];
              }
              EOF

              git -C "$clan" init -q -b main
              # In the repository rather than in the environment, because the
              # copies the suite makes of this clan are committed in by safix's own
              # runs and inherit only what travels with the directory.
              git -C "$clan" config user.name "safix-selftest"
              git -C "$clan" config user.email "selftest@example.com"
              git -C "$clan" add -A
              git -C "$clan" commit -qm "the throwaway clan" --no-gpg-sign

              nix flake lock "$clan" \
                --store "$CLAN_TEST_STORE" \
                --extra-experimental-features 'nix-command flakes'
              git -C "$clan" add -A
              git -C "$clan" commit -qm "lock it to store paths" --no-gpg-sign

              # One of the two, so that the other stays in the state a clan is in
              # before its first generation.
              clan vars generate --flake "$clan" --generator ntfy ${machine}

              # The shared generator, minted on the machine that declares
              # it. `${secondMachine}` never generates it and never declares
              # it, which is what group 8's addressing-search test needs: a
              # real, unrelated machine for the search to try and skip.
              clan vars generate --flake "$clan" --generator bothways ${machine}

              export SAFIX_TEST_REAL_CLAN_SEED="$clan"

              set +e
              output="$(${config.checks.safix-integration}/bin/real_clan \
                --nocapture --test-threads 1 2>&1)"
              status=$?
              set -e

              printf '%s\n' "$output"
              if [ "$status" != 0 ]; then
                echo "the suite refused real_clan" >&2
                exit "$status"
              fi

              # The same guard `integration.nix` applies, and it earns its place
              # twice over here: this target's tests report an absent clan and
              # return, so a run that reached none of them would look exactly like
              # a run that reached all of them.
              case "$output" in
                *"test result: ok. 0 passed"*)
                  echo "no test ran: real_clan built nothing to run" >&2
                  exit 1
                  ;;
                *"test result: ok."*) ;;
                *)
                  echo "the suite reported no result for real_clan" >&2
                  exit 1
                  ;;
              esac
              case "$output" in
                *"no real clan in this environment"*)
                  echo "the check supplied no clan, so its tests established nothing" >&2
                  exit 1
                  ;;
              esac

              touch "$out"
            '';
      };
    };
}
