# The declared relationship between a safix entry and an entry in the operator's
# password database.
#
# The same shape ./bridge.nix has, for the same reason: a standing relationship
# is written down rather than passed as arguments, so adding a mapping shows up
# in review as a line naming both endpoints, a run has no arguments to get
# wrong, evaluation can refuse a mapping whose safix side does not exist, and a
# report can enumerate the whole mirror without being told what it is.
#
# What differs is the far side. clan's half lives in another flake and cannot be
# looked at; the database's half lives on a filesystem, encrypted, and cannot be
# looked at either — so the refusals here are again exactly the ones local to
# the consumer's own declarations. Whether the group exists, whether the entry
# does, and whether either side holds a value are run-time questions the verb
# answers with the database open.
#
# The mode is per mapping rather than per run. A remembered flag on a verb is
# the drifting operational knowledge a declaration exists to end, and the four
# modes are the vocabulary this fleet's own Filen declaration already uses for
# sync pairs, minus deletion propagation in every direction.
{ lib }:
let
  resolve = import ./resolve.nix { inherit lib; };
  bridge = import ./bridge.nix { inherit lib; };

  # Written as their endpoints rather than as push and pull, which is the
  # decision ./bridge.nix records for `direction` and holds for the same reason:
  # a declaration is read by someone with no tool in hand to be relative to.
  #
  # `two-way` and `backup` are not endpoint pairs and do not need to be. They
  # name a relationship rather than a direction, and both words are this fleet's
  # own from the Filen declaration the vocabulary was taken from.
  modes = [
    "safix-to-keepassxc"
    "keepassxc-to-safix"
    "two-way"
    "backup"
  ];

  # The modes under which safix's side can be written, which is what makes a
  # generator on that side a second producer.
  pullCapable = mode: mode == "keepassxc-to-safix" || mode == "two-way";

  # The suffix safix reserves for the companion entry a `two-way` mapping records
  # its last agreement in, and the whole of how that name is kept out of a
  # consumer's reach.
  #
  # A reserved suffix rather than a reserved group, because it makes the
  # reservation structural: the companion of a declared path is that path plus
  # this suffix, and a declared path carrying the suffix is refused below — so no
  # admissible declaration can name any companion, and the two name spaces cannot
  # be made to overlap by adding a mapping.
  stateSuffix = ".safix-sync-state";

  companionOf = m: "${m.kdbx.path}${stateSuffix}";

  kdbxSide = lib.types.submodule {
    options = {
      path = lib.mkOption {
        type = lib.types.str;
        example = "ana/grafana";
        description = ''
          Where the entry sits under the declared group, as keepassxc's own
          command line spells an entry path.

          The last segment becomes the entry's title, and the leading segments
          are groups under `flake.safix.keepassxc.group`. Naming is the
          consumer's: safix derives nothing from the safix side's user or name,
          because what a person reading their own database wants to see there is
          a decision about their database rather than about safix.
        '';
      };

      username = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "ana@example.invalid";
        description = ''
          The username to set on the entry, or null to leave the field alone.

          The one field beyond the value and the title that a mapping may set.
          Arbitrary field templating is deliberately absent: every field safix
          writes is a field its report and its refusals have to be able to speak
          about, and a person's own database is not a projection of a
          declaration.
        '';
      };
    };
  };

  mapping = lib.types.submodule {
    options = {
      mode = lib.mkOption {
        type = lib.types.enum modes;
        example = "safix-to-keepassxc";
        description = ''
          Which way this mapping converges.

          `safix-to-keepassxc` makes the database follow safix: a database-side
          edit to a mapped entry is overwritten, and reported.

          `keepassxc-to-safix` makes safix follow the database, through the same
          write path a hand-set value takes, with every refusal on that path in
          force.

          `two-way` converges toward whichever side changed since the last
          agreement. Both sides changed is a conflict that writes nothing and
          names the two one-way commands that each resolve it.

          `backup` writes safix's value where the database holds none, and never
          overwrites one that differs — it reports the divergence instead.

          No mode deletes an entry on either side. A mapping that is removed
          stops being synced and its last database value stays until a person
          removes it, which is the one part of the Filen model deliberately not
          taken: an accidental deletion of a secret is not a state a sync should
          be able to reach.
        '';
      };

      safix = lib.mkOption {
        type = bridge.safixSide;
        description = "The safix half: a user, and a name that user holds.";
      };

      kdbx = lib.mkOption {
        type = kdbxSide;
        description = ''
          The database half: a path under the declared group, and optionally a
          username.

          Evaluation verifies neither. The group and the entry are content of an
          encrypted file, and the only thing that can answer whether they are
          there is the database itself, with a key.
        '';
      };
    };
  };

  # ── the refusals evaluation can reach ──
  #
  # Returned as a list rather than thrown, which is what lets a fixture assert
  # them against literals without building a derivation and lets a severity
  # drill run the same `refuseScript` bytes the real check runs.
  mappingsOf = keepassxc: lib.mapAttrsToList (id: m: m // { inherit id; }) keepassxc.mappings;

  # The entry path as the store's own command line takes it, and as every report
  # of the mapping names it.
  entryPathOf = group: m: "${group}/${m.kdbx.path}";

  violationsOf =
    registry: keepassxc:
    let
      inherit (registry) users;
      declared = mappingsOf keepassxc;

      # Custody has to resolve before a mapping's safix side can be looked up in
      # it, and a broken custody declaration already has its own refusal.
      # Staying silent here keeps one fault from producing two unrelated
      # sentences — ./bridge.nix short-circuits on the same list.
      placements = resolve.placementsOf registry;

      resolves = m: (placements.${m.safix.user} or { }) ? ${m.safix.name};

      unresolvableSafixSide = lib.concatMap (
        m:
        if !(users ? ${m.safix.user}) then
          [
            "flake.safix.keepassxc.mappings.${m.id} names the user '${m.safix.user}', which flake.safix.users does not declare"
          ]
        else if !(resolves m) then
          [
            "flake.safix.keepassxc.mappings.${m.id} names the secret '${m.safix.name}', which flake.safix.users.${m.safix.user} does not hold"
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
          "flake.safix.keepassxc.mappings.${m.id} is ${m.mode} into flake.safix.users.${m.safix.user}.${m.safix.name}, which a generator also produces — two producers for one value, and the winner is whichever ran last"
      ) sound;

      byEntry = lib.groupBy (entryPathOf keepassxc.group) sound;

      twoMappingsOneEntry = lib.concatLists (
        lib.mapAttrsToList (
          entry: group:
          lib.optional (builtins.length group > 1)
            "flake.safix.keepassxc.mappings ${
              lib.concatMapStringsSep " and " (m: m.id) group
            } both name the entry ${entry}"
        ) byEntry
      );

      # Judged over every declared mapping rather than over the sound ones, and
      # on the kdbx side alone: a mapping whose safix side is also wrong has two
      # faults and is entitled to hear about both.
      reservedName = lib.concatMap (
        m:
        lib.optional (lib.hasSuffix stateSuffix m.kdbx.path) "flake.safix.keepassxc.mappings.${m.id} names the entry ${entryPathOf keepassxc.group m}, and '${stateSuffix}' is the suffix safix reserves for the entry a two-way mapping records its last agreement in"
      ) declared;
    in
    if resolve.violations registry != [ ] then
      [ ]
    else
      unresolvableSafixSide ++ twoProducers ++ twoMappingsOneEntry ++ reservedName;
in
{
  inherit
    modes
    pullCapable
    stateSuffix
    companionOf
    mapping
    mappingsOf
    entryPathOf
    violationsOf
    ;
}
