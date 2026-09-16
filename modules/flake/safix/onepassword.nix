# The declared relationship between a safix entry and an item in the operator's
# 1Password vaults.
#
# The same shape ./keepassxc.nix has, for the same reason: a standing
# relationship is written down rather than passed as arguments, so adding a
# mapping shows up in review as a line naming both endpoints, a run has no
# arguments to get wrong, evaluation can refuse a mapping whose safix side does
# not exist, and a report can enumerate the whole mirror without being told what
# it is.
#
# What differs is the far side. A kdbx file is a local encrypted file; a vault is
# content of a remote service, reachable only with a session. So evaluation here
# refuses strictly less: whether the vault exists, whether the item does, and
# whether either side holds a value are run-time questions, and a declared vault
# this session cannot see is refused when a run reaches the mapping, naming the
# mapping and the vault.
#
# This target reserves no item name, and that is the consequence of where its
# memory lives. A `two-way` mapping records its last agreement in a concealed
# custom field of the mapped item itself, so there is no companion object beside
# the item, no name safix has to hold back, and therefore no `reservedName`
# refusal — four rules here where ./keepassxc.nix has five. keepassxc's
# companion exists only because `keepassxc-cli` cannot write a custom attribute
# on any verb; `op` can.
#
# No check of this repository drives a real `op`, and that absence is permanent
# rather than deferred. Three grounds, each sufficient on its own: `_1password-cli`
# at this flake's pin is unfree, so naming it from a check, a package or a
# devshell makes evaluation fail for every consumer who has not allowed unfree
# packages; there is no self-hostable 1Password server to point a sandboxed node
# at; and every authentication path needs the network, which no `nix build` and
# no hermetic VM node has. None of the three is a condition that may later be
# satisfied. The claim is written here, in the transport's own header, and in the
# `onepassword-sync` specification, because an unstated absence is a claim
# nobody decided to stop making.
{ lib }:
let
  resolve = import ./resolve.nix { inherit lib; };
  bridge = import ./bridge.nix { inherit lib; };
  fields = import ./fields.nix { inherit lib; };
  reserved = import ./reserved.nix;

  # Which channel this transport carries each field on, read straight off the
  # shared table rather than restated, so the rules below and the Rust half's
  # own `Capabilities` cannot be forked from it. Every row is `stdin` here: the
  # whole item crosses as one JSON object on standard input, which is why no
  # field of this target is refused for want of a channel.
  capabilities = fields.channels.onepassword;

  # Written as their endpoints rather than as push and pull, the decision
  # ./bridge.nix records for `direction` and ./keepassxc.nix holds for its own
  # modes: a declaration is read by someone with no tool in hand to be relative
  # to.
  modes = [
    "safix-to-1password"
    "1password-to-safix"
    "two-way"
    "backup"
  ];

  # The modes under which safix's side can be written, which is what makes a
  # generator on that side a second producer.
  pullCapable = mode: mode == "1password-to-safix" || mode == "two-way";

  opSide = lib.types.submodule {
    options = {
      vault = lib.mkOption {
        type = lib.types.str;
        example = "Private";
        description = ''
          The vault the mapped item lives in, by name.

          A string rather than a nix path, for the reason
          `flake.safix.keepassxc.database` is one: a nix path is copied into the
          store when it is interpolated, so a path here would put a copy of
          whatever it named into a world-readable store on every evaluation. And
          a second reason this option has and that one does not: an absolute
          string derived from the flake's own root resolves to two different
          values for the two `safix-examples` consumers, which compare this
          projection field for field.

          There is no default. A service account cannot reach the built-in
          Private, Personal or Employee vault at all, and `op item get` requires
          the vault under one, so a default would be a value that fails for the
          most likely automation posture.
        '';
      };

      item = lib.mkOption {
        type = lib.types.str;
        example = "grafana";
        description = ''
          The item's title inside that vault.

          A string rather than a nix path for the same two reasons `vault` is
          one: a path is copied into the world-readable store on every
          evaluation, and a root-dependent absolute string makes the two
          `safix-examples` consumers resolve this projection to different
          values.

          Naming is the consumer's: safix derives nothing from the safix side's
          user or name, because what a person reading their own vault wants to
          see there is a decision about their vault rather than about safix.
        '';
      };

      fields = lib.mkOption {
        type = fields.fields;
        default = { };
        description = ''
          What the item carries beside its value: a username, a url, notes and
          tags, as ./fields.nix declares them for every target.

          All four are carried, each on the standard input the value itself
          travels, so no field of this target is refused for want of a channel
          and an `{ entry = …; }` source is admissible on every one of them. A
          declared url becomes the item's own autofill website rather than a
          similarly named custom field, because the custom field is not the one
          a person's browser looks at.
        '';
      };
    };
  };

  mapping = lib.types.submodule {
    options = {
      mode = lib.mkOption {
        type = lib.types.enum modes;
        example = "safix-to-1password";
        description = ''
          Which way this mapping converges.

          `safix-to-1password` makes the item follow safix: an item-side edit to
          a mapped value is overwritten, and reported.

          `1password-to-safix` makes safix follow the item, through the same
          write path a hand-set value takes, with every refusal on that path in
          force.

          `two-way` converges toward whichever side changed since the last
          agreement, which is recorded in a concealed custom field of the item
          itself. Both sides changed is a conflict that writes nothing and names
          the two one-way commands that each resolve it.

          `backup` writes safix's value where the item does not exist or holds
          none, and never overwrites one that differs — it reports the
          divergence instead.

          No mode deletes an item, a field or a vault. A mapping that is removed
          stops being synced and its last item value stays until a person
          removes it: an accidental deletion of a secret is not a state a sync
          should be able to reach.
        '';
      };

      safix = lib.mkOption {
        type = bridge.safixSide;
        description = "The safix half: a user, and a name that user holds.";
      };

      onepassword = lib.mkOption {
        type = opSide;
        description = ''
          The 1Password half: a vault, an item in it, and the fields the item
          carries beside its value.

          Evaluation verifies neither the vault nor the item, because both are
          content of a remote service and answering needs a session. A vault
          this session cannot see is refused when a run reaches the mapping,
          naming the mapping and the vault.
        '';
      };
    };
  };

  # ── the refusals evaluation can reach ──
  #
  # Returned as a list rather than thrown, which is what lets a fixture assert
  # them against literals without building a derivation and lets a severity
  # drill run the same `refuseScript` bytes the real check runs.
  mappingsOf = onepassword: lib.mapAttrsToList (id: m: m // { inherit id; }) onepassword.mappings;

  # The vault-qualified item, and as every report of the mapping names it. The
  # vault is part of the identity rather than decoration: two mappings naming
  # one item name in two vaults are two items and do not collide.
  itemPathOf = m: "${m.onepassword.vault}/${m.onepassword.item}";

  violationsOf =
    registry: onepassword:
    let
      inherit (registry) users;
      declared = mappingsOf onepassword;

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
            "flake.safix.onepassword.mappings.${m.id} names the user '${m.safix.user}', which flake.safix.users does not declare"
          ]
        else if !(resolves m) then
          [
            "flake.safix.onepassword.mappings.${m.id} names the secret '${m.safix.name}', which flake.safix.users.${m.safix.user} does not hold"
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
          "flake.safix.onepassword.mappings.${m.id} is ${m.mode} into flake.safix.users.${m.safix.user}.${m.safix.name}, which a generator also produces — two producers for one value, and the winner is whichever ran last"
      ) sound;

      byItem = lib.groupBy itemPathOf sound;

      twoMappingsOneItem = lib.concatLists (
        lib.mapAttrsToList (
          item: group:
          lib.optional (builtins.length group > 1)
            "flake.safix.onepassword.mappings ${
              lib.concatMapStringsSep " and " (m: m.id) group
            } both name the item ${item}"
        ) byItem
      );

      # A mapping id spelled the same as a target keyword makes sync's and
      # audit's first argument ambiguous, the same way ./bridge.nix's own
      # `reservedId` refuses it over its own mappings. Judged over every
      # declared mapping rather than the sound ones, so a mapping with two
      # faults hears about both.
      reservedId = lib.concatMap (
        m:
        lib.optional (builtins.elem m.id reserved.ids) "flake.safix.onepassword.mappings.${m.id} is named '${m.id}', which sync and audit read as a target keyword rather than a mapping name"
      ) declared;
    in
    if resolve.violations registry != [ ] then
      [ ]
    else
      unresolvableSafixSide ++ twoProducers ++ twoMappingsOneItem ++ reservedId;
in
{
  inherit
    modes
    pullCapable
    capabilities
    opSide
    mapping
    mappingsOf
    itemPathOf
    violationsOf
    ;
}
