# Holds the refusals of ../safix/pass.nix against fleets built to break each
# one, and holds the boundary of what evaluation is allowed to claim.
#
# Every fixture is synthetic and no store exists. That is deliberate rather than
# incidental: the far side of a mapping is a gpg-encrypted file in a tree, and
# nothing here may reach it. A fixture whose store were real would make this
# file's passing depend on a key being present, which is precisely the property
# the surface is designed not to have — and would point a build at somebody's
# own password store.
#
# Each refusal is asserted as the message it produces, against a literal, and
# the well-formed declaration is asserted to produce none. The pair is what
# binds them: a refusal that stopped firing empties its own field, and a refusal
# that fired naming the wrong party fails the literal.
#
# The drill runs `refuseScript` — the same bytes `mkMessageCheck` runs — over a
# perturbed declaration, so the severity claim is executed rather than
# described.
#
# ── what this cannot check ──
# Whether the store exists, whether it declares recipients, whether an entry is
# there, or whether either side holds a value. All four need the store and the
# operator's own key, and a build that asserted any of them would be asserting
# something about whichever store happened to be in reach. `pass_path.rs` and
# `pass_cli.rs` are where those claims live — the first against a stub, the
# second against a real `pass` over a store the check mints in its own
# directory.
#
# And whether a record round-trips through the store's own command. The layout
# is safix's own (design D2) and its properties are byte-level ones, which is
# what `crates/safix-core/src/pass.rs`'s own unit tests and `safix-pass-cli`
# assert; nothing about a body's bytes is visible at evaluation.
#
# ── severity: proven by perturbation, one drill per claim ──
# Dropping `unresolvableSafixSide` from the list `violationsOf` returns empties
# `unknownUserMessages` and `unknownNameMessages` and moves no other field.
# Dropping `twoProducers` empties `pullOntoGeneratedMessages` and
# `twoWayOntoGeneratedMessages`, and leaves `pushOntoGeneratedMessages` empty as
# it already is: a push onto a generated entry is not a second producer, because
# safix's side is the one being read.
# Dropping `twoMappingsOneEntry` empties `oneEntryMessages`.
# Dropping `reservedName` empties `reservedNameMessages` and moves no other
# field, and that field is the whole of what makes the companion name
# structural: without the refusal a consumer can declare the entry a two-way
# mapping records its agreement in, and the two name spaces overlap.
# Judging `reservedName` over the sound mappings rather than over every declared
# one empties `bothFaultsMessages` of its reserved-name half, which is the claim
# that a mapping with two faults hears about both.
# Dropping `reservedId` from the list empties `reservedIdMessages`' six fields,
# one per reserved word a mapping id may collide with.
# Dropping the `resolve.violations` short-circuit fills `brokenCustody` with
# pass sentences about a fleet whose custody has not resolved, which is one
# fault producing two unrelated messages.
# Removing the mode enum lets `badMode` evaluate rather than throw.
# Giving `store` the type `lib.types.path` in `recordOf` below turns
# `storeDeclaredAsAPathTypechecks` true, which is the local half of the drill
# ../safix/options.nix's own reason states: a path stringifies to a
# root-dependent absolute path, so `safix-examples` reddens the moment an
# example declares one, and this file holds the type refusal that keeps that
# from being reachable — the same local-type limitation `database` and `group`
# in ./keepassxc.nix already have, because ../safix/options.nix is not
# standalone-evaluable outside the full flake-parts module.
# Forking `capabilities` in ../safix/pass.nix from ../safix/fields.nix's own row
# turns `capabilitiesAreTheSharedTable` false.
# Changing any row of that table from `"stdin"` to `"argv"` or `"unsupported"`
# turns `everyFieldCrossesOnAPipe` false, which is the evaluation half of "this
# target carries all four"; the runtime half is `pass.rs`'s own
# `capabilities_carry_all_four_fields_on_a_pipe`.
#
# ../safix/checks.nix needs no rule-specific `mk*Check` beyond `mkPassCheck`:
# every refusal here is a message of `pass.violationsOf`, which `passMessages`
# returns whole, so the refusal script this file drills is the one that already
# runs.
{
  perSystem =
    {
      pkgs,
      lib,
      ...
    }:
    let
      pass = import ../safix/pass.nix { inherit lib; };
      resolve = import ../safix/resolve.nix { inherit lib; };
      types = import ../safix/types.nix { inherit lib; };
      safixChecks = import ../safix/checks.nix { inherit lib; };
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      # Typed through the real option types, so a fixture cannot pass by
      # omitting a field the option system would have supplied, and an option
      # rename breaks this file along with the rest.
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
            store = lib.mkOption {
              type = lib.types.str;
              default = "~/.password-store";
            };
            mappings = lib.mkOption {
              type = lib.types.attrsOf pass.mapping;
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
      # the short-circuit — while custody is broken this target says nothing.
      brokenFleet = fleetOf {
        alice = {
          recipient = "age1fixtureaaa00000000000000000000000000000000000000000000000";
          private.tok = { };
          sharedWith.nobody.tok = { };
        };
      };

      # `fields` is curried ahead of the four positional endpoints rather than
      # added after them, because a nix function cannot take an optional
      # positional argument: `mapping` is `mappingWith { }`.
      mappingWith = fields: mode: user: name: path: {
        inherit mode;
        safix = { inherit user name; };
        pass = { inherit path fields; };
      };

      mapping = mappingWith { };

      violations = fleet': record: pass.violationsOf { users = fleet'; } (recordOf record);

      sound = {
        store = "~/.password-store";
        mappings = {
          push = mapping "safix-to-pass" "alice" "tok" "alice/grafana";
          pull = mapping "pass-to-safix" "bob" "tok" "bob/router";
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

      # `tryEval` catches the string type's refusal, the same way `badMode`
      # catches the mode enum's: a path is a distinct nix value from a string
      # and the module system refuses it before any rule here could look at it.
      storeDeclaredAsAPathTypechecks =
        (builtins.tryEval (builtins.deepSeq (recordOf { store = ./mk-structural-check.nix; }) "resolved"))
        .success;

      # One field per reserved word, six of them, so a word dropped from the
      # shared list empties exactly its own field.
      reservedIdMessagesFor =
        id:
        violations fleet {
          mappings.${id} = mapping "safix-to-pass" "alice" "tok" "alice/grafana";
        };

      reserved = import ../safix/reserved.nix;

      drill =
        pkgs.runCommand "safix-pass-drill" { meta.description = "severity drill: safix-pass-refusals"; }
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
      checks.safix-pass = mkStructuralCheck {
        name = "safix-pass";
        actual = {
          modes = pass.modes;

          # Which modes make safix's side a destination, read off the same
          # predicate the refusal reads. A mode added to the list above without
          # a decision about this fails here rather than at somebody's terminal.
          pullCapable = map pass.pullCapable pass.modes;

          stateSuffix = pass.stateSuffix;
          companion = pass.companionOf (mapping "two-way" "alice" "tok" "alice/mail");

          # No group argument, where `keepassxc.entryPathOf` takes one: a pass
          # path is already absolute within the store.
          entryPath = pass.entryPathOf (mapping "backup" "alice" "tok" "alice/mail");

          soundMessages = violations fleet sound;

          unknownUserMessages = violations fleet {
            mappings.a = mapping "safix-to-pass" "carol" "tok" "carol/x";
          };

          unknownNameMessages = violations fleet {
            mappings.a = mapping "safix-to-pass" "alice" "absent" "alice/x";
          };

          pullOntoGeneratedMessages = violations fleet {
            mappings.a = mapping "pass-to-safix" "alice" "minted" "alice/minted";
          };

          twoWayOntoGeneratedMessages = violations fleet {
            mappings.a = mapping "two-way" "alice" "minted" "alice/minted";
          };

          # A push onto a generated entry is the ordinary case: the generator is
          # the value's only producer and the store receives a copy of what it
          # produced. `backup` is the same shape and is asserted by `sound`.
          pushOntoGeneratedMessages = violations fleet {
            mappings.a = mapping "safix-to-pass" "alice" "minted" "alice/minted";
          };

          # One entry reached by two mappings that differ on the safix side, so
          # the duplicate is the store half and nothing else.
          oneEntryMessages = violations fleet {
            mappings = {
              a = mapping "safix-to-pass" "alice" "tok" "alice/grafana";
              b = mapping "safix-to-pass" "bob" "tok" "alice/grafana";
            };
          };

          reservedNameMessages = violations fleet {
            mappings.a = mapping "safix-to-pass" "alice" "tok" "alice/grafana.safix-sync-state";
          };

          reservedIdMessages = {
            clan = reservedIdMessagesFor "clan";
            keepassxc = reservedIdMessagesFor "keepassxc";
            pass = reservedIdMessagesFor "pass";
            bitwarden = reservedIdMessagesFor "bitwarden";
            onepassword = reservedIdMessagesFor "1password";
            all = reservedIdMessagesFor "all";
          };

          # The six words asserted above are the whole of the shared list rather
          # than a sample of it, so a word added there without a field here is a
          # failure rather than an untested refusal.
          reservedWords = reserved.ids;

          # Two faults in one mapping, both reported. `a`'s safix side does not
          # resolve and its entry path is reserved; the second fault is judged
          # on the store side alone, so the first does not suppress it.
          bothFaultsMessages = violations fleet {
            mappings.a = mapping "two-way" "carol" "tok" "carol/x.safix-sync-state";
          };

          # All four fields, declared as literals and one as an `{ entry = …; }`
          # source. Silent, because this is the one target where every channel
          # is a pipe: neither field refusal the other targets make is reachable
          # here, and that is held by this field rather than stated in prose.
          soundFieldsMessages = violations fleet {
            mappings.a = mappingWith {
              username = "alice@example.com";
              url = "https://grafana.example.com";
              notes = {
                entry = "tok";
              };
              tags = [ "work" ];
            } "safix-to-pass" "alice" "tok" "alice/grafana";
          };

          # The table this module reads is the shared one rather than a copy of
          # it, which is what stops the two from being forked.
          capabilities = pass.capabilities;
          capabilitiesAreTheSharedTable =
            pass.capabilities == (import ../safix/fields.nix { inherit lib; }).channels.pass;
          everyFieldCrossesOnAPipe =
            lib.all (name: pass.capabilities.${name} == "stdin")
              (import ../safix/fields.nix { inherit lib; }).fieldNames;

          # A declaration with no mapping is what a consumer who does not use
          # this evaluates, and it must be silent — including with the default
          # store, because that is the same consumer.
          emptyMirrorMessages = violations fleet { };

          brokenCustody = violations brokenFleet {
            mappings.a = mapping "safix-to-pass" "carol" "tok" "carol/x";
          };

          # Without this the field above is vacuous: an empty message list
          # proves the short-circuit only if the fleet it was computed over is
          # one custody actually refuses.
          brokenCustodyIsBroken = resolve.violations { users = brokenFleet; } != [ ];

          badMode = badMode;
          storeDeclaredAsAPathTypechecks = storeDeclaredAsAPathTypechecks;

          # The default the option carries, asserted here as a string so that
          # changing it to a nix path is caught by the field above rather than
          # by a consumer.
          defaultStore = (recordOf { }).store;
        };
        expected = {
          modes = [
            "safix-to-pass"
            "pass-to-safix"
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
          entryPath = "alice/mail";

          soundMessages = [ ];

          unknownUserMessages = [
            "flake.safix.pass.mappings.a names the user 'carol', which flake.safix.users does not declare"
          ];

          unknownNameMessages = [
            "flake.safix.pass.mappings.a names the secret 'absent', which flake.safix.users.alice does not hold"
          ];

          pullOntoGeneratedMessages = [
            "flake.safix.pass.mappings.a is pass-to-safix into flake.safix.users.alice.minted, which a generator also produces — two producers for one value, and the winner is whichever ran last"
          ];

          twoWayOntoGeneratedMessages = [
            "flake.safix.pass.mappings.a is two-way into flake.safix.users.alice.minted, which a generator also produces — two producers for one value, and the winner is whichever ran last"
          ];

          pushOntoGeneratedMessages = [ ];

          oneEntryMessages = [
            "flake.safix.pass.mappings a and b both name the entry alice/grafana"
          ];

          reservedNameMessages = [
            "flake.safix.pass.mappings.a names the entry alice/grafana.safix-sync-state, and '.safix-sync-state' is the suffix safix reserves for the entry a two-way mapping records its last agreement in"
          ];

          reservedIdMessages = {
            clan = [
              "flake.safix.pass.mappings.clan is named 'clan', which sync and audit read as a target keyword rather than a mapping name"
            ];
            keepassxc = [
              "flake.safix.pass.mappings.keepassxc is named 'keepassxc', which sync and audit read as a target keyword rather than a mapping name"
            ];
            pass = [
              "flake.safix.pass.mappings.pass is named 'pass', which sync and audit read as a target keyword rather than a mapping name"
            ];
            bitwarden = [
              "flake.safix.pass.mappings.bitwarden is named 'bitwarden', which sync and audit read as a target keyword rather than a mapping name"
            ];
            onepassword = [
              "flake.safix.pass.mappings.1password is named '1password', which sync and audit read as a target keyword rather than a mapping name"
            ];
            all = [
              "flake.safix.pass.mappings.all is named 'all', which sync and audit read as a target keyword rather than a mapping name"
            ];
          };

          reservedWords = [
            "clan"
            "keepassxc"
            "pass"
            "bitwarden"
            "1password"
            "all"
          ];

          bothFaultsMessages = [
            "flake.safix.pass.mappings.a names the user 'carol', which flake.safix.users does not declare"
            "flake.safix.pass.mappings.a names the entry carol/x.safix-sync-state, and '.safix-sync-state' is the suffix safix reserves for the entry a two-way mapping records its last agreement in"
          ];

          soundFieldsMessages = [ ];

          capabilities = {
            username = "stdin";
            url = "stdin";
            notes = "stdin";
            tags = "stdin";
          };
          capabilitiesAreTheSharedTable = true;
          everyFieldCrossesOnAPipe = true;

          emptyMirrorMessages = [ ];
          brokenCustody = [ ];
          brokenCustodyIsBroken = true;
          badMode = false;
          storeDeclaredAsAPathTypechecks = false;
          defaultStore = "~/.password-store";
        };
      };

      checks.safix-pass-drill = drill;
    };
}
