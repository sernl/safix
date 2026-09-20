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
  bridge = import ./bridge.nix { inherit lib; };
  keepassxc = import ./keepassxc.nix { inherit lib; };
  pass = import ./pass.nix { inherit lib; };
  bitwarden = import ./bitwarden.nix { inherit lib; };
  onepassword = import ./onepassword.nix { inherit lib; };

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
  custodyMessages = registry: resolve.violations registry ++ resolve.generatorViolations registry;

  mkCustodyCheck =
    pkgs: registry:
    mkMessageCheck pkgs {
      name = "safix-custody-refusals";
      subject = "safix custody: these declarations break rules the resolver refuses on.";
      messages = custodyMessages registry;
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
    registry:
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
      ) (resolve.placementsOf registry)
    );

  generatorToolMessages =
    pkgs: registry:
    let
      resolves = spec: lib.hasAttrByPath (lib.splitString "." spec) pkgs;
    in
    lib.concatMap (
      g:
      map (
        spec:
        "flake.safix.users.${g.user}'s generator on '${g.name}' names runtimeInputs '${spec}', which is not an attribute of nixpkgs"
      ) (lib.filter (spec: !(resolves spec)) g.generator.runtimeInputs)
    ) (generatorsDeclaredIn registry);

  mkGeneratorToolCheck =
    pkgs: registry:
    mkMessageCheck pkgs {
      name = "safix-generator-tools";
      subject = "safix generators: a declared generator names a runtime tool nixpkgs does not have.";
      messages = generatorToolMessages pkgs registry;
    };

  # ── the bridge ──
  # Only the half of each mapping that lives in the consumer's own flake. The
  # clan half is not checked here and cannot be: it lives in another flake, and
  # the only thing that can answer whether it resolves is clan itself.
  bridgeMessages = registry: bridgeRecord: bridge.violationsOf registry bridgeRecord;

  mkBridgeCheck =
    pkgs: registry: bridgeRecord:
    mkMessageCheck pkgs {
      name = "safix-bridge-refusals";
      subject = "safix bridge: these mappings break rules evaluation refuses on.";
      messages = bridgeMessages registry bridgeRecord;
    };

  # ── the keepassxc mirror ──
  # Only the half of each mapping that lives in the consumer's own declarations.
  # The database's half is not checked here and cannot be: the group and the
  # entry are content of an encrypted file, and answering whether they are there
  # needs a key.
  keepassxcMessages = registry: keepassxcRecord: keepassxc.violationsOf registry keepassxcRecord;

  mkKeepassxcCheck =
    pkgs: registry: keepassxcRecord:
    mkMessageCheck pkgs {
      name = "safix-keepassxc-refusals";
      subject = "safix keepassxc: these mappings break rules evaluation refuses on.";
      messages = keepassxcMessages registry keepassxcRecord;
    };

  # ── the pass store ──
  # Only the half of each mapping that lives in the consumer's own
  # declarations. The store's half is not checked here and cannot be: an entry
  # is a gpg-encrypted file, and answering whether the store or the entry is
  # there needs the operator's own key.
  passMessages = registry: passRecord: pass.violationsOf registry passRecord;

  mkPassCheck =
    pkgs: registry: passRecord:
    mkMessageCheck pkgs {
      name = "safix-pass-refusals";
      subject = "safix pass: these mappings break rules evaluation refuses on.";
      messages = passMessages registry passRecord;
    };

  # ── the bitwarden vault ──
  # Only the half of each mapping that lives in the consumer's own
  # declarations. The vault's half is not checked here and cannot be: whether
  # the item is there, whether the folder is, and whether the client is
  # unlocked or even logged in are run-time questions that need a session.
  # Five rules where the keepassxc mirror has five and the 1password one has
  # four: this target's memory is a hidden field of the mapped item, so it
  # reserves no name, and the field rule it does carry is `tags` alone.
  bitwardenMessages = registry: bitwardenRecord: bitwarden.violationsOf registry bitwardenRecord;

  mkBitwardenCheck =
    pkgs: registry: bitwardenRecord:
    mkMessageCheck pkgs {
      name = "safix-bitwarden-refusals";
      subject = "safix bitwarden: these mappings break rules evaluation refuses on.";
      messages = bitwardenMessages registry bitwardenRecord;
    };

  # ── the 1password mirror ──
  # Only the half of each mapping that lives in the consumer's own
  # declarations. The far half is not checked here and cannot be: the vault and
  # the item are content of a remote service, and answering whether either is
  # there needs a session. Four rules where the keepassxc mirror has five: this
  # target's memory is a field of the mapped item, so it reserves no name and
  # has no `reservedName` refusal.
  onepasswordMessages =
    registry: onepasswordRecord: onepassword.violationsOf registry onepasswordRecord;

  mkOnepasswordCheck =
    pkgs: registry: onepasswordRecord:
    mkMessageCheck pkgs {
      name = "safix-onepassword-refusals";
      subject = "safix 1password: these mappings break rules evaluation refuses on.";
      messages = onepasswordMessages registry onepasswordRecord;
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
      perFile =
        file: a:
        let
          format = a.format or "yaml";
          legacy = a.legacy or (format == "yaml");
          rules = lib.filter (r: (r.file or file) == file && r.audience == a.audience) plan.rules;
          expected =
            if legacy then "^${lib.escapeRegex a.dir}/[^/]*\\.yaml$" else "^${lib.escapeRegex file}$";
          badPaths = [
            "nested/${file}"
            "${file}.bak"
            "${a.dir}/nested/${builtins.baseNameOf file}"
            "${a.dir}/x.txt"
            "${a.dir}-other/${builtins.baseNameOf file}"
          ]
          ++ map (ext: "${a.dir}/${lib.removeSuffix ".${format}" (builtins.baseNameOf file)}.${ext}") (
            lib.filter (ext: ext != format) resolve.formats
          )
          ++ lib.optional (!legacy) "${a.dir}/beside.${format}";
        in
        lib.optional (rules == [ ]) "${file} has no rule for its audience"
        ++ lib.optional (builtins.length rules > 1) "${file} has duplicate rules for its audience"
        ++ lib.concatMap (
          rule:
          lib.optional (
            rule.pathRegex != expected
          ) "${rule.pathRegex} is not the anchored, scoped ${format} rule for ${file}"
          ++ lib.optional (!(matches rule.pathRegex file)) "${rule.pathRegex} does not match ${file}"
          ++ lib.optional (
            legacy && !(matches rule.pathRegex "${a.dir}/beside.yaml")
          ) "${rule.pathRegex} strands a legacy adjacent YAML file"
          ++ map (path: "${rule.pathRegex} reaches outside its scope at ${path}") (
            lib.filter (matches rule.pathRegex) badPaths
          )
        ) rules
        ++ lib.concatMap (
          rule:
          lib.optional (
            rule.audience != a.audience && matches rule.pathRegex file
          ) "${rule.pathRegex} gives ${file} a conflicting audience"
        ) plan.rules;
      orphanRules = lib.concatMap (
        rule:
        lib.optional (
          !(lib.any (file: audiences.${file}.audience == rule.audience && (rule.file or file) == file) (
            builtins.attrNames audiences
          ))
        ) "${rule.pathRegex} has no derived document in its audience"
      ) plan.rules;
    in
    lib.concatLists (lib.mapAttrsToList perFile audiences) ++ orphanRules;

  ruleShapeMessages =
    registry:
    ruleShapeMessagesOf {
      plan = policy.plan registry;
      audiences = resolve.audiencesOf registry;
    };

  mkRuleShapeCheck =
    pkgs: registry:
    mkMessageCheck pkgs {
      name = "safix-rule-shape";
      subject = "safix policy: a generated rule does not cover exactly the directory it was written for.";
      messages = ruleShapeMessages registry;
    };

  # ── no catch-all ──
  # An unmatched path must fail closed with sops' own "no matching creation
  # rules found" rather than acquiring a default recipient set. The probes carry
  # an uppercase element, which the name alphabet excludes, so no declaration can
  # ever make one of them a real directory and the claim cannot be weakened by
  # someone adding a user.
  # The public store's own shape is among them. A rule reaching it is a rule over
  # a tree nothing is placed *encrypted* in, which is what this check asks — and
  # it is asked here as well as by `safix-public-no-rule` on purpose. That one
  # asks "does a rule reach the public store"; this one asks "does a rule reach
  # anywhere nothing is placed". A refactor that weakened one is unlikely to
  # weaken both.
  #
  # The definition-record tree is the third and is here for the same reason. It
  # is no longer the case that nothing in nix computes its paths: the resolver
  # emits `definitionRecord` on every placement (design S8), so these two probes
  # now guard a tree nix itself places rather than describing the shape Rust
  # writes.
  #
  # The stamp records sit in the same tree and get their own two probes rather
  # than riding the definition record's: `.stamps` is a distinct path, and a
  # rule anchored to one of the two forms would be missed by a probe naming
  # only the other.
  #
  # The eight tree-shaped probes are derived from the configured roots and must
  # stay so. Left literal while the roots are the consumer's, this check would
  # probe trees nothing uses and stop probing the trees the consumer has: green,
  # and meaningless. A silently lost check is worse than a broken one (design
  # S10). The four non-tree probes stay literal, because what they are about is
  # paths outside every tree.
  catchAllProbesOf = storage: [
    "x.yaml"
    "UNCLAIMED.yaml"
    "UNCLAIMED/x.yaml"
    "${storage.encrypted}/users/UNCLAIMED/x.yaml"
    "${storage.encrypted}/shared/UNCLAIMED/x.yaml"
    "some/other/place/UNCLAIMED.yaml"
    "${storage.plaintextOutputs}/users/UNCLAIMED/x/value"
    "${storage.plaintextOutputs}/shared/UNCLAIMED/x/value"
    "${storage.generatorRecords}/UNCLAIMED/x"
    "${storage.generatorRecords}/shared/UNCLAIMED/x"
    "${storage.generatorRecords}/UNCLAIMED/x.stamps"
    "${storage.generatorRecords}/shared/UNCLAIMED/x.stamps"
  ];

  catchAllMessagesOf =
    storage: plan:
    lib.concatMap (
      r:
      map (
        p:
        "${r.pathRegex} matches ${p}, which no declaration places anything in, so it is a catch-all granting ${lib.concatStringsSep ", " r.audience} custody of whatever lands there"
      ) (lib.filter (matches r.pathRegex) (catchAllProbesOf storage))
    ) plan.rules;

  catchAllMessages =
    registry: catchAllMessagesOf (registry.storage or resolve.defaultStorage) (policy.plan registry);

  mkNoCatchAllCheck =
    pkgs: registry:
    mkMessageCheck pkgs {
      name = "safix-no-catch-all";
      subject = "safix policy: a generated rule matches a path no declaration places anything in.";
      messages = catchAllMessages registry;
    };

  # ── the public store is out of the policy's reach ──
  # A generator output declared `secret = false` is stored in the clear so that
  # a nix module can read it at evaluation. A creation rule reaching one of those
  # paths would encrypt the value whose whole purpose is being readable, and it
  # would do so at the moment somebody ran `sops` against the path rather than at
  # a point anyone was watching.
  #
  # The rules are anchored under `flake.safix.storage.encrypted` and terminate
  # on a supported format extension, so a public `value` file cannot match
  # either clause — but relying on that is relying on two independent accidents
  # staying true. Asserted by
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
    registry:
    publicRuleMessagesOf {
      plan = policy.plan registry;
      publicPaths = resolve.publicPathsOf registry;
    };

  mkPublicRuleCheck =
    pkgs: registry:
    mkMessageCheck pkgs {
      name = "safix-public-no-rule";
      subject = "safix policy: a generated rule matches a path the public store holds in the clear.";
      messages = publicRuleMessages registry;
    };

  # ── the audience separator ──
  # A shared audience's directory is its members joined by one character. That
  # character has to be outside the name alphabet, or two audiences reach one
  # directory and so one rule; and it has to be inert in a regex, or the rule
  # generated for `alice+bob` matches `alicebob` and never `alice+bob`, and
  # every file in that directory fails closed under a rule that reads as if it
  # covered them.
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
        file: a:
        let
          rules = lib.filter (r: (r.file or file) == file) (rulesFor plan a.audience);
          elided = "${lib.replaceStrings [ sep ] [ "" ] a.dir}/${builtins.baseNameOf file}";
        in
        lib.concatMap (
          rule:
          lib.optional (!(matches rule.pathRegex file))
            "${rule.pathRegex} does not match its own file ${file}, so the separator does not preserve its scope"
          ++ lib.optional (matches rule.pathRegex elided) "${rule.pathRegex} matches ${elided}, with the audience separator elided"
        ) rules;
    in
    alphabet ++ lib.concatLists (lib.mapAttrsToList inert shared);

  separatorMessages =
    registry:
    separatorMessagesOf {
      plan = policy.plan registry;
      audiences = resolve.audiencesOf registry;
      separator = resolve.audienceSeparator;
    };

  mkSeparatorCheck =
    pkgs: registry:
    mkMessageCheck pkgs {
      name = "safix-audience-separator";
      subject = "safix policy: the character joining a shared audience's members does not hold its directory apart.";
      messages = separatorMessages registry;
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
      machines ? { },
      services ? { },
      groups ? { },
      organizations ? { },
      silos ? { },
      # Accepted and otherwise unused: `default.nix`'s `registry` binding now
      # carries `namingKey` alongside the other seven records (`null` when no
      # vault is declared), and `mkChecks pkgs (registry // args)` forwards
      # the whole thing. None of the checks this function builds are
      # vault-aware, so the field is not read past this pattern.
      namingKey ? null,
      # Accepted and otherwise unused, for `namingKey`'s reason: the registry
      # `default.nix` forwards now carries the declared rotation policies, and
      # no check built here reads a deadline.
      rotation ? { },
      # Read, unlike `namingKey`: `safix-no-catch-all`'s tree-shaped probes and
      # every generated `pathRegex` follow the configured roots, so a check
      # built here over a renamed root must probe the renamed tree (design
      # S10).
      storage ? resolve.defaultStorage,
      committedPolicy ? null,
      materializations ? { },
      bridge ? {
        clanFlake = null;
        mappings = { };
      },
      keepassxc ? {
        database = null;
        group = "safix";
        mappings = { };
      },
      pass ? {
        store = "~/.password-store";
        mappings = { };
      },
      bitwarden ? {
        server = null;
        mappings = { };
      },
      onepassword ? {
        account = null;
        mappings = { };
      },
    }:
    let
      registry = {
        inherit
          users
          catalogue
          machines
          services
          groups
          organizations
          silos
          storage
          ;
      };
    in
    {
      safix-bridge-refusals = mkBridgeCheck pkgs registry bridge;
      safix-keepassxc-refusals = mkKeepassxcCheck pkgs registry keepassxc;
      safix-pass-refusals = mkPassCheck pkgs registry pass;
      safix-bitwarden-refusals = mkBitwardenCheck pkgs registry bitwarden;
      safix-onepassword-refusals = mkOnepasswordCheck pkgs registry onepassword;
      safix-custody-refusals = mkCustodyCheck pkgs registry;
      safix-generator-tools = mkGeneratorToolCheck pkgs registry;
      safix-rule-shape = mkRuleShapeCheck pkgs registry;
      safix-no-catch-all = mkNoCatchAllCheck pkgs registry;
      safix-public-no-rule = mkPublicRuleCheck pkgs registry;
      safix-audience-separator = mkSeparatorCheck pkgs registry;
    }
    // lib.optionalAttrs (committedPolicy != null) {
      safix-policy-drift = policy.mkDriftCheck pkgs {
        committed = committedPolicy;
        generated = policy.render registry;
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
    bridgeMessages
    mkBridgeCheck
    keepassxcMessages
    mkKeepassxcCheck
    passMessages
    mkPassCheck
    bitwardenMessages
    mkBitwardenCheck
    onepasswordMessages
    mkOnepasswordCheck
    generatorsDeclaredIn
    generatorToolMessages
    ruleShapeMessages
    ruleShapeMessagesOf
    catchAllMessages
    catchAllMessagesOf
    catchAllProbesOf
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
