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
  # `clan vars export <dir>` moves values *out of* clan; `safix export` moves
  # values *into* clan. Both words are correct relative to the tool that says
  # them, and a declaration is read by someone with no tool in hand to be
  # relative to. The verbs stay `import` and `export` on safix's command line,
  # where safix is the frame every other verb already assumes.
  directions = [
    "clan-to-safix"
    "safix-to-clan"
  ];

  clanSide = lib.types.submodule {
    options = {
      machine = lib.mkOption {
        type = lib.types.str;
        example = "meridian";
        description = "The clan machine the var belongs to.";
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
        example = "ana";
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

          The word `export` moves values out of clan when clan says it and into
          clan when safix says it, so a direction spelled with it means opposite
          things depending on which tool the reader has in mind. A declaration
          is read without a tool in hand to be relative to, so the endpoints are
          named instead.
        '';
      };

      clan = lib.mkOption {
        type = clanSide;
        description = ''
          The clan half: a machine, a generator, and a file that generator
          declares.

          Evaluation does not verify any of it. It lives in another flake, and
          the only thing that can answer whether it resolves is clan itself. A
          clan side that does not resolve is refused when a transfer reaches
          that mapping, and the refusal names all three.
        '';
      };

      safix = lib.mkOption {
        type = safixSide;
        description = "The safix half: a user, and a name that user holds.";
      };
    };
  };

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
    "${m.clan.machine}:${m.clan.generator}/${m.clan.file} <-> flake.safix.users.${m.safix.user}.${m.safix.name}";

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

      # Two producers for one value, refused by the rule already given for two
      # generators naming one output: the winner is whichever ran last, and
      # which that is depends on the order of a run rather than on anything
      # written down.
      twoProducers = lib.concatMap (
        m:
        lib.optional
          (m.direction == "clan-to-safix" && placements.${m.safix.user}.${m.safix.name}.generator != null)
          "flake.safix.bridge.mappings.${m.id} imports into flake.safix.users.${m.safix.user}.${m.safix.name}, which a generator also produces — two producers for one value, and the winner is whichever ran last"
      ) sound;

      targetOf =
        m:
        if m.direction == "clan-to-safix" then
          "flake.safix.users.${m.safix.user}.${m.safix.name}"
        else
          "${m.clan.machine} ${m.clan.generator}/${m.clan.file}";

      byTarget = lib.groupBy targetOf sound;

      twoMappingsOneTarget = lib.concatLists (
        lib.mapAttrsToList (
          target: group:
          lib.optional (builtins.length group > 1)
            "flake.safix.bridge.mappings ${
              lib.concatMapStringsSep " and " (m: m.id) group
            } both write ${target}"
        ) byTarget
      );

      # One pair of endpoints in two mappings with opposite directions is a
      # two-way synchronisation spelled as two declarations. A two-way sync with
      # no conflict resolution is a mechanism for losing whichever side was
      # edited first, silently, so it is refused rather than ordered.
      byPair = lib.groupBy endpointsOf sound;

      bothDirections = lib.concatLists (
        lib.mapAttrsToList (
          pair: group:
          lib.optional (lib.length (lib.unique (map (m: m.direction) group)) > 1)
            "flake.safix.bridge.mappings ${
              lib.concatMapStringsSep " and " (m: m.id) group
            } declare ${pair} in both directions, which is a two-way synchronisation and has no conflict resolution"
        ) byPair
      );

      # A mapping with nowhere to reach. Stated as its own sentence rather than
      # left to the run-time refusal, because the missing declaration is in the
      # consumer's own file and evaluation can see that it is missing.
      noClanFlake =
        lib.optional (declared != [ ] && bridge.clanFlake == null)
          "flake.safix.bridge declares ${toString (builtins.length declared)} mapping(s) and no clanFlake, so there is no clan for them to reach";
    in
    if resolve.violations registry != [ ] then
      [ ]
    else
      unresolvableSafixSide ++ twoProducers ++ twoMappingsOneTarget ++ bothDirections ++ noClanFlake;
in
{
  inherit
    directions
    mapping
    safixSide
    mappingsOf
    endpointsOf
    violationsOf
    ;
}
