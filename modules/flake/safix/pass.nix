# The declared relationship between a safix entry and an entry in the
# operator's `pass` store.
#
# The same shape ./keepassxc.nix has, for the same reason: a standing
# relationship is written down rather than passed as arguments, so adding a
# mapping shows up in review as a line naming both endpoints, a run has no
# arguments to get wrong, evaluation can refuse a mapping whose safix side does
# not exist, and a report can enumerate the whole mirror without being told
# what it is.
#
# What differs is the far side. A `pass` entry is one gpg-encrypted file in a
# tree, and evaluation cannot look at it: whether the store exists, whether it
# declares recipients, whether the entry is there and whether either side holds
# a value are run-time questions the verb answers with the store and the
# operator's own agent. So the refusals here are again exactly the ones local to
# the consumer's own declarations.
#
# The mode is per mapping rather than per run. A remembered flag on a verb is
# the drifting operational knowledge a declaration exists to end, and the four
# modes are the same vocabulary ./keepassxc.nix uses, with this target's word
# in place of its own.
#
# One thing is this target's alone: the record layout. `pass` fixes no metadata
# schema — its own convention is that the value is the first line and further
# information follows — so the layout of a safix-written record is safix's own
# decision, taken in `openspec/changes/add-pass-bridge/design.md` D2 and stated
# in `crates/safix-core/src/pass.rs`. A declaration here names which fields
# cross; what the bytes look like is that module's statement, not this one's.
{ lib }:
let
  resolve = import ./resolve.nix { inherit lib; };
  bridge = import ./bridge.nix { inherit lib; };
  fields = import ./fields.nix { inherit lib; };
  reserved = import ./reserved.nix;

  # Which channel this transport carries each field on, read straight off the
  # shared table rather than restated, so the rules below and the Rust half's
  # own `Capabilities` cannot be forked from it. Exported so
  # `modules/flake/checks/pass.nix` can assert that.
  #
  # Every row is `stdin` here, which is why neither of the two field refusals
  # ./keepassxc.nix makes is reachable for this target: the whole record body
  # crosses on standard input, so there is no field this store cannot carry and
  # no field whose value would have to travel an argument vector.
  capabilities = fields.channels.pass;

  # Written as their endpoints rather than as push and pull, which is the
  # decision ./bridge.nix records for `direction` and holds for the same reason:
  # a declaration is read by someone with no tool in hand to be relative to.
  modes = [
    "safix-to-pass"
    "pass-to-safix"
    "two-way"
    "backup"
  ];

  # The modes under which safix's side can be written, which is what makes a
  # generator on that side a second producer.
  pullCapable = mode: mode == "pass-to-safix" || mode == "two-way";

  # The suffix safix reserves for the companion entry a `two-way` mapping
  # records its last agreement in, and the whole of how that name is kept out of
  # a consumer's reach.
  #
  # A reserved suffix rather than a reserved subtree, because it makes the
  # reservation structural: the companion of a declared path is that path plus
  # this suffix, and a declared path carrying the suffix is refused below — so
  # no admissible declaration can name any companion, and the two name spaces
  # cannot be made to overlap by adding a mapping.
  stateSuffix = ".safix-sync-state";

  companionOf = m: "${m.pass.path}${stateSuffix}";

  passSide = lib.types.submodule {
    options = {
      path = lib.mkOption {
        type = lib.types.str;
        example = "alice/grafana";
        description = ''
          The entry path inside the declared store, as `pass` itself spells one:
          no leading slash and no `.gpg` suffix.

          Naming is the consumer's: safix derives nothing from the safix side's
          user or name, because what a person reading their own store wants to
          see there is a decision about their store rather than about safix.
        '';
      };

      fields = lib.mkOption {
        type = fields.fields;
        default = { };
        description = ''
          What the record carries beside its value: a username, a url, notes
          and tags, as ./fields.nix declares them for every target.

          This target carries all four, because the whole body crosses on
          standard input — the value's own bytes, then the declared fields as a
          trailing block of lines in the same record. So neither field refusal
          the other targets make is reachable here, and an
          `{ entry = "<name>"; }` source is admissible: the resolved value is a
          secret, and nothing this transport writes puts it in an argument
          vector.
        '';
      };
    };
  };

  mapping = lib.types.submodule {
    options = {
      mode = lib.mkOption {
        type = lib.types.enum modes;
        example = "safix-to-pass";
        description = ''
          Which way this mapping converges.

          `safix-to-pass` makes the store follow safix: a store-side edit to a
          mapped entry is overwritten, and reported.

          `pass-to-safix` makes safix follow the store, through the same write
          path a hand-set value takes, with every refusal on that path in
          force.

          `two-way` converges toward whichever side changed since the last
          agreement. Both sides changed is a conflict that writes nothing and
          names the two one-way commands that each resolve it.

          `backup` writes safix's value where the store holds no entry, and
          never overwrites one that differs — it reports the divergence
          instead, and writes no field either.

          No mode deletes an entry on either side, and `pass rm` is never run. A
          mapping that is removed stops being synced and its last store value
          stays until a person removes it: an accidental deletion of a secret is
          not a state a sync should be able to reach.
        '';
      };

      safix = lib.mkOption {
        type = bridge.safixSide;
        description = "The safix half: a user, and a name that user holds.";
      };

      pass = lib.mkOption {
        type = passSide;
        description = ''
          The store half: a path inside the declared store, and the fields the
          record carries beside its value.

          Evaluation verifies neither. The entry is one gpg-encrypted file, and
          the only thing that can answer whether it is there is the store
          itself, with the operator's own key.
        '';
      };
    };
  };

  # ── the refusals evaluation can reach ──
  #
  # Returned as a list rather than thrown, which is what lets a fixture assert
  # them against literals without building a derivation and lets a severity
  # drill run the same `refuseScript` bytes the real check runs.
  mappingsOf = pass: lib.mapAttrsToList (id: m: m // { inherit id; }) pass.mappings;

  # The entry path as the store's own command line takes it, and as every report
  # of the mapping names it.
  #
  # No group prefix, where `keepassxc.entryPathOf` takes one: a `pass` path is
  # already absolute within the store, and there is no group option here for it
  # to be relative to. The store root is the only location this target declares,
  # and it reaches the tool through the environment rather than through a name.
  entryPathOf = m: m.pass.path;

  violationsOf =
    registry: pass:
    let
      inherit (registry) users;
      declared = mappingsOf pass;

      # Custody has to resolve before a mapping's safix side can be looked up in
      # it, and a broken custody declaration already has its own refusal.
      # Staying silent here keeps one fault from producing two unrelated
      # sentences — ./bridge.nix and ./keepassxc.nix short-circuit on the same
      # list.
      placements = resolve.placementsOf registry;

      resolves = m: (placements.${m.safix.user} or { }) ? ${m.safix.name};

      unresolvableSafixSide = lib.concatMap (
        m:
        if !(users ? ${m.safix.user}) then
          [
            "flake.safix.pass.mappings.${m.id} names the user '${m.safix.user}', which flake.safix.users does not declare"
          ]
        else if !(resolves m) then
          [
            "flake.safix.pass.mappings.${m.id} names the secret '${m.safix.name}', which flake.safix.users.${m.safix.user} does not hold"
          ]
        else
          [ ]
      ) declared;

      # Only mappings whose safix side resolved are indexed below, because every
      # rule after this one reads that side's record.
      sound = lib.filter resolves (lib.filter (m: users ? ${m.safix.user}) declared);

      # Two producers for one value, refused by the rule already given for two
      # generators naming one output and for an import onto a generated entry:
      # the winner is whichever ran last, and which that is depends on the order
      # of a run rather than on anything written down.
      twoProducers = lib.concatMap (
        m:
        lib.optional (pullCapable m.mode && placements.${m.safix.user}.${m.safix.name}.generator != null)
          "flake.safix.pass.mappings.${m.id} is ${m.mode} into flake.safix.users.${m.safix.user}.${m.safix.name}, which a generator also produces — two producers for one value, and the winner is whichever ran last"
      ) sound;

      byEntry = lib.groupBy entryPathOf sound;

      twoMappingsOneEntry = lib.concatLists (
        lib.mapAttrsToList (
          entry: group:
          lib.optional (builtins.length group > 1)
            "flake.safix.pass.mappings ${
              lib.concatMapStringsSep " and " (m: m.id) group
            } both name the entry ${entry}"
        ) byEntry
      );

      # Judged over every declared mapping rather than over the sound ones, and
      # on the store side alone: a mapping whose safix side is also wrong has two
      # faults and is entitled to hear about both.
      reservedName = lib.concatMap (
        m:
        lib.optional (lib.hasSuffix stateSuffix m.pass.path) "flake.safix.pass.mappings.${m.id} names the entry ${entryPathOf m}, and '${stateSuffix}' is the suffix safix reserves for the entry a two-way mapping records its last agreement in"
      ) declared;

      # A mapping id spelled the same as a target keyword makes sync's and
      # audit's first argument ambiguous, the same way ./bridge.nix's and
      # ./keepassxc.nix's own `reservedId` refuses it over their mappings.
      # Judged over every declared mapping rather than the sound ones, so a
      # mapping with two faults hears about both.
      reservedId = lib.concatMap (
        m:
        lib.optional (builtins.elem m.id reserved.ids) "flake.safix.pass.mappings.${m.id} is named '${m.id}', which sync and audit read as a target keyword rather than a mapping name"
      ) declared;
    in
    if resolve.violations registry != [ ] then
      [ ]
    else
      unresolvableSafixSide ++ twoProducers ++ twoMappingsOneEntry ++ reservedName ++ reservedId;
in
{
  inherit
    modes
    pullCapable
    capabilities
    stateSuffix
    companionOf
    mapping
    mappingsOf
    entryPathOf
    violationsOf
    ;
}
