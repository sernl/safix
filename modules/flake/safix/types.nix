# The vocabulary of the safix registry, held apart from the flake module so that
# the resolver, the policy renderer and the checks can all type a value without
# taking a dependency on the option declarations.
#
# `entry` is what a secret consists of: the record `flake.safix.catalogue` holds
# one of per catalogue name, and the record a user's `private` holds one of per
# name they declare alone. `override` is a partial entry: every field nullOr with
# a null default, applied by taking only the non-null ones. `scope` is the
# add/omit/force triple over overrides. `grant` is one subject's statement that a
# name they hold reaches another. `profile` is the whole of what one person's
# custody is.
#
# `machine`, `group` and `silo` are the rest of the subject vocabulary. A subject
# is what can hold a key and appear in an audience: a person, a machine, or a
# group of subjects. They are three records rather than three grant surfaces,
# because one audience algebra over subjects is what keeps a second audience
# computation, a second policy renderer and a second revocation report from
# existing at all.
#
# `entry` is one type rather than a function of registry-wide defaults, so
# `flake.safix.catalogue.<n>` and `flake.safix.users.<u>.private.<n>` are the
# same submodule with the same defaults rather than two records that agree by
# inspection. Declaring an entry under `private` is therefore indistinguishable
# from declaring one in the catalogue, except in who can see it.
#
# An override is partial by construction. A scope able only to replace the whole
# record would force every per-host adjustment to restate the entry it is
# adjusting. Layer replacement at the name level is retained: within one
# resolution the last layer to name a secret supplies the whole override applied
# to that secret's base record.
{ lib }:
let
  promptKind = lib.types.enum [
    "hidden"
    "line"
    "multiline"
  ];

  prompt = lib.types.submodule (
    { name, ... }:
    {
      options = {
        type = lib.mkOption {
          type = promptKind;
          default = "hidden";
          description = ''
            How the operator's input is read. `hidden` takes one line without
            echoing it, `line` takes one line and echoes it, `multiline` takes
            every line until end of input.

            The default is `hidden`, because every prompt reachable from here
            feeds a secret's value: a prompt whose answer is not sensitive has no
            reason to be inside a generator when it could be a nix literal in the
            script.
          '';
        };
        description = lib.mkOption {
          type = lib.types.str;
          default = name;
          defaultText = lib.literalExpression "the prompt's own attribute name";
          description = "What the operator is being asked for, shown at the prompt.";
        };
      };
    }
  );

  # What `safix generate` runs to mint an entry's value. Every field is data
  # rather than a derivation, because the whole generator travels to the command
  # inside the placement map, which the command reads as JSON from a single `nix
  # eval`. A derivation cannot cross that boundary, so `runtimeInputs` names
  # nixpkgs attributes instead of holding packages; the generator check resolves
  # every one of those names against `pkgs`, so a misspelling fails a build
  # rather than an operator's rotation.
  #
  # `validation` is a check on the value rather than an invalidation hash whose
  # change re-runs the generator. The failure this registry has to prevent is a
  # bad value reaching a committed file rather than a stale one persisting:
  # `safix generate` writes into git, and a value committed is a value
  # distributed.
  #
  # `shared` lives on the entry rather than here, and it decides whether one
  # value serves many people. `share` below is that fact read back, derived.
  generatorFile = lib.types.submodule {
    options = {
      secret = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          Whether this output is encrypted, or stored in the repository in the
          clear as a public value.

          true, the default, writes the value through sops into the file the
          entry's audience picks, exactly as the entry a generator is declared
          on is written.

          false writes it to `public/safix/…/<name>/value` in plaintext, gives
          it no creation rule, and makes it readable at evaluation. That is what
          a public key, a fingerprint or a derived identifier is for: a nix
          module reads it directly rather than through a deployment-time
          indirection, which is how the keypair samples this contract was taken
          from are written.

          The default is true rather than clan's false, and the asymmetry is
          deliberate. A mistyped field that leaves a value encrypted is
          recoverable by fixing the typo; a mistyped field that publishes one is
          not — the value is in the repository's history, and only minting a new
          one revokes it.

          The public store sits under its own top-level prefix rather than
          inside `secrets/`, because a path named for secrets has to mean that
          everything under it is encrypted without qualification. That is the
          proposition every backup rule, every sync exclusion and every reviewer
          applies to it.
        '';
      };
    };
  };

  generator = lib.types.submodule {
    options = {
      script = lib.mkOption {
        type = lib.types.lines;
        example = lib.literalExpression ''"openssl rand -base64 32 > \"$out/api-token\""'';
        description = ''
          Shell fragment that produces this generator's output values.

          It runs under `bash -euo pipefail` with `runtimeInputs` prepended to
          PATH, with its working directory at a private staging root, and it
          writes one file per declared output:

          - `$out/<name>` is where each output's value goes. Every declared
            output must be present when the fragment exits, and a missing one
            refuses the whole run naming what `$out` did contain.
          - `$prompts/<name>` holds one answered prompt each, and exists only
            when this generator declares prompts.
          - `$in/<generator>/<name>` holds a dependency's plaintext, keyed by
            the generator that produces it.

          This is the interface clan's own executor implements, so a fragment
          written for either system runs under the other. What differs is that
          only the dependencies this generator declares appear under `$in`,
          where clan places every file of the dependency generator.

          An output's bytes are stored exactly as the fragment wrote them.
          Nothing is appended and nothing is stripped, so `echo` stores a
          trailing newline and `printf` does not — a convention that removed one
          would corrupt every key whose last byte is a newline. An output that
          is empty is refused and nothing is written, because an empty value is
          the state a truncated write leaves behind.

          Anything the fragment prints reaches the operator rather than a value,
          so diagnostics are free.

          The fragment runs inside a sandbox. The staging root — a mode-0700
          directory on a filesystem verified to be memory-backed, shredded
          however the run ends — is the only writable path, nixpkgs is readable
          because that is where `runtimeInputs` resolve to, and there is no
          network. A write outside `$out` fails rather than putting plaintext
          somewhere safix does not look and cannot shred. The envelope is
          bubblewrap on linux and `sandbox-exec` on darwin, which is clan's own,
          so a fragment meets the same confinement under either system's default
          executor; generation refuses rather than running unsandboxed where
          neither is available, and no flag suspends it.

          `network = true` re-shares the network and nothing else: the
          filesystem confinement stays in force. What remains the fragment
          author's to get right is what a granted connection carries — a value
          moved over it is outside what safix shreds or observes, and no
          declaration reopens the filesystem.
        '';
      };
      runtimeInputs = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ "coreutils" ];
        example = [
          "coreutils"
          "openssl"
        ];
        description = ''
          nixpkgs attribute names put on PATH while `script` runs, as strings
          rather than packages because the generator reaches the command as JSON.
          Dotted paths resolve as written, so `python3Packages.pyyaml` is one
          name. Each is resolved against nixpkgs by the generator check.
        '';
      };
      network = lib.mkOption {
        type = lib.types.bool;
        default = false;
        example = true;
        description = ''
          Whether this generator's fragments reach the network.

          false, the default, is a sandbox with no network at all: on linux the
          fragment gets a network namespace holding nothing but its own
          loopback, and on darwin the profile allows localhost alone.

          true re-shares the network and nothing else. The filesystem
          confinement is unchanged — the staging root stays the only writable
          path — so this grants an ACME client or a token generator the one
          capability it cannot work without and no others. It governs `script`
          and `validation` alike, because a validation that verifies a minted
          token against the API that issued it has the same need the script had.

          Declaring it here rather than passing it at the invocation is what
          makes it auditable: which generators may reach the network is a
          question the declarations answer at evaluation, with no runtime
          consulted and no record to keep of who passed which flag when.

          Flipping it is a change to the definition, so `safix check` reports
          every value minted before the flip until it is regenerated or the flip
          is reverted. A grant changes what a mint may do, and the ciphertext of
          a value minted without one is indistinguishable from the ciphertext of
          a value minted with it.

          What travels over a granted connection is outside what safix shreds or
          observes. That is the one part of a value's journey the runtime cannot
          contain, and it is why this is a declaration a reviewer sees rather
          than a default.
        '';
      };
      prompts = lib.mkOption {
        type = lib.types.attrsOf prompt;
        default = { };
        example = lib.literalExpression ''{ passphrase.description = "the account's login password"; }'';
        description = ''
          Values the operator supplies when this generator runs, each readable
          from the script at `$prompts/<name>`, holding exactly what the
          operator typed with nothing added and nothing removed. Nothing about a
          prompt reaches argv or the environment; the file is inside the staging
          root, and is shredded with it.

          `$prompts` exists only when a generator declares prompts, and is unset
          otherwise, so a script cannot distinguish "none declared" from "the
          directory is missing". That is clan's behaviour and is matched so no
          script comes to rely on the difference.

          A generator may have prompts, a script, and dependencies at once. That
          is the difference between this and `safix set`, which stays the
          hand-typed special case: `set` stores what the operator typed, and a
          prompt is an input to something the script computes from it.
        '';
      };
      dependencies = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        example = [ "root-ca-key" ];
        description = ''
          Other secrets of the same user whose plaintext this script reads, each
          readable at `$in/<generator>/<name>`, where `<generator>` is the entry
          the generator producing it is declared on. That keying is clan's, so a
          fragment written against `$in/openssh-ca/id_ed25519` means the same
          thing here.

          Only the dependencies named here are placed under `$in`. clan
          materializes every file of the dependency generator, which hands a
          script depending on a keypair's public half the private half as well;
          that is the one place this contract is deliberately narrower than the
          one it copies.

          The names are resolved against this user's own resolved set, and a
          dependency naming a secret they do not hold is refused at evaluation. A
          cycle is refused at evaluation too, with the cycle printed, because the
          only alternative is a command that walks an unrunnable graph and fails
          part-way through with values already committed. A generator depending
          on an output of its own run is the cycle of length one and is refused
          by name rather than as a cycle.

          Declaring a dependency also enrols this generator in the rotation of
          what it reads. `safix generate --regenerate` of a named generator
          re-runs everything downstream of it, transitively, in the same order;
          it lists the set and confirms before anything runs, and `--yes` answers
          that in advance. The alternative is a rotation that replaces an input
          and leaves every value derived from the retired one standing, which
          nothing afterwards can detect: a hash of a rotated password is
          indistinguishable from a hash of the current one.

          A dependency on another person's secret cannot be written: a name
          containing `/` is refused, and `/` is the only way to spell one, because
          the resolver forbids it inside a secret's own name. The refusal is
          structural rather than a policy this could relax — custody is
          independent, so the machine running the generator holds no identity that
          opens the other person's file and there is no plaintext for it to read.
        '';
      };
      files = lib.mkOption {
        type = lib.types.attrsOf generatorFile;
        default = { };
        example = lib.literalExpression "{ ssh-personal-pub.secret = false; }";
        description = ''
          Further outputs of the same user this one generator also writes,
          beside the entry it is declared on. A keypair is the case this exists
          for: one run mints a private half and a public half, and neither is
          meaningful without the other.

          Each name is a registry entry in its own right, so each carries its own
          `mode`, `path` and `sopsKey`; this record says which generator produces
          it and whether it is encrypted. An entry named here may not carry a
          generator of its own and may not be named by a second generator, both
          refused at evaluation, because two producers for one value is a race
          whose winner is whichever ran last.

          The entry a generator is declared on is an output too, and it is always
          encrypted. It has no `secret` slot of its own because its placement — a
          file, a key inside it, an audience — is the whole of how custody is
          expressed here, and a public value has none of the three. A generator
          that mints a public value declares it here.
        '';
      };

      share = lib.mkOption {
        type = lib.types.nullOr lib.types.bool;
        default = null;
        visible = false;
        defaultText = lib.literalMD "derived: true exactly when every entry this generator writes is `shared`";
        description = ''
          Read-only. Derived from the entries this generator writes rather than
          authored: it is true exactly when every one of them is `shared`, and a
          generator whose outputs disagree is refused at evaluation.

          Setting it is refused by name. `shared` lives on the entry, where it
          decides whether two carriers hold one value or two, and where the
          resolver, the policy renderer and the audience directory all read it.
          A second place to state the same fact is a second place for it to be
          wrong.

          It exists because a bridge to clan compares a generator's `share`
          against clan's own, and it has a second effect worth having on its own:
          a generator's outputs then always land in one audience, so one file, so
          a multi-output write is one rename.
        '';
      };
      validation = lib.mkOption {
        type = lib.types.nullOr lib.types.lines;
        default = null;
        example = lib.literalExpression ''"grep -q '^ssh-ed25519 '"'';
        description = ''
          Shell fragment that judges a candidate value before anything is
          written, or null to accept whatever the script produced.

          The candidate arrives on standard input and `$out_name` names which
          output is being judged, so one fragment covers a generator that writes
          several. It runs in the same shell with the same `runtimeInputs` as the
          script, because a validation unable to run the tool that produced the
          value could check almost nothing about it. A non-zero exit refuses the
          whole run: at that point the values are still only in the command's
          memory, so nothing has to be undone.

          It runs inside the same envelope as the script, under the same
          `network` grant. The one difference is not a choice: the staging root
          has already been shredded by the time a candidate is judged, so this
          fragment has no writable path outside the envelope's own temporary
          filesystem. The candidate is a pipe either way.
        '';
      };
      description = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "What this generator mints, shown by `safix list` and `safix check`.";
      };
    };
  };

  entry = lib.types.submodule {
    options = {
      mode = lib.mkOption {
        type = lib.types.str;
        default = "0400";
        description = "On-disk mode of the decrypted value. Matches the secret provisioner's own default, so an entry that names no mode reproduces what it would have done unaided.";
      };
      path = lib.mkOption {
        type = lib.types.nullOr (lib.types.functionTo lib.types.str);
        default = null;
        description = "Where the decrypted value is written, as a function of the configuration materializing it. null takes the provisioner's own default, which is a function of the name and so cannot collide.";
        example = lib.literalExpression ''cfg: "''${cfg.xdg.configHome}/example-app/credentials.toml"'';
      };
      owner = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = ''
          Owning account of the decrypted file, or null to leave it to the
          provisioner. Only system scope has an ownership axis; the user-scope
          materialization refuses an entry that sets this rather than dropping
          it, because a dropped ownership field reads afterwards as an ownership
          claim that was honoured.
        '';
      };
      group = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "Owning group of the decrypted file, or null to leave it to the provisioner. Refused by the user-scope materialization on the same ground as `owner`.";
      };
      shared = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = ''
          Whether the carriers of this entry hold one value between them or each
          hold their own.

          false, the default, resolves an entry two users carry to two audiences,
          so to two files and two ciphertexts, and the values in them are
          independent — minting one leaves the other standing.

          true makes the entry one value: one ciphertext, wrapped once per
          recipient, whose audience is every user whose `carries` names this
          entry.

          The value lives in the file that audience already picks, and `shared`
          adds no placement scheme of its own. An audience of one keeps that
          person's own directory; an audience of several lands in the directory
          named for its members, under one recipient rule, exactly as a
          `sharedWith` grant does.

          `carries` is the whole of the audience, because carrying is the
          declaration of custody and the file serves every host. A carrier who
          drops the entry on one host through perHost/perTag stays in the
          audience: they hold the value, they simply do not resolve it there.
          Reaching the entry only through a perHost or perTag `add` or `force` is
          refused, because a host-scoped selection puts nobody in the audience and
          would leave that person resolving a file they are not encrypted to.

          A carrier with no `recipient` is an evaluation error naming them, on the
          same ground `sharedWith` refuses a keyless recipient: there is no key to
          wrap the data key for, so no copy can be encrypted to them.

          Two mechanisms for one name is refused. An entry that is `shared` and
          also named in some owner's `flake.safix.users.<u>.sharedWith` has two
          statements of who its audience is, and they can disagree; drop the grant
          and let the recipient's `carries` say it.

          `private` cannot be shared. An entry under
          `flake.safix.users.<u>.private` has no carriers but its holder, so
          `shared = true` there is refused rather than quietly meaning nothing. A
          private entry whose name collides with a shared catalogue entry stays
          that person's own value in their own file.
        '';
      };
      sopsKey = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "Key to read inside the encrypted file. null uses the entry's own name.";
      };
      generator = lib.mkOption {
        type = lib.types.nullOr generator;
        default = null;
        description = ''
          How `safix generate` mints this secret's value, or null when the value
          comes from somewhere this repository cannot compute — a service that
          issues it, a person who chooses it, a key file that already exists. Only
          a value the holder is free to choose can have one: a generator that
          minted a fresh random string for a credential some server already knows
          would produce a value that opens nothing.
        '';
      };
      sopsFile = lib.mkOption {
        type = lib.types.nullOr lib.types.path;
        default = null;
        description = ''
          Refused if set, and declared here so that the refusal has a name to
          attach to.

          safix derives every entry's file from its audience, because an
          encrypted file has one data key wrapped once per recipient and so is
          readable in full by everyone it names. A file chosen by hand would carry
          recipients neither the audience computes nor the generated recipient
          policy writes a rule for, so the value would be encrypted to people no
          declaration names and no check can see. Widen the audience with
          `flake.safix.users.<u>.sharedWith` instead.

          Kept in the vocabulary rather than deleted: a field the resolver refuses
          by name tells the author where placement comes from, whereas an
          unknown-option error tells them only that they were wrong.
        '';
      };
    };
  };

  override = lib.types.submodule {
    options = {
      mode = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "Override the on-disk mode in this scope. null leaves the entry's mode standing.";
      };
      path = lib.mkOption {
        type = lib.types.nullOr (lib.types.functionTo lib.types.str);
        default = null;
        description = "Override the on-disk path in this scope, as a function of the configuration materializing it. null leaves the entry's path standing.";
      };
    };
  };

  scope = lib.types.submodule {
    options = {
      add = lib.mkOption {
        type = lib.types.attrsOf override;
        default = { };
        description = "Secrets carried in this scope.";
      };
      omit = lib.mkOption {
        type = lib.types.attrsOf override;
        default = { };
        description = "Secrets dropped in this scope (only the keys are used).";
      };
      force = lib.mkOption {
        type = lib.types.attrsOf override;
        default = { };
        description = "Secrets re-added after omit; beats omit within the same resolution.";
      };
    };
  };

  # A grant carries no fields. It is the owner's statement that a name they hold
  # is to reach one other person, and the recipient's copy is the owner's record
  # unchanged; the recipient's own perHost/perTag is where a recipient-side
  # adjustment belongs. Kept as a submodule rather than a bare attrset so a
  # mistyped field fails at evaluation instead of being carried silently, and so
  # that a later field lands here without changing the shape of the option.
  grant = lib.types.submodule { options = { }; };

  recoveryRecipient = lib.types.submodule {
    options = {
      key = lib.mkOption {
        type = lib.types.str;
        description = "The age public key, as it appears in the generated recipient policy.";
      };
      note = lib.mkOption {
        type = lib.types.nullOr lib.types.lines;
        default = null;
        description = "Prose emitted above this key in the generated policy's keys block.";
      };
    };
  };

  # A machine, as a subject. It carries no `carries`, no `private` and no
  # `sharedWith`: everything a machine holds arrives through a grant aimed at it,
  # so `flake.safix.users` stays the only place a secret is declared and a
  # machine is something an audience can name rather than a second authoring
  # surface for custody.
  machine = lib.types.submodule {
    options = {
      recipient = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "age1examplemachine00000000000000000000000000000000000000000000";
        description = ''
          The age public key this machine's system scope already decrypts with:
          `ssh-to-age` of the host's ed25519 key, which is the derivation clan
          uses for its own machine recipients and the key sops-nix's NixOS module
          reaches by defaulting `sops.age.sshKeyPaths` to
          `config.services.openssh.hostKeys`. Declaring a machine therefore mints
          no second identity and adds no enrollment step; it names a key the host
          already holds.

          A recipient, never an identity. Nothing here can decrypt anything, and
          no private half is named, escrowed or deployed by this field.

          The hardware-recipient refusal `safix adduser` applies to a person does
          not transfer to a machine, and the reason is the sentence that refusal
          rests on: a card needs a PIN and a touch once per file, while an
          activation decrypts non-interactively, so a person whose only recipient
          is a card cannot activate at all. A host identity decrypts
          non-interactively by nature — that is what it is — so the sentence is
          not true of one and the refusal has nothing to say about it.

          Null is inert rather than an error: a machine that records no recipient
          and that no audience names changes nothing. A grant naming one is
          refused, because there is no key to wrap a data key for.
        '';
      };

      recipientNote = lib.mkOption {
        type = lib.types.nullOr lib.types.lines;
        default = null;
        description = ''
          Prose emitted above this machine's key in the generated policy's keys
          block, where the key earns an anchor at all. What converts to it and
          which host holds the private half belongs here rather than in the
          generated file, which no one may edit by hand.
        '';
      };

      owner = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "ana";
        description = ''
          The `flake.safix.users` entry this machine belongs to, or null.

          A record, and in this model nothing more. It confers no powers: an
          owner does not thereby read the machine's entries or manage its users,
          because a record that silently granted either would be the escrowed
          custody safix already prints a warning about, arrived at by accident
          rather than declared.

          What it does is give a grant somewhere to resolve through. An owner
          named here is what makes `sharedWith."ownerOf.<machine>"` mean
          something, and a change of owner then re-wraps the grant toward the new
          one rather than leaving it pointed at whoever held the host when it was
          written.
        '';
      };

      tags = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        example = [ "laptop" ];
        description = ''
          The tags this machine carries, against which
          `flake.safix.users.<u>.perTag` adds, omits and forces entries.

          The one tag vocabulary safix holds. A profile still hands its own over
          through `safix.tags` and nothing here overrides that; a profile or
          system naming `safix.machine` takes these instead, which is what makes
          a hundred hosts declarable as tags on machines rather than as a hundred
          `perHost` blocks.
        '';
      };
    };
  };

  # A group, as a subject whose recipients are its members'. Membership lives in
  # one place and every audience naming the group follows it, which is the whole
  # difference between a group audience and an enumerated one.
  group = lib.types.submodule {
    options.members = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [
        "ana"
        "deck"
        "oncall"
      ];
      description = ''
        The subjects this group consists of: people, machines, or other groups.

        An audience naming this group is encrypted to the expanded membership's
        keys, so adding a member is a re-wrap of every file the group's audience
        names and removing one is the revocation it is — reported as such, with
        rotation named as the remedy, because a member who has held a file's data
        key has read what it holds and no re-wrap unreads it.

        A group naming itself, directly or through other groups, is refused at
        evaluation with the participants named: a membership that cannot be
        expanded is not a membership.
      '';
    };
  };

  # A silo set: named groups that no one file's audience may span.
  silo = lib.types.submodule {
    options.groups = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [
        "contractors"
        "staff"
      ];
      description = ''
        The `flake.safix.groups` this set holds mutually exclusive.

        Evaluation refuses any file whose audience would reach subjects of two of
        them, naming the file, the subjects and this declaration. A silo enforced
        where audiences are computed is a file that cannot exist; one enforced at
        read time is a policy hoping nobody misconfigured a file.

        Sets rather than pairs, so a declaration over n groups is one line rather
        than n² of them, and a group named in two sets is itself refused — which
        is what keeps the constraint linear instead of transitively closing over
        every set a group appears in.

        Deliberately not transitive over ownership. A person may own machines in
        two silos, because the operator administering both sides is the normal
        case; what is refused is one file readable from both, never a person's
        existence in both worlds.
      '';
    };
  };

  profile = lib.types.submodule {
    options = {
      recipient = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        example = "age1examplerecpent00000000000000000000000000000000000000000000";
        description = ''
          The age public key this person's secrets are encrypted to. A recipient,
          never an identity: nothing here can decrypt anything, and no private key
          is named, escrowed or deployed by this field. Setting it is what gives
          the person an anchor and a creation rule in the generated recipient
          policy; there is no second place to register them, and no catch-all rule
          to fall through to if this is left null.

          Owner-declared sharing needs it. A secret shared to this person is
          encrypted to this recipient in its audience's own file, so a grant aimed
          at a user who records none is refused by name rather than resolving into
          a provisioner entry no key on the machine can open. Owning a secret
          needs it for the same reason.

          Revocation is not retroactive. Clearing this field, or dropping the
          grant that reached this person, narrows the audience of every affected
          file and stops future encryptions reaching them. It takes nothing back:
          they have already read the values they could open, and a value once read
          stays read. Only minting a new value revokes, and that is an operator
          ceremony, never something a rebuild performs.

          Nor can rotation be automatic on revoke. An evaluation sees only the
          audience that is declared, never the audience that used to be, so
          nothing here can detect that someone was removed. `safix fix` re-wraps
          each governed file's data key to the audience this registry now
          declares, which aligns the ciphertext with the policy and is explicitly
          not revocation.
        '';
      };
      recipientNote = lib.mkOption {
        type = lib.types.nullOr lib.types.lines;
        default = null;
        description = ''
          Prose emitted above this person's key in the generated policy's keys
          block. What the key is — which file or device holds the private half,
          what converts to it, what decrypts with it — belongs here rather than in
          the generated file, which no one may edit by hand.
        '';
      };
      recoveryRecipients = lib.mkOption {
        type = lib.types.attrsOf recoveryRecipient;
        default = { };
        example = lib.literalExpression ''{ master = { key = "age1..."; }; }'';
        description = ''
          Further recipients belonging to this same person's custody, keyed by the
          anchor the generated policy defines them as. Every file whose audience
          includes this person is encrypted to these as well as to `recipient`.

          This is where escrowed custody and independent custody differ, and the
          difference is a property of the person rather than of a hand-written
          rule. Someone who holds an offline master identity lists it here and can
          therefore open their own files after losing the activation key. Listing
          an operator-held identity in a second person's entry is escrowed custody
          instead, and buys recoverability at the price of that operator reading
          everything that person holds.

          Leaving it empty for a second person is what keeps their custody
          independent, and it has a cost that no later edit undoes: with only their
          own recipient, losing that key makes their files unopenable by every
          party including the operator, because adding a recipient to an existing
          file requires decrypting it first. The mitigation that keeps
          independence is a second recipient the person themselves holds.
        '';
      };
      carries = lib.mkOption {
        type = lib.types.attrsOf override;
        default = { };
        description = "The `flake.safix.catalogue` entries this user carries on every host (name -> override).";
      };
      private = lib.mkOption {
        type = lib.types.attrsOf entry;
        default = { };
        description = "Secrets belonging to this user alone, declared here rather than in the catalogue. Declaring an entry is itself selecting it: there is no second selection step.";
      };
      sharedWith = lib.mkOption {
        type = lib.types.attrsOf (lib.types.attrsOf grant);
        default = { };
        example = lib.literalExpression ''
          {
            bo.wifi-psk = { };
            oncall.deploy-key = { };
            "ownerOf.deck".wifi-psk = { };
          }
        '';
        description = ''
          Outbound sharing, declared by the owner: <subject>.<name> makes this
          user's <name> resolve into that subject's secret set as well, at that
          subject's own path and with this user's declared mode unless a
          recipient adjusts it through their own perHost/perTag.

          A subject is a person, a machine, a group of subjects, or the owner of a
          machine written `ownerOf.<machine>` — one grant surface over all four,
          because a second one would be a second audience computation to keep in
          step with this one. A group grant reaches every expanded member's set
          and follows the membership; an `ownerOf` grant reaches whoever the
          machine's `owner` names and follows that record. `.` is outside the name
          alphabet, so `ownerOf.<machine>` can never collide with a declared
          subject's name.

          A grant widens the secret's audience, and the audience picks the file.
          An encrypted file has one data key wrapped once per recipient, so
          everyone a file names reads every value in it; sharing therefore moves
          the secret into a file shared by exactly that audience rather than
          adding a recipient to either person's own file, which would hand them
          everything that person holds.

          Revocation is not retroactive. Removing a grant narrows the audience and
          stops future encryptions reaching that person. It does not take back
          what they read: the value is theirs already, and only minting a new one
          revokes it.

          Rotation cannot be automatic on revoke, and nothing here pretends
          otherwise. Nix evaluation is stateless — it sees only the audience that
          is declared, and can never know that the audience used to include
          someone — so no rebuild can detect a removal, let alone rotate on it.
          Rotation is an explicit operator ceremony: mint a new value with `sops`
          against the narrowed audience's file, or `safix generate --regenerate
          <user> <name>` where the value has a generator. `safix fix` re-wraps
          existing files to the audiences now declared, which aligns ciphertext
          with policy and is not revocation.
        '';
      };
      perHost = lib.mkOption {
        type = lib.types.attrsOf scope;
        default = { };
        description = "Per-hostname add/omit/force adjustments to the union of carries, private and secrets shared to this user.";
      };
      perTag = lib.mkOption {
        type = lib.types.attrsOf scope;
        default = { };
        description = "Per-tag add/omit/force adjustments to the union of carries, private and secrets shared to this user.";
      };
    };
  };
in
{
  inherit
    entry
    generator
    override
    scope
    grant
    profile
    machine
    group
    silo
    ;
}
