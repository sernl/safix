# Holds the mirror refusals of ../safix/bitwarden.nix against fleets built to
# break each one, and holds the boundary of what evaluation is allowed to claim.
#
# Every fixture is synthetic and no vault exists. That is deliberate rather than
# incidental: the vault half of a mapping is content of a remote service, and
# nothing here may reach it. A fixture whose vault were real would make this
# file's passing depend on a session being present, which is precisely the
# property the surface is designed not to have — and would point a build at
# somebody's own vault.
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
# That the item exists, that the folder does, that either side holds a value, or
# that the client is unlocked — or even logged in. All four need a reachable
# vault and a session, and a build that asserted any of them would be asserting
# something about whichever vault happened to be in reach.
#
# Nor whether a declared `server` is the one the client reports reaching. That
# comparison needs the client's own answer, so it is `Error::BitwardenServerMismatch`
# at run time and `crates/safix/tests/bitwarden.rs`'s
# `a_declared_server_that_is_not_reached_refuses_before_any_read` is where it
# lives. Evaluation refuses nothing about the server, because there is no server
# safix could name that would be right.
#
# ── severity: proven by perturbation, one drill per claim ──
# Dropping `unresolvableSafixSide` from the list `violationsOf` returns empties
# `unknownUserMessages` and `unknownNameMessages` and moves no other field.
# Dropping `twoProducers` empties `pullOntoGeneratedMessages` and
# `twoWayOntoGeneratedMessages`, and leaves `pushOntoGeneratedMessages` empty as
# it already is: a push onto a generated entry is not a second producer, because
# safix's side is the one being read.
# Dropping `twoMappingsOneItem` empties `oneItemMessages`.
# Dropping `reservedId` from the list empties `reservedIdMessages`' three fields,
# one per reserved word a mapping id may collide with; replacing `reserved.ids`
# with an inline literal `[ "clan" "keepassxc" "all" ]` empties its `bitwarden`
# field alone while the other two stay populated, which is the evidence the list
# is read from one place rather than restated per module (task 1.15).
# Dropping `unsupportedField` empties `tagsMessages` and the field half of
# `bothFaultsMessages` and moves no other field (task 1.14), which is the
# evidence a declared `tags` is refused rather than dropped; and changing
# ../safix/fields.nix's `channels.bitwarden.tags` from `"unsupported"` to
# `"stdin"` empties `tagsMessages` alone — which is the evidence the refusal is
# driven by the shared table rather than by a hard-coded field name.
# Judging `unsupportedField` or `reservedId` over the sound mappings rather than
# over every declared one empties the second half of `bothFaultsMessages`, which
# is the claim that a mapping with two faults hears about both.
# Dropping the `resolve.violations` short-circuit fills `brokenCustody` with
# mirror sentences about a fleet whose custody has not resolved, which is one
# fault producing two unrelated messages.
# Removing the mode enum lets `badMode` evaluate rather than throw.
# Forking `capabilities` in ../safix/bitwarden.nix from ../safix/fields.nix's own
# row turns `capabilitiesAreTheSharedTable` false.
# Declaring `folder` as a `str` rather than a `nullOr str` turns
# `rootFolderTypechecks` false, which is how "null means the vault's root" is
# evaluated against the option tree rather than asserted in prose.
#
# ── the recorded absence ──
# No check in this repository runs a real `bw`, and this paragraph is where that
# is written down rather than left to be noticed. `bw` cannot authenticate
# without a network — `login` registers a device against an account, and every
# path to a session goes through it — and a `nix build` has no network. So the
# stand-in `crates/safix/tests/support/bw-stub.rs` is the only client any check
# of this repository drives, and what it cannot establish is stated in its own
# header: that the argument vector means to the real client what safix thinks it
# means.
#
# The doctrine for writing this down rather than leaving it silent is
# ./integration.nix's own comment on `runOneWith`'s `extra`: leaving a needed
# input out entirely "would make that check state an absence and pass — which is
# how a claim stops being made without anybody deciding to stop making it". An
# absence that is recorded is a decision; an absence that is silent is a claim
# nobody is making any more.
#
# The deferred alternative is a NixOS VM check against `services.vaultwarden`,
# beside ./installer-vm.nix, with a `bitwarden-cli` client node. The machinery
# exists and nixpkgs ships `nixos/tests/vaultwarden.nix` in that shape, which is
# why this is recorded as deferred rather than as impossible. It is not minted
# here because that upstream test's own `bw login`/`sync`/`list` assertions are
# commented out, so adopting its shape would produce a check whose green means
# nothing was measured — the exact failure ./integration.nix names.
#
# What would unblock it, and what was measured toward it on 2026-09-16 against
# `bitwarden-cli` 2026.8.0, this flake's pin (task 9.4):
#
#   - Measured, out of the shipped bundle at
#     `lib/node_modules/@bitwarden/clients/build/bw.js`: the client does carry
#     `login --apikey`, and it reads the pair `BW_CLIENTID`/`BW_CLIENTSECRET`
#     from its environment. So the credential channel a VM node would need
#     exists in the pinned client, and an API-key login is not a capability that
#     would have to be added.
#   - Measured, the same way: the bundle carries no `rejectUnauthorized` and no
#     TLS-trust override of its own, so a snakeoil certificate would have to be
#     trusted through node's own environment rather than through a client flag.
#     That is one more thing a VM node has to get right, and it is not settled
#     here.
#   - **Not measured**: that `bw login --apikey` followed by `bw unlock`
#     completes against a local `services.vaultwarden` with snakeoil
#     certificates. That is the measurement 9.4 names, and it needs a booted VM
#     node rather than a bundle read — a device registration against a running
#     server. It was not performed in this change, and this paragraph says so
#     rather than implying the two reads above stand in for it.
#
# Per 9.4, no VM check is minted here whatever that measurement would say: the
# scope of this change is the target, and a VM check is its own change with its
# own runtime budget.
{
  perSystem =
    {
      pkgs,
      lib,
      ...
    }:
    let
      bitwarden = import ../safix/bitwarden.nix { inherit lib; };
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
      # ./keepassxc.nix's own `recordOf` already is not either.
      recordOf =
        record:
        typed (lib.types.submodule {
          options = {
            server = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
            };
            mappings = lib.mkOption {
              type = lib.types.attrsOf bitwarden.mapping;
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

      # `fields` and `folder` are curried ahead of the positional endpoints for
      # the reason ./keepassxc.nix's `mappingWith` curries its own: a nix
      # function cannot take an optional positional argument, so `mapping` is
      # the folderless, fieldless form and every fixture that needs neither
      # reads as it would have before either existed.
      mappingWith = folder: fields: mode: user: name: item: {
        inherit mode;
        safix = { inherit user name; };
        bitwarden = { inherit folder item fields; };
      };

      mapping = mappingWith null { };

      violations = fleet': record: bitwarden.violationsOf { users = fleet'; } (recordOf record);

      sound = {
        server = null;
        mappings = {
          push = mapping "safix-to-bitwarden" "alice" "tok" "pushed";
          pull = mapping "bitwarden-to-safix" "bob" "tok" "pulled";
          both = mappingWith "fleet" { } "two-way" "alice" "tok" "both";
          copy = mapping "backup" "alice" "minted" "copied";
        };
      };

      # `tryEval` catches the enum's refusal, which is a throw rather than a
      # message: a mode outside the four is refused by the type before any rule
      # in `violationsOf` could look at it.
      badMode =
        (builtins.tryEval (
          builtins.deepSeq (recordOf {
            mappings.a = mapping "push" "alice" "tok" "pushed";
          }) "resolved"
        )).success;

      # A folderless mapping typechecks, which is how "null means the vault's
      # root" is held against the option tree. Declaring `folder` as a `str`
      # turns this false.
      rootFolderTypechecks =
        (builtins.tryEval (
          builtins.deepSeq (recordOf {
            mappings.a = mapping "safix-to-bitwarden" "alice" "tok" "pushed";
          }) "resolved"
        )).success;

      drill =
        pkgs.runCommand "safix-bitwarden-drill"
          { meta.description = "severity drill: safix-bitwarden-refusals"; }
          ''
            messages=$(mktemp)
            printf '%s\n' ${
              lib.escapeShellArg (
                builtins.head (violations fleet { mappings.a = mapping "two-way" "carol" "tok" "x"; })
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
      checks.safix-bitwarden = mkStructuralCheck {
        name = "safix-bitwarden";
        actual = {
          modes = bitwarden.modes;

          # Which modes make safix's side a destination, read off the same
          # predicate the refusal reads. A mode added to the list above without a
          # decision about this fails here rather than at somebody's terminal.
          pullCapable = map bitwarden.pullCapable bitwarden.modes;

          # The address every report and every refusal names, in both its
          # branches. Asserted here as well as in `Bitwarden::address_of`'s own
          # unit test, because the two sides have to agree and neither is the
          # other's oracle.
          rootAddress = bitwarden.itemPathOf (mapping "backup" "alice" "tok" "router");
          folderAddress = bitwarden.itemPathOf (mappingWith "fleet" { } "backup" "alice" "tok" "router");

          carriedFields = bitwarden.carriedFields;

          soundMessages = violations fleet sound;

          unknownUserMessages = violations fleet {
            mappings.a = mapping "safix-to-bitwarden" "carol" "tok" "x";
          };

          unknownNameMessages = violations fleet {
            mappings.a = mapping "safix-to-bitwarden" "alice" "absent" "x";
          };

          pullOntoGeneratedMessages = violations fleet {
            mappings.a = mapping "bitwarden-to-safix" "alice" "minted" "minted";
          };

          twoWayOntoGeneratedMessages = violations fleet {
            mappings.a = mapping "two-way" "alice" "minted" "minted";
          };

          # A push onto a generated entry is the ordinary case: the generator is
          # the value's only producer and the vault receives a copy of what it
          # produced. `backup` is the same shape and is asserted by `sound`.
          pushOntoGeneratedMessages = violations fleet {
            mappings.a = mapping "safix-to-bitwarden" "alice" "minted" "minted";
          };

          # One item reached by two mappings that differ on the safix side, so
          # the duplicate is the vault half and nothing else.
          oneItemMessages = violations fleet {
            mappings = {
              a = mapping "safix-to-bitwarden" "alice" "tok" "pushed";
              b = mapping "safix-to-bitwarden" "bob" "tok" "pushed";
            };
          };

          # And the same two items under different folders are two addresses
          # rather than one. Without this the field above is satisfied by a rule
          # that grouped on the item name alone, which would refuse a fleet that
          # legitimately holds one name in two folders.
          oneItemAcrossFoldersMessages = violations fleet {
            mappings = {
              a = mappingWith "fleet" { } "safix-to-bitwarden" "alice" "tok" "pushed";
              b = mappingWith "people" { } "safix-to-bitwarden" "bob" "tok" "pushed";
            };
          };

          reservedIdMessages = {
            clan = violations fleet {
              mappings.clan = mapping "safix-to-bitwarden" "alice" "tok" "pushed";
            };
            bitwarden = violations fleet {
              mappings.bitwarden = mapping "safix-to-bitwarden" "alice" "tok" "pushed";
            };
            all = violations fleet {
              mappings.all = mapping "safix-to-bitwarden" "alice" "tok" "pushed";
            };
          };

          # Two faults in one mapping, both reported. `a`'s safix side does not
          # resolve and its id is reserved; `b`'s safix side does not resolve and
          # it declares a field this transport cannot carry. Both second faults
          # are judged over every declared mapping, so the first does not
          # suppress them.
          bothFaultsMessages = violations fleet {
            mappings = {
              all = mapping "two-way" "carol" "tok" "x";
              b = mappingWith null { tags = [ "work" ]; } "two-way" "carol" "tok" "y";
            };
          };

          # The one field this transport cannot carry at all, declared. Asserted
          # against the sentence rather than against a count, so a refusal that
          # fired naming the wrong field fails the literal.
          tagsMessages = violations fleet {
            mappings.a = mappingWith null { tags = [ "work" ]; } "safix-to-bitwarden" "alice" "tok" "pushed";
          };

          # The three fields this transport does carry, declared as literals and
          # sourced from another entry. Without this the field above is vacuous:
          # a rule that refused every declaration would satisfy it. `{ entry = …
          # }` is permitted here where the keepassxc target refuses it, because
          # this transport's fields travel standard input — B7 records why, and
          # this field is what holds it.
          soundFieldsMessages = violations fleet {
            mappings.a = mappingWith null {
              username = "alice@example.com";
              url = "https://grafana.example.invalid";
              notes = {
                entry = "grafana-note";
              };
            } "safix-to-bitwarden" "alice" "tok" "pushed";
          };

          # The table this module refuses on is the shared one rather than a
          # copy of it, which is what stops the two from being forked.
          capabilities = bitwarden.capabilities;
          capabilitiesAreTheSharedTable =
            bitwarden.capabilities == (import ../safix/fields.nix { inherit lib; }).channels.bitwarden;

          # A declaration with no mapping is what a consumer who does not use
          # this evaluates, and it must be silent — including with no server
          # named, because that is the same consumer.
          emptyMirrorMessages = violations fleet { };

          # A mapping with no server named produces no message either, and this
          # is the whole of B2 at evaluation: an undeclared server is a working
          # configuration rather than a missing declaration.
          noServerMessages = violations fleet {
            mappings.a = mapping "safix-to-bitwarden" "alice" "tok" "pushed";
          };

          brokenCustody = violations brokenFleet {
            mappings.a = mapping "safix-to-bitwarden" "carol" "tok" "x";
          };

          # Without this the field above is vacuous: an empty message list proves
          # the short-circuit only if the fleet it was computed over is one
          # custody actually refuses.
          brokenCustodyIsBroken = resolve.violations { users = brokenFleet; } != [ ];

          badMode = badMode;
          rootFolderTypechecks = rootFolderTypechecks;
        };
        expected = {
          modes = [
            "safix-to-bitwarden"
            "bitwarden-to-safix"
            "two-way"
            "backup"
          ];

          pullCapable = [
            false
            true
            true
            false
          ];

          rootAddress = "router";
          folderAddress = "fleet/router";

          carriedFields = [
            "username"
            "url"
            "notes"
          ];

          soundMessages = [ ];

          unknownUserMessages = [
            "flake.safix.bitwarden.mappings.a names the user 'carol', which flake.safix.users does not declare"
          ];

          unknownNameMessages = [
            "flake.safix.bitwarden.mappings.a names the secret 'absent', which flake.safix.users.alice does not hold"
          ];

          pullOntoGeneratedMessages = [
            "flake.safix.bitwarden.mappings.a is bitwarden-to-safix into flake.safix.users.alice.minted, which a generator also produces — two producers for one value, and the winner is whichever ran last"
          ];

          twoWayOntoGeneratedMessages = [
            "flake.safix.bitwarden.mappings.a is two-way into flake.safix.users.alice.minted, which a generator also produces — two producers for one value, and the winner is whichever ran last"
          ];

          pushOntoGeneratedMessages = [ ];

          oneItemMessages = [
            "flake.safix.bitwarden.mappings a and b both name the item pushed"
          ];

          oneItemAcrossFoldersMessages = [ ];

          reservedIdMessages = {
            clan = [
              "flake.safix.bitwarden.mappings.clan is named 'clan', which sync and audit read as a target keyword rather than a mapping name"
            ];
            bitwarden = [
              "flake.safix.bitwarden.mappings.bitwarden is named 'bitwarden', which sync and audit read as a target keyword rather than a mapping name"
            ];
            all = [
              "flake.safix.bitwarden.mappings.all is named 'all', which sync and audit read as a target keyword rather than a mapping name"
            ];
          };

          bothFaultsMessages = [
            "flake.safix.bitwarden.mappings.all names the user 'carol', which flake.safix.users does not declare"
            "flake.safix.bitwarden.mappings.b names the user 'carol', which flake.safix.users does not declare"
            "flake.safix.bitwarden.mappings.all is named 'all', which sync and audit read as a target keyword rather than a mapping name"
            "flake.safix.bitwarden.mappings.b declares the field 'tags', which the bitwarden target cannot carry: this vault has no tag concept, only folders and collections, so a declared tag would either be silently dropped or mean something the declaration did not say"
          ];

          tagsMessages = [
            "flake.safix.bitwarden.mappings.a declares the field 'tags', which the bitwarden target cannot carry: this vault has no tag concept, only folders and collections, so a declared tag would either be silently dropped or mean something the declaration did not say"
          ];

          soundFieldsMessages = [ ];

          capabilities = {
            username = "stdin";
            url = "stdin";
            notes = "stdin";
            tags = "unsupported";
          };
          capabilitiesAreTheSharedTable = true;

          emptyMirrorMessages = [ ];
          noServerMessages = [ ];
          brokenCustody = [ ];
          brokenCustodyIsBroken = true;
          badMode = false;
          rootFolderTypechecks = true;
        };
      };

      checks.safix-bitwarden-drill = drill;
    };
}
