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
      reported together, and at once, because the resolver would otherwise
      raise the first of them from inside the manifest build, where the trace
      names a derivation and not the declaration that broke.
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
      A person is not a host: the system scope derives an identity from the
      host's ed25519 ssh keys, and there is no per-user equivalent to derive,
      because where a person's key lives is a property of how their machine was
      provisioned rather than of anything safix can see. Nor would a guess be
      free: safix's own installer treats a keyFile that is set and turns out to
      be unreadable as fatal, naming the path, where a missing ssh key path is
      written to stderr and skipped — so a non-null default would abort
      activation on every machine that lacks the path.

      Set safix.enable = false to keep the declarations and suppress their
      arrival here.

      safix refuses here, at evaluation, so that the refusal names these two
      options while nothing has been applied. The installer's own check is
      later and narrower: it runs at activation and reports presence and
      readability of the paths it was given, which is no help to a profile that
      named no path at all.
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
  # usually derivable from the host's ssh keys, and nothing else refuses at
  # all: safix is the only installer on this path, and its own installer's
  # check runs at activation over the paths it was given, so a configuration
  # that named none would otherwise evaluate green and install nothing
  # decryptable.
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

      safix refuses at evaluation because nothing later will refuse usefully:
      the installer's own check reports presence and readability of the paths
      it was handed, and a configuration that named none hands it nothing to
      report on, so the configuration would evaluate green and install nothing
      decryptable.
    '';

  # The two refusals safix's manifest builder carries, named here for the
  # reason every message above is: a check can read what a consumer would see,
  # where a string assembled inside a `throw` is one nothing can hold.
  # They are safix's own, declared here because safix's own entry type carries
  # neither of them: the type coerces and defaults fields and asks nothing
  # about the filesystem, so a document that does not exist or lies outside the
  # store passes it unchanged and is refused by the manifest builder in
  # `./installer.nix` instead, under `safix.installer.validate`.
  sopsFileMissingMessage =
    { name, file }: "safix: cannot find '${file}', the sops file of resolved entry '${name}'";

  sopsFileOutsideStoreMessage =
    { name, file }:
    "safix: '${file}', the sops file of resolved entry '${name}', is not in the Nix store. Add it to the Nix store or set safix.installer.validate to false";

  # The refusal a consumer meets where safix's own binary is not in their
  # package set. Named here for the same reason as every message above, and it
  # is a refusal rather than a default because there is nothing honest to
  # default to: this module imports nothing, so it holds no input to read a
  # package out of, and a package it guessed from a name in the consumer's tree
  # would make that tree's layout part of safix's interface.
  installerPackageMessage = ''
    safix: safix.installer.package is unset and `pkgs.safix` does not exist in
    this configuration's package set, so there is no `safix install` to run at
    activation.

    Set it to the build this tree already has:

      safix.installer.package = inputs.safix.packages.''${pkgs.stdenv.hostPlatform.system}.safix;

    or add safix to `nixpkgs.overlays` and leave this option alone. Under a
    cross build set safix.installer.validationPackage as well: the manifest's
    own check runs on the build platform, where the activation runs on the
    host's.
  '';

  # The type every resolved entry at system scope passes through: safix's own,
  # declared here rather than read off another framework's option tree in the
  # same evaluation, which is what made that framework's module a required
  # import for a module whose contract is that it imports nothing.
  #
  # It carries exactly the twelve fields the installer manifest carries, and
  # the manifest schema in `crates/safix-core/src/install.rs` refuses a
  # thirteenth, so a field added here without the schema moving is a failing
  # check rather than a field the reader drops.
  #
  # `sopsFileHash` is deliberately absent. The provisioner's type defaulted it
  # to `builtins.hashFile "sha256" sopsFile` under its own validation switch,
  # which is what made editing a ciphertext file change the manifest
  # derivation. safix keeps that behaviour and moves it: `./installer.nix`
  # emits one `manifestInputHash` over every distinct document under
  # `safix.installer.validate`, which is the same guarantee for one
  # `hashFile` per document rather than one per entry, and the installer
  # ignores the field entirely.
  #
  # `cfg` is the system-scope `config.safix`, and the only thing read off it is
  # `cfg.installer.symlinkPath`, inside `path`'s default, so a check can call
  # this with a stub carrying that one field.
  secretEntryType =
    { cfg }:
    types.submodule (
      { config, ... }:
      {
        options = {
          name = mkOption {
            type = types.str;
            default = config._module.args.name;
            defaultText = lib.literalMD "the attribute name";
            description = ''
              The entry's name: the attribute this entry was declared under, and
              the file name it arrives under inside safix's store. The installer
              reads it to name the file it writes and to diff this generation
              against the previous one.
            '';
          };

          key = mkOption {
            type = types.str;
            default = config.name;
            defaultText = lib.literalExpression "config.name";
            description = ''
              Which key inside the sops document holds this entry's value, as a
              `/`-nested path. The installer resolves it against the decrypted
              document and refuses, naming entry, document and key, when any
              segment is absent.
            '';
          };

          path = mkOption {
            type = types.str;
            default = "${cfg.installer.symlinkPath}/${config.name}";
            defaultText = lib.literalExpression "\"\${config.safix.installer.symlinkPath}/\${name}\"";
            description = ''
              Where this entry arrives on the host. The installer writes every
              entry into the generation directory and then symlinks any entry
              whose path is not `''${symlinkPath}/<name>` to where its path
              names.

              The default is `''${safix.installer.symlinkPath}/<name>`, and it
              lives here rather than in either consumption module on purpose:
              the store root and this default have to move together. A root
              moved without the default would make every entry a symlink
              target outside safix's store — writing safix's links into
              whatever store owns the old root, instead of colliding with it
              where a collision would be visible.
            '';
          };

          mode = mkOption {
            type = types.str;
            default = "0400";
            example = "0440";
            description = ''
              The permission bits the installer creates this entry's file with,
              as an octal string. Parsed as octal by the installer, which
              refuses a string that is not, naming the entry and the string.
            '';
          };

          owner = mkOption {
            type = types.nullOr types.str;
            default = null;
            example = "nginx";
            description = ''
              The user the installer chowns this entry's file to, or null for
              `uid`. Resolved against the host's user database at activation, so
              a name no database knows is a refusal naming the entry and the
              name. Ignored under a user-mode install, which chowns nothing.
            '';
          };

          group = mkOption {
            type = types.nullOr types.str;
            default = null;
            example = "nginx";
            description = ''
              The group the installer chowns this entry's file to, or null for
              `gid`. Resolved and refused exactly as `owner` is.
            '';
          };

          uid = mkOption {
            type = types.int;
            default = 0;
            description = ''
              The numeric owner the installer uses where `owner` is null. Zero
              by default, which is what a system-scope store of root-readable
              files wants, and what makes a sandboxed check need no user
              database at all.
            '';
          };

          gid = mkOption {
            type = types.int;
            default = 0;
            description = ''
              The numeric group the installer uses where `group` is null, with
              the same reasoning as `uid`.
            '';
          };

          sopsFile = mkOption {
            type = types.path;
            description = ''
              The sops document this entry's value is read out of. No default:
              every resolved entry carries one the resolver derived from the
              entry's audience, so a default would be a value no declaration can
              produce.

              The installer decrypts each distinct document once, so entries
              sharing an audience share one subprocess.
            '';
          };

          format = mkOption {
            type = types.enum [ "yaml" ];
            default = "yaml";
            description = ''
              The document's format. A single-member enum because safix's
              resolver mints yaml documents and nothing else, and the recipient
              policy's own rules match on a `.yaml` suffix — so a second format
              is a value no declaration here can produce. It widens compatibly
              if one ever becomes resolvable.
            '';
          };

          restartUnits = mkOption {
            type = types.listOf types.str;
            default = [ ];
            example = [ "nginx.service" ];
            description = ''
              Units the installer restarts when this entry is new or its value
              changed since the previous generation. Propagated through
              `systemctl` from the unit mechanism and through
              `/run/nixos/activation-restart-list` from an activation script.
              Skipped wholesale under a user-mode install, which restarts
              nothing.
            '';
          };

          reloadUnits = mkOption {
            type = types.listOf types.str;
            default = [ ];
            example = [ "nginx.service" ];
            description = ''
              Units the installer reloads rather than restarts, on the same
              new-or-changed condition as `restartUnits`.
            '';
          };
        };
      }
    );
