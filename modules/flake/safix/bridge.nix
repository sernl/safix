# The declared relationship between a var in clan and an entry in safix.
#
# A bridge is a standing relationship rather than an event, so it is written
# down rather than passed as arguments. A declaration is diffable, so adding a
# mapping shows up in review as a line naming both endpoints; it is repeatable,
# so a run has no arguments to get wrong; it is checkable, so evaluation refuses
# a mapping whose safix side does not exist; and it is enumerable, so the audit
# can report the whole bridge without being told what the bridge is.
#
# Half of every mapping lives in another flake, and this file cannot see it.
# What is refused here is exactly the half that is local to the consumer. A
# clan side that does not resolve is a run-time refusal naming the machine, the
# generator and the file — stated in the option documentation rather than left
# for an operator to infer from an empty report.
{ lib }:
let
  resolve = import ./resolve.nix { inherit lib; };

  # Written as its endpoints rather than as a verb.
  #
  # `clan vars export <dir>` moves values *out of* clan; a safix-to-clan
  # mapping's convergence moves a value the opposite way. Both are correct
  # relative to the tool that moves them, and a declaration is read by
  # someone with no tool in hand to be relative to — so the endpoints are
  # named instead of a verb either tool speaks.
  directions = [
    "clan-to-safix"
    "safix-to-clan"
    "two-way"
  ];

  # clan's own placement is a three-way sum — Shared, PerMachine, PerExport
  # (clan_lib/vars/_types.py) — of which `clan vars get`/`set` can resolve only
  # the first two through a machine: `get_machine_generators`
  # (clan_lib/vars/generator.py:229-351) never constructs a PerExport
  # placement for any machine it is asked about, so a PerExport var is
  # unreachable through the two contracts safix uses regardless of what a
  # mapping declared. Only the two placements safix can actually address are
  # offered here.
  clanPlacements = [
    "shared"
    "per-machine"
  ];

  clanSide = lib.types.submodule {
    options = {
      placement = lib.mkOption {
        type = lib.types.enum clanPlacements;
        default = "per-machine";
        example = "shared";
        description = ''
          Which of clan's own placements the var is declared under.

          `per-machine` is the default so every mapping declared before this
          option existed parses unchanged. `shared` names a var one generator
          declares once for the whole fleet; the machine that answers for it
          on clan's command line is discovered at run time (see `machine`
          below) rather than declared, because a second declared field would
          be a copy of a fact only clan's own flake holds.
        '';
      };

      machine = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "meridian";
        description = ''
          The clan machine the var belongs to.

          Required when `placement` is `per-machine`, and refused (must stay
          null) when `placement` is `shared`: a shared var is not owned by any
          one machine, and naming one here would let two mappings of the same
          shared var name different machines and evade the duplicate-target
          and two-way-conflict detection that groups mappings by this pair.
          The machine clan's command line actually takes for a shared mapping
          is discovered at run time by asking clan which machines it has.
        '';
      };

      generator = lib.mkOption {
        type = lib.types.str;
        example = "ntfy";
        description = "The clan generator that declares the var.";
      };

      file = lib.mkOption {
        type = lib.types.str;
        example = "token";
        description = ''
          The file the generator declares, named as clan names it. The pair is
          spelled `<generator>/<file>` when it reaches clan's command, which is
          the var id clan's own `get` and `set` take.
        '';
      };
    };
  };

  # One submodule for "a user and a name that user holds", exported because
  # ./keepassxc.nix declares the same half of its own mappings and the runtime
  # deserializes both through one `SafixSide`. Two option types would be two
  # documentations of one pair, free to drift.
  safixSide = lib.types.submodule {
    options = {
      user = lib.mkOption {
        type = lib.types.str;
        example = "alice";
        description = "The `flake.safix.users` entry that holds the value.";
      };

      name = lib.mkOption {
        type = lib.types.str;
        example = "ntfy-token";
        description = "The secret that user holds, as they hold it.";
      };
    };
  };

  mapping = lib.types.submodule {
    options = {
      direction = lib.mkOption {
        type = lib.types.enum directions;
        example = "clan-to-safix";
        description = ''
          Which way the value moves, written as its endpoints.

          clan's own `vars export` moves values out of clan; a safix-to-clan
          mapping's convergence moves a value the opposite way, so a word one
          tool already uses for its own verb would mean the opposite thing if
          reused for this option. A declaration is read without a tool in
          hand to be relative to, so the endpoints are named instead.

          `two-way` names neither a source nor a destination, because the
          value may originate on either side; it converges toward whichever
          side changed since the last agreement, and is declared once rather
          than as two opposing one-way mappings.
        '';
      };

      clan = lib.mkOption {
        type = clanSide;
        description = ''
          The clan half: a placement, a machine or nothing as that placement
          requires, a generator, and a file that generator declares.

          Evaluation does not verify any of it. It lives in another flake, and
          the only thing that can answer whether it resolves is clan itself. A
          clan side that does not resolve is refused when a transfer reaches
          that mapping, and the refusal names all of it.
        '';
      };

      safix = lib.mkOption {
        type = safixSide;
        description = "The safix half: a user, and a name that user holds.";
      };
    };
  };

  # The suffix safix reserves for the companion entry a `two-way` mapping
  # records its last agreement in.
  #
  # Drawn from `resolve.nix`'s own `wellFormedName` alphabet
  # (`[a-z0-9][a-z0-9_-]*`) rather than `store.rs`'s dot-prefixed
  # `.safix-sync-state`: a companion here is a safix entry name, and
  # `resolve.nix` refuses any declared entry name outside that alphabet
  # before this reservation could ever see it, which would make the
  # collision this suffix exists to catch unreachable. `store.rs`'s suffix
  # names a kdbx path instead, which is under no such constraint. The two
  # strings are distinct either way, so the two mechanisms' reservations
  # stay independently checkable.
  stateSuffix = "-safix-bridge-sync-state";

  companionOf = m: "${m.safix.name}${stateSuffix}";

  # Every two-way mapping mints a second placement beside the one it maps,
  # sharing its file and its audience and distinguished only by the reserved
  # key suffix, so the companion resolves inside the same encrypted document
  # at no extra custody grant.
  #
  # Minted here, over the already-resolved placement set, rather than folded
  # into `resolve.nix`'s own `sourcesIn`/`placementsIn` algebra: a companion is
  # never carried, never private and never shared in its own right — it exists
  # only because a two-way mapping was declared, and its file, audience and
  # ownership are entirely derived from the entry it mirrors. Registering it as
  # a fourth source in the general algebra would give it declarations
  # (`carries`/`private`/`sharedWith`) it can never actually have.
  #
  # A mapping whose safix side does not resolve mints no companion: that
  # mapping already has its own evaluation refusal, and a companion for an
  # entry that does not exist would have no file to share.
  companionsOf =
    registry: bridgeCfg:
    let
      placements = resolve.placementsOf registry;
      twoWay = lib.filter (m: m.direction == "two-way") (mappingsOf bridgeCfg);
      resolves = m: (placements.${m.safix.user} or { }) ? ${m.safix.name};
    in
    lib.foldl' (
      acc: m:
      if !(resolves m) then
        acc
      else
        let
          mapped = placements.${m.safix.user}.${m.safix.name};
        in
        lib.recursiveUpdate acc {
          ${m.safix.user}.${companionOf m} = {
            inherit (mapped) origin owner shared file;
            key = "${mapped.key}${stateSuffix}";
            generator = null;
            public = null;
          };
        }
    ) { } twoWay;

  # A mapping's clan-side address, as clan's command line and every report
  # name it: a shared var is identified by its generator and file alone,
  # because no one machine owns it and the address discovered at run time is
  # not part of the declared identity; a per-machine var is identified by the
  # machine that owns it as well.
  clanAddressOf =
    m:
    if m.clan.placement == "shared" then
      "shared:${m.clan.generator}/${m.clan.file}"
    else
      "${m.clan.machine}:${m.clan.generator}/${m.clan.file}";

  safixAddressOf = m: "flake.safix.users.${m.safix.user}.${m.safix.name}";

  # ── the refusals evaluation can reach ──
  #
  # Every message below is a statement about the consumer's own declarations.
  # They are returned as a list rather than thrown for the reason the custody
  # and generator families are: a list can be asserted against a literal by a
  # fixture without building a derivation, and a severity drill can run the
  # same `refuseScript` bytes the real check runs over a perturbed fleet.
  mappingsOf = bridge: lib.mapAttrsToList (id: m: m // { inherit id; }) bridge.mappings;

  endpointsOf =
    m:
    "${clanAddressOf m} <-> ${safixAddressOf m}";

  violationsOf =
    registry: bridge:
    let
      users = registry.users;
      declared = mappingsOf bridge;

      # Custody has to resolve before a mapping's safix side can be looked up in
      # it, and a broken custody declaration already has its own refusal. Saying
      # nothing here while that one is outstanding keeps one fault from
      # producing two unrelated sentences.
      placements = resolve.placementsOf registry;

      resolves = m: (placements.${m.safix.user} or { }) ? ${m.safix.name};

      unresolvableSafixSide = lib.concatMap (
        m:
        if !(users ? ${m.safix.user}) then
          [
            "flake.safix.bridge.mappings.${m.id} names the user '${m.safix.user}', which flake.safix.users does not declare"
          ]
        else if !(resolves m) then
          [
            "flake.safix.bridge.mappings.${m.id} names the secret '${m.safix.name}', which flake.safix.users.${m.safix.user} does not hold"
          ]
        else
          [ ]
      ) declared;

      # Only mappings whose safix side resolved are indexed below, because every
      # rule after this one reads that side's record.
      sound = lib.filter resolves (lib.filter (m: users ? ${m.safix.user}) declared);

      # A placement's required field, absent or present out of place. Judged
      # over every declared mapping rather than the sound ones: this rule
      # reads only the mapping's own clan side, so it owes no debt to whether
      # the safix side resolves.
      placementConsistency = lib.concatMap (
        m:
        if m.clan.placement == "per-machine" && m.clan.machine == null then
          [
            "flake.safix.bridge.mappings.${m.id} has placement = \"per-machine\" and declares no machine"
          ]
        else if m.clan.placement == "shared" && m.clan.machine != null then
          [
            "flake.safix.bridge.mappings.${m.id} has placement = \"shared\" and declares a machine, which a shared placement does not take: the machine that answers for it is discovered at run time"
          ]
        else
          [ ]
      ) declared;

      # Only mappings whose clan side is placement-consistent are indexed by
      # a clan address below: `clanAddressOf` reads `m.clan.machine`
      # unconditionally outside the shared branch, and a per-machine mapping
      # with no machine has nothing there to read. The inconsistency already
      # has its own refusal above; this only keeps the address-keyed rules
      # from evaluating a field the mapping never gave them.
      placementSound = lib.filter (
        m:
        !(
          (m.clan.placement == "per-machine" && m.clan.machine == null)
          || (m.clan.placement == "shared" && m.clan.machine != null)
        )
      ) sound;

      # Two producers for one value, refused by the rule already given for two
      # generators naming one output: the winner is whichever ran last, and
      # which that is depends on the order of a run rather than on anything
      # written down. Broadened to two-way: a two-way mapping's safix side can
      # be pulled into exactly as a clan-to-safix mapping's can, so a generator
      # on that side is the same hazard.
      twoProducers = lib.concatMap (
        m:
        lib.optional
          (
            (m.direction == "clan-to-safix" || m.direction == "two-way")
            && placements.${m.safix.user}.${m.safix.name}.generator != null
          )
          "flake.safix.bridge.mappings.${m.id} imports into flake.safix.users.${m.safix.user}.${m.safix.name}, which a generator also produces — two producers for one value, and the winner is whichever ran last"
      ) sound;

      # A safix-to-clan mapping's declared placement must agree with the
      # generator that produces its source, when one does: clan derives
      # `share` from its own placement, and a mismatch here is a mapping
      # declared against a placement clan does not actually have.
      #
      # Scoped to safix-to-clan alone: a clan-to-safix or two-way mapping whose
      # safix side is generator-produced is already refused by `twoProducers`
      # above, so no other direction reaches this comparison with a generator
      # on the safix side. A hand-set source has no generator to derive a
      # share from and is exempt, the same as every other generator-shaped
      # rule exempts one.
      sharePlacementMismatch = lib.concatMap (
        m:
        if m.direction != "safix-to-clan" then
          [ ]
        else
          let
            generator = placements.${m.safix.user}.${m.safix.name}.generator;
          in
          if generator == null then
            [ ]
          else if generator.share && m.clan.placement != "shared" then
            [
              "flake.safix.bridge.mappings.${m.id} exports from a generator whose derived share is true into placement = \"${m.clan.placement}\", which clan would derive as shared"
            ]
          else if !generator.share && m.clan.placement == "shared" then
            [
              "flake.safix.bridge.mappings.${m.id} exports from a generator whose derived share is false into placement = \"shared\", which clan would derive as per-machine"
            ]
          else
            [ ]
      ) sound;

      # The destination(s) one mapping could write. A one-way mapping writes
      # exactly one side; a two-way mapping can converge toward either, so it
      # is a producer of both and collides with anything else claiming either
      # one as its own target.
      targetsOf =
        m:
        if m.direction == "clan-to-safix" then
          [ (safixAddressOf m) ]
        else if m.direction == "safix-to-clan" then
          [ (clanAddressOf m) ]
        else
          [
            (safixAddressOf m)
            (clanAddressOf m)
          ];

      byTarget = lib.groupBy (claim: claim.target) (
        lib.concatMap (m: map (target: { inherit m target; }) (targetsOf m)) placementSound
      );

      twoMappingsOneTarget = lib.concatLists (
        lib.mapAttrsToList (
          target: claims:
          let
            ids = lib.unique (map (claim: claim.m.id) claims);
          in
          lib.optional (builtins.length ids > 1)
            "flake.safix.bridge.mappings ${lib.concatMapStringsSep " and " (id: id) ids} both write ${target}"
        ) byTarget
      );

      # One pair of endpoints in two mappings with opposite directions is a
      # two-way relationship spelled as two declarations. It is declared once,
      # as a single mapping whose direction is two-way, and spelling it as two
      # opposing one-way mappings is refused because that shape is a
      # mechanism for losing whichever side was edited first, silently, with
      # no ordering to prevent it.
      byPair = lib.groupBy endpointsOf placementSound;

      bothDirections = lib.concatLists (
        lib.mapAttrsToList (
          pair: group:
          lib.optional (lib.length (lib.unique (map (m: m.direction) group)) > 1)
            "flake.safix.bridge.mappings ${
              lib.concatMapStringsSep " and " (m: m.id) group
            } declare ${pair} in both directions, which is a two-way relationship and is declared once, as a single mapping whose direction is \"two-way\""
        ) byPair
      );

      # A mapping with nowhere to reach. Stated as its own sentence rather than
      # left to the run-time refusal, because the missing declaration is in the
      # consumer's own file and evaluation can see that it is missing.
      noClanFlake =
        lib.optional (declared != [ ] && bridge.clanFlake == null)
          "flake.safix.bridge declares ${toString (builtins.length declared)} mapping(s) and no clanFlake, so there is no clan for them to reach";

      # A mapping id spelled the same as a target keyword makes sync's and
      # audit's first argument ambiguous: there would be no way to tell
      # `audit clan` (the target) from a mapping literally named `clan` (an
      # id). Judged over every declared mapping rather than the sound ones,
      # the same way `keepassxc.nix`'s `reservedName` is, so a mapping with
      # two faults hears about both.
      reservedId = lib.concatMap (
        m:
        lib.optional (builtins.elem m.id [
          "clan"
          "keepassxc"
          "all"
        ]) "flake.safix.bridge.mappings.${m.id} is named '${m.id}', which sync and audit read as a target keyword rather than a mapping name"
      ) declared;

      # A two-way mapping's companion reserves a name in its own user's
      # namespace; a hand-declared entry of that same name would collide with
      # it silently the moment the companion is minted. Judged over the sound
      # two-way mappings alone: an unresolvable safix side already has its own
      # refusal above, and there is no owning user's record to check a
      # collision against until it resolves.
      reservedCompanionName = lib.concatMap (
        m:
        if m.direction != "two-way" then
          [ ]
        else
          let
            reserved = companionOf m;
            userRec = users.${m.safix.user};
          in
          lib.optional (userRec.carries ? ${reserved} || userRec.private ? ${reserved})
            "flake.safix.users.${m.safix.user} declares '${reserved}', and '${stateSuffix}' is the suffix flake.safix.bridge.mappings.${m.id} reserves for the entry its two-way convergence records its last agreement in"
      ) sound;
    in
    if resolve.violations registry != [ ] then
      [ ]
    else
      unresolvableSafixSide
      ++ placementConsistency
      ++ twoProducers
      ++ sharePlacementMismatch
      ++ twoMappingsOneTarget
      ++ bothDirections
      ++ noClanFlake
      ++ reservedId
      ++ reservedCompanionName;
in
{
  inherit
    directions
    clanPlacements
    mapping
    safixSide
    mappingsOf
    endpointsOf
    clanAddressOf
    safixAddressOf
    stateSuffix
    companionOf
    companionsOf
    violationsOf
    ;
}
