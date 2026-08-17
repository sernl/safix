# Holds the custody rules of ../safix/resolve.nix against fleets built to break
# each one.
#
# Every fixture is synthetic. The resolver is parameterized by the two records
# rather than by the flake config, so this module reads nothing from
# `flakeArgs` — it does not take them — and that absence is itself the evidence
# for the decoupling: an error path here is exercised against a fleet written
# three lines above it rather than against whatever the repository happens to
# declare.
#
# Each rule is asserted twice: the message `violations` produces, against a
# literal, and that a resolution of that fleet actually throws rather than
# resolving. The pair is what binds them — the message list is the same list
# `guard` throws, so a refusal that stopped firing fails the second half and a
# refusal that fired with a message naming the wrong party fails the first.
#
# `builtins.tryEval` catches these because every refusal is a `throw`. Forcing is
# `deepSeq`: `sourcesOf` returns an attrset whose values carry the throws that
# happen per name, and weak head normal form would step over them.
#
# The fixtures are typed. A synthetic user goes through the real `profile`
# submodule and a synthetic catalogue through the real `entry` submodule, so a
# fixture cannot pass by omitting a field the option system would have supplied,
# and a rename of an option breaks this file along with the rest.
#
# Severity: proven by perturbation, one drill per claim.
# Deleting the keyless-recipient refusal from resolve.nix — dropping
# noRecipientKey from the list `violations` returns — fails this check on
# keylessRecipientMessages and keylessRecipientFires and no other field. Every
# other refusal behaves the same way.
# Weakening `guard` to report only the first violation fails `sharedTwiceFires`
# and the multi-message fixtures on the message list.
# Removing the union from the scope algebra — resolving perHost/perTag against
# `carries` alone — fails `inboundResolves`, since the shared name would vanish
# from the recipient's set.
# Having `audienceOf` ignore the catalogue's `shared` flag — returning the
# owner-plus-grantees audience unconditionally — fails `sharedPairFiles`, whose
# two carriers then resolve a file each. `unsharedPairFiles` does not move under
# that drill and must not: it is what says the flag was read rather than that
# audience-keyed placement was replaced by something that shares everything.
# Dropping `keylessCarrier` from `violations` fails `sharedKeylessCarrier` on
# both fields. `ownerWithoutRecipient` does not cover it and must not — its
# sentence is that the file has no recipient at all, which is false of a shared
# file the other carriers hold keys for, so the two rules are separate or the
# refusal reads as a fact that is not true.
# Deriving `isShared` from the catalogue alone — dropping the clause that reads
# the holder's `private` — fails this check through `sharedNameHeldPrivately`,
# and not as a differing field: `alice` holds that name privately and so does
# not carry it, which is exactly the shape the perHost/perTag refusal is worded
# for, so the fixture throws that refusal at her instead of resolving. The two
# rules interlock, and the message is the evidence — a person is told her own
# private entry was reached through a host-scoped selection she never wrote. The
# fixture has two other carriers rather than one so that the answer being
# defended is a shared file bearing two other names, not another person's own
# file.
# Setting the audience separator to one the name alphabet admits fails
# `audienceSeparator`'s own assertion before any check evaluates, so nothing here
# is reached. Joining with `-and-` directly, bypassing that assertion, is what
# exercises `audienceFilesDistinct`: both pairs collapse onto one filename,
# `distinct` flips false on each, and the two `file` projections become equal.
# That is the whole defect — one file, one rule, two audiences — and
# `listToAttrs` reports none of it.
# Dropping `unsafeSecretName` from the list `violations` returns empties
# `unsafeSecretNameMessages` and `unsafeCarriedNameMessages` and turns
# `unsafeSecretNameFires` false. The last only moves because that fixture's sole
# defect is the name: its grant names a secret the owner holds and its recipient
# records a key, so nothing else is left to stop the resolution.
# `legalSecretNameMessages` does not move under that drill and must not: it is
# what says a predicate tightened to catch a traversal has not also started
# refusing a leading digit, an underscore, or a single-character name.
# Removing the membership guard from `selectFor` fails `undeclaredUserFires`
# and `undeclaredUserMessage`. `fires` moves under that drill and is not
# redundant with the message: without the guard the selection dies as
# `attribute 'zed' missing` against a line of resolve.nix, which names no
# declaration and no option, and which `builtins.tryEval` does not catch.
# Scoping `violations` to the user under resolution rather than to the whole
# record fails `dormantMessages` and `dormantFiresForOthers`: the broken
# declaration belongs to a user no fixture resolves, which is the shape a
# per-resolution validation leaves dormant until whichever party is built next.
# Emptying any fixture fleet fails `fixtureRosters`, which is what stops a claim
# from passing by having nothing to judge.
{
  perSystem =
    { pkgs, lib, ... }:
    let
      resolve = import ../safix/resolve.nix { inherit lib; };
      types = import ../safix/types.nix { inherit lib; };
      mkStructuralCheck = import ./mk-structural-check.nix pkgs;

      sorted = lib.sort (a: b: a < b);

      typed =
        optionType: definition:
        (lib.evalModules {
          modules = [
            { options.value = lib.mkOption { type = optionType; }; }
            { value = definition; }
          ];
        }).config.value;

      catalogue = typed (lib.types.attrsOf types.entry) {
        shared-token = { };
        pinned.path = _cfg: "/home/alice/pinned";
      };

      mkUser =
        {
          recipient ? null,
          recoveryRecipients ? { },
          custody ? { },
        }:
        typed types.profile (
          custody
          // {
            inherit recipient;
            recoveryRecipients = lib.mapAttrs (_n: key: {
              inherit key;
              note = null;
            }) recoveryRecipients;
          }
        );

      fleetOf = lib.mapAttrs (_name: mkUser);

      # A recipient string that decrypts nothing anywhere: these fixtures assert
      # on the null/non-null distinction the rule tests, never on a real key.
      fixtureRecipient = "age1fixture000000000000000000000000000000000000000000000000000";

      violationsOf = users: resolve.violations { inherit users catalogue; };

      fires = e: !(builtins.tryEval (builtins.deepSeq e e)).success;

      resolvesFor =
        users: user:
        resolve.sourcesOf {
          inherit users catalogue user;
        };

      # Fixture resolutions place files under an empty root, so a resolved
      # sopsFile reads as the repository-relative path the audience derived.
      selectsAt =
        users: user: hostname: tags:
        resolve.selectFor {
          inherit
            users
            catalogue
            user
            hostname
            tags
            ;
          root = "";
        };

      selectsFor = users: user: selectsAt users user "somewhere" [ ];

      filesOf = users: user: lib.mapAttrs (_n: s: s.sopsFile) (selectsFor users user);

      # ── the control: a grant that is valid in every respect ──
      valid = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody = {
            carries.shared-token = { };
            sharedWith.bob.shared-token = { };
          };
        };
        bob.recipient = fixtureRecipient;
      };

      # All three sources at once, under one scope block. `own-note` is in no
      # catalogue, which is the point of `private`: declaring it is selecting it,
      # and nothing registry-wide has to learn the name.
      union = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody = {
            carries.shared-token = { };
            sharedWith.bob.shared-token = { };
          };
        };
        bob = {
          recipient = fixtureRecipient;
          custody = {
            carries.pinned = { };
            private.own-note = {
              mode = "0600";
              sopsKey = "note";
            };
            perHost.trimmed.omit = {
              own-note = { };
              shared-token = { };
            };
            perTag.laptop.force.own-note = { };
          };
        };
      };

      unknownRecipient = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody = {
            carries.shared-token = { };
            sharedWith.nobody.shared-token = { };
          };
        };
      };

      notHeld = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody = {
            carries.shared-token = { };
            sharedWith.bob.absent = { };
          };
        };
        bob.recipient = fixtureRecipient;
      };

      keylessRecipient = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody = {
            carries.shared-token = { };
            sharedWith.carol.shared-token = { };
          };
        };
        carol = { };
      };

      # A defect belonging to a user no fixture below resolves. `violations`
      # reads the whole record rather than the user under resolution, so this is
      # reported and every other user's resolution is refused along with it. A
      # per-resolution validation would leave it dormant until whichever party
      # is built next.
      dormant = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody.carries.shared-token = { };
        };
        carol = {
          recipient = fixtureRecipient;
          custody.sharedWith.nobody.shared-token = { };
        };
      };

      # ── the catalogue keyword ──
      # A second catalogue differing from the first in one field, so every claim
      # below is a claim about `shared` rather than about the fixture: the same
      # name, the same carriers and the same users resolve two ways depending on
      # which catalogue they are read against.
      sharedCatalogue = typed (lib.types.attrsOf types.entry) {
        shared-token.shared = true;
        pinned.path = _cfg: "/home/alice/pinned";
      };

      sharedViolationsOf =
        users:
        resolve.violations {
          inherit users;
          catalogue = sharedCatalogue;
        };

      resolvesInShared =
        users: user:
        resolve.sourcesOf {
          inherit users user;
          catalogue = sharedCatalogue;
        };

      selectsInShared =
        users: user:
        resolve.selectFor {
          inherit users user;
          catalogue = sharedCatalogue;
          hostname = "somewhere";
          tags = [ ];
          root = "";
        };

      # Two carriers and nothing else. Against the shared catalogue this is one
      # value in one file; against the unshared one it is two independent copies.
      sharedPair = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody.carries.shared-token = { };
        };
        bob = {
          recipient = fixtureRecipient;
          custody.carries.shared-token = { };
        };
      };

      # The same shared entry with one carrier. Read beside `sharedPair`, which
      # differs only by bob carrying it too, this is what says an audience
      # change is a change of file: the entry does not stay put and gain a
      # recipient, it moves. That is why widening and narrowing an audience are
      # migrations here rather than re-wraps, and why `safix fix` cannot be the
      # remedy for either.
      sharedSolo = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody.carries.shared-token = { };
        };
        bob.recipient = fixtureRecipient;
      };

      # A private entry whose name collides with a shared catalogue entry, while
      # two other people carry that entry. The holder's own declaration wins: it
      # is her value in her own file, not a resolution into an audience file that
      # bears two other names and none of her keys.
      sharedNameHeldPrivately = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody.private.shared-token = { };
        };
        bob = {
          recipient = fixtureRecipient;
          custody.carries.shared-token = { };
        };
        carol = {
          recipient = fixtureRecipient;
          custody.carries.shared-token = { };
        };
      };

      # A carrier with no key. Carrying is what puts them in the audience, so the
      # data key would have to be wrapped for a recipient that does not exist.
      sharedKeylessCarrier = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody.carries.shared-token = { };
        };
        carol.custody.carries.shared-token = { };
      };

      # Both mechanisms naming one audience. bob receives the grant and does not
      # carry the entry, so this fires on the double declaration alone and not
      # also on the own-and-granted collision.
      sharedAndGranted = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody = {
            carries.shared-token = { };
            sharedWith.bob.shared-token = { };
          };
        };
        bob.recipient = fixtureRecipient;
      };

      # `shared` on an entry that has no carriers but its holder.
      sharedPrivately = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody.private.own-note.shared = true;
        };
      };

      # Reaching a shared entry through a host-scoped selection. One file serves
      # every host, so this puts bob nowhere in the audience while resolving the
      # audience's file for him.
      sharedViaScope = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody.carries.shared-token = { };
        };
        bob = {
          recipient = fixtureRecipient;
          custody.perHost.somewhere.add.shared-token = { };
        };
      };

      carriedAndPrivate = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody = {
            carries.shared-token = { };
            private.shared-token = { };
          };
        };
      };

      ownAndShared = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody = {
            carries.shared-token = { };
            sharedWith.bob.shared-token = { };
          };
        };
        bob = {
          recipient = fixtureRecipient;
          custody.carries.shared-token = { };
        };
      };

      sharedTwice = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody = {
            carries.shared-token = { };
            sharedWith.bob.shared-token = { };
          };
        };
        dave = {
          recipient = fixtureRecipient;
          custody = {
            carries.shared-token = { };
            sharedWith.bob.shared-token = { };
          };
        };
        bob.recipient = fixtureRecipient;
      };

      # The same fleet as `valid` with the grant withdrawn: the secret narrows
      # back to its owner's own file and leaves the recipient's set entirely.
      revokedGrant = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody.carries.shared-token = { };
        };
        bob.recipient = fixtureRecipient;
      };

      ownerWithoutRecipient = fleetOf {
        carol.custody.carries.shared-token = { };
      };

      anchorConflict = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          recoveryRecipients.master = "age1fixture111111111111111111111111111111111111111111111111111";
        };
        dave = {
          recipient = fixtureRecipient;
          recoveryRecipients.master = "age1fixture222222222222222222222222222222222222222222222222222";
        };
      };

      unsafeUserName = fleetOf {
        Alice.recipient = fixtureRecipient;
      };

      # A traversal or a path separator in every authoring surface that can put a
      # name into a resolved set and that resolves without the catalogue. The
      # grant names a secret alice does hold and bob records a key, so the name rule
      # is the only one this fleet breaks — which is what makes the refusal, and
      # not some second defect, the thing that stops the resolution.
      unsafeSecretName = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody = {
            private."tokens/linear" = { };
            sharedWith.bob."tokens/linear" = { };
            perHost.somewhere.add."shared,token" = { };
            perTag.laptop.force."../escapes" = { };
          };
        };
        bob.recipient = fixtureRecipient;
      };

      # `carries` separately, and asserted on its message alone: a traversal name
      # is in no catalogue either, so its resolution throws whether or not the
      # name rule stands and a `fires` claim over it would pass vacuously. The
      # fixture above is where the refusal is shown to be what does the stopping.
      unsafeCarriedName = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody.carries."../shared-token" = { };
        };
      };

      # The legal alphabet at its edges — a leading digit, an underscore, a
      # single character — so that tightening the predicate to catch the fleet
      # above cannot quietly start refusing names it is meant to admit.
      legalSecretNames = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody.private = {
            "0-leading-digit" = { };
            under_score = { };
            a = { };
          };
        };
      };

      # Audience pairs a `-and-` join would map onto one filename. Every name is
      # well-formed and every list is sorted and distinct, so nothing else in the
      # registry would have refused them; only the separator being outside the
      # name alphabet keeps them apart. The second pair is the one that rules out
      # fixing this by refusing names that contain the separator: it forges
      # `-and-` across an element boundary while no name contains it.
      collidingAudiences = [
        {
          label = "separator inside a name";
          a = [
            "alice"
            "bob-and-carol"
          ];
          b = [
            "alice-and-bob"
            "carol"
          ];
        }
        {
          label = "separator forged across an element boundary";
          a = [
            "a"
            "and-b"
          ];
          b = [
            "a-and"
            "b"
          ];
        }
      ];

      # Not a custody violation but a placement error, in the same shape as
      # offCatalogue below: the fleet breaks no rule and still cannot resolve,
      # because safix derives every file from its audience and an entry naming
      # one of its own would carry recipients nothing computes.
      authoredSopsFile = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody.private.own-note.sopsFile = "/elsewhere/secrets.yaml";
        };
      };

      # Not a custody violation but a selection error, and the two must stay
      # distinguishable: this fleet breaks no rule and still cannot resolve.
      offCatalogue = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody.carries.absent = { };
        };
      };

      # Two entries on one path. The provisioner unlinks whatever occupies a path
      # it manages, so the second to activate deletes the first's output.
      collidingPaths = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody = {
            carries.pinned = { };
            private.shadow.path = _cfg: "/home/alice/pinned";
          };
        };
      };

      # An ownership field on an entry materialized where the provisioner has no
      # ownership axis. Refused rather than dropped: a dropped ownership field
      # reads afterwards as an ownership claim that was honoured.
      ownedEntry = fleetOf {
        alice = {
          recipient = fixtureRecipient;
          custody.private.service-key.owner = "svc";
        };
      };

      materializes =
        users: scope:
        resolve.materializeFor {
          inherit users catalogue scope;
          root = "";
          user = "alice";
          hostname = "somewhere";
          tags = [ ];
        } { };
    in
    {
      checks.safix-custody = mkStructuralCheck {
        name = "safix-custody";
        actual = {
          validViolations = violationsOf valid;
          validResolves = !(fires (resolvesFor valid "bob"));

          # An emptied fixture would otherwise let every claim below pass by
          # having nothing to judge.
          fixtureRosters = lib.mapAttrs (_n: f: sorted (builtins.attrNames f)) {
            inherit
              valid
              union
              dormant
              sharedPair
              sharedNameHeldPrivately
              sharedTwice
              ;
          };

          # The premise the keyless drills rest on, pinned so that a fixture user
          # gaining a key is a change to this table rather than a drill quietly
          # going stale.
          recipientsRecorded = lib.mapAttrs (_n: u: u.recipient != null) (
            keylessRecipient // sharedKeylessCarrier
          );

          # The shared name reaches the recipient's set with the owner's record,
          # which is the whole point of the grant.
          inboundResolves = lib.mapAttrs (_n: s: {
            inherit (s) origin owner;
            inherit (s.base) mode;
          }) (resolvesFor valid "bob");
          inboundSelected = builtins.attrNames (selectsFor valid "bob");
          outboundStillOwned = builtins.attrNames (selectsFor valid "alice");

          # A shared secret is a single ciphertext in a file both parties read,
          # not a copy in each of their own files. An encrypted file's data key
          # is wrapped once per recipient, so a copy per person would be two
          # values to rotate and two ceremonies to keep in step, and putting it
          # in either person's own file would hand the other everything that
          # person holds.
          sharedPlacement = {
            owner = filesOf valid "alice";
            recipient = filesOf valid "bob";
          };

          # Withdrawing the grant narrows the audience, so the secret goes back
          # to its owner's own file and leaves the recipient's set. What it does
          # not do is unread the value: only a new one does that.
          revokedPlacement = {
            owner = filesOf revokedGrant "alice";
            recipient = filesOf revokedGrant "bob";
          };

          # All three sources at once, each landing in the file its own audience
          # picks.
          unionPlacement = filesOf union "bob";

          # A private entry selects itself and keeps its own fields, with no
          # catalogue entry of that name anywhere.
          privateEntry =
            let
              own = (selectsFor union "bob").own-note;
            in
            {
              inherit (own) mode sopsKey;
              pathDeclared = own.path != null;
              inCatalogue = catalogue ? own-note;
            };

          # One scope block over the union: omit reaches a private entry and an
          # inbound one, and force still beats omit within a resolution.
          unionUnscoped = builtins.attrNames (selectsAt union "bob" "everywhere" [ ]);
          unionOmitted = builtins.attrNames (selectsAt union "bob" "trimmed" [ ]);
          unionForced = builtins.attrNames (selectsAt union "bob" "trimmed" [ "laptop" ]);

          unknownRecipientMessages = violationsOf unknownRecipient;
          unknownRecipientFires = fires (resolvesFor unknownRecipient "alice");

          notHeldMessages = violationsOf notHeld;
          notHeldFires = fires (resolvesFor notHeld "bob");

          keylessRecipientMessages = violationsOf keylessRecipient;
          keylessRecipientFires = fires (resolvesFor keylessRecipient "carol");

          # Validation is record-wide: the defect is carol's and alice's
          # resolution is refused by it.
          dormantMessages = violationsOf dormant;
          dormantFiresForOthers = fires (resolvesFor dormant "alice");

          sharedPairMessages = sharedViolationsOf sharedPair;
          sharedPairFiles = {
            alice = (selectsInShared sharedPair "alice").shared-token.sopsFile;
            bob = (selectsInShared sharedPair "bob").shared-token.sopsFile;
          };
          unsharedPairFiles = {
            alice = (selectsFor sharedPair "alice").shared-token.sopsFile;
            bob = (selectsFor sharedPair "bob").shared-token.sopsFile;
          };

          sharedSoloFile = (selectsInShared sharedSolo "alice").shared-token.sopsFile;

          sharedNameHeldPrivatelyFiles = {
            alice = (selectsInShared sharedNameHeldPrivately "alice").shared-token.sopsFile;
            bob = (selectsInShared sharedNameHeldPrivately "bob").shared-token.sopsFile;
          };

          sharedKeylessCarrierMessages = sharedViolationsOf sharedKeylessCarrier;
          sharedKeylessCarrierFires = fires (resolvesInShared sharedKeylessCarrier "carol");

          sharedAndGrantedMessages = sharedViolationsOf sharedAndGranted;
          sharedAndGrantedFires = fires (resolvesInShared sharedAndGranted "bob");

          sharedPrivatelyMessages = sharedViolationsOf sharedPrivately;
          sharedPrivatelyFires = fires (resolvesInShared sharedPrivately "alice");

          sharedViaScopeFires = fires (selectsInShared sharedViaScope "bob");

          carriedAndPrivateMessages = violationsOf carriedAndPrivate;
          carriedAndPrivateFires = fires (resolvesFor carriedAndPrivate "alice");

          ownAndSharedMessages = violationsOf ownAndShared;
          ownAndSharedFires = fires (resolvesFor ownAndShared "bob");

          sharedTwiceMessages = violationsOf sharedTwice;
          sharedTwiceFires = fires (resolvesFor sharedTwice "bob");

          ownerWithoutRecipientMessages = violationsOf ownerWithoutRecipient;
          ownerWithoutRecipientFires = fires (resolvesFor ownerWithoutRecipient "carol");

          anchorConflictMessages = violationsOf anchorConflict;
          anchorConflictFires = fires (resolvesFor anchorConflict "alice");

          unsafeUserNameMessages = violationsOf unsafeUserName;
          unsafeUserNameFires = fires (resolvesFor unsafeUserName "Alice");

          unsafeSecretNameMessages = violationsOf unsafeSecretName;
          unsafeSecretNameFires = fires (resolvesFor unsafeSecretName "alice");

          unsafeCarriedNameMessages = violationsOf unsafeCarriedName;

          legalSecretNameMessages = violationsOf legalSecretNames;
          legalSecretNameResolves = !(fires (resolvesFor legalSecretNames "alice"));

          # Selecting for a person nobody declared. This is not a violation of
          # the declarations — they may be entirely well-formed — but of the
          # selection made against them, so it is refused where the selection
          # happens rather than added to the list `violations` returns. Both
          # halves are asserted: that the selection refuses, and what the
          # refusal says, which is the whole of its value. The message is read
          # off the named function because `builtins.tryEval` reports that
          # something fired and never what it said.
          undeclaredUserFires = fires (selectsFor legalSecretNames "zed");
          undeclaredUserMessage = resolve.unknownUserMessage legalSecretNames "zed";

          # audienceFileOf has to be injective. Two audiences reaching one file
          # would give that file a single recipient rule naming one audience's
          # recipients while holding the other audience's secrets, and
          # audiencesOf cannot report it: listToAttrs keeps the first binding and
          # drops the second without a word. The resolved filenames are pinned
          # rather than only their inequality, so an encoding that separates
          # these two by becoming unreadable fails here as well.
          audienceFilesDistinct = map (p: {
            inherit (p) label;
            distinct = resolve.audienceFileOf p.a != resolve.audienceFileOf p.b;
            fileA = resolve.audienceFileOf p.a;
            fileB = resolve.audienceFileOf p.b;
          }) collidingAudiences;

          # The premise that claim rests on: the separator is drawn from outside
          # the alphabet wellFormedName admits, so no name can carry it and the
          # join has no other route to ambiguity.
          separatorOutsideNameAlphabet = builtins.match "[a-z0-9_-]*" resolve.audienceSeparator == null;

          # A placement error rather than a custody one, and it stays legible as
          # such: no rule is broken and the resolution still refuses.
          authoredSopsFileMessages = violationsOf authoredSopsFile;
          authoredSopsFileFires = fires (selectsFor authoredSopsFile "alice");

          # A selection error rather than a custody one, and it has to stay
          # legible as such: no rule is broken and the resolution still refuses.
          offCatalogueMessages = violationsOf offCatalogue;
          offCatalogueFires = fires (selectsFor offCatalogue "alice");

          collidingPathsMessages = violationsOf collidingPaths;
          collidingPathsFires = fires (materializes collidingPaths "system");

          # The same declaration, refused at one scope and carried at the other.
          ownedEntryFiresAtUserScope = fires (materializes ownedEntry "user");
          ownedEntryAtSystemScope = (materializes ownedEntry "system").service-key;
        };
        expected = {
          validViolations = [ ];
          validResolves = true;

          fixtureRosters = {
            valid = [
              "alice"
              "bob"
            ];
            union = [
              "alice"
              "bob"
            ];
            dormant = [
              "alice"
              "carol"
            ];
            sharedPair = [
              "alice"
              "bob"
            ];
            sharedNameHeldPrivately = [
              "alice"
              "bob"
              "carol"
            ];
            sharedTwice = [
              "alice"
              "bob"
              "dave"
            ];
          };

          recipientsRecorded = {
            alice = true;
            carol = false;
          };

          inboundResolves.shared-token = {
            origin = "shared";
            owner = "alice";
            mode = "0400";
          };
          inboundSelected = [ "shared-token" ];
          outboundStillOwned = [ "shared-token" ];

          sharedPlacement = {
            owner.shared-token = "/secrets/safix/shared/alice,bob/secrets.yaml";
            recipient.shared-token = "/secrets/safix/shared/alice,bob/secrets.yaml";
          };

          revokedPlacement = {
            owner.shared-token = "/secrets/safix/users/alice/secrets.yaml";
            recipient = { };
          };

          unionPlacement = {
            own-note = "/secrets/safix/users/bob/secrets.yaml";
            pinned = "/secrets/safix/users/bob/secrets.yaml";
            shared-token = "/secrets/safix/shared/alice,bob/secrets.yaml";
          };

          privateEntry = {
            mode = "0600";
            sopsKey = "note";
            pathDeclared = false;
            inCatalogue = false;
          };

          unionUnscoped = [
            "own-note"
            "pinned"
            "shared-token"
          ];
          unionOmitted = [ "pinned" ];
          unionForced = [
            "own-note"
            "pinned"
          ];

          unknownRecipientMessages = [
            "flake.safix.users.alice.sharedWith names 'nobody', which is not a declared subject of flake.safix.users, flake.safix.machines, flake.safix.services, flake.safix.groups or flake.safix.organizations"
          ];
          unknownRecipientFires = true;

          notHeldMessages = [
            "flake.safix.users.alice.sharedWith.bob names 'absent', which flake.safix.users.alice declares in neither carries nor private"
          ];
          notHeldFires = true;

          keylessRecipientMessages = [
            "flake.safix.users.alice.sharedWith.carol shares 'shared-token', but flake.safix.users.carol.recipient is null, so no copy can be encrypted to them"
          ];
          keylessRecipientFires = true;

          dormantMessages = [
            "flake.safix.users.carol.sharedWith names 'nobody', which is not a declared subject of flake.safix.users, flake.safix.machines, flake.safix.services, flake.safix.groups or flake.safix.organizations"
          ];
          dormantFiresForOthers = true;

          # One value: both carriers resolve one file. Against the unshared
          # catalogue the same two declarations resolve to a file each, which is
          # what `shared = false` means and what the flag changes.
          sharedPairMessages = [ ];
          sharedPairFiles = {
            alice = "/secrets/safix/shared/alice,bob/secrets.yaml";
            bob = "/secrets/safix/shared/alice,bob/secrets.yaml";
          };
          unsharedPairFiles = {
            alice = "/secrets/safix/users/alice/secrets.yaml";
            bob = "/secrets/safix/users/bob/secrets.yaml";
          };

          sharedSoloFile = "/secrets/safix/users/alice/secrets.yaml";

          sharedNameHeldPrivatelyFiles = {
            alice = "/secrets/safix/users/alice/secrets.yaml";
            bob = "/secrets/safix/shared/bob,carol/secrets.yaml";
          };

          sharedKeylessCarrierMessages = [
            "flake.safix.users.carol.carries names 'shared-token', which flake.safix.catalogue.shared-token shares, but flake.safix.users.carol.recipient is null, so no copy can be encrypted to them"
          ];
          sharedKeylessCarrierFires = true;

          sharedAndGrantedMessages = [
            "flake.safix.catalogue.shared-token is shared, so its audience is every user whose carries names it, and flake.safix.users.alice.sharedWith.bob shares a secret of that name as well; drop the grant and let flake.safix.users.bob.carries say it"
          ];
          sharedAndGrantedFires = true;

          sharedPrivatelyMessages = [
            "flake.safix.users.alice.private.own-note sets shared = true, but a private entry has no carriers other than its holder; declare it in flake.safix.catalogue and let each carrier's carries select it"
          ];
          sharedPrivatelyFires = true;

          sharedViaScopeFires = true;

          carriedAndPrivateMessages = [
            "flake.safix.users.alice declares 'shared-token' in both flake.safix.users.alice.carries and flake.safix.users.alice.private"
          ];
          carriedAndPrivateFires = true;

          ownAndSharedMessages = [
            "flake.safix.users.bob declares 'shared-token' in flake.safix.users.bob.carries, and flake.safix.users.alice.sharedWith.bob shares a secret of that name"
          ];
          ownAndSharedFires = true;

          sharedTwiceMessages = [
            "flake.safix.users.bob receives 'shared-token' from more than one grant: flake.safix.users.alice.sharedWith.bob and flake.safix.users.dave.sharedWith.bob"
          ];
          sharedTwiceFires = true;

          ownerWithoutRecipientMessages = [
            "flake.safix.users.carol declares 'shared-token', but flake.safix.users.carol.recipient is null, so secrets/safix/users/carol/secrets.yaml has no recipient to encrypt it to"
          ];
          ownerWithoutRecipientFires = true;

          anchorConflictMessages = [
            "the declarations give the recipient policy anchor 'master' more than one key, declared by flake.safix.users.alice and flake.safix.users.dave"
          ];
          anchorConflictFires = true;

          unsafeUserNameMessages = [
            "flake.safix.users names 'Alice', which is not [a-z0-9][a-z0-9_-]* and so cannot be interpolated into a secrets path or a recipient rule's path_regex"
          ];
          unsafeUserNameFires = true;

          unsafeSecretNameMessages = [
            "flake.safix.users.alice.private names 'tokens/linear', which is not [a-z0-9][a-z0-9_-]* and so cannot be the last component of the path the provisioner parks it at"
            "flake.safix.users.alice.sharedWith.bob names 'tokens/linear', which is not [a-z0-9][a-z0-9_-]* and so cannot be the last component of the path the provisioner parks it at"
            "flake.safix.users.alice.perHost.somewhere.add names 'shared,token', which is not [a-z0-9][a-z0-9_-]* and so cannot be the last component of the path the provisioner parks it at"
            "flake.safix.users.alice.perTag.laptop.force names '../escapes', which is not [a-z0-9][a-z0-9_-]* and so cannot be the last component of the path the provisioner parks it at"
          ];
          unsafeSecretNameFires = true;

          unsafeCarriedNameMessages = [
            "flake.safix.users.alice.carries names '../shared-token', which is not [a-z0-9][a-z0-9_-]* and so cannot be the last component of the path the provisioner parks it at"
          ];

          undeclaredUserFires = true;
          undeclaredUserMessage = ''
            safix: 'zed' is not a declared user of flake.safix.users.

            Declared users:
              - alice

            A profile selects with safix.user, which at user scope defaults to the
            profile's own username, so an account name that differs from the
            declaration key arrives here. Name one of the above, or declare this one in
            flake.safix.users.
          '';

          legalSecretNameMessages = [ ];
          legalSecretNameResolves = true;

          audienceFilesDistinct = [
            {
              label = "separator inside a name";
              distinct = true;
              fileA = "secrets/safix/shared/alice,bob-and-carol/secrets.yaml";
              fileB = "secrets/safix/shared/alice-and-bob,carol/secrets.yaml";
            }
            {
              label = "separator forged across an element boundary";
              distinct = true;
              fileA = "secrets/safix/shared/a,and-b/secrets.yaml";
              fileB = "secrets/safix/shared/a-and,b/secrets.yaml";
            }
          ];

          separatorOutsideNameAlphabet = true;

          authoredSopsFileMessages = [ ];
          authoredSopsFileFires = true;

          offCatalogueMessages = [ ];
          offCatalogueFires = true;

          collidingPathsMessages = [ ];
          collidingPathsFires = true;

          ownedEntryFiresAtUserScope = true;
          ownedEntryAtSystemScope = {
            mode = "0400";
            owner = "svc";
            sopsFile = "/secrets/safix/users/alice/secrets.yaml";
          };
        };
      };
    };
}
