# The checks safix hands a consumer, as builders rather than as checks.
#
# Everything here is a function of the two records and a package set, so a
# consumer instantiates them over their own declarations and this repository
# instantiates the same functions over a fixture fleet. That is what makes the
# fixture suites evidence about the code a consumer runs: there is one
# implementation of each claim, and both callers reach it.
#
# Each family is split into a message function and a builder over it. The
# message function is a pure value a fixture can assert against a literal, and
# the builder is the derivation that fails while the list is non-empty. The
# split is what lets a severity drill be executed rather than described: a drill
# runs `refuseScript` — the same bytes the real check runs — over the messages a
# perturbed fleet produces, and asserts the failure and what it names.
#
# A claim belongs here when it is a statement about declarations a consumer
# writes. A claim about safix's own algebra belongs in a fixture suite under
# ../checks, judged against fleets written beside it.
{ lib }:
let
  resolve = import ./resolve.nix { inherit lib; };
  policy = import ./policy.nix { inherit lib; };

  # The one shell every message-bearing check runs: it fails while the file it
  # is handed has any line in it. Exposed so a drill runs these bytes rather
  # than a copy of them.
  refuseScript =
    pkgs:
    pkgs.writeShellScript "safix-refuse" ''
      set -eu
      messages="$1"
      subject="$2"
      if [ -s "$messages" ]; then
        {
          printf '%s\n\n' "$subject"
          while IFS= read -r line; do
            printf '  - %s\n' "$line"
          done < "$messages"
        } >&2
        exit 1
      fi
    '';

  mkMessageCheck =
    pkgs:
    {
      name,
      subject,
      messages,
    }:
    pkgs.runCommand name
      {
        messagesText = lib.concatMapStrings (m: m + "\n") messages;
        passAsFile = [ "messagesText" ];
        meta.description = "structural check: ${name}";
      }
      ''
        ${refuseScript pkgs} "$messagesTextPath" ${lib.escapeShellArg subject}
        touch "$out"
      '';

  # ── custody ──
  # Every rule the resolvers throw on, as messages. Both halves, because a
  # generator rule is a statement about a resolved set and custody is what
  # resolves it. This covers the users no configuration builds and so never
  # forces a resolution of.
  custodyMessages =
    users: catalogue: resolve.violations users catalogue ++ resolve.generatorViolations users catalogue;

  mkCustodyCheck =
    pkgs: users: catalogue:
    mkMessageCheck pkgs {
      name = "safix-custody-refusals";
      subject = "safix custody: these declarations break rules the resolver refuses on.";
      messages = custodyMessages users catalogue;
    };

  # ── generator runtime tools ──
  # `runtimeInputs` names nixpkgs attributes as strings, because a generator
  # travels to the command as JSON and a derivation cannot cross that boundary.
  # Strings are unchecked by construction, so `openssl` and `opensll` are equally
  # well-typed and the second is otherwise discovered at a rotation. Resolution
  # is by path, so a dotted `python3Packages.pyyaml` resolves the way it is
  # written, and `hasAttrByPath` forces the attribute's existence and never its
  # value.
  generatorsDeclaredIn =
    users: catalogue:
    lib.concatLists (
      lib.mapAttrsToList (
        user: names:
        lib.concatLists (
          lib.mapAttrsToList (
            name: record:
            lib.optional (record.generator != null) {
              inherit user name;
              inherit (record) generator;
            }
          ) names
        )
      ) (resolve.placementsOf users catalogue)
    );

  generatorToolMessages =
    pkgs: users: catalogue:
    let
      resolves = spec: lib.hasAttrByPath (lib.splitString "." spec) pkgs;
    in
    lib.concatMap (
      g:
      map (
        spec:
        "flake.safix.users.${g.user}'s generator on '${g.name}' names runtimeInputs '${spec}', which is not an attribute of nixpkgs"
      ) (lib.filter (spec: !(resolves spec)) g.generator.runtimeInputs)
    ) (generatorsDeclaredIn users catalogue);

  mkGeneratorToolCheck =
    pkgs: users: catalogue:
    mkMessageCheck pkgs {
      name = "safix-generator-tools";
      subject = "safix generators: a declared generator names a runtime tool nixpkgs does not have.";
      messages = generatorToolMessages pkgs users catalogue;
    };

  # ── the shape of a generated rule ──
  # Asserted behaviourally, by matching each rule against paths derived from the
  # directory it was written for, rather than by inspecting the regex as a
  # string. A regex read as text says what it looks like; a match says what sops
  # will do with it, and sops matching is the whole subject.
  matches = pattern: path: builtins.match pattern path != null;

  rulesFor = plan: audience: lib.filter (r: r.audience == audience) plan.rules;

  ruleShapeMessagesOf =
    { plan, audiences }:
    let
      named = a: lib.concatStringsSep ", " a.audience;

      perAudience =
        _file: a:
        let
          rules = rulesFor plan a.audience;
          rule = builtins.head rules;
          probe = suffix: matches rule.pathRegex "${a.dir}/${suffix}";
        in
        if rules == [ ] then
          [ "the audience ${named a} has a file at ${a.dir} and no rule, so every value in it fails closed" ]
        else if builtins.length rules > 1 then
          [
            "the audience ${named a} has ${toString (builtins.length rules)} rules, and sops applies the first that matches"
          ]
        else
          lib.optional (
            !(lib.hasPrefix "^" rule.pathRegex)
          ) "${rule.pathRegex} is not start-anchored, so it also matches its own suffix under any prefix"
          ++
            lib.optional (!(lib.hasSuffix "\\.yaml$" rule.pathRegex))
              "${rule.pathRegex} does not terminate on the extension, so it reaches encrypted material safix did not place"
          ++
            lib.optional (!(probe "secrets.yaml"))
              "${rule.pathRegex} does not match ${a.dir}/secrets.yaml, which is the file that audience's secrets are placed in"
          ++
            lib.optional (!(probe "beside.yaml"))
              "${rule.pathRegex} does not match a second file in ${a.dir}, so a file placed beside that audience's secrets is stranded with no rule"
          ++ lib.optional (probe "nested/x.yaml") "${rule.pathRegex} matches a subdirectory of ${a.dir}, so a file dropped one level down silently inherits that audience's recipients"
          ++ lib.optional (probe "x.txt") "${rule.pathRegex} matches a path that is not a .yaml under ${a.dir}"
          ++ lib.optional (matches rule.pathRegex "nested/${a.dir}/x.yaml") "${rule.pathRegex} matches ${a.dir} under a prefix, so material outside the tree safix places into acquires that audience";
    in
    lib.concatLists (lib.mapAttrsToList perAudience audiences);

  ruleShapeMessages =
    users: catalogue:
    ruleShapeMessagesOf {
      plan = policy.plan users catalogue;
      audiences = resolve.audiencesOf users catalogue;
    };

  mkRuleShapeCheck =
    pkgs: users: catalogue:
    mkMessageCheck pkgs {
      name = "safix-rule-shape";
      subject = "safix policy: a generated rule does not cover exactly the directory it was written for.";
      messages = ruleShapeMessages users catalogue;
    };

  # ── no catch-all ──
  # An unmatched path must fail closed with sops' own "no matching creation
  # rules found" rather than acquiring a default recipient set. The probes carry
  # an uppercase element, which the name alphabet excludes, so no declaration can
  # ever make one of them a real directory and the claim cannot be weakened by
  # someone adding a user.
  # The last two are the public store's own shape. A rule reaching them is a
  # rule over a tree nothing is placed *encrypted* in, which is what this check
  # asks — and it is asked here as well as by `safix-public-no-rule` on purpose.
  # That one asks "does a rule reach the public store"; this one asks "does a
  # rule reach anywhere nothing is placed". A refactor that weakened one is
  # unlikely to weaken both.
  catchAllProbes = [
    "x.yaml"
    "UNCLAIMED.yaml"
    "UNCLAIMED/x.yaml"
    "secrets/safix/users/UNCLAIMED/x.yaml"
    "secrets/safix/shared/UNCLAIMED/x.yaml"
    "some/other/place/UNCLAIMED.yaml"
    "public/safix/users/UNCLAIMED/x/value"
    "public/safix/shared/UNCLAIMED/x/value"
  ];

  catchAllMessagesOf =
    plan:
    lib.concatMap (
      r:
      map (
        p:
        "${r.pathRegex} matches ${p}, which no declaration places anything in, so it is a catch-all granting ${lib.concatStringsSep ", " r.audience} custody of whatever lands there"
      ) (lib.filter (matches r.pathRegex) catchAllProbes)
    ) plan.rules;

  catchAllMessages = users: catalogue: catchAllMessagesOf (policy.plan users catalogue);

  mkNoCatchAllCheck =
    pkgs: users: catalogue:
    mkMessageCheck pkgs {
      name = "safix-no-catch-all";
      subject = "safix policy: a generated rule matches a path no declaration places anything in.";
      messages = catchAllMessages users catalogue;
    };

  # ── the public store is out of the policy's reach ──
  # A generator output declared `secret = false` is stored in the clear so that
  # a nix module can read it at evaluation. A creation rule reaching one of those
  # paths would encrypt the value whose whole purpose is being readable, and it
  # would do so at the moment somebody ran `sops` against the path rather than at
  # a point anyone was watching.
  #
  # The rules are anchored under `secrets/safix/` and terminate on `\.yaml$`, so
  # a `value` file under `public/` cannot match either clause — but relying on
  # that is relying on two independent accidents staying true. Asserted by
  # matching each rule against each real public path rather than by reading a
  # pattern as a string: a pattern read as text says what it looks like, and a
  # match says what sops will do with it.
  publicRuleMessagesOf =
    { plan, publicPaths }:
    lib.concatMap (
      r:
      map (
        p:
        "${r.pathRegex} matches ${p}, which is a public output stored in the clear, so a value declared readable at evaluation would be encrypted to ${lib.concatStringsSep ", " r.audience} the next time sops was run against that path"
      ) (lib.filter (matches r.pathRegex) publicPaths)
    ) plan.rules;

  publicRuleMessages =
    users: catalogue:
    publicRuleMessagesOf {
      plan = policy.plan users catalogue;
      publicPaths = resolve.publicPathsOf users catalogue;
    };

  mkPublicRuleCheck =
    pkgs: users: catalogue:
    mkMessageCheck pkgs {
      name = "safix-public-no-rule";
      subject = "safix policy: a generated rule matches a path the public store holds in the clear.";
      messages = publicRuleMessages users catalogue;
    };

  # ── the audience separator ──
  # A shared audience's directory is its members joined by one character. That
  # character has to be outside the name alphabet, or two audiences reach one
  # directory and so one rule; and it has to be inert in a regex, or the rule
  # generated for `ana+bo` matches `anabo` and never `ana+bo`, and every file in
  # that directory fails closed under a rule that reads as if it covered them.
  # The second is the one an injectivity claim alone misses, so it is asserted
  # here by matching rather than by inspecting the character.
  separatorMessagesOf =
    {
      plan,
      audiences,
      separator,
    }:
    let
      sep = separator;

      alphabet =
        lib.optional (builtins.match "[a-z0-9_-]*" sep != null)
          "the audience separator '${sep}' is drawn from the alphabet a name is drawn from, so two audiences can reach one directory and so one rule";

      shared = lib.filterAttrs (_file: a: builtins.length a.audience > 1) audiences;

      inert =
        _file: a:
        let
          rules = rulesFor plan a.audience;
          elided = lib.replaceStrings [ sep ] [ "" ] a.dir;
        in
        lib.optionals (rules != [ ]) (
          let
            rule = builtins.head rules;
          in
          lib.optional (!(matches rule.pathRegex "${a.dir}/secrets.yaml"))
            "${rule.pathRegex} does not match its own directory ${a.dir}, so the separator is not inert in a regex and every file there fails closed"
          ++ lib.optional (matches rule.pathRegex "${elided}/secrets.yaml") "${rule.pathRegex} matches ${elided}, which is ${a.dir} with the separator elided, so the separator is a regex metacharacter rather than a literal"
        );
    in
    alphabet ++ lib.concatLists (lib.mapAttrsToList inert shared);

  separatorMessages =
    users: catalogue:
    separatorMessagesOf {
      plan = policy.plan users catalogue;
      audiences = resolve.audiencesOf users catalogue;
      separator = resolve.audienceSeparator;
    };

  mkSeparatorCheck =
    pkgs: users: catalogue:
    mkMessageCheck pkgs {
      name = "safix-audience-separator";
      subject = "safix policy: the character joining a shared audience's members does not hold its directory apart.";
      messages = separatorMessages users catalogue;
    };

  # ── path collisions ──
  # `materializeFor` refuses two entries resolving onto one path, because
  # whichever declaration activates second unlinks the first's output. The
  # refusal only fires where something forces that materialization, which is the
  # configuration a person's host builds — so a consumer whose fleet has hosts
  # nobody has built this week has entries nothing has looked at. This forces
  # every materialization it is handed, which is how the refusal reaches them.
  #
  # It takes materialized sets rather than the records, because an entry's path
  # is a function of the configuration it lands in: safix cannot compute one
  # without the consumer's own config, and inventing a fixture config here would
  # be checking a path nobody deploys.
  mkPathCollisionCheck =
    pkgs: materializations:
    pkgs.runCommand "safix-path-collision"
      {
        forced = builtins.deepSeq materializations (
          lib.concatStringsSep "\n" (builtins.attrNames materializations)
        );
        passAsFile = [ "forced" ];
        meta.description = "structural check: safix-path-collision";
      }
      ''
        cp "$forcedPath" "$out"
      '';

  # ── the whole family ──
  # One call a consumer makes with their two records, returning checks named the
  # way they appear in `nix flake show`. The two conditional members are the ones
  # that need something safix cannot derive: a committed file to compare against,
  # and the materializations only the consumer's own configurations produce.
  mkChecks =
    pkgs:
    {
      users,
      catalogue ? { },
      committedPolicy ? null,
      materializations ? { },
    }:
    {
      safix-custody-refusals = mkCustodyCheck pkgs users catalogue;
      safix-generator-tools = mkGeneratorToolCheck pkgs users catalogue;
      safix-rule-shape = mkRuleShapeCheck pkgs users catalogue;
      safix-no-catch-all = mkNoCatchAllCheck pkgs users catalogue;
      safix-public-no-rule = mkPublicRuleCheck pkgs users catalogue;
      safix-audience-separator = mkSeparatorCheck pkgs users catalogue;
    }
    // lib.optionalAttrs (committedPolicy != null) {
      safix-policy-drift = policy.mkDriftCheck pkgs {
        committed = committedPolicy;
        generated = policy.render users catalogue;
      };
    }
    // lib.optionalAttrs (materializations != { }) {
      safix-path-collision = mkPathCollisionCheck pkgs materializations;
    };
in
{
  inherit
    refuseScript
    mkMessageCheck
    custodyMessages
    generatorsDeclaredIn
    generatorToolMessages
    ruleShapeMessages
    ruleShapeMessagesOf
    catchAllMessages
    catchAllMessagesOf
    catchAllProbes
    publicRuleMessages
    publicRuleMessagesOf
    separatorMessages
    separatorMessagesOf
    mkCustodyCheck
    mkGeneratorToolCheck
    mkRuleShapeCheck
    mkNoCatchAllCheck
    mkPublicRuleCheck
    mkSeparatorCheck
    mkPathCollisionCheck
    mkChecks
    ;
}
