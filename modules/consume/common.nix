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

  # Built by a named function rather than inline in the `throw` so that a check
  # can read what a consumer would see. `builtins.tryEval` reports that a throw
  # fired and never what it said, so a message assembled inside one is a message
  # nothing can hold.
  violationMessage =
    cfg:
    ''
      safix: the declarations this ${scopeNoun} is bound to do not resolve.

      safix.user = ${cfg.user}, safix.hostname = ${cfg.hostname}, scope = ${scope}

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
  # can hold. It names the user scope literally because there is no system-scope
  # counterpart — sops-nix's NixOS module defaults `sops.age.sshKeyPaths` to the
  # ed25519 keys of `config.services.openssh.hostKeys`, so a system
  # configuration that names no identity usually still has one.
  noIdentityMessage =
    { cfg, resolved }:
    ''
      safix: ${toString (builtins.length (builtins.attrNames resolved))} secret(s) resolve for ${cfg.user} on ${cfg.hostname}, and this
      home-manager profile names no decryption identity.

      Name one:

        safix.identity.sshKeyPaths = [ "/home/${cfg.user}/.ssh/id_ed25519" ];

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
      thing to refuse would be sops-nix's own key-source assertion, which names
      its five and neither of safix's.
    '';
in
{
  inherit missingLibMessage violationMessage noIdentityMessage;

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
        default = userDefault;
        defaultText = userDefaultText;
        example = "jane";
        description = ''
          Which `flake.safix.users` entry this ${scopeNoun} serves.

          This selects; it does not declare. The named entry must already exist
          in the consumer's `flake.safix.users`, and everything about who may
          read what — the recipient, the recovery identities, the grants — is
          stated there, where every user is visible at once.
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
        default = [ ];
        example = [ "laptop" ];
        description = ''
          The tags this host carries, against which
          `flake.safix.users.<u>.perTag` adds, omits and forces entries.

          safix has no host registry and derives no tag from anything: a tag
          vocabulary is the consumer's, and this is where theirs is handed over.
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
  resolvedFor =
    { cfg, target }:
    if cfg.lib == null || cfg.user == null || cfg.hostname == null then
      { }
    else if cfg.lib.violations != [ ] then
      throw (violationMessage cfg)
    else
      cfg.lib.materialize {
        inherit (cfg) user hostname tags;
        inherit scope;
      } target;

  # The wiring mistakes that are cheap to name exactly. Each fires only once the
  # profile has been bound at all, so importing the module and setting nothing
  # stays a no-op rather than becoming a demand.
  assertionsFor = cfg: [
    {
      assertion = cfg.lib == null || cfg.user != null;
      message = ''
        safix: this ${scopeNoun} is bound to a set of declarations but names no person.

        Set safix.user to the flake.safix.users entry this profile serves.
      '';
    }
    {
      assertion = cfg.lib == null || cfg.hostname != null;
      message = ''
        safix: this ${scopeNoun} is bound to a set of declarations but names no host.

        Resolution is host-scoped — flake.safix.users.<u>.perHost and .perTag
        select by host — so there is no set to resolve without one. Set
        safix.hostname.
      '';
    }
  ];
}
