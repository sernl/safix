# The flake-parts module a consumer imports.
#
# It declares the two records in ./options.nix and binds the algebra in
# ./resolve.nix and the renderer in ./policy.nix to them, exposing the result
# under `flake.safix.lib`. Everything below is a projection of
# `flake.safix.catalogue` and `flake.safix.users` and reads no option outside
# this namespace, which is what makes an adapter written by a consumer
# sufficient on its own.
{
  lib,
  config,
  self,
  ...
}:
let
  resolve = import ./resolve.nix { inherit lib; };
  policy = import ./policy.nix { inherit lib; };

  cfg = config.flake.safix;

  sortNames = lib.sort (a: b: a < b);

  audiences = resolve.audiencesOf cfg.users cfg.catalogue;

  # Every entry's file is derived from its audience, and the result is a path
  # under the flake source rather than a repository-relative string, because that
  # is what the provisioner's `sopsFile` takes. It resolves the same from a
  # standalone user profile and from a system configuration.
  bound =
    args:
    args
    // {
      inherit (cfg) users catalogue;
      root = self;
    };

  resolveSet = args: resolve.selectFor (bound args);

  resolveNames = args: builtins.attrNames (resolveSet args);

  materialize = args: resolve.materializeFor (bound args);
in
{
  imports = [ ./options.nix ];

  options.flake.safix = {
    extraGovernedFiles = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "secrets/safix/users/ana/ops-tooling.yaml" ];
      description = ''
        Encrypted files a consumer wants governed that no declaration implies,
        as repository-relative paths.

        A file placed beside a person's secrets already rides that person's rule,
        because every rule covers one directory level rather than one literal
        filename. What it does not do is appear in the set `safix fix` re-wraps,
        so a change of audience would reach every file safix placed and leave
        this one behind, encrypted to whoever it was encrypted to when it was
        written. Naming it here puts it in that set.

        Only files a rule already covers belong here. Naming a path no rule
        matches does not create a rule for it: there is no catch-all, and a new
        rule comes from a new user record with a recipient.
      '';
    };

    lib = lib.mkOption {
      type = lib.types.attrs;
      readOnly = true;
      description = "The resolution helpers, maps and generated policy this module derives from the two records. Read-only: every value here is a projection of what a consumer declared.";
    };
  };

  config.flake.safix.lib = {
    # The scoped view of one user's secrets on one host, and the names alone.
    inherit resolveSet resolveNames;

    # The scoped view materialized into the shape the provisioner's option tree
    # takes, for `scope = "system"` or `scope = "user"`.
    inherit materialize;

    # Every custody and generator rule the declarations break, as messages. The
    # resolvers throw on this list; a check asserts it empty, which covers the
    # users no configuration builds and so never forces a resolution.
    #
    # The two halves concatenate rather than interleave, and the generator half
    # is empty while the custody half is not: a generator rule is a statement
    # about one user's resolved set, and there is no resolved set to state it
    # against until custody resolves.
    violations =
      resolve.violations cfg.users cfg.catalogue ++ resolve.generatorViolations cfg.users cfg.catalogue;

    # file -> { audience; dir; recipients; }: who can open each encrypted file a
    # secret is placed in. The recipient policy and the resolved entries are both
    # derived from this, so a file, its rule and its stanzas cannot disagree by
    # construction — only by someone editing ciphertext or the policy out from
    # under it.
    inherit audiences;

    # user -> name -> { file; key; origin; owner; shared; generator; }: the
    # inverse of the map above, keyed the way an operator asks the question.
    # `audiences` answers "who can open this file"; this answers "which file
    # holds this name, under which key", which is what the command reads so that
    # setting a value names a secret and never a path. Both are the same audience
    # computation, so a value written through it lands in the file the policy
    # writes a rule for.
    placements = resolve.placementsOf cfg.users cfg.catalogue;

    # user -> [ recipient ]: every key a declared user can open a file with,
    # their own and their recovery keys alike. `audiences` answers the same
    # question per file and unions the members' keys into one list, which loses
    # which key is whose; a check that has found a stanza on a file and wants to
    # say who left it there needs the direction this way round.
    recipients = lib.mapAttrs (_: resolve.recipientsOf) cfg.users;

    # user -> { order; outputs; inputs; }: the order `safix generate` runs that
    # user's generators in, what each one writes, and the name space its script
    # addresses its prompts and dependencies by.
    generatorPlan = resolve.generatorPlanOf cfg.users cfg.catalogue;

    # The alphabet a user, anchor or secret name must be drawn from, as the
    # unanchored pattern resolve.nix matches with. `safix adduser` reads it to
    # refuse a malformed name at the point it would otherwise scaffold one: a
    # name only becomes subject to the resolver's own check once it is a declared
    # user, which is one commit too late to be a refusal.
    nameRegex = resolve.nameRegex;

    # Every file the recipient policy governs, split by where it comes from.
    # `required` is computed from the audiences the declarations imply;
    # `extra` is what the consumer named through `extraGovernedFiles`.
    #
    # `safix fix` re-wraps `managed`, and the checks judge `required` for
    # existence and `extra` for its stanzas. The union is what keeps those the
    # same set: a file judged out of policy that no sanctioned command can name
    # is a file that can only drift further.
    #
    # The source this was ported from computed the second half by reading a fixed
    # directory of the consumer's tree. That is a layout assumption safix does
    # not get to make, and it also fails the other way: a registry entry exists
    # before anyone encrypts its file, so a readDir and the declarations were
    # never the same set in either direction.
    governedFiles =
      let
        extra = sortNames (lib.unique cfg.extraGovernedFiles);
        required = sortNames (builtins.attrNames audiences);
      in
      {
        inherit extra required;
        managed = sortNames (lib.unique (extra ++ required));
      };

    # The recipient policy these declarations imply, as text and as the
    # structured plan the text renders. The sops CLI reads the committed file off
    # disk, so the text is an artifact that must be regenerated and committed,
    # never something a build alone can satisfy.
    policyText = policy.render cfg.users cfg.catalogue;
    policyPlan = policy.plan cfg.users cfg.catalogue;

    # The check a consumer instantiates over its own committed policy file. Built
    # here so that its failure and the generated header name one command.
    mkDriftCheck =
      pkgs: committed:
      policy.mkDriftCheck pkgs {
        inherit committed;
        generated = policy.render cfg.users cfg.catalogue;
      };
  };
}
