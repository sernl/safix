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
# Loosening `keyFile`'s type in `recordOf` above, from `nullOr str` to
# `nullOr (either str path)`, lets `keyFileDeclaredAsAPathTypechecks` evaluate
# true rather than throw — the same local-type limitation `database` and
# `group` above already have: this file's fixture types are hand-written to
# match ../safix/options.nix rather than derived from it, because that file is
# not standalone-evaluable outside the full flake-parts module.
# Dropping `reservedId` from the list empties `reservedIdMessages`' three
# fields, one per reserved word a mapping id may collide with.
# Dropping `fieldUnsupported` from the list empties `tagsRefusedMessages` and
# the field half of `bothFaultsMessages`, and changing
# `../safix/fields.nix`'s `channels.keepassxc.tags` from `"unsupported"` to
# `"argv"` empties `tagsRefusedMessages` alone — which is the evidence the
# refusal is driven by the shared table rather than by a hard-coded field name.
# Dropping `fieldSourceInArgv` empties `entrySourcedFieldMessages`.
# Judging either new rule over the sound mappings rather than over every
# declared one empties the field half of `bothFaultsMessages`, which is the
# same drill `reservedName` already carries.
# Forking `capabilities` in ../safix/keepassxc.nix from
# `../safix/fields.nix`'s own row turns `capabilitiesAreTheSharedTable` false.
# Re-adding a `username` option to `kdbxSide` turns
# `oldUsernameSpellingTypechecks` true, which is how the migration's
# before-and-after is evaluated against the real option tree rather than
# asserted in the changelog's prose.
#
# ../safix/checks.nix needs no new `mk*Check` for any of this: the field
# refusals are messages of `keepassxc.violationsOf`, which `keepassxcMessages`
# already returns whole, so the refusal script this file drills is the one that
# already runs.
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

      # Local rather than exported from ../safix/options.nix: that file is not
      # standalone-evaluable outside the full flake-parts module, the way
      # `database`, `group` and `mappings` below already are not either.
      yubikeySide = lib.types.submodule {
        options = {
          slot = lib.mkOption { type = lib.types.str; };
          serial = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
          };
        };
      };

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
            yubikey = lib.mkOption {
              type = lib.types.nullOr yubikeySide;
              default = null;
            };
            keyFile = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
            };
            mappings = lib.mkOption {
              type = lib.types.attrsOf keepassxc.mapping;
              default = { };
            };
          };
        }) record;

      # One fleet for every fixture, so a message that names the wrong person is
      # a failure rather than a coincidence. alice holds a hand-set entry and a
      # generated one; bob holds a hand-set entry alone.
      fleet = fleetOf {
        alice = {
          recipient = "age1fixtureaaa00000000000000000000000000000000000000000000000";
          private.tok = { };
          private.minted.generator.script = ''printf '%s' x > "$out/minted"'';
        };
        bob = {
          recipient = "age1fixturebbb00000000000000000000000000000000000000000000000";
          private.tok = { };
        };
      };

      # A fleet whose custody does not resolve: a grant to nobody. Used to hold
      # the short-circuit — while custody is broken the mirror says nothing.
      brokenFleet = fleetOf {
        alice = {
          recipient = "age1fixtureaaa00000000000000000000000000000000000000000000000";
          private.tok = { };
          sharedWith.nobody.tok = { };
        };
      };

      # `fields` is curried ahead of the four positional endpoints rather than
      # added after them, because a nix function cannot take an optional
      # positional argument: `mapping` is `mappingWith { }`, so every fixture
      # that declares no field reads exactly as it did before fields existed.
      mappingWith = fields: mode: user: name: path: {
        inherit mode;
        safix = { inherit user name; };
        kdbx = { inherit path fields; };
      };

      mapping = mappingWith { };

      violations = fleet': record: keepassxc.violationsOf { users = fleet'; } (recordOf record);

      sound = {
        database = "/nonexistent/master.kdbx";
        group = "safix";
        mappings = {
          push = mapping "safix-to-keepassxc" "alice" "tok" "alice/grafana";
          pull = mapping "keepassxc-to-safix" "bob" "tok" "bob/router";
          both = mapping "two-way" "alice" "tok" "alice/mail";
          copy = mapping "backup" "alice" "minted" "alice/minted";
        };
      };

      # `tryEval` catches the enum's refusal, which is a throw rather than a
      # message: a mode outside the four is refused by the type before any rule
      # in `violationsOf` could look at it.
      badMode =
        (builtins.tryEval (
          builtins.deepSeq (recordOf {
            mappings.a = mapping "push" "alice" "tok" "alice/grafana";
          }) "resolved"
        )).success;

      # A declared composite key reaches the projection unchanged: `recordOf`
      # types it through the real option shape, so a value that survives here
      # is a value the option system did not coerce or drop a field from.
      compositeKeyDeclared = recordOf {
        database = "/nonexistent/master.kdbx";
        yubikey = {
          slot = "1";
          serial = "12345678";
        };
        keyFile = "/home/alice/.keys/master.keyx";
      };

      # `tryEval` catches the string type's refusal, the same way `badMode`
      # catches the mode enum's: a path is a distinct nix value from a string
      # and the module system refuses it before any rule here could look at it.
      #
      # Severity: loosening `keyFile`'s type in ../safix/options.nix from
      # `nullOr str` to `nullOr (either str path)` turns this `true`, because a
      # path would then typecheck where today it is refused.
      keyFileDeclaredAsAPathTypechecks =
        (builtins.tryEval (builtins.deepSeq (recordOf { keyFile = ./mk-structural-check.nix; }) "resolved"))
        .success;

      # The migration, evaluated rather than asserted in prose. The spelling
      # this change deleted is refused by the module system itself, which is
      # the whole of what makes `kdbx.fields.username` a cutover rather than a
      # second convention beside an existing one.
      #
      # Severity: re-adding a `username` option to `kdbxSide` turns this `true`.
      oldUsernameSpellingTypechecks =
        (builtins.tryEval (
          builtins.deepSeq (recordOf {
            mappings.a = {
              mode = "safix-to-keepassxc";
              safix = {
                user = "alice";
                name = "tok";
              };
              kdbx = {
                path = "alice/grafana";
                username = "alice@example.com";
              };
            };
          }) "resolved"
        )).success;

      drill =
        pkgs.runCommand "safix-keepassxc-drill"
          { meta.description = "severity drill: safix-keepassxc-refusals"; }
          ''
            messages=$(mktemp)
            printf '%s\n' ${
              lib.escapeShellArg (
                builtins.head (violations fleet { mappings.a = mapping "two-way" "carol" "tok" "carol/x"; })
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
          companion = keepassxc.companionOf (mapping "two-way" "alice" "tok" "alice/mail");
          entryPath = keepassxc.entryPathOf "vault" (mapping "backup" "alice" "tok" "alice/mail");

          soundMessages = violations fleet sound;

          unknownUserMessages = violations fleet {
            mappings.a = mapping "safix-to-keepassxc" "carol" "tok" "carol/x";
          };

          unknownNameMessages = violations fleet {
            mappings.a = mapping "safix-to-keepassxc" "alice" "absent" "alice/x";
          };

          pullOntoGeneratedMessages = violations fleet {
            mappings.a = mapping "keepassxc-to-safix" "alice" "minted" "alice/minted";
          };

          twoWayOntoGeneratedMessages = violations fleet {
            mappings.a = mapping "two-way" "alice" "minted" "alice/minted";
          };

          # A push onto a generated entry is the ordinary case: the generator is
          # the value's only producer and the database receives a copy of what it
          # produced. `backup` is the same shape and is asserted by `sound`.
          pushOntoGeneratedMessages = violations fleet {
            mappings.a = mapping "safix-to-keepassxc" "alice" "minted" "alice/minted";
          };

          # One entry reached by two mappings that differ on the safix side, so
          # the duplicate is the database half and nothing else.
          oneEntryMessages = violations fleet {
            mappings = {
              a = mapping "safix-to-keepassxc" "alice" "tok" "alice/grafana";
              b = mapping "safix-to-keepassxc" "bob" "tok" "alice/grafana";
            };
          };

          reservedNameMessages = violations fleet {
            mappings.a = mapping "safix-to-keepassxc" "alice" "tok" "alice/grafana.safix-sync-state";
          };

          reservedIdMessages = {
            clan = violations fleet {
              mappings.clan = mapping "safix-to-keepassxc" "alice" "tok" "alice/grafana";
            };
            keepassxc = violations fleet {
              mappings.keepassxc = mapping "safix-to-keepassxc" "alice" "tok" "alice/grafana";
            };
            all = violations fleet {
              mappings.all = mapping "safix-to-keepassxc" "alice" "tok" "alice/grafana";
            };
          };

          # Two faults in one mapping, both reported, over the old rules and
          # the new ones alike. `a`'s safix side does not resolve and its entry
          # path is reserved; `b`'s safix side does not resolve and it declares
          # a field this transport cannot carry. Both second faults are judged
          # on the kdbx side alone, so the first does not suppress them.
          bothFaultsMessages = violations fleet {
            mappings = {
              a = mapping "two-way" "carol" "tok" "carol/x.safix-sync-state";
              b = mappingWith { tags = [ "work" ]; } "two-way" "carol" "tok" "carol/y";
            };
          };

          # A field this transport cannot carry at all, declared. Asserted
          # against the sentence rather than against a count, so a refusal that
          # fired naming the wrong field fails the literal.
          tagsRefusedMessages = violations fleet {
            mappings.a = mappingWith { tags = [ "work" ]; } "safix-to-keepassxc" "alice" "tok" "alice/grafana";
          };

          # A field this transport carries in an argument vector, sourced from
          # another entry — a secret value, and argv is not a channel one may
          # travel.
          entrySourcedFieldMessages = violations fleet {
            mappings.a = mappingWith {
              notes = {
                entry = "grafana-note";
              };
            } "safix-to-keepassxc" "alice" "tok" "alice/grafana";
          };

          # The three fields this transport does carry, declared as literals.
          # Without this the two fields above are vacuous: a rule that refused
          # every declaration would satisfy them both.
          soundFieldsMessages = violations fleet {
            mappings.a = mappingWith {
              username = "alice@example.com";
              url = "https://grafana.example.com";
              notes = "minted by safix";
            } "safix-to-keepassxc" "alice" "tok" "alice/grafana";
          };

          # The table this module refuses on is the shared one rather than a
          # copy of it, which is what stops the two from being forked.
          capabilities = keepassxc.capabilities;
          capabilitiesAreTheSharedTable =
            keepassxc.capabilities == (import ../safix/fields.nix { inherit lib; }).channels.keepassxc;

          # A declaration with no mapping is what a consumer who does not use
          # this evaluates, and it must be silent — including with no database
          # named, because that is the same consumer.
          emptyMirrorMessages = violations fleet { };

          # Mappings with no database named produce no message either. It is a
          # run-time refusal naming the option, because a consumer mid-way
          # through writing their declarations has a tree that still evaluates.
          noDatabaseMessages = violations fleet {
            mappings.a = mapping "safix-to-keepassxc" "alice" "tok" "alice/grafana";
          };

          brokenCustody = violations brokenFleet {
            mappings.a = mapping "safix-to-keepassxc" "carol" "tok" "carol/x";
          };

          # Without this the field above is vacuous: an empty message list proves
          # the short-circuit only if the fleet it was computed over is one
          # custody actually refuses.
          brokenCustodyIsBroken = resolve.violations { users = brokenFleet; } != [ ];

          badMode = badMode;
          compositeKeyDeclared = compositeKeyDeclared;
          keyFileDeclaredAsAPathTypechecks = keyFileDeclaredAsAPathTypechecks;
          oldUsernameSpellingTypechecks = oldUsernameSpellingTypechecks;
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
          companion = "alice/mail.safix-sync-state";
          entryPath = "vault/alice/mail";

          soundMessages = [ ];

          unknownUserMessages = [
            "flake.safix.keepassxc.mappings.a names the user 'carol', which flake.safix.users does not declare"
          ];

          unknownNameMessages = [
            "flake.safix.keepassxc.mappings.a names the secret 'absent', which flake.safix.users.alice does not hold"
          ];

          pullOntoGeneratedMessages = [
            "flake.safix.keepassxc.mappings.a is keepassxc-to-safix into flake.safix.users.alice.minted, which a generator also produces — two producers for one value, and the winner is whichever ran last"
          ];

          twoWayOntoGeneratedMessages = [
            "flake.safix.keepassxc.mappings.a is two-way into flake.safix.users.alice.minted, which a generator also produces — two producers for one value, and the winner is whichever ran last"
          ];

          pushOntoGeneratedMessages = [ ];

          oneEntryMessages = [
            "flake.safix.keepassxc.mappings a and b both name the entry safix/alice/grafana"
          ];

          reservedNameMessages = [
            "flake.safix.keepassxc.mappings.a names the entry safix/alice/grafana.safix-sync-state, and '.safix-sync-state' is the suffix safix reserves for the entry a two-way mapping records its last agreement in"
          ];

          reservedIdMessages = {
            clan = [
              "flake.safix.keepassxc.mappings.clan is named 'clan', which sync and audit read as a target keyword rather than a mapping name"
            ];
            keepassxc = [
              "flake.safix.keepassxc.mappings.keepassxc is named 'keepassxc', which sync and audit read as a target keyword rather than a mapping name"
            ];
            all = [
              "flake.safix.keepassxc.mappings.all is named 'all', which sync and audit read as a target keyword rather than a mapping name"
            ];
          };

          bothFaultsMessages = [
            "flake.safix.keepassxc.mappings.a names the user 'carol', which flake.safix.users does not declare"
            "flake.safix.keepassxc.mappings.b names the user 'carol', which flake.safix.users does not declare"
            "flake.safix.keepassxc.mappings.a names the entry safix/carol/x.safix-sync-state, and '.safix-sync-state' is the suffix safix reserves for the entry a two-way mapping records its last agreement in"
            "flake.safix.keepassxc.mappings.b declares the field 'tags', and keepassxc cannot carry it: the store's own command has no way to write it, so accepting the declaration would mean writing less than it says"
          ];

          tagsRefusedMessages = [
            "flake.safix.keepassxc.mappings.a declares the field 'tags', and keepassxc cannot carry it: the store's own command has no way to write it, so accepting the declaration would mean writing less than it says"
          ];

          entrySourcedFieldMessages = [
            "flake.safix.keepassxc.mappings.a sources the field 'notes' from the entry 'grafana-note', and keepassxc carries that field in an argument vector, where a secret value may not travel"
          ];

          soundFieldsMessages = [ ];

          capabilities = {
            username = "argv";
            url = "argv";
            notes = "argv";
            tags = "unsupported";
          };
          capabilitiesAreTheSharedTable = true;

          emptyMirrorMessages = [ ];
          noDatabaseMessages = [ ];
          brokenCustody = [ ];
          brokenCustodyIsBroken = true;
          badMode = false;
          compositeKeyDeclared = {
            database = "/nonexistent/master.kdbx";
            group = "safix";
            yubikey = {
              slot = "1";
              serial = "12345678";
            };
            keyFile = "/home/alice/.keys/master.keyx";
            mappings = { };
          };
          keyFileDeclaredAsAPathTypechecks = false;
          oldUsernameSpellingTypechecks = false;
        };
      };

      checks.safix-keepassxc-drill = drill;
    };
}
