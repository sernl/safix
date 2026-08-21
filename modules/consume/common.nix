# The half of the consumption namespace both scopes share.
#
# A plain function rather than a module, because the two scopes disagree about
# exactly three things — where the person defaults from, where the host defaults
# from, and which scope the resolution materializes at — and a shared module
# would have to read one of them out of the other's option tree to know which it
# was in.
#
# Everything declared here is selection or decryption: which resolved set arrives
# in this profile, and how this machine opens it. Nothing here declares a secret,
# a recipient, a grant or an audience, and that is structural rather than a
# convention. An audience is a function of every user's declarations at once —
# one person's `sharedWith` widens the file another person reads — and the
# recipient policy is a single repository-global file that the sops CLI reads off
# disk. A machine's module system sees one machine, so it could compute neither.
# Custody therefore stays at `flake.safix.*` and these modules consume what it
# resolved.
{ lib, scope }:
let
  inherit (lib) mkOption types;

  scopeNoun = if scope == "system" then "system configuration" else "home-manager profile";

  # Named rather than inline in the `throw` for the same reason as
  # `violationMessage` below: what a consumer reads here is the whole value of
  # the refusal, and a message assembled inside a `throw` is one no check can
  # hold.
  missingLibMessage = ''
    safix: safix.flake was set to a value carrying no `safix.lib`.

    safix.lib is published by flake-parts as the `safix` output of a flake that
    imports inputs.safix.flakeModules.default. A flake that does not import it
    has no safix outputs at all, which is the usual cause; passing something
    other than a flake is the other.

    Set safix.flake to your own flake — `inputs.self` from inside it — or set
    safix.lib directly to the projection.
  '';

  # A profile that names a person or a host and is bound to nothing. Without
  # this the state is silent rather than wrong: `safix.lib` null makes every
  # assertion below vacuously true and `resolvedFor` return the empty set, so
  # `enable` defaults to false and the profile establishes nothing, reports
  # nothing, and looks exactly like a profile whose person holds nothing here.
  flakelessMessage = ''
    safix: this ${scopeNoun} names a person or a host and is bound to no
    declarations, so it resolves nothing and would establish nothing.

    safix.lib is null. It defaults from safix.flake, which is the one thing a
    module cannot derive: a ${scopeNoun} receives `config`, `lib`, `pkgs` and
    whatever its evaluator put in `extraSpecialArgs` or `specialArgs`, and
    requiring a particular name there would make every consumer's evaluation
    seam part of safix's interface. So it is named once:

      safix.flake = inputs.self;

    Set safix.lib directly instead where the projection reaches this
    ${scopeNoun} by another route.

    Importing the module and setting nothing at all stays a no-op. This fires
    only because a person or a host was named, which asks for a resolution
    there is nothing to resolve against.
  '';

  # Built by a named function rather than inline in the `throw` so that a check
  # can read what a consumer would see. `builtins.tryEval` reports that a throw
  # fired and never what it said, so a message assembled inside one is a message
  # nothing can hold.
  violationMessage =
    cfg:
    ''
      safix: the declarations this ${scopeNoun} is bound to do not resolve.

      safix.user = ${toString cfg.user}, safix.machine = ${toString (cfg.machine or null)}, safix.hostname = ${toString cfg.hostname}, scope = ${scope}

    ''
    + lib.concatMapStrings (v: "  - ${v}\n") cfg.lib.violations
    + ''

      Every one of these is a statement about flake.safix.* in the flake
      safix.flake names, and none of them is repairable from here. They are
      reported together, and by safix, because the resolver would otherwise
      raise the first of them from inside the secret provisioner's own
      evaluation, where the trace names the provisioner and not the declaration
      that broke.
    '';

  # User scope only, and named here rather than in ./home.nix for the same
  # reason as the two above: a check can read a string off this file without
  # evaluating a module, and a message assembled inside a `throw` is one nothing
  # can hold. It names the user scope literally because the system scope's
  # identity story is different twice over and has its own message below: there
  # an identity is usually derivable from the host's ssh keys, and the refusal
  # exists because nothing else would fire at all.
  noIdentityMessage =
    { cfg, resolved }:
    let
      # Whichever subject this profile serves, and where the profile is. A machine
      # profile resolves without a hostname, because a machine is the host.
      subject = if cfg.machine or null != null then cfg.machine else cfg.user;
      where = if cfg.hostname or null == null then subject else cfg.hostname;
    in
    ''
      safix: ${toString (builtins.length (builtins.attrNames resolved))} secret(s) resolve for ${subject} on ${where}, and this
      home-manager profile names no decryption identity.

      Name one:

        safix.identity.sshKeyPaths = [ "/home/${subject}/.ssh/id_ed25519" ];

      or set safix.identity.keyFile to an age key file this machine holds.

      Neither has a default at user scope, and that is not an omission.
      sops-nix's home-manager module has no identity of its own to fall back on:
      its NixOS module defaults sops.age.sshKeyPaths to the ed25519 keys of
      config.services.openssh.hostKeys, and a person is not a host, so there is
      no per-user equivalent to take. safix cannot supply one either — where a
      person's key lives is a property of how their machine was provisioned, and
      a keyFile that is set and turns out to be absent is fatal inside
      sops-install-secrets rather than skipped, so a guessed default would abort
      activation on every machine that lacks the path.

      Set safix.enable = false to keep the declarations and suppress their
      arrival here.

      safix refuses first so that the refusal names these two options. The next
      thing to refuse would be sops-nix's own key-source assertion, whose
      condition tests four of its options, whose message names three, and whose
      block's two assertions together name four distinct options — none of them
      safix's.
    '';
  # Whether a consumer wrote a definition for an option, rather than the
  # option's own default standing in for one.
  #
  # `option.isDefined` cannot answer this and reads true for every option here:
  # the module system injects `mkOptionDefault opt.default` into the definition
  # list before merging, so a declared default is a definition by the time
  # `isDefined` is computed. The priority does answer it — that injected
  # definition carries `mkOptionDefault`'s priority, and anything a consumer
  # writes is numerically lower, which is what `highestPrio` reports. The
  # constant is read off `lib` rather than written as 1500 so that the two
  # cannot drift apart.
  optionDefaultPriority = (lib.mkOptionDefault null).priority;

  wasSet = option: option.highestPrio < optionDefaultPriority;
  # System scope only, beside `noIdentityMessage` for the same reason: a check
  # can read the string without evaluating a module. The system scope has its
  # own message because its identity story differs twice over — an identity is
  # usually derivable from the host's ssh keys, and the refusal that would
  # otherwise fire does not: the provisioner's key-source assertion sits
  # inside `mkIf (cfg.secrets != { })` (`modules/sops/default.nix:432-441`),
  # and safix now leaves that option empty, so without safix's own refusal
  # nothing refuses at all and the configuration evaluates green while
  # installing nothing decryptable.
  noSystemIdentityMessage =
    { cfg, resolved }:
    let
      subject = if cfg.machine or null != null then cfg.machine else cfg.user;
    in
    ''
      safix: ${toString (builtins.length (builtins.attrNames resolved))} secret(s) resolve for ${subject} on this system
      configuration, and no decryption identity is configured or derivable.

      Name one:

        safix.identity.sshKeyPaths = [ "/etc/ssh/ssh_host_ed25519_key" ];

      or set safix.identity.keyFile to an age key file this machine holds — or
      leave safix.identity.deriveHostKeys on and let openssh manage host keys
      outside safix's own store, which is what the derivation reads.

      safix refuses first because nothing else will: the secret provisioner's
      key-source assertion is conditional on its own secrets option, which
      safix leaves empty, so without this refusal the configuration evaluates
      green and installs nothing decryptable.
    '';

  # The two refusals safix's manifest builder copies from the provisioner's
  # (`manifest-for.nix:11-28`), named here for the reason every message above
  # is: a check can read what a consumer would see, where a string assembled
  # inside a `throw` is one nothing can hold. Without this copy nothing
  # refuses at all — the type these entries pass through carries neither
  # refusal, and the builder that does lives in the provisioner's tree, which
  # safix no longer calls.
  sopsFileMissingMessage =
    { name, file }: "safix: cannot find '${file}', the sops file of resolved entry '${name}'";

  sopsFileOutsideStoreMessage =
    { name, file }:
    "safix: '${file}', the sops file of resolved entry '${name}', is not in the Nix store. Add it to the Nix store or set sops.validateSopsFiles to false";

  # System scope only: the resolved set typed by the secret provisioner's own
  # entry type, read off the provisioner's option declaration in the same
  # evaluation rather than restated, so every entry carries the provisioner's
  # mode, owner, group, uid and gid coercions, its `sopsFile` default, and its
  # `sopsFileHash` default, which forces `builtins.hashFile "sha256"` under
  # `sops.validateSopsFiles`.
  #
  # The type carries exactly that and no more, settled by evaluating it: an
  # entry whose `sopsFile` lies outside the nix store passes this type
  # unchanged, because the provisioner's `pathNotInStore` is declared at
  # `modules/sops/default.nix:19-25` and applied at one site, `sops.age.keyFile`
  # (`:338`), never to a secret entry, whose `sopsFile` is plain
  # `lib.types.path` (`:137`) — and the store-membership refusal lives at
  # `manifest-for.nix:19-23`, inside the builder safix does not call. What
  # refuses instead is the copy of that block in `./installer.nix`.
  installedOptionFor =
    options:
    mkOption {
      type = options.sops.secrets.type;
      default = { };
      description = ''
        What safix resolved for this system configuration, typed by the secret
        provisioner's own entry type and read back by safix's installer. This
        is where the resolved set arrives: safix builds its installer manifest
        from this option and leaves the provisioner's `sops.secrets` empty.
      '';
    };