in
{
  inherit
    missingLibMessage
    flakelessMessage
    violationMessage
    noIdentityMessage
    noSystemIdentityMessage
    installerPackageMessage
    secretEntryType
    sopsFileMissingMessage
    sopsFileOutsideStoreMessage
    wasSet
    ;

  # The options that read the same in either scope. The five arguments are what
  # the scopes disagree about, passed in rather than branched on, so a scope that
  # cannot derive a default or a type says so in its own file.
  sharedOptions =
    {
      cfg,
      userDefault,
      userDefaultText,
      hostnameDefault,
      hostnameDefaultText,
      secretsType,
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
          decrypts with is the identity it already had — at system scope
          `safix.identity.deriveHostKeys` takes the host's ed25519 keys, which
          is the same key `flake.safix.machines.<m>.recipient` is the age form
          of, so a machine entry needs no identity named here.

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
        type = secretsType;
        readOnly = true;
        description = ''
          What safix resolved for this profile. Read-only: it is a projection of
          `flake.safix.*` for this person on this host at this scope.

          Empty whenever the profile is unbound — no `safix.lib`, no
          `safix.user` or no `safix.hostname` — which is what lets the
          assertions below report the mistake instead of a resolution throwing
          before they are reached.

          Typed per scope, because the typing each scope needs is its own: at
          system scope this is `common.secretEntryType`, safix's own entry
          submodule, so every entry carries safix's own defaults and the path
          the entry will arrive at; at user scope it is the untyped projection,
          and the user-mode manifest `home.nix` builds is where each entry's
          twelve fields are filled, since a second typing pass would restate
          the same defaults.
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
            differently. safix's own installer treats a set-but-unreadable key
            file as fatal, naming the path, where a missing ssh key path is
            written to stderr and skipped. A non-null default would therefore
            abort activation on every machine that happens to lack the path,
            while an unset one costs nothing.

            Read by safix's own installer manifest at both scopes, and by the
            pre-decryption identity check that refuses before any document is
            read. No option outside safix's namespace is defined from it: a
            consumer who runs another secret-management framework for their own
            secrets configures that framework's key sources themselves, and
            safix neither reads nor writes them.
          '';
        };

        sshKeyPaths = mkOption {
          type = types.listOf types.str;
          default = [ ];
          example = [ "/home/jane/.ssh/agenix" ];
          description = ''
            ssh private keys this machine decrypts with. safix's own installer
            runs each through `ssh-to-age` while assembling the identity it
            hands the backend, so the age recipient the recipient policy names
            is the converted public half.

            Each path is individually skipped with a line to stderr when absent
            or unconvertible, so these are load-bearing only collectively, and
            only while they are the sole identity source.

            At system scope, naming none of these and no `keyFile` leaves
            `safix.identity.deriveHostKeys` to derive an identity from the
            host's ed25519 keys outside safix's own store; at user scope there
            is nothing to derive, and the resolution refuses instead.
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
