# The records safix reads, and the whole of what a consumer declares.
#
# Every one is a plain `attrsOf` option with no default derived from anything
# outside this namespace. That is the property an adapter rests on: a module that
# sets `flake.safix.users` from a `mapAttrs` over a consumer's own registry and a
# module that sets it by hand are indistinguishable to the resolver, so bridging
# an existing user vocabulary is a projection the consumer writes rather than an
# integration point safix has to offer. The same holds of a consumer's host
# inventory projected into `flake.safix.machines`.
#
# Attrsets merge, so declarations may be scattered one per file anywhere in a
# consumer's tree and the resolver still sees one record. safix reads no path, no
# filename and no directory structure to find them; the only requirement is that
# the consumer's flake imports the modules.
{ lib, ... }:
let
  types = import ./types.nix { inherit lib; };
  bridge = import ./bridge.nix { inherit lib; };
  keepassxc = import ./keepassxc.nix { inherit lib; };

  # The declared factors that unlock a database whose own composite key needs
  # more than a password. Local to this file rather than exported from
  # ./keepassxc.nix: nothing else in this option surface declares one, so there
  # is no second documentation of the same pair to keep in sync with this one.
  yubikeySide = lib.types.submodule {
    options = {
      slot = lib.mkOption {
        type = lib.types.str;
        example = "1";
        description = ''
          The YubiKey challenge-response slot the database's own composite key
          reads, as keepassxc-cli's `-y` flag takes it.
        '';
      };

      serial = lib.mkOption {
        default = null;
        type = lib.types.nullOr lib.types.str;
        example = "12345678";
        description = ''
          The card's serial number, disambiguating which connected YubiKey
          answers the challenge, or null to accept whichever one does.
        '';
      };
    };
  };

  # One consumer bridges one clan. Two definitions of a scalar cannot both
  # survive into a list for `violationsOf` to count, so this refusal is a merge
  # that throws rather than a message in that family — it is the one bridge rule
  # whose evidence is gone by the time the resolver could look at it.
  oneClanFlake = lib.types.path // {
    merge =
      loc: defs:
      if builtins.length defs > 1 then
        throw "safix bridge: flake.safix.bridge.clanFlake is declared ${toString (builtins.length defs)} times, in ${
          lib.concatMapStringsSep " and " (d: toString d.file) defs
        }. One consumer bridges one clan."
      else
        lib.types.path.merge loc defs;
  };
