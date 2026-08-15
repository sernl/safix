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
  checks = import ./checks.nix { inherit lib; };

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
  imports = [
    ./options.nix
  ];

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

    onboardingHook = lib.mkOption {
      type = lib.types.nullOr lib.types.lines;
      default = null;
      example = ''
        name="$1"
        recipient="$2"
        shift 2
        for host in "$@"; do
          printf 'attach %s on %s\n' "$name" "$host"
        done
      '';
      description = ''
        A shell fragment `safix adduser` runs after the person's declaration and
        the regenerated policy are committed, receiving their name as `$1`,
        their recipient as `$2`, and every `--host` given as `$3` onward. It
        runs with the repository root as its working directory.

        Everything onboarding does beyond writing a custody record is here.
        Attaching an account on a host, allocating an identifier, editing a
        host's module imports: each is a property of one consumer's module tree,
        and safix has no way to know its shape, so it passes the two facts it
        does know and makes no assumption about what happens next.

        Whatever the hook writes is left uncommitted, so safix's own commit
        names only what safix did. A non-zero exit is reported and not
        interpreted.

        Unset is a supported configuration: `adduser` succeeds without it,
        having done less. `--host` is refused while it is unset, because there
        is nothing for a hostname to reach.
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

    # Every repository-relative path the plaintext store holds, sorted. What the
    # generated recipient policy is checked against: no rule may match any of
    # them, because a rule reaching the public store would encrypt a value the
    # whole point of which is being readable at evaluation.
    publicPaths = resolve.publicPathsOf cfg.users cfg.catalogue;

    # The two accessors a consuming module reads an output through.
    #
    # `path` answers for every output, secret or public, and is a path rather
    # than a value: for a secret it is where the provisioner will place the
    # decrypted file, and reading it decrypts nothing.
    #
    # `value` answers only for a public output, and reads the file at evaluation
    # — which is the whole reason `files.<n>.secret = false` exists. A public
    # output that has not been generated yet fails with the command that would
    # produce it, because an evaluation failing with "run `safix generate ana
    # wg-public`" is strictly better than one failing with a path that is not
    # there.
    #
    # Reaching for `value` on a secret output fails with a sentence rather than
    # with nix's generic undefined-option message, which is where this departs
    # from clan: clan leaves the option undefined under `mkIf (secret == false)`.
    # The cost is one evaluated thunk. What it buys is that the likeliest
    # authoring mistake in this whole surface — reaching for `.value` on a secret
    # because the sibling public output has one — produces a sentence saying what
    # to do instead of one saying that an option was used but not defined.
    publicValue =
      user: name:
      let
        placement =
          (resolve.placementsOf cfg.users cfg.catalogue).${user}.${name} or (throw
            "safix public: flake.safix.users.${user} holds no secret named '${name}', so it has no value to read"
          );
        path = "${toString self}/${placement.public}";
      in
      if placement.public == null then
        throw "safix public: '${name}' of flake.safix.users.${user} is a secret, so it has no value readable at evaluation — that is what being encrypted means. Use flake.safix.lib.outputPath ${user} \"${name}\" for where the decrypted file is placed at activation, or declare the output with `files.${name}.secret = false` if it is meant to be public."
      else if builtins.pathExists path then
        builtins.readFile path
      else
        throw "safix public: '${name}' of flake.safix.users.${user} has not been generated yet, so ${placement.public} does not exist. Run `safix generate ${user} ${name}`.";

    # The repository-relative path of an output, secret or public. A path, never
    # a value.
    outputPath =
      user: name:
      let
        placement =
          (resolve.placementsOf cfg.users cfg.catalogue).${user}.${name} or (throw
            "safix public: flake.safix.users.${user} holds no secret named '${name}', so it has no path"
          );
      in
      if placement.public != null then placement.public else placement.file;

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

    # Every check safix has to offer, over the declarations this flake carries,
    # as an attrset a consumer assigns straight into `perSystem.checks`. The two
    # optional arguments are the ones safix cannot derive: the committed policy
    # file to compare the generated one against, and the materializations that
    # only the consumer's own configurations produce.
    #
    # ../checks calls the same builders with fleets written beside them, so a
    # claim asserted there is asserted about the function a consumer runs rather
    # than about a second copy of it.
    mkChecks =
      pkgs: args:
      checks.mkChecks pkgs (
        {
          inherit (cfg) users catalogue;
        }
        // args
      );
  };
}
