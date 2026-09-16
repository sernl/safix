# The declared relationship between a safix entry and an item in the operator's
# Bitwarden vault.
#
# The same shape ./keepassxc.nix has, for the same reason: a standing
# relationship is written down rather than passed as arguments, so adding a
# mapping shows up in review as a line naming both endpoints, a run has no
# arguments to get wrong, evaluation can refuse a mapping whose safix side does
# not exist, and a report can enumerate the whole mirror without being told what
# it is.
#
# What differs is the far side. clan's half lives in another flake, the
# database's half lives on a filesystem, and this one lives behind a network
# service — so the refusals here are again exactly the ones local to the
# consumer's own declarations. Whether the vault holds the item, whether the
# client is unlocked or even logged in, and whether the declared folder exists
# are run-time questions the verb answers with a session in hand, and nothing
# evaluation can reach.
#
# An item is addressed by folder and name rather than by the vault's own item
# identifier. An identifier is opaque to review, is not what the person holding
# the item sees, and is reissued when a vault is exported and re-imported, so a
# declaration written against one silently stops naming anything; folder and
# name are the address the person uses. `design.md`'s B1 records the decision.
{ lib }:
let
  resolve = import ./resolve.nix { inherit lib; };
  bridge = import ./bridge.nix { inherit lib; };
  fields = import ./fields.nix { inherit lib; };
  reserved = import ./reserved.nix;

  # Which channel this transport carries each field on, read straight off the
  # shared table rather than restated, so the rules below and the Rust half's
  # own `Capabilities` cannot be forked from it. Exported so
  # `modules/flake/checks/bitwarden.nix` can assert that.
  capabilities = fields.channels.bitwarden;

  # Written as their endpoints rather than as push and pull, the decision
  # ./bridge.nix records for `direction` and ./keepassxc.nix repeats: a
  # declaration is read by someone with no tool in hand to be relative to.
  #
  # `two-way` and `backup` are not endpoint pairs and do not need to be. They
  # name a relationship rather than a direction.
  modes = [
    "safix-to-bitwarden"
    "bitwarden-to-safix"
    "two-way"
    "backup"
  ];

  # The modes under which safix's side can be written, which is what makes a
  # generator on that side a second producer.
  pullCapable = mode: mode == "bitwarden-to-safix" || mode == "two-way";

  # The fields this target carries, and what each one becomes in Bitwarden's own
  # item schema: `username` is `login.username`, `url` is the first entry of
  # `login.uris`, and `notes` is the item's own top-level note.
  #
  # `tags` is absent because this vault has no tag concept at all. A folder is a
  # single placement and a collection is an organizational permission boundary,
  # so neither is a label — which is why a declared tag is refused below rather
  # than approximated onto one of them (B7).
  carriedFields = [
    "username"
    "url"
    "notes"
  ];

  bitwardenSide = lib.types.submodule {
    options = {
      folder = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "fleet";
        description = ''
          The vault folder the item sits in, or null for the vault's root, which
          is where an item in no folder lives.

          A folder rather than a collection: a collection is an organization's
          permission boundary with its own addressing space, and this target
          addresses a personal vault.
        '';
      };

      item = lib.mkOption {
        type = lib.types.str;
        example = "grafana";
        description = ''
          The item's name, as the person holding it sees it in their vault.

          Naming is the consumer's: safix derives nothing from the safix side's
          user or name, because what somebody reading their own vault wants to
          see there is a decision about their vault rather than about safix.

          Never the vault's own item identifier. An identifier says nothing a
          reviewer can check, is not what the person sees, and is reissued by a
          restore of the vault.
        '';
      };

      fields = lib.mkOption {
        type = fields.fields;
        default = { };
        description = ''
          What the item carries beside its value: a username, a url, notes and
          tags, as ./fields.nix declares them for every target.

          `tags` is refused for this target rather than accepted and dropped:
          this vault has no tag concept, only folders and collections, and both
          are placements rather than labels. An `{ entry = …; }` source is
          permitted for every field this target does carry, because they all
          travel the client's standard input rather than an argument vector.
        '';
      };
    };
  };

  mapping = lib.types.submodule {
    options = {
      mode = lib.mkOption {
        type = lib.types.enum modes;
        example = "safix-to-bitwarden";
        description = ''
          Which way this mapping converges.

          `safix-to-bitwarden` makes the vault follow safix: a vault-side edit to
          a mapped item is overwritten, and reported.

          `bitwarden-to-safix` makes safix follow the vault, through the same
          write path a hand-set value takes, with every refusal on that path in
          force.

          `two-way` converges toward whichever side changed since the last
          agreement. Both sides changed is a conflict that writes nothing and
          names the two one-way commands that each resolve it.

          `backup` writes safix's value where the vault holds no such item, and
          never overwrites one that differs — it reports the divergence instead.

          No mode deletes an item, a folder or a field on either side. A mapping
          that is removed stops being synced and its last vault value stays
          until a person removes it: an accidental deletion of a secret is not a
          state a sync should be able to reach.
        '';
      };

      safix = lib.mkOption {
        type = bridge.safixSide;
        description = "The safix half: a user, and a name that user holds.";
      };

      bitwarden = lib.mkOption {
        type = bitwardenSide;
        description = ''
          The vault half: an optional folder, the item's name, and the fields the
          item carries beside its value.

          Evaluation verifies none of it. The folder and the item are content of
          a vault behind a network service, and the only thing that can answer
          whether they are there is that service, with an unlocked session.
        '';
      };
    };
  };

  # ── the refusals evaluation can reach ──
  #
  # Returned as a list rather than thrown, which is what lets a fixture assert
  # them against literals without building a derivation and lets a severity
  # drill run the same `refuseScript` bytes the real check runs.
  mappingsOf = bitwarden: lib.mapAttrsToList (id: m: m // { inherit id; }) bitwarden.mappings;

  # The address every report and every refusal of the mapping names, and which
  # is no argument any command takes: the transport resolves it to an item
  # identifier at run time and hands that to the client.
  itemPathOf =
    m:
    if m.bitwarden.folder == null then
      m.bitwarden.item
    else
      "${m.bitwarden.folder}/${m.bitwarden.item}";

  violationsOf =
    registry: bitwarden:
    let
      inherit (registry) users;
      declared = mappingsOf bitwarden;

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
            "flake.safix.bitwarden.mappings.${m.id} names the user '${m.safix.user}', which flake.safix.users does not declare"
          ]
        else if !(resolves m) then
          [
            "flake.safix.bitwarden.mappings.${m.id} names the secret '${m.safix.name}', which flake.safix.users.${m.safix.user} does not hold"
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
        lib.optional (pullCapable m.mode && placements.${m.safix.user}.${m.safix.name}.generator != null)
          "flake.safix.bitwarden.mappings.${m.id} is ${m.mode} into flake.safix.users.${m.safix.user}.${m.safix.name}, which a generator also produces — two producers for one value, and the winner is whichever ran last"
      ) sound;

      byItem = lib.groupBy itemPathOf sound;

      twoMappingsOneItem = lib.concatLists (
        lib.mapAttrsToList (
          item: group:
          lib.optional (builtins.length group > 1)
            "flake.safix.bitwarden.mappings ${
              lib.concatMapStringsSep " and " (m: m.id) group
            } both name the item ${item}"
        ) byItem
      );

      # A mapping id spelled the same as a target keyword makes sync's and
      # audit's first argument ambiguous, the same way ./bridge.nix's own
      # `reservedId` refuses it over its own mappings. Read from ./reserved.nix
      # rather than restated, so the list is one artifact. Judged over every
      # declared mapping rather than the sound ones, so a mapping with two
      # faults hears about both.
      reservedId = lib.concatMap (
        m:
        lib.optional (builtins.elem m.id reserved.ids) "flake.safix.bitwarden.mappings.${m.id} is named '${m.id}', which sync and audit read as a target keyword rather than a mapping name"
      ) declared;

      # Whether the mapping says anything about one field, which is what makes
      # it subject to the rule below. An unnamed field is not a claim: an item's
      # own username is the operator's until a declaration takes it.
      declaresField =
        m: name:
        let
          value = m.bitwarden.fields.${name};
        in
        if name == "tags" then value != [ ] else value != null;

      # A field this transport cannot carry at all, which for this target is
      # tags and nothing else. Judged over every declared mapping rather than
      # the sound ones, for `reservedId`'s reason: a mapping with two faults is
      # entitled to hear about both.
      unsupportedField = lib.concatMap (
        m:
        lib.concatMap (
          name:
          lib.optional (capabilities.${name} == "unsupported" && declaresField m name)
            "flake.safix.bitwarden.mappings.${m.id} declares the field '${name}', which the bitwarden target cannot carry: this vault has no tag concept, only folders and collections, so a declared tag would either be silently dropped or mean something the declaration did not say"
        ) fields.fieldNames
      ) declared;
    in
    if resolve.violations registry != [ ] then
      [ ]
    else
      unresolvableSafixSide ++ twoProducers ++ twoMappingsOneItem ++ reservedId ++ unsupportedField;
in
{
  inherit
    modes
    pullCapable
    capabilities
    carriedFields
    mapping
    mappingsOf
    itemPathOf
    violationsOf
    ;
}
