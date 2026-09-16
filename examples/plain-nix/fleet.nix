# The fleet this example resolves, as one file. Passed to `lib.mkVault`'s
# `modules` list in ./entry.nix, and read the identical way by
# `modules/flake/checks/examples.nix`, which compares it field for field
# against ../dendritic's scattered declarations of the same fleet. The same
# fleet is also what `examples/profiles/` consumes at both scopes, so this is
# the one fleet in the repository and there is nothing for a second one to
# disagree with.
#
# alice is the person the fleet is written around: she carries the three
# catalogue entries, holds private values including six generators, grants one
# entry each to a person, a machine, a service, a group, an organization and
# the owner of a machine, consents to acme's recovery custody, holds an offline
# master identity of her own, and adjusts her own placement per host and per
# tag. bob carries the same catalogue entries as a second, independent carrier
# and is managed by acme. carol carries nothing and exists so the second silo
# group has a member. deck is the machine alice owns, web the service that runs
# on it, rack a machine acme owns — which is what makes an `ownerOf` grant
# resolve to custody keys rather than to a person's recipient. oncall is the
# group of people, infra the group whose members are a machine, a service and
# another group, contractors the second group corp holds apart from oncall.
# acme is the organization: it holds the escrow custody, owns rack, and names
# alice as the manager who scaffolds for bob.
#
# Deliberately not declared here: `flake.safix.storage` and
# `flake.safix.vault`. Both are fleet-wide — relocating the roots moves every
# path in every compared field and a naming key makes every name opaque — so
# they are declared in `examples/profiles/relocated.nix`, merged beside this
# fleet for the one evaluation whose consequence is visible.
{
  flake.safix = {
    catalogue = {
      # Carried separately by alice and bob: each holds their own copy.
      shelf-item = { };

      # Carried by both: one value, one ciphertext, shared between them.
      team-wifi.shared = true;

      # An entry that names its own mode, which no placement field carries:
      # `examples/profiles/` is where it is observable, at the scope that
      # materializes it.
      deploy-key = {
        mode = "0440";
      };
    };

    users = {
      alice = {
        recipient = "age1exampleaaa00000000000000000000000000000000000000000000000";
        recipientNote = "alice — example identity, decrypts nothing";

        # A second recipient alice holds herself, which is what independent
        # custody looks like: losing the activation key leaves her files
        # openable by her rather than by the operator.
        recoveryRecipients.master = {
          key = "age1examplemaster00000000000000000000000000000000000000000000";
          note = "alice's offline master identity — held by her, not by the operator";
        };

        carries = {
          shelf-item = { };
          team-wifi = { };
          deploy-key = { };
        };

        private = {
          laptop-token = { };

          generated-token.generator = {
            script = ''openssl rand -hex 32 > "$out/generated-token"'';
            runtimeInputs = [ "openssl" ];
          };

          # Each of the six below is granted onward through sharedWith; a
          # grant hands on an entry the granter already holds, it does not
          # create one.
          handoff-note = { };
          fleet-token = { };
          web-token = { };
          pager-token = { };
          corp-handover = { };
          escrow-note = { };

          # An entry whose landed path is a function of the configuration
          # materializing it. `functionTo str` cannot be serialized, so no
          # compared field carries it and `examples/profiles/home.nix` is where
          # the applied string is observable.
          app-credentials = {
            path = cfg: "${cfg.home.homeDirectory}/.config/example-app/credentials.toml";
          };

          # One run, two further outputs: a private half that is encrypted like
          # any other entry, and a public half a nix module reads at
          # evaluation. The reading expression belongs in a consumer's own
          # module, as `config.flake.safix.lib.publicValue "alice" "wg-public"`
          # — safix writes no module that reads it for you.
          wg-key.generator = {
            script = ''
              wg genkey > "$out/wg-private"
              wg pubkey < "$out/wg-private" > "$out/wg-public"
              printf 'example\n' > "$out/wg-key"
            '';
            runtimeInputs = [
              "coreutils"
              "wireguard-tools"
            ];
            files = {
              wg-private.secret = true;
              wg-public.secret = false;
            };
          };

          # The two outputs wg-key's generator writes. Each is an entry in its
          # own right, which is what gives each its own placement, and neither
          # carries a generator of its own.
          wg-private = { };
          wg-public = { };

          prompted-token.generator = {
            prompts.passphrase = {
              type = "hidden";
              description = "the passphrase the token is derived from";
            };
            script = ''sha256sum < "$prompts/passphrase" | cut -d' ' -f1 > "$out/prompted-token"'';
            runtimeInputs = [ "coreutils" ];
          };

          derived-token.generator = {
            dependencies = [ "generated-token" ];
            script = ''
              sha256sum < "$in/generated-token/generated-token" | cut -d' ' -f1 > "$out/derived-token"
            '';
            runtimeInputs = [ "coreutils" ];
          };

          validated-token.generator = {
            script = ''openssl rand -hex 16 > "$out/validated-token"'';
            runtimeInputs = [ "openssl" ];
            validation = "grep -Eq '^[0-9a-f]{32}$' ";
          };

          # The one generator granted the network. It widens what a mint may
          # reach and nothing else: the filesystem confinement stays, so the
          # staging root is still the only writable path.
          fetched-token.generator = {
            network = true;
            script = ''curl -fsS https://tokens.example/new > "$out/fetched-token"'';
            runtimeInputs = [
              "coreutils"
              "curl"
            ];
          };

          # The safix side of every sync mapping this fleet declares. They are
          # ordinary private entries: a mapping names an entry, it does not
          # declare one.
          ntfy-token = { };
          grafana-password = { };
          deploy-token = { };
          deploy-username = { };
          vpn-password = { };
          registry-token = { };
        };

        sharedWith = {
          bob.handoff-note = { }; # a person
          deck.fleet-token = { }; # a machine
          web.web-token = { }; # a service
          oncall.pager-token = { }; # a group
          acme.escrow-note = { }; # an organization, as a grant audience
          "ownerOf.rack".corp-handover = { }; # whoever owns rack, which is acme
        };

        # acme's recovery custody can open everything alice holds.
        escrowedTo = [ "acme" ];

        # Placement adjustments: still alice's everywhere, simply not landed
        # here. `add.web-token` is legal because alice already holds
        # `web-token` through `private`, so it adjusts that entry's placement
        # in this scope rather than being the only route to it.
        perHost.deck = {
          omit.laptop-token = { };
          add.web-token = {
            mode = "0440";
          };
        };

        # `force` beats `omit` within one resolution, which is the only way the
        # third field of the scope submodule is exercised.
        perTag.portable = {
          omit.shelf-item = { };
          force.shelf-item = { };
        };
      };

      bob = {
        recipient = "age1examplebbb00000000000000000000000000000000000000000000000";
        recipientNote = "bob — example identity, decrypts nothing";

        carries = {
          shelf-item = { };
          team-wifi = { };
          deploy-key = { };
        };

        # bob's half of the delegation. acme's managers scaffold for him; the
        # other half is `organizations.acme.managers` below, and both halves
        # must name each other for a scaffold to be judged at all.
        managedBy = "acme";
      };

      carol = {
        recipient = "age1examplecarol000000000000000000000000000000000000000000000";
        recipientNote = "carol — example identity, decrypts nothing";
      };
    };

    machines = {
      deck = {
        recipient = "age1exampledeck0000000000000000000000000000000000000000000000";
        recipientNote = "deck — the age form of a host identity that does not exist";
        owner = "alice";
        tags = [ "portable" ];
      };

      rack = {
        recipient = "age1examplerack0000000000000000000000000000000000000000000000";
        recipientNote = "rack — a machine acme owns";
        owner = "acme";
      };
    };

    services.web = {
      machines = [ "deck" ];
      owner = "alice";
      user = "web";
      group = "web";
    };

    groups = {
      oncall.members = [
        "alice"
        "bob"
      ];

      # A machine, a service and a nested group as members, where oncall holds
      # people only.
      infra.members = [
        "deck"
        "web"
        "oncall"
      ];

      contractors.members = [ "carol" ];
    };

    organizations.acme = {
      custody.acme-escrow = {
        key = "age1exampleacme0000000000000000000000000000000000000000000000";
        note = "acme's escrow — held offline by the operator";
      };

      # The organization's half of the delegation: alice scaffolds for the
      # people whose `managedBy` names acme. It places no key in any audience.
      managers = [ "alice" ];
    };

    # Two groups, which is the only shape in which the non-overlap refusal
    # means anything: no one file's audience may reach both oncall and
    # contractors.
    silos.corp.groups = [
      "oncall"
      "contractors"
    ];

    # The clan this consumer bridges to, and one mapping into it. The path is
    # this example's own root, declared relative to this file, so the dendritic
    # side's own root is the same declaration against a different tree — which
    # is what the comparison's root elision exists for.
    bridge = {
      clanFlake = ./.;
      mappings.ntfy-token = {
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
    };

    # The password database, and one mapping into it. The far side carries
    # `username`, `url` and `notes` as literals; `tags` and an
    # `{ entry = …; }` source are both refused for this target, because its
    # only channel for a field is an argument vector.
    keepassxc = {
      database = "/home/alice/.keys/example.kdbx";
      group = "safix";
      mappings.grafana = {
        mode = "safix-to-keepassxc";
        safix = {
          user = "alice";
          name = "grafana-password";
        };
        kdbx = {
          path = "alice/grafana";
          fields = {
            username = "alice@example.com";
            url = "https://grafana.example";
            notes = "example mapping — no real database";
          };
        };
      };
    };

    # The `pass` store, and one mapping into it.
    # This is the one mapping that carries all four fields, and the only one
    # whose `username` is an `{ entry = …; }` source rather than a literal:
    # every field of this target crosses on standard input, so a resolved
    # secret never reaches an argument vector.
    # The path carries no `.safix-sync-state` suffix, which is the name safix
    # reserves for the companion a `two-way` mapping records its last
    # agreement in.
    pass = {
      store = "~/.password-store";
      mappings.deploy = {
        mode = "two-way";
        safix = {
          user = "alice";
          name = "deploy-token";
        };
        pass = {
          path = "alice/deploy";
          fields = {
            username = {
              entry = "deploy-username";
            };
            url = "https://deploy.example";
            notes = "example mapping";
            tags = [ "example" ];
          };
        };
      };
    };

    # The Bitwarden vault, and one mapping into it.
    # `server` is null, which names whichever server the operator's own client
    # is already configured against.
    # No `tags`: this vault has no tag concept, so a declared tag is refused at
    # evaluation rather than approximated onto a folder.
    bitwarden = {
      server = null;
      mappings.vpn = {
        mode = "safix-to-bitwarden";
        safix = {
          user = "alice";
          name = "vpn-password";
        };
        bitwarden = {
          folder = "safix";
          item = "vpn";
          fields = {
            username = "alice";
            url = "https://vpn.example";
            notes = "example mapping";
          };
        };
      };
    };

    # The 1Password mirror, and one mapping into it.
    # `account` is null, which lets `op` resolve its own.
    # The whole item crosses as one JSON object on standard input, so this
    # target carries all four fields and an `{ entry = …; }` source is
    # admissible on every one of them.
    onepassword = {
      account = null;
      mappings.registry = {
        mode = "backup";
        safix = {
          user = "alice";
          name = "registry-token";
        };
        onepassword = {
          vault = "Private";
          item = "registry";
          fields = {
            username = {
              entry = "deploy-username";
            };
            url = "https://registry.example";
            notes = "example mapping";
            tags = [ "example" ];
          };
        };
      };
    };

    # A file that rides an existing creation rule and that no declaration
    # implies, so nothing but this line puts it in policy.
    extraGovernedFiles = [ "secrets/safix/users/alice/legacy.yaml" ];
  };
}
