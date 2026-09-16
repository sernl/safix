# Holds the mirror refusals of ../safix/onepassword.nix against fleets built to
# break each one, and holds the boundary of what evaluation is allowed to claim.
#
# Every fixture is synthetic and no vault exists. That is deliberate rather than
# incidental: the far half of a mapping is content of a remote service, and
# nothing here may reach it. A fixture whose vault were real would make this
# file's passing depend on a session being present — precisely the property the
# surface is designed not to have — and would point a build at somebody's own
# 1Password. `fixture-vault` and `other-vault` are the only vault names in this
# repository, and `fixture.example.com` the only account shorthand.
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
# Whether the vault exists, whether the item does, whether either side holds a
# value, and whether the session authenticates. All four are content of a remote
# service or a property of an operator's own environment, and a build that
# asserted any of them would be asserting something about whichever session
# happened to be in reach. A declared vault this session cannot see is refused
# when a run reaches the mapping, and `onepassword_path.rs`'s
# `the_refusals_each_have_their_own_code_and_leave_both_sides_alone` is where
# that refusal lives.
#
# There is no field refusal here at all, and the absence is a decision rather
# than an omission: every row of `../safix/fields.nix`'s `channels.onepassword`
# is `stdin`, so neither `fieldUnsupported` nor `fieldSourceInArgv` has a case
# to fire on for this target. `soundFieldsMessages` below is what states that
# positively — all four fields, one of them sourced from another entry, and no
# message.
#
# ── severity: proven by perturbation, one drill per claim ──
# Dropping `unresolvableSafixSide` from the list `violationsOf` returns empties
# `unknownUserMessages` and `unknownNameMessages` and moves no other field.
# Dropping `twoProducers` empties `pullOntoGeneratedMessages` and
# `twoWayOntoGeneratedMessages`, and leaves `pushOntoGeneratedMessages` empty as
# it already is: a push onto a generated entry is not a second producer, because
# safix's side is the one being read.
# Dropping `twoMappingsOneItem` empties `oneItemMessages` and moves no other
# field.
# Changing `itemPathOf` from `"${vault}/${item}"` to the item alone reddens
# `twoVaultsOneItemNameMessages`, which is today empty: two mappings naming one
# item name in two vaults name two items and are accepted. That field is the
# evidence the vault is part of the identity rather than decoration.
# Judging `reservedId` over the sound mappings rather than over every declared
# one empties `bothFaultsMessages`' second half, which is the claim that a
# mapping with two faults hears about both.
# Dropping `reservedId` from the list empties every field of
# `reservedIdMessages`, one per reserved word a mapping id may collide with.
# Dropping the `resolve.violations` short-circuit fills `brokenCustody` with
# mirror sentences about a fleet whose custody has not resolved, which is one
# fault producing two unrelated messages.
# Removing the mode enum lets `badMode` evaluate rather than throw.
# Loosening `vault`'s type in `recordOf` below, from `str` to `either str path`,
# lets `vaultDeclaredAsAPathTypechecks` evaluate true rather than throw — the
# same local-type limitation ./keepassxc.nix records for its own fixture types,
# which are hand-written to match ../safix/options.nix because that file is not
# standalone-evaluable outside the full flake-parts module.
# Putting a function in the flattened record, or an absolute path derived from
# the flake's own root, turns `recordIsSerializable` or `recordNamesNoStorePath`
# false — which is what keeps ./examples.nix's field-for-field comparison
# between the two `safix-examples` consumers meaningful, since a function is
# invisible to it and a root-dependent string resolves to two different values.
#
# ── no check anywhere drives a real `op`, and none ever will ──
# Three grounds, each sufficient alone: `_1password-cli` at this flake's pin is
# unfree, so naming it from a check, a package or a development shell makes
# evaluation fail for every consumer who has not allowed unfree packages; there
# is no self-hostable 1Password server to point a sandboxed node at; and every
# authentication path needs the network, which no `nix build` and no hermetic VM
# node has. None of the three is a condition that may later be satisfied, so the
# absence is permanent rather than deferred — and it is written here, in
# ../safix/onepassword.nix's header and in ../../crates/safix-core/src/onepassword.rs's
# rather than left to be inferred from a missing file, because an unstated
# absence is a claim nobody decided to stop making.
{
  perSystem =
    {
      pkgs,
      lib,
      ...
    }:
    let
      onepassword = import ../safix/onepassword.nix { inherit lib; };
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
            account = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
            };
            mappings = lib.mkOption {
              type = lib.types.attrsOf onepassword.mapping;
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

      # `fields` is curried ahead of the positional endpoints for the reason
      # ./keepassxc.nix curries its own: a nix function cannot take an optional
      # positional argument, so `mapping` is `mappingWith { }` and every fixture
      # that declares no field reads as short as it would have.
      mappingWith = fields: mode: user: name: vault: item: {
        inherit mode;
        safix = { inherit user name; };
        onepassword = { inherit vault item fields; };
      };

      mapping = mappingWith { };

      violations = fleet': record: onepassword.violationsOf { users = fleet'; } (recordOf record);

      wellFormed = {
        account = "fixture.example.com";
        mappings = {
          push = mapping "safix-to-1password" "alice" "tok" "fixture-vault" "grafana";
          pull = mapping "1password-to-safix" "bob" "tok" "fixture-vault" "router";
          both = mapping "two-way" "alice" "tok" "fixture-vault" "mail";
          copy = mapping "backup" "alice" "minted" "fixture-vault" "minted";
        };
      };

      # The flattened record ../safix/default.nix projects, built here the same
      # way: the account, and one record per mapping carrying its own id.
      flattened = {
        account = (recordOf wellFormed).account;
        mappings = onepassword.mappingsOf (recordOf wellFormed);
      };

      # `tryEval` catches the enum's refusal, which is a throw rather than a
      # message: a mode outside the four is refused by the type before any rule
      # in `violationsOf` could look at it. `op-to-safix` rather than a nonsense
      # word, because the plausible wrong spelling is the program's name.
      badMode =
        (builtins.tryEval (
          builtins.deepSeq (recordOf {
            mappings.a = mapping "op-to-safix" "alice" "tok" "fixture-vault" "grafana";
          }) "resolved"
        )).success;

      # `tryEval` catches the string type's refusal, the same way `badMode`
      # catches the mode enum's: a path is a distinct nix value from a string
      # and the module system refuses it before any rule here could look at it.
      vaultDeclaredAsAPathTypechecks =
        (builtins.tryEval (
          builtins.deepSeq (recordOf {
            mappings.a = mapping "safix-to-1password" "alice" "tok" ./mk-structural-check.nix "grafana";
          }) "resolved"
        )).success;

      drill =
        pkgs.runCommand "safix-onepassword-drill"
          { meta.description = "severity drill: safix-onepassword-refusals"; }
          ''
            messages=$(mktemp)
            printf '%s\n' ${
              lib.escapeShellArg (
                builtins.head (
                  violations fleet {
                    mappings.a = mapping "two-way" "carol" "tok" "fixture-vault" "x";
                  }
                )
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
      checks.safix-onepassword = mkStructuralCheck {
        name = "safix-onepassword";
        actual = {
          modes = onepassword.modes;

          # Which modes make safix's side a destination, read off the same
          # predicate the refusal reads. A mode added to the list above without
          # a decision about this fails here rather than at somebody's terminal.
          pullCapable = map onepassword.pullCapable onepassword.modes;

          # The vault-qualified item, which is the address every report, refusal
          # and read names.
          itemPath = onepassword.itemPathOf (mapping "backup" "alice" "tok" "fixture-vault" "alice/mail");

          wellFormed = violations fleet wellFormed;

          unknownUserMessages = violations fleet {
            mappings.a = mapping "safix-to-1password" "carol" "tok" "fixture-vault" "x";
          };

          unknownNameMessages = violations fleet {
            mappings.a = mapping "safix-to-1password" "alice" "absent" "fixture-vault" "x";
          };

          pullOntoGeneratedMessages = violations fleet {
            mappings.a = mapping "1password-to-safix" "alice" "minted" "fixture-vault" "minted";
          };

          twoWayOntoGeneratedMessages = violations fleet {
            mappings.a = mapping "two-way" "alice" "minted" "fixture-vault" "minted";
          };

          # A push onto a generated entry is the ordinary case: the generator is
          # the value's only producer and the item receives a copy of what it
          # produced. `backup` is the same shape and is asserted by `wellFormed`.
          pushOntoGeneratedMessages = violations fleet {
            mappings.a = mapping "safix-to-1password" "alice" "minted" "fixture-vault" "minted";
          };

          # One item reached by two mappings that differ on the safix side, so
          # the duplicate is the far half and nothing else.
          oneItemMessages = violations fleet {
            mappings = {
              a = mapping "safix-to-1password" "alice" "tok" "fixture-vault" "grafana";
              b = mapping "safix-to-1password" "bob" "tok" "fixture-vault" "grafana";
            };
          };

          # The same item name in two vaults is two items, and accepted. The
          # vault is part of the address rather than decoration.
          twoVaultsOneItemNameMessages = violations fleet {
            mappings = {
              a = mapping "safix-to-1password" "alice" "tok" "fixture-vault" "grafana";
              b = mapping "safix-to-1password" "bob" "tok" "other-vault" "grafana";
            };
          };

          reservedIdMessages = {
            clan = violations fleet {
              mappings.clan = mapping "safix-to-1password" "alice" "tok" "fixture-vault" "grafana";
            };
            keepassxc = violations fleet {
              mappings.keepassxc = mapping "safix-to-1password" "alice" "tok" "fixture-vault" "grafana";
            };
            pass = violations fleet {
              mappings.pass = mapping "safix-to-1password" "alice" "tok" "fixture-vault" "grafana";
            };
            bitwarden = violations fleet {
              mappings.bitwarden = mapping "safix-to-1password" "alice" "tok" "fixture-vault" "grafana";
            };
            "1password" = violations fleet {
              mappings."1password" = mapping "safix-to-1password" "alice" "tok" "fixture-vault" "grafana";
            };
            all = violations fleet {
              mappings.all = mapping "safix-to-1password" "alice" "tok" "fixture-vault" "grafana";
            };
          };

          # Two faults in one mapping, both reported: the id is a reserved word
          # and the safix side does not resolve. `reservedId` is judged over
          # every declared mapping rather than the sound ones, so the first
          # fault does not suppress the second.
          bothFaultsMessages = violations fleet {
            mappings.pass = mapping "two-way" "carol" "tok" "fixture-vault" "x";
          };

          # All four fields, one of them sourced from another entry, and no
          # message: this target carries every field on standard input, so it
          # has no channel refusal to make. Without this row the absence of
          # `fieldUnsupported` and `fieldSourceInArgv` here would be
          # indistinguishable from an omission.
          soundFieldsMessages = violations fleet {
            mappings.a = mappingWith {
              username = "alice@example.com";
              url = "https://grafana.example.invalid";
              notes = {
                entry = "grafana-note";
              };
              tags = [ "work" ];
            } "safix-to-1password" "alice" "tok" "fixture-vault" "grafana";
          };

          # The table this module refuses on is the shared one rather than a
          # copy of it, which is what stops the two from being forked.
          capabilities = onepassword.capabilities;
          capabilitiesAreTheSharedTable =
            onepassword.capabilities == (import ../safix/fields.nix { inherit lib; }).channels.onepassword;

          # A declaration with no mapping is what a consumer who does not use
          # this evaluates, and it must be silent — including with no account
          # named, because that is the same consumer and an undeclared account
          # is a working configuration.
          emptyMirrorMessages = violations fleet { };

          noAccountMessages = violations fleet {
            mappings.a = mapping "safix-to-1password" "alice" "tok" "fixture-vault" "grafana";
          };

          brokenCustody = violations brokenFleet {
            mappings.a = mapping "safix-to-1password" "carol" "tok" "fixture-vault" "x";
          };

          # Without this the field above is vacuous: an empty message list
          # proves the short-circuit only if the fleet it was computed over is
          # one custody actually refuses.
          brokenCustodyIsBroken = resolve.violations { users = brokenFleet; } != [ ];

          # The projection ./examples.nix compares field for field between the
          # two `safix-examples` consumers. `toJSON` refuses a function, so a
          # record that serializes has none; and it carries no absolute path
          # derived from the flake's own root, which would resolve to two
          # different values for those two consumers.
          recordIsSerializable =
            (builtins.tryEval (builtins.deepSeq (builtins.toJSON flattened) "resolved")).success;
          recordNamesNoStorePath = !(lib.hasInfix "/nix/store" (builtins.toJSON flattened));
          recordAccount = flattened.account;
          recordIds = map (m: m.id) flattened.mappings;

          badMode = badMode;
          vaultDeclaredAsAPathTypechecks = vaultDeclaredAsAPathTypechecks;
        };
        expected = {
          modes = [
            "safix-to-1password"
            "1password-to-safix"
            "two-way"
            "backup"
          ];

          pullCapable = [
            false
            true
            true
            false
          ];

          itemPath = "fixture-vault/alice/mail";

          wellFormed = [ ];

          unknownUserMessages = [
            "flake.safix.onepassword.mappings.a names the user 'carol', which flake.safix.users does not declare"
          ];

          unknownNameMessages = [
            "flake.safix.onepassword.mappings.a names the secret 'absent', which flake.safix.users.alice does not hold"
          ];

          pullOntoGeneratedMessages = [
            "flake.safix.onepassword.mappings.a is 1password-to-safix into flake.safix.users.alice.minted, which a generator also produces — two producers for one value, and the winner is whichever ran last"
          ];

          twoWayOntoGeneratedMessages = [
            "flake.safix.onepassword.mappings.a is two-way into flake.safix.users.alice.minted, which a generator also produces — two producers for one value, and the winner is whichever ran last"
          ];

          pushOntoGeneratedMessages = [ ];

          oneItemMessages = [
            "flake.safix.onepassword.mappings a and b both name the item fixture-vault/grafana"
          ];

          twoVaultsOneItemNameMessages = [ ];

          reservedIdMessages = {
            clan = [
              "flake.safix.onepassword.mappings.clan is named 'clan', which sync and audit read as a target keyword rather than a mapping name"
            ];
            keepassxc = [
              "flake.safix.onepassword.mappings.keepassxc is named 'keepassxc', which sync and audit read as a target keyword rather than a mapping name"
            ];
            pass = [
              "flake.safix.onepassword.mappings.pass is named 'pass', which sync and audit read as a target keyword rather than a mapping name"
            ];
            bitwarden = [
              "flake.safix.onepassword.mappings.bitwarden is named 'bitwarden', which sync and audit read as a target keyword rather than a mapping name"
            ];
            "1password" = [
              "flake.safix.onepassword.mappings.1password is named '1password', which sync and audit read as a target keyword rather than a mapping name"
            ];
            all = [
              "flake.safix.onepassword.mappings.all is named 'all', which sync and audit read as a target keyword rather than a mapping name"
            ];
          };

          bothFaultsMessages = [
            "flake.safix.onepassword.mappings.pass names the user 'carol', which flake.safix.users does not declare"
            "flake.safix.onepassword.mappings.pass is named 'pass', which sync and audit read as a target keyword rather than a mapping name"
          ];

          soundFieldsMessages = [ ];

          capabilities = {
            username = "stdin";
            url = "stdin";
            notes = "stdin";
            tags = "stdin";
          };
          capabilitiesAreTheSharedTable = true;

          emptyMirrorMessages = [ ];
          noAccountMessages = [ ];
          brokenCustody = [ ];
          brokenCustodyIsBroken = true;

          recordIsSerializable = true;
          recordNamesNoStorePath = true;
          recordAccount = "fixture.example.com";
          recordIds = [
            "both"
            "copy"
            "pull"
            "push"
          ];

          badMode = false;
          vaultDeclaredAsAPathTypechecks = false;
        };
      };

      checks.safix-onepassword-drill = drill;
    };
}
