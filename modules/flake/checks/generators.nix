# Holds the generator rules of ../safix/resolve.nix, and the one thing about a
# declared generator that a build can settle: that the packages its script asks
# for exist.
#
# `runtimeInputs` names nixpkgs attributes as strings rather than holding
# packages, because the whole generator travels to the command as JSON through
# one `nix eval` and a derivation cannot cross that boundary. Strings are
# unchecked by construction, so `openssl` and `opensll` are equally well-typed
# and the second is discovered when an operator runs `safix generate` — at a
# rotation, which is the worst moment to learn that a declaration was never
# right. Resolving each name moves that discovery to a build.
#
# Resolution is by path rather than by attribute name, so a dotted
# `python3Packages.pyyaml` resolves the way it is written; `hasAttrByPath` forces
# the attribute's existence and never its value, so this costs nixpkgs'
# attribute set and not a single package's evaluation.
#
# ── what this cannot check ──
# That the script runs, or that what it prints is a usable value. Running a
# generator mints a secret, which is not something a build may do: it would need
# an identity, and its output would be a value in the store. `safix generate`
# refuses an empty value and runs the entry's `validation` fragment over a
# candidate, and those two are where a script's behaviour is judged.
#
# Every fixture is synthetic, and the resolver-facing half asserts against fleets
# written here rather than against whatever a consumer declares. The name
# resolver is asserted against a name nixpkgs has, a dotted path it has, a name
# it does not, and a dotted path whose first component exists and whose second
# does not. A predicate weakened to `pkgs ? name` passes the first three and
# fails the fourth; one weakened to a non-empty check passes all four.
#
# Each refusal is asserted twice — the message `generatorViolations` produces,
# against a literal, and that `generatorPlanOf` over that fleet actually throws
# rather than returning a plan. The pair is what binds them: `guardGenerators`
# throws exactly this list, so a refusal that stopped firing fails the second
# half, and one that fired naming the wrong generator fails the first.
#
# Severity: proven by perturbation, one drill per claim. Dropping any single name
# from the `structural` list in resolve.nix empties that rule's Messages field
# and turns its Fires field false, and moves no other field. Two are worth
# stating separately because they are not independent:
#   - Dropping `selfDependency` does not turn `selfDependencyFires` false by way
#     of the cycle refusal. `generatorEdges` drops the self-edge, so `topoSplit`
#     finds nothing stuck and the plan is returned; that is the defect the rule
#     was added for, and this is the fixture that holds it.
#   - `cycleMessages` covers the length-two case and must not move under that
#     drill: it is what says refusing self-dependency by name has not also
#     stopped the graph-level refusal from seeing a cycle an operator did write.
{
  perSystem =
    {
      pkgs,
      lib,
      ...
    }:
    let
      resolve = import ../safix/resolve.nix { inherit lib; };
      types = import ../safix/types.nix { inherit lib; };
      checks = import ../safix/checks.nix { inherit lib; };
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      resolves = spec: lib.hasAttrByPath (lib.splitString "." spec) pkgs;

      # ── oracle fixtures: the shapes the resolver must tell apart ──
      oracle = {
        plainPresent = resolves "coreutils";
        dottedPresent = resolves "python3Packages.pyyaml";
        plainAbsent = resolves "safix-oracle-no-such-package";
        dottedAbsent = resolves "python3Packages.safix-oracle-no-such-package";
      };

      # Typed through the real option types, so a fixture cannot pass by omitting
      # a field the option system would have supplied and an option rename breaks
      # this file with the rest. Every generator fixture is built from `private`
      # entries alone: a generator hangs off an entry, and a private entry is one
      # this file can declare without a catalogue to carry it.
      typed =
        optionType: definition:
        (lib.evalModules {
          modules = [
            { options.value = lib.mkOption { type = optionType; }; }
            { value = definition; }
          ];
        }).config.value;

      # These fixtures assert on the shape of a generator declaration and never
      # on a key, so the recipient only has to be non-null: an owner declaring a
      # secret with no recipient is a custody violation, and `generatorViolations`
      # returns nothing at all while any custody rule is broken.
      fixtureRecipient = "age1fixture000000000000000000000000000000000000000000000000000";

      # One user named `ana`, holding exactly the private entries given.
      fleetOf = private: {
        ana = typed types.profile {
          inherit private;
          recipient = fixtureRecipient;
          recoveryRecipients = { };
        };
      };

      # A generator record with everything defaulted but what a fixture states.
      # The default script references `$out`, because a script that never does is
      # itself a refusal under the 0.2 contract and every fixture below would
      # otherwise report that instead of the rule it is about.
      gen = attrs: {
        generator = {
          script = ''printf x > "$out/value"'';
          runtimeInputs = [ "coreutils" ];
        }
        // attrs;
      };

      # `files` is an attrset now, and every fixture that only cares which names
      # a generator claims spells them through this.
      encrypted = names: lib.genAttrs names (_: { });

      plain = { };

      violationsOf = private: resolve.generatorViolations (fleetOf private) { };

      fires = e: !(builtins.tryEval (builtins.deepSeq e e)).success;

      # `generatorPlanOf` is the surface `guardGenerators` wraps, so this is the
      # throw an operator running `safix generate` would meet.
      planFires = private: fires (resolve.generatorPlanOf (fleetOf private) { });

      # The runtime-tool claim, restated over a fleet written here rather than
      # over whatever a consumer declares. The declaration is well-formed in
      # every other respect, so the unresolved name is the only thing this
      # reports.
      declaredIn =
        private:
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
          ) (resolve.placementsOf (fleetOf private) { })
        );

      unresolvedIn =
        private:
        lib.concatMap (
          g:
          map (
            spec:
            "flake.safix.users.${g.user}'s generator on '${g.name}' names runtimeInputs '${spec}', which is not an attribute of nixpkgs"
          ) (lib.filter (spec: !(resolves spec)) g.generator.runtimeInputs)
        ) (declaredIn private);

      fixtures = {
        # Well-formed, so nothing below is passing vacuously: this same shape
        # resolves to a plan.
        valid = {
          root = gen { };
          leaf = gen {
            dependencies = [ "root" ];
            files = encrypted [ "leaf-pub" ];
          };
          leaf-pub = plain;
        };

        misspelledTool.a = gen {
          runtimeInputs = [
            "coreutils"
            "opensll"
          ];
        };

        crossUser.a = gen { dependencies = [ "bo/their-token" ]; };

        unknownDependency.a = gen { dependencies = [ "absent" ]; };

        # The cycle of length one, in both spellings that reach it: a generator
        # naming itself, and one naming a further output of its own run.
        selfDependency = {
          a = gen { dependencies = [ "a" ]; };
          b = gen {
            dependencies = [ "b-pub" ];
            files = encrypted [ "b-pub" ];
          };
          b-pub = plain;
        };

        unknownFile.a = gen { files = encrypted [ "absent" ]; };

        selfFile.a = gen { files = encrypted [ "a" ]; };

        fileHasGenerator = {
          a = gen { files = encrypted [ "b" ]; };
          b = gen { };
        };

        fileClaimedTwice = {
          a = gen { files = encrypted [ "shared-out" ]; };
          b = gen { files = encrypted [ "shared-out" ]; };
          shared-out = plain;
        };

        unsafePromptName.a = gen { prompts."Pass Phrase" = { }; };

        # ── the 0.2 contract ──
        # `share` is derived, so authoring it is refused by name rather than
        # accepted and then contradicted by the entries.
        authoredShare.a = gen { share = true; };

        # The retired descriptor interface, in the three spellings evaluation
        # can see before anything runs.
        retiredInput.a = gen { script = ''cat "$in_seed" > "$out/a"''; };

        retiredOutputName.a = gen {
          script = ''printf '%s' "$out_name" > "$out/a"'';
        };

        noOutputReference.a = gen { script = "printf x"; };

        # A public half beside an encrypted one, which is the keypair shape this
        # contract exists for: one generator, two outputs, one of them readable
        # at evaluation.
        publicOutput = {
          keys = gen {
            script = ''
              printf priv > "$out/keys"
              printf pub > "$out/keys-pub"
            '';
            files.keys-pub.secret = false;
          };
          keys-pub = plain;
        };

        # Length two, which is what `cyclic` sees and `selfDependency` must not
        # take over.
        cycle = {
          a = gen { dependencies = [ "b" ]; };
          b = gen { dependencies = [ "a" ]; };
        };
      };

      # A generator whose two outputs land in two audiences, which needs a shared
      # catalogue entry and so a second user to carry it — a fleet rather than a
      # bare `private` record.
      disagreeing =
        let
          bo = "age1fixturebbb00000000000000000000000000000000000000000000000";
        in
        {
          catalogue.split-shared = typed types.entry { shared = true; };
          private = {
            split = gen {
              script = ''
                printf a > "$out/split"
                printf b > "$out/split-shared"
              '';
              files = encrypted [ "split-shared" ];
            };
          };
          users = {
            ana = typed types.profile {
              private = disagreeing.private;
              carries.split-shared = { };
              recipient = fixtureRecipient;
            };
            bo = typed types.profile {
              carries.split-shared = { };
              recipient = bo;
            };
          };
        };
    in
    {
      checks.safix-generators = mkStructuralCheck {
        name = "safix-generators";
        actual = {
          inherit oracle;

          validTools = unresolvedIn fixtures.valid;
          misspelledTool = unresolvedIn fixtures.misspelledTool;

          validMessages = violationsOf fixtures.valid;
          validPlans = !(planFires fixtures.valid);
          # The order the well-formed fixture resolves to, so the claim above is
          # that a plan comes back and not merely that nothing threw.
          validOrder = (resolve.generatorPlanOf (fleetOf fixtures.valid) { }).ana.order;

          crossUserMessages = violationsOf fixtures.crossUser;
          crossUserFires = planFires fixtures.crossUser;

          unknownDependencyMessages = violationsOf fixtures.unknownDependency;
          unknownDependencyFires = planFires fixtures.unknownDependency;

          selfDependencyMessages = violationsOf fixtures.selfDependency;
          selfDependencyFires = planFires fixtures.selfDependency;

          unknownFileMessages = violationsOf fixtures.unknownFile;
          unknownFileFires = planFires fixtures.unknownFile;

          selfFileMessages = violationsOf fixtures.selfFile;
          selfFileFires = planFires fixtures.selfFile;

          fileHasGeneratorMessages = violationsOf fixtures.fileHasGenerator;
          fileHasGeneratorFires = planFires fixtures.fileHasGenerator;

          fileClaimedTwiceMessages = violationsOf fixtures.fileClaimedTwice;
          fileClaimedTwiceFires = planFires fixtures.fileClaimedTwice;

          authoredShareMessages = violationsOf fixtures.authoredShare;
          authoredShareFires = planFires fixtures.authoredShare;

          retiredInputMessages = violationsOf fixtures.retiredInput;
          retiredInputFires = planFires fixtures.retiredInput;

          retiredOutputNameMessages = violationsOf fixtures.retiredOutputName;
          retiredOutputNameFires = planFires fixtures.retiredOutputName;

          noOutputReferenceMessages = violationsOf fixtures.noOutputReference;
          noOutputReferenceFires = planFires fixtures.noOutputReference;

          shareDisagreementMessages = resolve.generatorViolations disagreeing.users disagreeing.catalogue;
          shareDisagreementFires = fires (resolve.generatorPlanOf disagreeing.users disagreeing.catalogue);

          # A public output resolves to a path in the plaintext store, an
          # encrypted one to null, and the two prefixes do not overlap.
          publicMessages = violationsOf fixtures.publicOutput;
          publicPaths = resolve.publicPathsOf (fleetOf fixtures.publicOutput) { };
          publicOnTheEncryptedHalf =
            (resolve.placementsOf (fleetOf fixtures.publicOutput) { }).ana.keys.public;
          publicShare = (resolve.placementsOf (fleetOf fixtures.publicOutput) { }).ana.keys.generator.share;
          publicRuleReaches = checks.publicRuleMessages (fleetOf fixtures.publicOutput) { };

          unsafePromptNameMessages = violationsOf fixtures.unsafePromptName;
          unsafePromptNameFires = planFires fixtures.unsafePromptName;

          cycleMessages = violationsOf fixtures.cycle;
          cycleFires = planFires fixtures.cycle;
        };
        expected = {
          oracle = {
            plainPresent = true;
            dottedPresent = true;
            plainAbsent = false;
            dottedAbsent = false;
          };

          validTools = [ ];
          misspelledTool = [
            "flake.safix.users.ana's generator on 'a' names runtimeInputs 'opensll', which is not an attribute of nixpkgs"
          ];

          validMessages = [ ];
          validPlans = true;
          validOrder = [
            "root"
            "leaf"
          ];

          crossUserMessages = [
            "flake.safix.users.ana's generator on 'a' depends on 'bo/their-token', which names another person's secret. Custody here is independent: the machine running the generator holds no identity that opens another person's file, so there is no plaintext for the script to read. Give this user their own entry for that value instead."
          ];
          crossUserFires = true;

          unknownDependencyMessages = [
            "flake.safix.users.ana's generator on 'a' depends on 'absent', which flake.safix.users.ana does not hold"
          ];
          unknownDependencyFires = true;

          selfDependencyMessages = [
            "flake.safix.users.ana's generator on 'a' depends on 'a', which this same generator produces; a generator cannot read an output of its own run"
            "flake.safix.users.ana's generator on 'b' depends on 'b-pub', which this same generator produces; a generator cannot read an output of its own run"
          ];
          selfDependencyFires = true;

          unknownFileMessages = [
            "flake.safix.users.ana's generator on 'a' names 'absent' in its files, which flake.safix.users.ana does not hold"
          ];
          unknownFileFires = true;

          selfFileMessages = [
            "flake.safix.users.ana's generator on 'a' names 'a' in its own files; the entry a generator is declared on is already one of its outputs"
          ];
          selfFileFires = true;

          fileHasGeneratorMessages = [
            "flake.safix.users.ana's generator on 'a' names 'b' in its files, and 'b' carries a generator of its own. One value cannot have two producers: whichever ran last would win, and which ran last is not a declaration."
          ];
          fileHasGeneratorFires = true;

          fileClaimedTwiceMessages = [
            "flake.safix.users.ana has 'shared-out' named in the files of more than one generator: 'a' and 'b'"
          ];
          fileClaimedTwiceFires = true;

          authoredShareMessages = [
            "flake.safix.users.ana's generator on 'a' sets `share` directly, which is derived and not authored. It is true exactly when every entry the generator writes is `shared`; set `shared` on those entries instead."
          ];
          authoredShareFires = true;

          retiredInputMessages = [
            "flake.safix.users.ana's generator on 'a' references an input as $in_<name>, which was the read-once descriptor interface safix 0.1 used and 0.2 removed (openspec change 'clan-generator-contract'). A prompt is now the file $prompts/<name>; a dependency is now the file $in/<generator>/<name>, where <generator> is the entry the generator producing it is declared on. Both are re-readable, which a descriptor was not."
          ];
          retiredInputFires = true;

          retiredOutputNameMessages = [
            "flake.safix.users.ana's generator on 'a' references $out_name in its script, where it names nothing. $out_name belongs to `validation`, which is unchanged: it names the output under judgement, and the candidate still arrives on standard input. A script addresses its outputs as $out/<name>."
          ];
          retiredOutputNameFires = true;

          noOutputReferenceMessages = [
            "flake.safix.users.ana's generator on 'a' never references $out, so it would write no output file and be refused at run time with \"did not write a file for 'a'\" — a message naming the symptom rather than the cause. Under the 0.2 contract (openspec change 'clan-generator-contract') a script writes each declared output to $out/<name>; standard output is no longer the value."
          ];
          noOutputReferenceFires = true;

          shareDisagreementMessages = [
            "flake.safix.users.ana's generator on 'split' writes outputs that disagree about sharing: 'split-shared' is shared and 'split' is not. A generator's outputs resolve to one audience, so one file, so one write. Make them agree, or split this into two generators and have the second depend on the first."
          ];
          shareDisagreementFires = true;

          publicMessages = [ ];
          publicPaths = [ "public/safix/users/ana/keys-pub/value" ];
          publicOnTheEncryptedHalf = null;
          publicShare = false;
          publicRuleReaches = [ ];

          unsafePromptNameMessages = [
            "flake.safix.users.ana's generator on 'a' declares a prompt named 'Pass Phrase', which is not [a-z0-9][a-z0-9_-]* and so cannot be addressed from the script"
          ];
          unsafePromptNameFires = true;

          cycleMessages = [
            "flake.safix.users.ana declares a cycle of generators: 'a' -> 'b' -> 'a'. Nothing can run first, so nothing runs."
          ];
          cycleFires = true;
        };
      };
    };
}