in
{
  inherit
    missingLibMessage
    flakelessMessage
    violationMessage
    noIdentityMessage
    noSystemIdentityMessage
    sopsFileMissingMessage
    sopsFileOutsideStoreMessage
    installedOptionFor
    wasSet
    ;

  # The options that read the same in either scope. The four arguments are what
  # the scopes disagree about, passed in rather than branched on, so a scope that
  # cannot derive a default says so in its own file.
  sharedOptions =
    {
      cfg,
      userDefault,
      userDefaultText,
      hostnameDefault,
      hostnameDefaultText,
    }:
    {
      enable = mkOption {
        type = types.bool;
        default = cfg.secrets != { };
        defaultText = lib.literalExpression "config.safix.secrets != { }";
        description = ''
          Whether this ${scopeNoun} establishes the secrets safix resolved for it.

          The default is whether anything resolved, and the whole of this
          module's configuration is conditional on it, so a profile whose person
          holds nothing on this host defines nothing at all: no secrets, no
          identity, no activation entry and no unit. Turning it off by hand is
          the way to keep the declarations and suppress their arrival here.
        '';
      };

      flake = mkOption {
        type = types.nullOr types.raw;
        default = null;
        example = lib.literalExpression "inputs.self";
        description = ''
          The consumer's own flake — the one whose `flake.safix.catalogue` and
          `flake.safix.users` hold the declarations this profile serves.

          safix reads `${"\${flake}"}.safix.lib` from it, which flake-parts publishes as
          a flake output for any flake importing `inputs.safix.flakeModules.default`.

          This is the one thing a module cannot derive. A ${scopeNoun} receives
          `config`, `lib`, `pkgs` and whatever its evaluator chose to put in
          `extraSpecialArgs` or `specialArgs`, and requiring a particular name
          there would make every consumer's evaluation seam part of safix's
          interface — the same assumption safix refuses to make about a
          consumer's user registry. So it is named once, here.

          Leave it null and set `safix.lib` instead where the projection reaches
          this profile by some other route.
        '';
      };

      lib = mkOption {
        type = types.nullOr types.raw;
        default = if cfg.flake == null then null else cfg.flake.safix.lib or (throw missingLibMessage);
        defaultText = lib.literalExpression "config.safix.flake.safix.lib";
        description = ''
          The resolver projection safix derives from the declarations: the
          resolution helpers, the audience and placement maps, and the violation
          list. Defaults from `safix.flake` and is settable directly for a
          consumer whose flake reaches this profile by another route.

          Read-only in substance — every value in it is a projection of what was
          declared at `flake.safix.*` — but not declared `readOnly`, because
          supplying it is how a profile is bound at all.
        '';
      };

      user = mkOption {
        type = types.nullOr types.str;
        default = if cfg.machine == null then userDefault else null;
        defaultText = lib.literalMD "${userDefaultText.text or "null"}, or null where `safix.machine` is set";
        example = "jane";
        description = ''
          Which `flake.safix.users` entry this ${scopeNoun} serves.

          This selects; it does not declare. The named entry must already exist
          in the consumer's `flake.safix.users`, and everything about who may
          read what — the recipient, the recovery identities, the grants — is
          stated there, where every user is visible at once.

          Defaults to null where `safix.machine` is set, so a profile that serves
          a machine names one option rather than two. Defining both is refused: a
          profile serves one subject, and a resolution of two would be two
          subjects' custody in one set of files.
        '';
      };

      machine = mkOption {
        type = types.nullOr types.str;
        default = null;
        example = "workstation";
        description = ''
          Which `flake.safix.machines` entry this ${scopeNoun} serves, instead of
          a person.

          A machine holds what people have granted it, and that is the whole of
          what arrives here: a machine declares no secrets of its own. What it
          decrypts with is the identity it already had — at system scope the
          provisioner defaults to the host's ed25519 keys, which is the same key
          `flake.safix.machines.<m>.recipient` is the age form of, so a machine
          entry needs no identity named here.

          It has no default. safix holds no host inventory and derives no machine
          from a hostname, because a hostname is not an identity: two hosts can
          share one and a declaration cannot.

          Selection is custody and custody has no scope, so a machine resolves
          the same set in a ${scopeNoun} as anywhere else. Nothing about this
          requires NixOS: the entries are files with recipients, and a standalone
          home-manager profile on any distribution that names the machine
          resolves them identically.
        '';
      };

      hostname = mkOption {
        type = types.nullOr types.str;
        default = hostnameDefault;
        defaultText = hostnameDefaultText;
        example = "workstation";
        description = ''
          Which host this ${scopeNoun} resolves on.

          The resolution is host-scoped: `flake.safix.users.<u>.perHost` adds,
          omits and forces entries by hostname, which is the reason a hostname is
          an argument at all. A profile that names the wrong one resolves a
          different set, silently and correctly.
        '';
      };

      tags = mkOption {
        type = types.listOf types.str;
        default = if cfg.machine == null then [ ] else cfg.lib.subjects.machines.${cfg.machine}.tags;
        defaultText = lib.literalExpression "the declared tags of config.safix.machine, or [ ]";
        example = [ "laptop" ];
        description = ''
          The tags this host carries, against which
          `flake.safix.users.<u>.perTag` adds, omits and forces entries.

          safix derives no tag from anything but a declaration: a tag vocabulary
          is the consumer's, and this is where theirs is handed over. A profile
          that names `safix.machine` is the one case where the declarations do
          hold them, and they default from there — which is what makes a hundred
          hosts declarable as tags on machines rather than as a hundred `perHost`
          blocks.
        '';
      };

      secrets = mkOption {
        type = types.attrsOf types.raw;
        readOnly = true;
        description = ''
          What safix resolved for this profile, in the shape the secret
          provisioner's own option tree takes. Read-only: it is a projection of
          `flake.safix.*` for this person on this host at this scope.

          Empty whenever the profile is unbound — no `safix.lib`, no
          `safix.user` or no `safix.hostname` — which is what lets the
          assertions below report the mistake instead of a resolution throwing
          before they are reached.
        '';
      };

      identity = {
        keyFile = mkOption {
          type = types.nullOr types.path;
          default = null;
          example = "/var/lib/sops/age/keys.txt";
          description = ''
            An age key file this machine decrypts with, or null.

            Null is the default because the two identity sources fail
            differently. `sops-install-secrets` treats a set-but-unreadable key
            file as fatal — "cannot read keyfile '%s'", inside `installSecrets`
            — whereas a missing ssh key path is written to stderr and skipped.
            A non-null default would therefore abort activation on every machine
            that happens to lack the path, while an unset one costs nothing.

            Set on the provisioner at normal priority whenever this module is
            enabled, including when it is null. That is deliberate: a `mkDefault`
            elsewhere in a consumer's tree loses to it, so the null cannot be
            silently replaced by a path that re-arms the abort, and a plain
            definition elsewhere conflicts loudly instead of one of them winning
            by accident.
          '';
        };

        sshKeyPaths = mkOption {
          type = types.listOf types.str;
          default = [ ];
          example = [ "/home/jane/.ssh/agenix" ];
          description = ''
            ssh private keys this machine decrypts with. `sops-install-secrets`
            runs each through ssh-to-age, so the age recipient the recipient
            policy names is the converted public half.

            Each path is individually skipped with a line to stderr when absent,
            so these are load-bearing only collectively, and only while they are
            the sole identity source.

            Defined on the provisioner only when non-empty, so that a scope whose
            provisioner derives its own default — the system scope, which takes
            the host's ed25519 keys — keeps it.
          '';
        };
      };
    };

  # The materialization, or an empty set for a profile that is not bound.
  #
  # Empty rather than an error is what makes the assertions below reachable:
  # `enable` defaults to whether this is non-empty, so a resolution that threw
  # here would pre-empt every message that names the option actually at fault.
  #
  # A profile serving a machine needs no hostname. A machine holds only what was
  # granted to it and has no per-host layer to select through — it is the host —
  # so requiring one would make a standalone profile that names a machine resolve
  # nothing for want of a value that decides nothing.
  resolvedFor =
    { cfg, target }:
    let
      unbound = cfg.lib == null || (cfg.user == null && cfg.machine == null);
      unaddressed = cfg.machine == null && cfg.hostname == null;
    in
    if unbound || unaddressed || (cfg.user != null && cfg.machine != null) then
      { }
    else if cfg.lib.violations != [ ] then
      throw (violationMessage cfg)
    else
      cfg.lib.materialize {
        inherit (cfg)
          user
          machine
          hostname
          tags
          ;
        inherit scope;
      } target;

  # The wiring mistakes that are cheap to name exactly.
  #
  # `configured` is whether the consumer wrote a definition for `safix.user` or
  # `safix.hostname` — the module's own `options`, not `cfg`, since both carry
  # defaults a consumer never wrote and the user scope's is never null. It is
  # the only signal that separates the three states a null `safix.lib` covers:
  # imported and unconfigured, which must stay a no-op; configured and bound,
  # which resolves; and configured and flakeless, which is the one below. Each
  # scope computes it in its own file, so this one needs no module system of its
  # own.
  assertionsFor =
    { cfg, configured }:
    [
      {
        assertion = cfg.lib != null || !configured;
        message = flakelessMessage;
      }
      {
        assertion = cfg.lib == null || cfg.user != null || cfg.machine != null;
        message = ''
          safix: this ${scopeNoun} is bound to a set of declarations but names no subject.

          Set safix.user to the flake.safix.users entry this profile serves, or
          safix.machine to the flake.safix.machines entry it serves.
        '';
      }
      {
        assertion = cfg.user == null || cfg.machine == null;
        message = ''
          safix: this ${scopeNoun} names both safix.user = ${toString cfg.user} and
          safix.machine = ${toString cfg.machine}.

          A profile serves one subject. Resolving both would put two subjects'
          entries in one set of files, and which of the two a file belonged to
          would be a question the declarations no longer answer.

          Naming safix.machine alone is enough: safix.user then defaults to null
          rather than to this profile's own username, so the two are alternatives
          without a second option to unset.
        '';
      }
      {
        assertion = cfg.lib == null || cfg.machine != null || cfg.hostname != null;
        message = ''
          safix: this ${scopeNoun} is bound to a set of declarations but names no host.

          A person's resolution is host-scoped — flake.safix.users.<u>.perHost and
          .perTag select by host — so there is no set to resolve without one. Set
          safix.hostname.

          A profile serving safix.machine needs none: a machine holds what was
          granted to it and has no per-host layer to select through.
        '';
      }
    ];
}