in
{
  options.flake.safix = {
    catalogue = lib.mkOption {
      default = { };
      type = lib.types.attrsOf types.entry;
      description = ''
        The secret catalogue: one definition per secret more than one person may
        hold, selected by name in `flake.safix.users.<u>.carries`.

        A secret only one person will ever hold does not have to be published
        here to be resolvable — `flake.safix.users.<u>.private` takes the same
        entry submodule with the same defaults, and declaring one there is itself
        selecting it.
      '';
    };

    users = lib.mkOption {
      default = { };
      type = lib.types.attrsOf types.profile;
      description = ''
        Who holds what. This is safix's own user record and carries only custody:
        a recipient, further recipients of the same person's custody, the
        catalogue entries they carry, the secrets they declare alone, the secrets
        they grant outward, the organizations they consent to the escrow and the
        management of, and their per-host and per-tag adjustments.

        It is deliberately not a consumer's user registry and never reads one. A
        consumer with its own users writes a projection from theirs into this one;
        the two are different objects that happen to share a name.
      '';
    };

    machines = lib.mkOption {
      default = { };
      type = lib.types.attrsOf types.machine;
      example = lib.literalExpression ''
        {
          deck = {
            recipient = "age1...";
            owner = "alice";
            tags = [ "laptop" ];
          };
        }
      '';
      description = ''
        The machines an audience may name: each one's recipient — the age form of
        the host identity its system scope already decrypts with — the person who
        owns it, and its tags.

        Declaring a machine is inert until something names it. A tree with
        machines declared and no audience reaching one generates the same policy,
        the same rules and the same files as a tree without them, byte for byte:
        a machine earns an anchor when a rule needs its key and not before.

        A machine holds nothing of its own. There is no `carries`, no `private`
        and no `sharedWith` here, because everything a machine holds arrives
        through a grant aimed at it from
        `flake.safix.users.<u>.sharedWith.<machine>`.
      '';
    };

    services = lib.mkOption {
      default = { };
      type = lib.types.attrsOf types.service;
      example = lib.literalExpression ''
        {
          nginx = {
            machines = [ "deck" ];
            owner = "alice";
            user = "nginx";
            group = "nginx";
          };
        }
      '';
      description = ''
        The services an audience may name: the machines each one runs on, the
        person who owns it, and the unix user and group its landed entries belong
        to.

        A service resolves to its machines' recipients and mints nothing, and the
        boundary that leaves is stated rather than implied away. A service grant
        narrows what is declared and what is placed — the audience names the
        service, so review reads who a secret is for, and the landed file belongs
        to the service's unix user and group, which the host enforces. It does not
        narrow what decrypts: the host identity remains what opens the file, so
        the machine is the trust boundary for everything running on it. Nothing in
        safix calls a service grant an isolation mechanism.

        A per-service identity would be a second key the same host must read at
        activation to place the service's files, so it would leave that boundary
        where it is while adding minting, custody, enrollment into every audience
        file, and rotation on every service move.

        Declaring a service is inert until something names it, on the same terms
        as `machines` and `groups`. A service holds nothing of its own: there is
        no `carries`, no `private` and no `sharedWith` here, because everything a
        service holds arrives through a grant aimed at it from
        `flake.safix.users.<u>.sharedWith.<service>`.
      '';
    };

    groups = lib.mkOption {
      default = { };
      type = lib.types.attrsOf types.group;
      example = lib.literalExpression ''{ oncall.members = [ "alice" "bob" ]; }'';
      description = ''
        The groups an audience may name, each a set of subjects — people,
        machines, services, or other groups.

        A group audience is encrypted to the expanded membership's keys, and the
        file it lands in is named for the group rather than for its members, so
        membership change is a re-wrap of one file rather than a migration to
        another. A hundred-member guest list in a directory name is not a name.

        Declaring a group is inert until an audience names it, on the same terms
        as `machines`.
      '';
    };

    organizations = lib.mkOption {
      default = { };
      type = lib.types.attrsOf types.organization;
      example = lib.literalExpression ''
        {
          acme = {
            custody.acme-escrow = {
              key = "age1...";
              note = "acme's escrow — held offline by the operator";
            };
            managers = [ "alice" ];
          };
        }
      '';
      description = ''
        The organizations an audience may name: each one's recovery custody, and
        the people who scaffold for it.

        An organization is a principal rather than a people-set. It holds escrow
        identities, it names managers, it may own machines and services, and a
        grant may name it — and it has no membership, because a person relates to
        an organization here in exactly two ways, both declared in that person's
        own record: `flake.safix.users.<u>.escrowedTo` consents to its escrow and
        `flake.safix.users.<u>.managedBy` consents to its management. Nothing an
        organization declares widens anyone's audience, and naming a manager
        widens nothing at all — a manager scaffolds and never reads by virtue of
        managing.

        The keys live here so that rotation does. An organization rotates a
        custody key in this declaration and every consenting person's files
        re-wrap in one `safix fix`, which is the property the raw-key
        arrangement this replaces never had.

        Declaring an organization is inert until something references it, on the
        same terms as `machines`, `services` and `groups`: a tree with
        organizations declared that no escrow, grant or ownership record names
        generates the same policy, the same rules and the same files, byte for
        byte. A delegation is inert in the stronger sense that it stays inert
        once it is referenced — `managers` and `managedBy` decide which acting
        identity a verb accepts and place no key anywhere, so a fleet that
        declares both generates the byte-identical tree.
      '';
    };

    silos = lib.mkOption {
      default = { };
      type = lib.types.attrsOf types.silo;
      example = lib.literalExpression ''{ corp.groups = [ "contractors" "staff" ]; }'';
      description = ''
        Named sets of `flake.safix.groups` that no one file's audience may span.

        Evaluation refuses any audience reaching subjects of two groups in one
        set, naming the file, the subjects and the set. That is the only place a
        silo can be strong: computed where audiences are, it is a file that cannot
        exist rather than a policy hoping nobody misconfigured one.

        Inert until a group it names appears in an audience.
      '';
    };

    vault = lib.mkOption {
      default = null;
      type = lib.types.nullOr (
        lib.types.submodule {
          options = {
            root = lib.mkOption {
              type = lib.types.path;
              example = lib.literalExpression "inputs.vault";
              description = ''
                The repository every audience file's `sopsFile`, every
                generated public value and every generator definition record
                resolves rooted at, in place of this flake's own source.

                Typically a `flake = false` input's own path (`inputs.vault`),
                fetched by nix and evaluated as a plain, read-only tree —
                never itself evaluated as a flake, so nothing in this option
                requires the vault to declare one. The command-line runtime
                writes here too, at the operator-supplied working tree named
                by `SAFIX_VAULT_ROOT`, a different, mutable path from the
                locked, store-copied one this option resolves at evaluation.
              '';
            };

            namingKey = lib.mkOption {
              type = lib.types.str;
              default = "";
              example = "<64 or more lowercase hexadecimal characters>";
              description = ''
                A string of at least 64 lowercase hexadecimal characters — 32
                bytes of entropy, hex-encoded — that every vault-rooted name
                is a keyed hash of. Mint one with `openssl rand -hex 32`, or,
                without openssl, `head -c 32 /dev/urandom | od -An -tx1 | tr
                -d ' \n'`.

                A `path`-typed option would be store-copied at evaluation
                exactly like every other path in this flake, carrying the
                identical exposure a string does while adding a second file
                to keep in sync with this one — which is why this is typed
                `str` rather than `path`, and never should be one.

                The key is visible to anyone who can evaluate the declaring
                flake: every local user of a machine that has it in the nix
                store, because nix has no keyed hash and this key must
                itself be an evaluation-time value. It hides a vault-rooted
                name only from the vault's own host and from a reader
                holding only the vault, never from the store or from this
                repository.

                Evaluation refuses a declared vault whose naming key is
                unset, shorter than 64 characters, or contains a character
                outside `[0-9a-f]`, naming this option and the failing
                condition.
              '';
            };
          };
        }
      );
      example = lib.literalExpression ''
        {
          root = inputs.vault;
          namingKey = "<64 or more lowercase hexadecimal characters>";
        }
      '';
      description = ''
        A separate repository every ciphertext document, generated public
        value and generator definition record moves to, in place of this
        flake's own source. `null` — the default — leaves every one of them
        exactly where they sit today; setting a value moves them under
        `root`, each renamed to an opaque hash of `namingKey`, a
        use-specific tag and today's readable name, so that a vault host or
        a reader holding only the vault learns none of the audience, key or
        secret names the declaring flake's own tree carries.

        The recipient policy, `.sops.yaml`, never moves: it stays committed
        at this flake's own source in every case, because the encryption
        tool reads the committed file from there and because a vault host's
        own copy of it would be the richest document this scheme could
        hide.

        Naming a vault is not itself a nix evaluation of its tree: it is
        fetched as a plain, `flake = false` tree and read as paths and file
        contents, the same as this flake's own source is when no vault is
        declared.
      '';
    };

    bridge = {
      clanFlake = lib.mkOption {
        default = null;
        type = lib.types.nullOr oneClanFlake;
        example = lib.literalExpression "./.";
        description = ''
          The clan this consumer bridges to, as the flake reference clan's own
          command takes for `--flake`.

          Declared once for the consumer rather than once per mapping. A
          consumer with two clans is not a case this supports, and declaring a
          second one is refused rather than resolved by taking the first.
        '';
      };

      mappings = lib.mkOption {
        default = { };
        type = lib.types.attrsOf bridge.mapping;
        example = lib.literalExpression ''
          {
            ntfy-token = {
              direction = "clan-to-safix";
              clan = {
                machine = "meridian";
                generator = "ntfy";
                file = "token";
              };
              safix = {
                user = "alice";
                name = "ntfy-token";
              };
            };
          }
        '';
        description = ''
          Every standing relationship between a clan var and a safix entry.

          The attribute name is the mapping's own identifier. It appears in
          reports, in commit messages and in refusals, and it is not derived
          from either endpoint — a name taken from one side reads wrongly in a
          sentence about the other.

          Evaluation refuses a mapping whose safix side does not resolve, whose
          import target a generator also produces, which writes a target another
          mapping also writes, or which pairs one set of endpoints in both
          directions. It refuses nothing about the clan side: that half lives in
          another flake, and a clan side that does not resolve is refused when a
          transfer reaches the mapping, naming the machine, the generator and
          the file.
        '';
      };
    };

    keepassxc = {
      database = lib.mkOption {
        default = null;
        type = lib.types.nullOr lib.types.str;
        example = "/home/alice/.keys/master.kdbx";
        description = ''
          The password database `safix sync` converges against, as an absolute
          path on the machine the verb runs on.

          A string rather than a nix path, and that is not a style choice: a nix
          path is copied into the store when it is interpolated, so declaring
          the database as one would put a copy of the whole encrypted file — 292
          MB on the fleet this was written for — in a world-readable store, on
          every evaluation.

          There is no default, because there is no database safix could name
          that would be the right one. Unset with no mapping declared is the
          configuration of a consumer who does not use this at all; unset with
          mappings declared is refused when `safix sync` runs, naming this
          option.
        '';
      };

      yubikey = lib.mkOption {
        default = null;
        type = lib.types.nullOr yubikeySide;
        example = {
          slot = "1";
          serial = "12345678";
        };
        description = ''
          A YubiKey challenge-response slot the database's own composite key
          requires to open, or null when the database opens on its password
          alone.

          `safix sync`'s read, write, group-creation and listing commands, and
          `safix enroll --store-database`'s write, all carry the declared slot
          alongside the single password prompt those commands already ask for.
          Nothing here programs, reprograms or deletes the slot: reading it to
          answer the database's own unlock challenge is the only operation any
          command this declaration reaches performs.
        '';
      };

      keyFile = lib.mkOption {
        default = null;
        type = lib.types.nullOr lib.types.str;
        example = "/home/alice/.keys/master.keyx";
        description = ''
          A key file the database's own composite key requires to open, as an
          absolute path on the machine the verb runs on, or null when the
          database opens on its password alone.

          A string for a different reason than `database` is one above: a key
          file is typically a few kilobytes, so copying it into the store would
          not be expensive, but it would be wrong, because a key file is not the
          encrypted thing but one of the secrets the encryption depends on. A
          nix path interpolated into this declaration is copied into the
          world-readable store on every evaluation, and here that copy would be
          the very secret this option exists to keep out of it.
        '';
      };

      group = lib.mkOption {
        default = "safix";
        type = lib.types.str;
        example = "credentials/fleet";
        description = ''
          The group every mapped entry's path is relative to.

          A group has to exist for a path to be under, so this carries a default
          where `database` cannot: the default names safix rather than inventing
          a taxonomy for somebody's database, and a consumer with an existing
          layout names their own group here.

          `safix sync` creates the group, and the groups a mapping's path names
          under it, where they are absent. It removes none of them, ever.
        '';
      };

      mappings = lib.mkOption {
        default = { };
        type = lib.types.attrsOf keepassxc.mapping;
        example = lib.literalExpression ''
          {
            grafana = {
              mode = "safix-to-keepassxc";
              safix = {
                user = "alice";
                name = "grafana-password";
              };
              kdbx = {
                path = "alice/grafana";
                username = "alice@example.com";
              };
            };
          }
        '';
        description = ''
          Every standing relationship between a safix entry and an entry in the
          database.

          The attribute name is the mapping's own identifier, for the reason
          `flake.safix.bridge.mappings` gives: it appears in reports and in
          refusals, and a name taken from one side reads wrongly in a sentence
          about the other.

          Evaluation refuses a mapping whose safix side does not resolve, a
          pull-capable mapping onto an entry a generator also produces, two
          mappings naming one entry, and an entry path carrying the suffix safix
          reserves for a two-way mapping's recorded agreement. It refuses
          nothing about the database: the group and the entry are content of an
          encrypted file, and answering whether they are there needs a key.
        '';
      };
    };
  };
}
