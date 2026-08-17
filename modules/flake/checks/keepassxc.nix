# Holds the mirror refusals of ../safix/keepassxc.nix against fleets built to
# break each one, and holds the boundary of what evaluation is allowed to claim.
#
# Every fixture is synthetic and no database exists. That is deliberate rather
# than incidental: the database half of a mapping is content of an encrypted
# file, and nothing here may reach it. A fixture whose database were real would
# make this file's passing depend on a key being present, which is precisely the
# property the surface is designed not to have — and would point a build at
# somebody's own password store.
#
# Each refusal is asserted as the message it produces, against a literal, and
# the well-formed declaration is asserted to produce none. The pair is what binds
# them: a refusal that stopped firing empties its own field, and a refusal that
# fired naming the wrong party fails the literal.
#
# The drill runs `refuseScript` — the same bytes `mkMessageCheck` runs — over a
# perturbed declaration, so the severity claim is executed rather than described.
#
# ── what this cannot check ──
# That the group exists, that the entry does, that either side holds a value, or
# that the database opens. All four need the database and a key, and a build that
# asserted any of them would be asserting something about whichever database
# happened to be in reach.
#
# And whether a value can round-trip through the store's own command, which is
# the one refusal a reader might expect here and which cannot live here: the
# command's entry password is a single line, so a value carrying a newline cannot
# survive it, and whether a value carries one is invisible at evaluation.
# `sync_path.rs`'s `a_value_carrying_a_newline_is_refused_rather_than_normalised`
# is where that refusal lives.
#
# ── severity: proven by perturbation, one drill per claim ──
# Dropping `unresolvableSafixSide` from the list `violationsOf` returns empties
# `unknownUserMessages` and `unknownNameMessages` and moves no other field.
# Dropping `twoProducers` empties `pullOntoGeneratedMessages` and
# `twoWayOntoGeneratedMessages`, and leaves `pushOntoGeneratedMessages` empty as
# it already is: a push onto a generated entry is not a second producer, because
# safix's side is the one being read.
# Dropping `twoMappingsOneEntry` empties `oneEntryMessages`.
# Dropping `reservedName` empties `reservedNameMessages`, and that field is the
# whole of what makes the companion name structural: without the refusal a
# consumer can declare the entry a two-way mapping records its agreement in, and
# the two name spaces overlap.
# Judging `reservedName` over the sound mappings rather than over every declared
# one empties `bothFaultsMessages` of its reserved-name half, which is the claim
# that a mapping with two faults hears about both.
# Dropping the `resolve.violations` short-circuit fills `brokenCustody` with
# mirror sentences about a fleet whose custody has not resolved, which is one
# fault producing two unrelated messages.
# Removing the mode enum lets `badMode` evaluate rather than throw.
{
  perSystem =
    {
      pkgs,
      lib,
      ...
    }:
    let
      keepassxc = import ../safix/keepassxc.nix { inherit lib; };
      resolve = import ../safix/resolve.nix { inherit lib; };
      types = import ../safix/types.nix { inherit lib; };
      safixChecks = import ../safix/checks.nix { inherit lib; };
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      # Typed through the real option types, so a fixture cannot pass by omitting
      # a field the option system would have supplied, and an option rename
      # breaks this file along with the rest.
      typed =
        optionType: definition:
        (lib.evalModules {
          modules = [
            { options.value = lib.mkOption { type = optionType; }; }
            { value = definition; }
          ];
        }).config.value;

      fleetOf = users: typed (lib.types.attrsOf types.profile) users;

      recordOf =
        record:
        typed (lib.types.submodule {
          options = {
            database = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
            };
            group = lib.mkOption {
              type = lib.types.str;
              default = "safix";
            };
            mappings = lib.mkOption {
              type = lib.types.attrsOf keepassxc.mapping;
              default = { };
            };
          };
        }) record;

      # One fleet for every fixture, so a message that names the wrong person is
      # a failure rather than a coincidence. ana holds a hand-set entry and a
      # generated one; bo holds a hand-set entry alone.
      fleet = fleetOf {
        ana = {
          recipient = "age1fixtureaaa00000000000000000000000000000000000000000000000";
          private.tok = { };
          private.minted.generator.script = ''printf '%s' x > "$out/minted"'';
        };
        bo = {
          recipient = "age1fixturebbb00000000000000000000000000000000000000000000000";
          private.tok = { };
        };
      };

      # A fleet whose custody does not resolve: a grant to nobody. Used to hold
      # the short-circuit — while custody is broken the mirror says nothing.
      brokenFleet = fleetOf {
        ana = {
          recipient = "age1fixtureaaa00000000000000000000000000000000000000000000000";
          private.tok = { };
          sharedWith.nobody.tok = { };
        };
      };

      mapping = mode: user: name: path: {
        inherit mode;
        safix = { inherit user name; };
        kdbx = { inherit path; };
      };

      violations = fleet': record: keepassxc.violationsOf fleet' { } (recordOf record);

      sound = {
        database = "/nonexistent/master.kdbx";
        group = "safix";
        mappings = {
          push = mapping "safix-to-keepassxc" "ana" "tok" "ana/grafana";
          pull = mapping "keepassxc-to-safix" "bo" "tok" "bo/router";
          both = mapping "two-way" "ana" "tok" "ana/mail";
          copy = mapping "backup" "ana" "minted" "ana/minted";
        };
      };

      # `tryEval` catches the enum's refusal, which is a throw rather than a
      # message: a mode outside the four is refused by the type before any rule
      # in `violationsOf` could look at it.
      badMode =
        (builtins.tryEval (
          builtins.deepSeq (recordOf {
            mappings.a = mapping "push" "ana" "tok" "ana/grafana";
          }) "resolved"
        )).success;

      drill =
        pkgs.runCommand "safix-keepassxc-drill"
          { meta.description = "severity drill: safix-keepassxc-refusals"; }
          ''
            messages=$(mktemp)
            printf '%s\n' ${
              lib.escapeShellArg (
                builtins.head (violations fleet { mappings.a = mapping "two-way" "cy" "tok" "cy/x"; })
              )
            } > "$messages"

            if ${safixChecks.refuseScript pkgs} "$messages" "subject" 2> refused; then
              echo "the refusal script accepted a non-empty message list" >&2
              exit 1
            fi
            grep -q "which flake.safix.users does not declare" refused
            grep -q "subject" refused

            : > "$messages"
            ${safixChecks.refuseScript pkgs} "$messages" "subject"
            touch "$out"
          '';
    in
    {
      checks.safix-keepassxc = mkStructuralCheck {
        name = "safix-keepassxc";
        actual = {
          modes = keepassxc.modes;

          # Which modes make safix's side a destination, read off the same
          # predicate the refusal reads. A mode added to the list above without a
          # decision about this fails here rather than at somebody's terminal.
          pullCapable = map keepassxc.pullCapable keepassxc.modes;

          stateSuffix = keepassxc.stateSuffix;
          companion = keepassxc.companionOf (mapping "two-way" "ana" "tok" "ana/mail");
          entryPath = keepassxc.entryPathOf "vault" (mapping "backup" "ana" "tok" "ana/mail");

          soundMessages = violations fleet sound;

          unknownUserMessages = violations fleet {
            mappings.a = mapping "safix-to-keepassxc" "cy" "tok" "cy/x";
          };

          unknownNameMessages = violations fleet {
            mappings.a = mapping "safix-to-keepassxc" "ana" "absent" "ana/x";
          };

          pullOntoGeneratedMessages = violations fleet {
            mappings.a = mapping "keepassxc-to-safix" "ana" "minted" "ana/minted";
          };

          twoWayOntoGeneratedMessages = violations fleet {
            mappings.a = mapping "two-way" "ana" "minted" "ana/minted";
          };

          # A push onto a generated entry is the ordinary case: the generator is
          # the value's only producer and the database receives a copy of what it
          # produced. `backup` is the same shape and is asserted by `sound`.
          pushOntoGeneratedMessages = violations fleet {
            mappings.a = mapping "safix-to-keepassxc" "ana" "minted" "ana/minted";
          };

          # One entry reached by two mappings that differ on the safix side, so
          # the duplicate is the database half and nothing else.
          oneEntryMessages = violations fleet {
            mappings = {
              a = mapping "safix-to-keepassxc" "ana" "tok" "ana/grafana";
              b = mapping "safix-to-keepassxc" "bo" "tok" "ana/grafana";
            };
          };

          reservedNameMessages = violations fleet {
            mappings.a = mapping "safix-to-keepassxc" "ana" "tok" "ana/grafana.safix-sync-state";
          };

          # Two faults in one mapping, both reported. The safix side does not
          # resolve and the entry path is reserved, and the second is judged on
          # the database half alone so the first does not suppress it.
          bothFaultsMessages = violations fleet {
            mappings.a = mapping "two-way" "cy" "tok" "cy/x.safix-sync-state";
          };

          # A declaration with no mapping is what a consumer who does not use
          # this evaluates, and it must be silent — including with no database
          # named, because that is the same consumer.
          emptyMirrorMessages = violations fleet { };

          # Mappings with no database named produce no message either. It is a
          # run-time refusal naming the option, because a consumer mid-way
          # through writing their declarations has a tree that still evaluates.
          noDatabaseMessages = violations fleet {
            mappings.a = mapping "safix-to-keepassxc" "ana" "tok" "ana/grafana";
          };

          brokenCustody = violations brokenFleet {
            mappings.a = mapping "safix-to-keepassxc" "cy" "tok" "cy/x";
          };

          # Without this the field above is vacuous: an empty message list proves
          # the short-circuit only if the fleet it was computed over is one
          # custody actually refuses.
          brokenCustodyIsBroken = resolve.violations brokenFleet { } != [ ];

          badMode = badMode;
        };
        expected = {
          modes = [
            "safix-to-keepassxc"
            "keepassxc-to-safix"
            "two-way"
            "backup"
          ];

          pullCapable = [
            false
            true
            true
            false
          ];

          stateSuffix = ".safix-sync-state";
          companion = "ana/mail.safix-sync-state";
          entryPath = "vault/ana/mail";

          soundMessages = [ ];

          unknownUserMessages = [
            "flake.safix.keepassxc.mappings.a names the user 'cy', which flake.safix.users does not declare"
          ];

          unknownNameMessages = [
            "flake.safix.keepassxc.mappings.a names the secret 'absent', which flake.safix.users.ana does not hold"
          ];

          pullOntoGeneratedMessages = [
            "flake.safix.keepassxc.mappings.a is keepassxc-to-safix into flake.safix.users.ana.minted, which a generator also produces — two producers for one value, and the winner is whichever ran last"
          ];

          twoWayOntoGeneratedMessages = [
            "flake.safix.keepassxc.mappings.a is two-way into flake.safix.users.ana.minted, which a generator also produces — two producers for one value, and the winner is whichever ran last"
          ];

          pushOntoGeneratedMessages = [ ];

          oneEntryMessages = [
            "flake.safix.keepassxc.mappings a and b both name the entry safix/ana/grafana"
          ];

          reservedNameMessages = [
            "flake.safix.keepassxc.mappings.a names the entry safix/ana/grafana.safix-sync-state, and '.safix-sync-state' is the suffix safix reserves for the entry a two-way mapping records its last agreement in"
          ];

          bothFaultsMessages = [
            "flake.safix.keepassxc.mappings.a names the user 'cy', which flake.safix.users does not declare"
            "flake.safix.keepassxc.mappings.a names the entry safix/cy/x.safix-sync-state, and '.safix-sync-state' is the suffix safix reserves for the entry a two-way mapping records its last agreement in"
          ];

          emptyMirrorMessages = [ ];
          noDatabaseMessages = [ ];
          brokenCustody = [ ];
          brokenCustodyIsBroken = true;
          badMode = false;
        };
      };

      checks.safix-keepassxc-drill = drill;
    };
}
