# The resolution algebra, as pure functions of the records it reads — `users`,
# `catalogue`, `machines`, `services`, `groups`, `organizations` and `silos` —
# rather than of the flake config. ./default.nix binds them to `flake.safix.*`;
# the checks bind them to synthetic fleets, which is the only way an error path in
# here can be shown to fire.
#
# Every entry point takes those records as one attrset with the five subject
# records defaulted to empty, so a call that names none of them is exactly the
# tree that declares none. That is what makes the inertness property structural
# rather than a claim about a code path: there is one algebra, and a fleet with no
# machines, services, groups, organizations or silos travels the same one.
#
# ── one audience algebra over subjects ──
# A subject is what can hold a key and appear in an audience: a person, a
# machine, a service running on machines, a group of subjects, or an organization
# holding recovery custody of its own. A machine's recipient is the age form of
# the host identity its system scope already decrypts with, so a machine subject
# introduces no identity and no enrollment step; a group's recipients are its
# expanded membership's; a service's are its machines', so a service mints nothing
# either and the machine stays the trust boundary for everything running on it; an
# organization's are the custody keys it declares, which is the one place they are
# written and so the one place a rotation of them happens.
#
# A grant names a subject by reference, and a reference is either a subject's own
# name or `ownerOf.<machine>`, which resolves through that machine's `owner`. An
# audience is the sorted list of *elements* those references render as: a subject
# enumerated in place is its own name, and a reference resolved through a
# declaration is marked — see `audienceMarkers` — so a reader can tell from the
# path whether a file's readership is the list in front of them or a record that
# can change under it. The file is named for the elements rather than for the
# expansion, which is what makes a membership change or an ownership change a
# re-wrap of one file rather than a migration to another.
#
# ── three sources, one name space ──
# A user's secrets come from three places. `carries` selects from the catalogue,
# `private` declares entries this user holds alone, and other users'
# `sharedWith.<this user>` grants reach in from outside. All three land in one
# attrset keyed by name, so a name arriving from two of them would have one
# declaration silently win — the whole record replaced, its mode, path and key
# with it. Every such collision is a violation below, and every violation names
# both sources.
#
# ── validation is record-wide and eager ──
# `violations` reads the whole `users` record, not just the user being resolved,
# and `sourcesOf` refuses to return anything while any violation stands. A grant
# is a statement about two people, so half of every rule here lives in a record
# other than the one under resolution: a grant is wrong for reasons that live in
# the recipient's record (not a declared user, no recipient) and reasons that
# live in the owner's (does not hold the name). Scoping validation to the
# resolved user would leave a malformed grant dormant until whichever party is
# built next. The cost is that a bad declaration anywhere fails every build,
# which is the intended direction: custody declarations are not the place for a
# latent error.
#
# ── what a recipient receives ──
# The owner's record for that name, unchanged but for the file it is read from:
# the catalogue entry with the owner's `carries` override applied, or the owner's
# `private` entry. Mode, key and path are the owner's, and a recipient-side
# adjustment belongs in the recipient's own perHost/perTag.
#
# ── one file per audience ──
# An encrypted file has a single data key, wrapped once per recipient, so anyone
# who can open the file opens every value in it. Recipients are therefore a
# property of the file and never of a key inside it, and sharing two of ten
# values means splitting by audience into separate files rather than adding a
# recipient to a personal one.
#
# `audienceOf` is that split, and it reads the audience from whichever of the two
# mechanisms declares one. An unshared secret's audience is its owner plus
# everyone the owner grants it to through `sharedWith`; a catalogue entry marked
# `shared` has no owner to start from, so its audience is every user whose
# `carries` selects it. `audienceFileOf` maps each distinct audience to one file:
# a singleton audience keeps the person's own directory, a wider audience gets a
# directory named for its members, so that a reader can tell from the path alone
# who can open it.
#
# That map has to be injective, and the separator is what makes it so: see
# `audienceSeparator` below for why two audiences reaching one filename would be
# a disclosure rather than an untidiness.
#
# Placement is therefore derived, never authored: `selectFor` refuses an entry
# carrying a `sopsFile` of its own, because such a file's recipients are outside
# what the audience computes and outside what the recipient policy generates a
# rule for.
#
# ── delegation is not part of the algebra ──
# `managers` and `managedBy` are read here and reach no audience, no placement and
# no policy: `delegationIn` projects them for the verbs and `violationsIn` refuses
# either side naming what the fleet does not declare, and that is the whole of
# their effect on resolution. A fleet that declares a delegation therefore derives
# the byte-identical tree, which is the property the change adding them rests on
# and the reason they are a projection rather than a term in `audienceOf`.
#
# ── revocation, and what a rebuild cannot do ──
# Revocation is not retroactive. Removing a grant narrows the audience, so the
# secret moves back to a narrower file and future encryptions stop reaching the
# removed person. It does not take back what they already read: the value they
# saw remains theirs, and only minting a new value revokes it. Nothing here can
# rotate on revoke either — an evaluation sees only the audience that is
# declared, never the audience that used to be, so rotation is an operator
# ceremony and is never something a rebuild does.
{ lib }:
let
  mergeSets = lib.foldl' (a: b: a // b) { };

  sortNames = lib.sort (a: b: a < b);

  applyOverride = base: override: base // lib.filterAttrs (_: v: v != null) override;

  # The records the resolver reads, as one value. The five subject records
  # default to empty and the pattern is closed, so a misspelled record name is an
  # error rather than a silent fall-back to the empty one.
  registryOf =
    {
      users,
      catalogue ? { },
      machines ? { },
      services ? { },
      groups ? { },
      organizations ? { },
      silos ? { },
    }:
    {
      inherit
        users
        catalogue
        machines
        services
        groups
        organizations
        silos
        ;
    };

  holds = userRec: name: userRec.carries ? ${name} || userRec.private ? ${name};

  ownedNames =
    userRec: sortNames (builtins.attrNames userRec.carries ++ builtins.attrNames userRec.private);

  # Every recipient one person's own custody consists of: the key their
  # activation decrypts as, plus any recovery identities they hold. This is what
  # makes escrowed and independent custody a property of the person rather than
  # of a hand-written rule, and it is the person's own record alone — the keys an
  # `escrowedTo` consent adds belong to the organization and are expanded at
  # resolution time by `audienceKeysOf`.
  recipientsOf =
    userRec:
    map (r: r.key) (lib.attrValues userRec.recoveryRecipients)
    ++ lib.optional (userRec.recipient != null) userRec.recipient;

  # Every recipient one organization's custody consists of.
  custodyOf = organizationRec: map (c: c.key) (lib.attrValues organizationRec.custody);

  # Three name spaces are interpolated into generated text: a user name and an
  # anchor name into a `path_regex` and a path under the secrets directory, and a
  # secret name into the path the provisioner parks that secret at.
  # `builtins.match` anchors the whole string, so this admits nothing that could
  # act as a regex metacharacter and widen the rule it lands in — a widened rule
  # is how a recipient sweep reaches a file it was never meant to touch — and
  # nothing that could act as a path separator or walk out of the directory it is
  # meant to land in.
  #
  # The pattern is named rather than inlined because `safix adduser` has to apply
  # it too, before the name it is checking is a declared user and so before any
  # resolution here could reject it. It reads this string and anchors it itself,
  # which keeps one definition of the alphabet rather than a shell copy that
  # agrees with this one by inspection.
  nameRegex = "[a-z0-9][a-z0-9_-]*";
  wellFormedName = n: builtins.match nameRegex n != null;

  # `audienceFileOf` joins an audience with this to name one directory, and that
  # join is injective only while no name can contain the separator. Two distinct
  # audiences reaching one filename is a disclosure, not an untidiness: the file
  # gets one recipient rule, `builtins.listToAttrs` keeps whichever audience came
  # first, and every secret of the other audience then lands in a file encrypted
  # to people its owner never granted it to.
  #
  # Injectivity therefore rests on the alphabet `wellFormedName` admits, and the
  # assertion is the whole of that argument. It cannot instead rest on refusing
  # names that contain the separator: with `-and-`, the audiences [ "a" "and-b" ]
  # and [ "a-and" "b" ] both join to `a-and-and-b` while no name contains
  # `-and-`, because the separator is forgeable across an element boundary once
  # its characters are in the alphabet.
  audienceSeparator =
    let
      sep = ",";
    in
    assert lib.assertMsg (builtins.match "[a-z0-9_-]*" sep == null)
      "safix placement: the audience separator '${sep}' is drawn from the alphabet wellFormedName admits, so a user name can contain it and two distinct audiences can be joined into one directory name — one recipient rule over two audiences' secrets. Choose a separator outside [a-z0-9_-] rather than refusing names that contain this one; the separator is forgeable across an element boundary, so no refusal restores injectivity.";
    sep;

  # ── the subject name space ──
  # One name space over all five kinds. Two kinds of declaration sharing a name
  # is refused rather than resolved by precedence, because every audience element,
  # every path and every anchor is derived from the name alone: a precedence rule
  # would decide who reads a file, silently, at the point one of the two
  # declarations was written.
  isPerson = r: name: r.users ? ${name};
  isMachine = r: name: r.machines ? ${name};
  isService = r: name: r.services ? ${name};
  isGroup = r: name: r.groups ? ${name};
  isOrganization = r: name: r.organizations ? ${name};
  isSubject =
    r: name:
    isPerson r name || isMachine r name || isService r name || isGroup r name || isOrganization r name;

  # The subject kinds, as the record each is declared in paired with the option
  # path a refusal about one names. Named once so that a kind added to the model
  # reaches the name-space collision rule, the name-alphabet rule and every
  # refusal's prose rather than whichever of them was remembered.
  subjectKinds = [
    "users"
    "machines"
    "services"
    "groups"
    "organizations"
  ];

  # "flake.safix.users, flake.safix.machines, flake.safix.services,
  # flake.safix.groups or flake.safix.organizations", as every refusal about an
  # undeclared subject says it.
  declaredSubjectPaths =
    let
      paths = map (field: "flake.safix.${field}") subjectKinds;
    in
    "${lib.concatStringsSep ", " (lib.init paths)} or ${lib.last paths}";

  # The reference a grant names a subject by: the subject's own name, or the owner
  # of a machine. `.` is outside the alphabet `wellFormedName` admits, so
  # `ownerOf.<machine>` can never be a declared subject's name and the two forms
  # need no disambiguation beyond this prefix.
  ownerRefPrefix = "ownerOf.";
  isOwnerRef = ref: lib.hasPrefix ownerRefPrefix ref;
  ownerRefMachine = ref: lib.removePrefix ownerRefPrefix ref;

  # How a reference is written in the directory its audience's file sits in.
  #
  # A subject enumerated in place is its own name; a reference resolved through a
  # declaration is marked. The marker is what a reader needs: an unmarked element
  # names a readership the path states in full, and a marked one names a
  # declaration that can change the readership without changing the path — which
  # is the point, because that is what makes membership and ownership changes
  # re-wraps of one file rather than migrations to another.
  #
  # Every marker is drawn from outside the alphabet `wellFormedName` admits and
  # from outside the separator, so no name can carry one and the five element
  # forms partition by their leading characters. Two of them nest deliberately:
  # the owner marker extends the group marker with a character a group name cannot
  # start with, which is what keeps `@<group>` and `@~<machine>` distinct. The
  # service and organization markers share no prefix with either or with each
  # other, so they need no such argument, and the assertion states the general
  # rule rather than the pair it was first written for: wherever one marker is a
  # prefix of another, the remainder has to be outside the alphabet too, or a name
  # could forge the longer form under the shorter one.
  #
  # It is the same argument `audienceSeparator` makes: an ambiguous element form is
  # two audiences reaching one directory, so one recipient rule over both
  # audiences' secrets.
  #
  # One shell nuance rides the organization marker and is accepted: zsh expands a
  # word that *begins* with `=` (EQUALS expansion), so a user cd'd into shared/
  # typing `cat =acme/…` unquoted needs a quote or noequals there. As a path
  # component after `secrets/…/` — every rendered use — no shell touches it, and
  # the candidates that avoid the quirk each trip something stronger: `~` and `!`
  # expand in more shells in more positions, `+` and `.` and `^` are regex atoms
  # the policy's own path_regex would then have to escape.
  audienceMarkers =
    let
      markers = {
        group = "@";
        owner = "@~";
        service = "%";
        organization = "=";
      };
      outsideAlphabet = m: builtins.match "[a-z0-9_-]*" m == null;
      values = lib.attrValues markers;

      # The remainder of every marker that extends another, over ordered pairs
      # rather than the one nesting this started with.
      nestedTails = lib.concatMap (
        outer:
        map (inner: lib.removePrefix inner outer) (
          lib.filter (inner: inner != outer && lib.hasPrefix inner outer) values
        )
      ) values;
    in
    assert lib.assertMsg
      (
        lib.all outsideAlphabet values
        && lib.all outsideAlphabet nestedTails
        && !(lib.any (m: lib.hasInfix audienceSeparator m) values)
      )
      "safix placement: an audience marker is drawn from the alphabet wellFormedName admits or carries the audience separator, so a subject name can forge one and two distinct audiences can be joined into one directory name — one recipient rule over two audiences' secrets. Choose markers outside [a-z0-9_-], and where one extends another keep the remainder outside it too.";
    markers;

  elementOf =
    r: ref:
    if isOwnerRef ref then
      "${audienceMarkers.owner}${ownerRefMachine ref}"
    else if isService r ref then
      "${audienceMarkers.service}${ref}"
    else if isOrganization r ref then
      "${audienceMarkers.organization}${ref}"
    else if isGroup r ref then
      "${audienceMarkers.group}${ref}"
    else
      ref;

  # The inverse, which is what turns a file's audience back into the references
  # its recipients are computed from. The owner form is tested before the group
  # form because it extends it; the service and organization forms share a prefix
  # with neither, so their position among them is free.
  refOfElement =
    element:
    if lib.hasPrefix audienceMarkers.owner element then
      "${ownerRefPrefix}${lib.removePrefix audienceMarkers.owner element}"
    else if lib.hasPrefix audienceMarkers.service element then
      lib.removePrefix audienceMarkers.service element
    else if lib.hasPrefix audienceMarkers.organization element then
      lib.removePrefix audienceMarkers.organization element
    else if lib.hasPrefix audienceMarkers.group element then
      lib.removePrefix audienceMarkers.group element
    else
      element;

  isMarkedElement = element: lib.any (m: lib.hasPrefix m element) (lib.attrValues audienceMarkers);

  # Every subject a reference reaches once group membership is expanded. Groups
  # only: a service in the result is still the service it is, which is what lets
  # `leafRowsOf` say through which service each machine was reached.
  #
  # Bounded by the number of declared groups rather than recursive, so a cycle
  # among group definitions leaves groups unexpanded here instead of failing to
  # terminate. `violations` refuses that cycle by name, which is the only report
  # of it anyone should ever read; this bound is what makes that true.
  expandGroups =
    r: names:
    let
      step =
        current:
        lib.unique (lib.concatMap (n: if isGroup r n then r.groups.${n}.members else [ n ]) current);
    in
    lib.foldl' (acc: _: step acc) names (lib.range 0 (builtins.length (builtins.attrNames r.groups)));

  # One row per leaf subject a reference reaches, carrying the service the leaf
  # was reached through or null where it was reached directly.
  #
  # A group expands to its members and theirs, a service to the machines it runs
  # on, an `ownerOf` reference to the one subject the machine's record names, and
  # anything else to itself. A service closes no cycle of its own — it names
  # machines, which are leaves — so it needs no bound of its own, and neither does
  # an organization, which names keys.
  #
  # The attribution is what a service grant costs and buys. Two services on one
  # machine granted the same name are two entries rather than one silently won, and
  # the service each was reached through is what keys them apart; without it the
  # machine's resolved set could not tell them apart at all.
  leafRowsOf =
    r: ref:
    let
      machine = ownerRefMachine ref;
      seed =
        if isOwnerRef ref then
          lib.optionals (r.machines ? ${machine} && r.machines.${machine}.owner != null) [
            r.machines.${machine}.owner
          ]
        else
          [ ref ];
      expanded = expandGroups r seed;
      direct = map (leaf: {
        inherit leaf;
        service = null;
      }) (lib.filter (n: isPerson r n || isMachine r n || isOrganization r n) expanded);
      viaService = lib.concatMap (
        service:
        map (leaf: { inherit leaf service; }) (lib.filter (isMachine r) r.services.${service}.machines)
      ) (lib.filter (isService r) expanded);
    in
    direct ++ viaService;

  # Every leaf subject a reference reaches: the persons and machines whose keys a
  # file's data key is wrapped for.
  leavesOf = r: ref: sortNames (lib.unique (map (row: row.leaf) (leafRowsOf r ref)));

  # Every key one leaf subject holds of its own. A person's is their custody —
  # their own recipient and their recovery identities; a machine's is the one host
  # identity it already decrypts with, and it has no recovery axis because the
  # grant that reached it always names its owner too, whose custody is the one
  # that has a recovery story; an organization's is the custody it declares.
  #
  # A person's escrow consent is deliberately absent. This is what a subject holds,
  # which is what every refusal about a subject with no key asks; what a file gains
  # from having that subject in its audience is `audienceKeysOf`.
  subjectRecipientsOf =
    r: name:
    if isMachine r name then
      lib.optional (r.machines.${name}.recipient != null) r.machines.${name}.recipient
    else if isPerson r name then
      recipientsOf r.users.${name}
    else if isOrganization r name then
      custodyOf r.organizations.${name}
    else
      [ ];

  # The custody keys one person's `escrowedTo` adds to every file their audience
  # covers, expanded here rather than written into their `recoveryRecipients`.
  #
  # That is the whole of design decision D3. The person's record holds the consent
  # and the organization's holds the keys, so rotating one re-wraps every
  # consenting person's files and no person's declaration changes; writing the keys
  # into each person's record instead would put the rotation in as many places as
  # there are consenting people and leave nothing to say whose custody a key is.
  #
  # An organization no declaration covers contributes nothing here and is refused
  # by name in `violations`, so this is never the report of that mistake.
  escrowedKeysOf =
    r: userRec:
    lib.concatMap (o: custodyOf r.organizations.${o}) (
      lib.filter (o: isOrganization r o) userRec.escrowedTo
    );

  # Every key one leaf subject can open an audience's file with: what it holds,
  # plus — for a person — the custody of every organization they consent to the
  # escrow of.
  audienceKeysOf =
    r: name:
    subjectRecipientsOf r name ++ lib.optionals (isPerson r name) (escrowedKeysOf r r.users.${name});

  # Every key an audience's file has to be wrapped for, expanded from the elements
  # the directory is named after. Marked elements expand; unmarked ones are
  # themselves.
  audienceRecipients =
    r: audience:
    lib.unique (
      lib.concatMap (
        element: lib.concatMap (leaf: audienceKeysOf r leaf) (leavesOf r (refOfElement element))
      ) audience
    );

  # A catalogue entry's `shared` flag, asked of one person's name — which may not
  # be a catalogue entry at all. A `private` declaration is that person's own
  # value and nobody else's to carry, so it stays unshared even where a catalogue
  # entry of the same name is marked shared; without that clause a private name
  # colliding with a shared catalogue name would resolve into a file its holder
  # is not a member of.
  isShared =
    r: user: name:
    r.catalogue ? ${name} && r.catalogue.${name}.shared && !(r.users.${user}.private ? ${name});

  # Who a shared entry's audience consists of. `carries` alone, because carrying
  # is the declaration of custody: one file serves every host, so a perHost or
  # perTag selection cannot contribute a member without making the audience a
  # function of which host is being evaluated.
  carriersOf =
    r: name: sortNames (lib.filter (u: r.users.${u}.carries ? ${name}) (builtins.attrNames r.users));

  # The references one owner's `sharedWith` aims a name at.
  grantRefsOf =
    r: owner: name:
    lib.filter (ref: (r.users.${owner}.sharedWith.${ref} or { }) ? ${name}) (
      builtins.attrNames r.users.${owner}.sharedWith
    );

  # A shared entry has one audience for every carrier, so `owner` does not enter
  # it — which is the point: a catalogue entry has no owner, and deriving the
  # audience from the selection is the only statement of custody available to
  # one. An unshared name is its owner plus the elements every reference their
  # `sharedWith` names renders as.
  audienceOf =
    r: owner: name:
    if isShared r owner name then
      carriersOf r name
    else
      sortNames (lib.unique ([ owner ] ++ map (elementOf r) (grantRefsOf r owner name)));

  # One file per distinct audience. A lone unmarked element keeps that subject's
  # own directory; anything wider is named for its elements in sorted order, so
  # the path states who can open it.
  #
  # A lone marked element takes the wider form, because a directory named
  # `users/<x>` has to mean a subject's own custody: an audience that is one
  # group is a readership its own declaration decides, which is the opposite
  # claim. Nothing derives one today — an entry's owner is always in its audience
  # — and the branch is here so that the invariant does not rest on that staying
  # true.
  audienceFileOf =
    audience:
    if builtins.length audience == 1 && !(isMarkedElement (builtins.head audience)) then
      "secrets/safix/users/${builtins.head audience}/secrets.yaml"
    else
      "secrets/safix/shared/${lib.concatStringsSep audienceSeparator audience}/secrets.yaml";

  # file -> { audience; recipients; dir; }, over every secret anyone owns. Keyed
  # on the file because that is what the recipient policy writes rules for and
  # what a data key is wrapped for. A file two audiences claimed would be a
  # contradiction this cannot report — `builtins.listToAttrs` keeps the first
  # binding and drops the second silently — so it is ruled out upstream by
  # `audienceFileOf` being injective, not detected here.
  audiencesIn =
    r:
    let
      owned = lib.concatMap (owner: map (name: audienceOf r owner name) (ownedNames r.users.${owner})) (
        builtins.attrNames r.users
      );
    in
    guard r (
      lib.listToAttrs (
        map (
          audience:
          lib.nameValuePair (audienceFileOf audience) {
            inherit audience;
            dir = builtins.dirOf (audienceFileOf audience);
            recipients = audienceRecipients r audience;
          }
        ) (lib.unique owned)
      )
    );

  # user -> name -> where one person's copy of a secret is written and read: the
  # file its audience picks, the key inside that file, and which of the three
  # sources put the name in their set. Derived from `sourcesOf`, which is the
  # same three sources `audiencesOf` iterates, so the file a value is written
  # into is a file the recipient policy has generated a rule for. A surface
  # `audiencesOf` does not read — perHost and perTag `add`/`force` — is absent
  # here deliberately: those slots are host-scoped adjustments rather than
  # declarations of custody, and one file serves every host that resolves a
  # secret, so a value placed through one host's adjustment would apply
  # everywhere the secret resolves.
  #
  # `key` is the entry's `sopsKey` or, when it names none, the secret's own name
  # — the same fallback the provisioner applies. It is typed `str` rather than
  # run through `wellFormedName`, so a consumer must encode it rather than
  # interpolate it.
  #
  # `shared` rides along for the same reason `file` does: it is what the file
  # means. A shared name's file holds one value for every carrier, so a copy of
  # that name under a carrier's own per-user file is a leftover rather than
  # theirs, and `safix check` needs the flag to tell the two apart.
  #
  # `generator` rides along because this record is the whole of what the command
  # reads: one `nix eval --json` answers where a value goes and how to mint one,
  # so `generate` cannot resolve a file by one computation and a generator by
  # another and have the two disagree about which entry they mean. It is the
  # owner's generator, like every other field a recipient receives: regenerating
  # a shared secret mints one value into the one file its audience opens, which
  # is what sharing means.
  #
  # The entry read is the base record with this user's own override applied, and
  # no host scope: placement is host-independent.
  # Where a public output's plaintext lives, derived from the same audience
  # computation `audienceFileOf` uses so the two cannot place one entry in two
  # places. The leaf is a directory named for the output holding a file named
  # `value`, which is clan's shape; the prefix is a top-level sibling of the
  # ciphertext tree rather than a path inside it, so the two are separable by
  # prefix — which is what a `.gitignore`, an `rsync --exclude`, a backup policy
  # and a reviewer all actually operate on.
  publicFileOf =
    audience: name:
    if builtins.length audience == 1 && !(isMarkedElement (builtins.head audience)) then
      "public/safix/users/${builtins.head audience}/${name}/value"
    else
      "public/safix/shared/${lib.concatStringsSep audienceSeparator audience}/${name}/value";

  placementsIn =
    r:
    lib.mapAttrs (
      user: _:
      let
        sources = sourcesIn r user;
        entryOf = src: applyOverride src.base src.override;

        # Which generator, if any, declares this name as a public output. Read
        # off the same `files` record the executor reads, so "public" means one
        # thing across the two halves.
        publiclyDeclared =
          name:
          lib.any (
            other:
            let
              g = (entryOf sources.${other}).generator;
            in
            g != null && g.files ? ${name} && !g.files.${name}.secret
          ) (builtins.attrNames sources);
      in
      lib.mapAttrs (
        name: src:
        let
          entry = entryOf src;
          audience = audienceOf r src.owner name;
        in
        {
          inherit (src) origin owner;
          file = audienceFileOf audience;
          shared = isShared r src.owner name;
          key = if entry.sopsKey != null then entry.sopsKey else name;
          public = if publiclyDeclared name then publicFileOf audience name else null;
          generator =
            if entry.generator == null then
              null
            else
              entry.generator
              // {
                # Derived rather than authored, and true only when every entry
                # this generator writes agrees. A generator whose outputs
                # disagree never reaches here: `generatorViolations` refuses it,
                # naming both sides.
                share = lib.all (output: sources ? ${output} && isShared r sources.${output}.owner output) (
                  [ name ] ++ builtins.attrNames entry.generator.files
                );
              };
        }
      ) sources
    ) r.users;

  # The two accessors a consuming module reads an output through, as functions
  # of a root rather than of the flake, so a check can point them at a fixture
  # tree and watch each of the three answers happen.
  #
  # `outputPathOf` answers for every output and is a path, never a value: for a
  # secret it is the document the provisioner decrypts, for a public output the
  # file holding its bytes.
  outputPathIn =
    r: user: name:
    let
      placement = placementOrThrow r user name "has no path";
    in
    if placement.public != null then placement.public else placement.file;

  # `publicValueOf` reads the file at evaluation, which is the whole reason
  # `files.<n>.secret = false` exists: a public key or a fingerprint reaches a
  # module without a deployment-time indirection.
  #
  # Three answers, and two of them are throws. An ungenerated public output names
  # the command that would produce it, because an evaluation failing with "run
  # `safix generate alice wg-public`" is strictly better than one failing with a
  # path that is not there — that message is clan's and is copied.
  #
  # A secret output is where this departs from clan, which leaves the option
  # undefined under `mkIf (secret == false)` and so produces nix's generic
  # "option used but not defined". The cost of a stated refusal is one evaluated
  # thunk; what it buys is that the likeliest authoring mistake in this surface —
  # reaching for a value on a secret because the sibling public output has one —
  # produces a sentence saying what to do instead.
  publicValueIn =
    r: root: user: name:
    let
      placement = placementOrThrow r user name "has no value to read";
      path = "${toString root}/${placement.public}";
    in
    if placement.public == null then
      throw "safix public: '${name}' of flake.safix.users.${user} is a secret, so it has no value readable at evaluation — that is what being encrypted means. Use flake.safix.lib.outputPath \"${user}\" \"${name}\" for the path the decrypted file is placed at, or declare the output with files.${name}.secret = false if it is meant to be public."
    else if builtins.pathExists path then
      builtins.readFile path
    else
      throw "safix public: '${name}' of flake.safix.users.${user} has not been generated yet, so ${placement.public} does not exist. Run `safix generate ${user} ${name}`.";

  placementOrThrow =
    r: user: name: what:
    (placementsIn r).${user}.${name}
      or (throw "safix public: flake.safix.users.${user} holds no secret named '${name}', so it ${what}");

  # Every path the public store holds, over every user. What the recipient
  # policy is checked against: no generated creation rule may match any of them.
  publicPathsIn =
    r:
    let
      placements = placementsIn r;
    in
    sortNames (
      lib.unique (
        lib.concatMap (
          user:
          lib.concatMap (
            name: lib.optional (placements.${user}.${name}.public != null) placements.${user}.${name}.public
          ) (builtins.attrNames placements.${user})
        ) (builtins.attrNames r.users)
      )
    );

  # ── the generator graph ──
  # A generator is data on an entry; the graph over generators is what says in
  # which order they may run and whether they may run at all. Both questions are
  # answered here, at evaluation, rather than by the command that walks them,
  # because `safix generate` writes into git as it goes: a cycle found part-way
  # through a run is a run that has already committed values and cannot finish,
  # and a committed value is a distributed one.
  #
  # `producers` is the inverse of `files`. An entry carrying a generator produces
  # itself, and every name in that generator's `files` is produced by it too, so
  # a dependency on either kind of output resolves to the one generator that has
  # to run first.
  producersOf =
    placements:
    lib.foldl' (
      acc: name:
      let
        g = placements.${name}.generator;
      in
      if g == null then acc else acc // lib.genAttrs ([ name ] ++ builtins.attrNames g.files) (_: name)
    ) { } (builtins.attrNames placements);

  generatorsIn =
    placements: lib.filter (n: placements.${n}.generator != null) (builtins.attrNames placements);

  # Generator name -> the generators it must run after. A dependency on a secret
  # nobody generates contributes no edge: its value is already in the file, and a
  # generator waiting on a hand-set value would wait forever.
  #
  # A dependency this same generator produces contributes no edge either, and
  # that omission is why `selfDependency` in `generatorViolations` exists: a
  # self-edge would make the generator permanently unready and surface as a cycle
  # through the stuck set, so the declaration is refused by name before the graph
  # is walked rather than reported as a cycle nobody wrote.
  generatorEdges =
    placements:
    let
      producers = producersOf placements;
    in
    lib.genAttrs (generatorsIn placements) (
      n:
      lib.unique (
        lib.filter (p: p != null && p != n) (
          map (d: producers.${d} or null) placements.${n}.generator.dependencies
        )
      )
    );

  # Kahn's algorithm, deterministic because attribute iteration is sorted: each
  # round emits every generator whose prerequisites are already emitted, in name
  # order. What remains when a round emits nothing is exactly the generators
  # inside a cycle or downstream of one, and `order` is a run order for the rest.
  topoSplit =
    edges:
    let
      names = builtins.attrNames edges;
      step =
        state:
        let
          ready = lib.filter (
            n: !(builtins.elem n state.order) && lib.all (m: builtins.elem m state.order) edges.${n}
          ) names;
        in
        if ready == [ ] then state // { settled = true; } else state // { order = state.order ++ ready; };
      final = lib.foldl' (s: _: if s.settled then s else step s) {
        order = [ ];
        settled = false;
      } (lib.range 0 (builtins.length names));
    in
    {
      inherit (final) order;
      stuck = lib.filter (n: !(builtins.elem n final.order)) names;
    };

  # One concrete cycle out of the stuck set, so a refusal names the edges an
  # operator has to break rather than a set they have to search. Every stuck
  # generator has at least one stuck prerequisite — that is what being stuck
  # means — so walking from the first stuck name and always taking its first
  # stuck prerequisite revisits a name within `length stuck` steps, and the path
  # from that name's first occurrence is a cycle.
  firstCycle =
    edges: stuck:
    let
      succ = n: lib.filter (m: builtins.elem m stuck) edges.${n};
      step =
        state:
        if state.closed || succ state.at == [ ] then
          state // { closed = true; }
        else
          let
            nxt = lib.head (succ state.at);
          in
          {
            at = nxt;
            path = state.path ++ [ nxt ];
            closed = builtins.elem nxt state.path;
          };
      walked = lib.foldl' (s: _: step s) {
        at = lib.head stuck;
        path = [ (lib.head stuck) ];
        closed = false;
      } (lib.range 1 (builtins.length stuck + 1));
      repeated = lib.last walked.path;
      indices = lib.range 0 (builtins.length walked.path - 1);
      from = lib.foldl' (
        acc: i:
        if acc != null then
          acc
        else if lib.elemAt walked.path i == repeated then
          i
        else
          null
      ) null indices;
    in
    lib.drop from walked.path;

  # Every way a set of generator declarations can be wrong, in the same shape
  # `violations` uses and for the same reason: a list can be asserted against a
  # literal, a throw cannot.
  #
  # Empty while any custody declaration is invalid. A generator rule is a
  # statement about one user's resolved set, and there is no resolved set to
  # state it against until custody resolves — `placementsOf` throws through
  # `guard` rather than returning a partial answer.
  generatorViolationsIn =
    r:
    if violationsIn r != [ ] then
      [ ]
    else
      let
        placements = placementsIn r;

        perUser =
          user:
          let
            p = placements.${user};
            gens = generatorsIn p;

            # The record before `placementsIn` replaces `share` with the derived
            # value, which is the only place the authored one is still visible.
            raw = sourcesIn r user;
            authoredShareOf = n: (applyOverride raw.${n}.base raw.${n}.override).generator.share;
            producers = producersOf p;
            at = n: "flake.safix.users.${user}'s generator on '${n}'";

            deps = n: p.${n}.generator.dependencies;
            files = n: builtins.attrNames p.${n}.generator.files;
            promptNames = n: builtins.attrNames p.${n}.generator.prompts;
            script = n: p.${n}.generator.script;
            validationOf = n: p.${n}.generator.validation;
            outputsOf = n: [ n ] ++ files n;

            crossUser = lib.concatMap (
              n:
              map (
                d:
                "${at n} depends on '${d}', which names another person's secret. Custody here is independent: the machine running the generator holds no identity that opens another person's file, so there is no plaintext for the script to read. Give this user their own entry for that value instead."
              ) (lib.filter (d: lib.hasInfix "/" d) (deps n))
            ) gens;

            unknownDependency = lib.concatMap (
              n:
              map (d: "${at n} depends on '${d}', which flake.safix.users.${user} does not hold") (
                lib.filter (d: !(lib.hasInfix "/" d) && !(p ? ${d})) (deps n)
              )
            ) gens;

            # A dependency this same generator produces — the entry it is
            # declared on, or one of that entry's further `files`. It is a cycle
            # of length one, and it is the one cycle `cyclic` below cannot see:
            # `generatorEdges` drops an edge from a generator to itself, because
            # a self-edge would leave that generator permanently unready and
            # report every self-dependent declaration as a cycle through the
            # stuck set rather than as the single misdeclared name it is. Stated
            # here instead, so the refusal names the dependency, and stated at
            # evaluation like the rest: at runtime the descriptor would carry the
            # value this run is about to replace, and reading a value one is in
            # the middle of rotating is not a rotation idiom safix offers.
            selfDependency = lib.concatMap (
              n:
              map (
                d:
                "${at n} depends on '${d}', which this same generator produces; a generator cannot read an output of its own run"
              ) (lib.filter (d: (p ? ${d}) && (producers.${d} or null) == n) (deps n))
            ) gens;

            unknownFile = lib.concatMap (
              n:
              map (f: "${at n} names '${f}' in its files, which flake.safix.users.${user} does not hold") (
                lib.filter (f: !(p ? ${f})) (files n)
              )
            ) gens;

            selfFile = lib.concatMap (
              n:
              lib.optional (builtins.elem n (files n)) "${at n} names '${n}' in its own files; the entry a generator is declared on is already one of its outputs"
            ) gens;

            fileHasGenerator = lib.concatMap (
              n:
              map (
                f:
                "${at n} names '${f}' in its files, and '${f}' carries a generator of its own. One value cannot have two producers: whichever ran last would win, and which ran last is not a declaration."
              ) (lib.filter (f: f != n && p ? ${f} && p.${f}.generator != null) (files n))
            ) gens;

            fileClaimedTwice =
              let
                claims = lib.concatMap (n: map (f: { inherit n f; }) (files n)) gens;
              in
              lib.concatLists (
                lib.mapAttrsToList (
                  f: by:
                  lib.optional (builtins.length by > 1) (
                    "flake.safix.users.${user} has '${f}' named in the files of more than one generator: "
                    + lib.concatMapStringsSep " and " (c: "'${c.n}'") by
                  )
                ) (lib.groupBy (c: c.f) claims)
              );

            # A generator's outputs land in one file, which is what makes a
            # multi-output write one rename, and they land in one file only if
            # they land in one audience. Refused rather than split, and the
            # refusal names both sides because the remedy is a choice: make them
            # agree, or write two generators and have the second depend on the
            # first.
            #
            # This forbids something 0.1 permitted — one generator writing a
            # private entry for one person and a shared entry for several — and
            # that is stated rather than left to be discovered.
            shareDisagreement = lib.concatMap (
              n:
              let
                outputs = outputsOf n;
                sharedness = map (o: {
                  name = o;
                  shared = p ? ${o} && isShared r p.${o}.owner o;
                }) outputs;
                yes = lib.filter (o: o.shared) sharedness;
                no = lib.filter (o: !o.shared) sharedness;
                say = set: lib.concatMapStringsSep ", " (o: "'${o.name}'") set;
              in
              lib.optional (yes != [ ] && no != [ ]) (
                "${at n} writes outputs that disagree about sharing: ${say yes} ${
                  if builtins.length yes == 1 then "is" else "are"
                } shared and ${say no} ${
                  if builtins.length no == 1 then "is" else "are"
                } not. A generator's outputs resolve to one audience, so one file, so one write. "
                + "Make them agree, or split this into two generators and have the second depend on the first."
              )
            ) gens;

            # `share` is derived from the entries, and a second place to state
            # one fact is a second place for it to be wrong.
            authoredShare = lib.concatMap (
              n:
              lib.optional (authoredShareOf n != null) (
                "${at n} sets `share` directly, which is derived and not authored. It is true exactly when every entry the generator writes is `shared`; set `shared` on those entries instead."
              )
            ) gens;

            # ── the retired descriptor interface ──
            # Detection is a string match on a nix string, so it is total and
            # available before anything executes. Retained permanently rather
            # than deleted once the fleet has migrated: it costs a comparison
            # during evaluation, and what it prevents is a generator that
            # silently produces no value or reads an empty input.
            #
            # `bash -euo pipefail` would fail on `$in_foo` as an unbound variable
            # anyway. Both firing is not redundancy worth removing: "unbound
            # variable" names a symptom inside a script the operator did not just
            # write, while this names the interface change and the rewrite.
            retiredInput = lib.concatMap (
              n:
              lib.optional (lib.hasInfix "$in_" (script n) || lib.hasInfix "\${in_" (script n)) (
                "${at n} references an input as $in_<name>, which was the read-once descriptor interface safix 0.1 used and 0.2 removed (openspec change 'clan-generator-contract'). A prompt is now the file $prompts/<name>; a dependency is now the file $in/<generator>/<name>, where <generator> is the entry the generator producing it is declared on. Both are re-readable, which a descriptor was not."
              )
            ) gens;

            # `$out_name` is validation's, and validation is unchanged: it still
            # names the output under judgement and the candidate still arrives on
            # standard input. In a *script* it names nothing, so its only
            # plausible origin is a 0.1 validation fragment pasted into one — and
            # under `set -u` it would fail as an unbound variable naming a
            # variable the operator never wrote.
            retiredOutputName = lib.concatMap (
              n:
              lib.optional (lib.hasInfix "$out_name" (script n) || lib.hasInfix "\${out_name}" (script n)) (
                "${at n} references $out_name in its script, where it names nothing. $out_name belongs to `validation`, which is unchanged: it names the output under judgement, and the candidate still arrives on standard input. A script addresses its outputs as $out/<name>."
              )
            ) gens;

            # A script that never mentions the output directory writes no output
            # file, and would otherwise be refused at run time by a message that
            # names the symptom rather than the interface change.
            noOutputReference = lib.concatMap (
              n:
              lib.optional (!(lib.hasInfix "$out" (script n) || lib.hasInfix "\${out}" (script n))) (
                "${at n} never references $out, so it would write no output file and be refused at run time with \"did not write a file for '${n}'\" — a message naming the symptom rather than the cause. Under the 0.2 contract (openspec change 'clan-generator-contract') a script writes each declared output to $out/<name>; standard output is no longer the value."
              )
            ) gens;

            unsafePromptName = lib.concatMap (
              n:
              map (
                q:
                "${at n} declares a prompt named '${q}', which is not [a-z0-9][a-z0-9_-]* and so cannot be addressed from the script"
              ) (lib.filter (q: !(wellFormedName q)) (promptNames n))
            ) gens;

            # Judged over the edges alone, and only once the rules above pass: an
            # edge into a name this user does not hold is already reported, and
            # `generatorEdges` would resolve it to no producer and hide the cycle
            # behind a missing node. Cycles of length one are not judged here at
            # all — `generatorEdges` drops the self-edge, so `cyclic` cannot see
            # them and `selfDependency` above is what refuses them.
            structural =
              crossUser
              ++ unknownDependency
              ++ selfDependency
              ++ unknownFile
              ++ selfFile
              ++ fileHasGenerator
              ++ fileClaimedTwice
              ++ shareDisagreement
              ++ authoredShare
              ++ retiredInput
              ++ retiredOutputName
              ++ noOutputReference
              ++ unsafePromptName;

            cyclic =
              let
                split = topoSplit (generatorEdges p);
              in
              lib.optional (split.stuck != [ ]) (
                "flake.safix.users.${user} declares a cycle of generators: "
                + lib.concatMapStringsSep " -> " (n: "'${n}'") (firstCycle (generatorEdges p) split.stuck)
                + ". Nothing can run first, so nothing runs."
              );
          in
          if structural != [ ] then structural else cyclic;
      in
      lib.concatMap perUser (builtins.attrNames r.users);

  guardGenerators =
    r: value:
    let
      found = generatorViolationsIn r;
    in
    if found == [ ] then
      value
    else
      throw (
        "safix generators: ${toString (builtins.length found)} invalid declaration(s)\n"
        + lib.concatMapStrings (m: "  - ${m}\n") found
      );

  # user -> { order; outputs; inputs; }: the run plan `safix generate` walks.
  # `order` is a topological order over that user's generators, `outputs` names
  # every entry each generator writes with the entry it is declared on first, and
  # `inputs` is the script's name space — every prompt and every dependency,
  # under the identifier the script addresses it by.
  #
  # Computed here rather than in the command so there is one implementation of
  # the order and one of the refusals that make an order exist at all.
  generatorPlanIn =
    r:
    guardGenerators r (
      lib.mapAttrs (
        user: _:
        let
          mine = (placementsIn r).${user};
        in
        {
          order = (topoSplit (generatorEdges mine)).order;
          outputs = lib.genAttrs (generatorsIn mine) (
            n: [ n ] ++ builtins.attrNames mine.${n}.generator.files
          );
          # Keyed by the declared name, because that is now the name the script
          # addresses. The hyphen-to-underscore mapping the descriptor interface
          # needed is gone with it, and with it the collision it could produce: a
          # prompt and a dependency of the same name no longer share a name
          # space, because prompts live under $prompts and dependencies under
          # $in.
          inputs = lib.genAttrs (generatorsIn mine) (
            n:
            lib.listToAttrs (
              map (q: {
                name = q;
                value = {
                  kind = "prompt";
                  name = q;
                };
              }) (builtins.attrNames mine.${n}.generator.prompts)
              ++ map (d: {
                name = d;
                value = {
                  kind = "dependency";
                  name = d;
                };
              }) mine.${n}.generator.dependencies
            )
          );
        }
      ) r.users
    );

  grantsOf =
    r: owner:
    lib.concatLists (
      lib.mapAttrsToList (
        reference: granted: map (name: { inherit owner reference name; }) (builtins.attrNames granted)
      ) r.users.${owner}.sharedWith
    );

  allGrants = r: lib.concatMap (grantsOf r) (builtins.attrNames r.users);

  # Whether a grant's reference names something that resolves: a declared subject,
  # or the owner of a declared machine that records one.
  referenceResolves =
    r: ref:
    if isOwnerRef ref then
      r.machines ? ${ownerRefMachine ref} && r.machines.${ownerRefMachine ref}.owner != null
    else
      isSubject r ref;

  # The declaration a refusal about one subject points at.
  subjectPath =
    r: name:
    if isMachine r name then
      "flake.safix.machines.${name}"
    else if isService r name then
      "flake.safix.services.${name}"
    else if isGroup r name then
      "flake.safix.groups.${name}"
    else if isOrganization r name then
      "flake.safix.organizations.${name}"
    else
      "flake.safix.users.${name}";

  # The declaration a refusal about one reference points at. A reference that is
  # not its own only leaf is a group, a service or an owner record, which is the
  # whole reason this exists: an operator told that some subject cannot be
  # encrypted to has to know which declaration put them in the audience.
  referenceNoun =
    r: ref:
    if isOwnerRef ref then
      "the owner flake.safix.machines.${ownerRefMachine ref} records"
    else
      subjectPath r ref;

  grantPath =
    g:
    "flake.safix.users.${g.owner}.sharedWith.${
      if isOwnerRef g.reference then "\"${g.reference}\"" else g.reference
    }";

  # Every grant flattened to one row per leaf subject it reaches, each carrying
  # the service it was reached through and the key it resolves under.
  #
  # The owner is dropped from their own grant's reach. A group naming the person
  # who granted to it is the ordinary case — sharing with the team one is on — and
  # they already hold the name through the declaration the grant is about, so
  # counting them again would report that case as a collision with itself.
  reachesOf =
    r:
    lib.concatMap (
      g:
      map (row: g // row // { key = resolvedKey row.service g.name; }) (
        lib.filter (row: row.leaf != g.owner) (leafRowsOf r g.reference)
      )
    ) (lib.filter (g: referenceResolves r g.reference) (allGrants r));

  # The name a reached entry resolves under, which is the name the provisioner
  # parks it at. A service's entries take its own name as a leading component, so
  # two services on one machine granted one name land at two paths and the
  # provisioner's own default — a function of the name — is the service prefix,
  # with no path authored anywhere.
  #
  # The separator is safe here and is refused inside a declared name for a reason
  # that does not apply: both halves are drawn from the alphabet `wellFormedName`
  # admits, so neither can be `..` and the composed name lands one level inside the
  # directory the provisioner manages rather than walking out of it. Widening that
  # alphabet to admit `.` would break this, which is why the argument is written
  # where the name is composed.
  resolvedKey = service: name: if service == null then name else "${service}/${name}";

  # Named where a grant reached its subject through a declaration rather than by
  # naming them, and empty where it named them, so the sentence a direct grant
  # produces is the one it has always produced.
  reachClause =
    r: g:
    lib.optionalString (
      g.reference != g.leaf
    ) " with ${subjectPath r g.leaf}, reached through ${referenceNoun r g.reference}";

  # Group name -> the groups it names as members. The whole of the graph a cycle
  # can live in: a member that is not a group is a leaf and closes nothing.
  groupEdges = r: lib.mapAttrs (_g: declared: lib.filter (isGroup r) declared.members) r.groups;

  # Which silo set holds which of its groups over one leaf subject. A subject in
  # two groups of one set carries the span by itself, which is why this returns
  # every membership rather than the first.
  siloMembershipsOf =
    r: leaf:
    lib.concatLists (
      lib.mapAttrsToList (
        silo: declared:
        map (group: { inherit silo group; }) (
          lib.filter (group: isGroup r group && builtins.elem leaf (leavesOf r group)) declared.groups
        )
      ) r.silos
    );

  # ── delegation, which decides who may act and never who may read ──
  # Which organizations' silo declarations cover one group.
  #
  # A silo set is the one organizational boundary over groups this model has, and
  # it names groups rather than an owner, so the organization a set answers to is
  # the one whose managed people its groups reach. Every group in such a set is
  # that organization's to manage, including a group holding none of its people —
  # which is what a silo is for: the two sides of a boundary are administered
  # together and held apart.
  #
  # A group no silo set names is covered by nobody and is editable by anyone who
  # can commit, exactly as it was before delegation was recorded. That default is
  # deliberate and takes the safe direction: a delegation nobody declared cannot
  # refuse an edit that was always allowed.
  coveringOrganizations =
    r: group:
    let
      reaches =
        organization: g:
        lib.any (leaf: isPerson r leaf && r.users.${leaf}.managedBy == organization) (leavesOf r g);
      claims =
        organization: silo: lib.any (g: isGroup r g && reaches organization g) r.silos.${silo}.groups;
      sets = lib.filter (silo: builtins.elem group r.silos.${silo}.groups) (builtins.attrNames r.silos);
    in
    sortNames (
      lib.unique (
        lib.concatMap (silo: lib.filter (o: claims o silo) (builtins.attrNames r.organizations)) sets
      )
    );

  # What the scaffolding verbs read to know whose act they are performing.
  #
  # One record rather than three attributes, because one evaluation of one set of
  # declarations answers all of it and a second attribute would be a second answer
  # to the same question: which organization manages this person, whom that
  # organization declares as a manager, and which organizations' silo declarations
  # cover this group. A group's membership and the subject name space ride along
  # because the verb that asks the third question is the verb that edits that
  # membership, and it has to refuse a group or a subject the fleet does not
  # declare before it edits anything.
  #
  # `managedBy` carries only the people who declare one, so a fleet that declares
  # no delegation evaluates to three empty records and a name space — which is
  # what makes "the verbs consult nothing for an unmanaged target" a property of
  # the data rather than of a code path.
  delegationIn =
    r:
    guard r {
      managers = lib.mapAttrs (_o: declared: sortNames declared.managers) r.organizations;
      managedBy = lib.filterAttrs (_u: organization: organization != null) (
        lib.mapAttrs (_u: profile: profile.managedBy) r.users
      );
      groups = lib.mapAttrs (group: declared: {
        inherit (declared) members;
        organizations = coveringOrganizations r group;
      }) r.groups;
      subjects = sortNames (lib.concatMap (field: builtins.attrNames r.${field}) subjectKinds);
    };

  # Every way a set of custody declarations can be wrong, as messages rather than
  # as a throw, so that the same list can be asserted against a literal and
  # thrown from. Ordered, and deterministic: attribute iteration is sorted.
  violationsIn =
    r:
    let
      names = builtins.attrNames r.users;
      grants = allGrants r;

      # Rules below index into a resolved reference's own record, so a grant
      # naming nobody is reported and then dropped rather than turned into a
      # missing-attribute error with no message of ours on it.
      resolvable = lib.filter (g: referenceResolves r g.reference) grants;
      reaches = reachesOf r;

      # Checked first because every message and every generated path below
      # interpolates these names.
      unsafeUserName = lib.concatMap (
        user:
        lib.optional (!(wellFormedName user))
          "flake.safix.users names '${user}', which is not [a-z0-9][a-z0-9_-]* and so cannot be interpolated into a secrets path or a recipient rule's path_regex"
      ) names;

      # A machine's name, a service's and a group's reach the same generated text a
      # person's does: each becomes an audience element, and an audience element is
      # a directory component and part of a rule's path_regex. A service's reaches
      # one further place — the key its granted entries resolve under, so the last
      # directory component of the path the provisioner parks them at. A silo's
      # name is not judged here because it reaches prose alone.
      unsafeSubjectName = lib.concatMap (
        field:
        map (
          name:
          "flake.safix.${field} names '${name}', which is not [a-z0-9][a-z0-9_-]* and so cannot be interpolated into a secrets path or a recipient rule's path_regex"
        ) (lib.filter (n: !(wellFormedName n)) (builtins.attrNames r.${field}))
      ) (lib.filter (field: field != "users") subjectKinds);

      unsafeAnchorName = lib.concatMap (
        user:
        lib.concatMap (
          anchor:
          lib.optional (!(wellFormedName anchor))
            "flake.safix.users.${user}.recoveryRecipients names '${anchor}', which is not [a-z0-9][a-z0-9_-]* and so cannot be a recipient policy anchor"
        ) (builtins.attrNames r.users.${user}.recoveryRecipients)
      ) names;

      # An organization's custody keys are anchored the way a person's recovery
      # identities are, so the anchor they are defined as is judged the same way.
      unsafeCustodyAnchorName = lib.concatMap (
        organization:
        lib.concatMap (
          anchor:
          lib.optional (!(wellFormedName anchor))
            "flake.safix.organizations.${organization}.custody names '${anchor}', which is not [a-z0-9][a-z0-9_-]* and so cannot be a recipient policy anchor"
        ) (builtins.attrNames r.organizations.${organization}.custody)
      ) (builtins.attrNames r.organizations);

      # One name space over the four kinds of subject. Two declarations of one
      # name is refused rather than resolved by precedence, because a precedence
      # rule would decide who reads a file — an audience element, a directory and
      # an anchor are each derived from the name alone — silently, at the point one
      # of the two declarations was written.
      subjectDeclarations = lib.concatMap (
        field: map (name: { inherit name field; }) (builtins.attrNames r.${field})
      ) subjectKinds;

      subjectNameCollision = lib.concatLists (
        lib.mapAttrsToList (
          name: declared:
          lib.optional (builtins.length declared > 1) (
            "'${name}' is declared as more than one kind of subject, by "
            + lib.concatMapStringsSep " and " (d: "flake.safix.${d.field}") declared
            + "; people, machines, services, groups and organizations share one name space"
          )
        ) (lib.groupBy (d: d.name) subjectDeclarations)
      );

      # Every authoring surface that can put a name into a resolved set, paired
      # with where it was written, so the refusal below names the declaration
      # rather than only the name.
      #
      # `omit` is left out on purpose: its names remove entries from a resolved
      # set and reach no path, so refusing one would refuse a harmless no-op.
      secretNameSites = lib.concatMap (
        user:
        let
          profile = r.users.${user};
          site = where: declared: map (name: { inherit user where name; }) (builtins.attrNames declared);
          scopeSites =
            field:
            lib.concatLists (
              lib.mapAttrsToList (
                key: scope: site "${field}.${key}.add" scope.add ++ site "${field}.${key}.force" scope.force
              ) profile.${field}
            );
        in
        site "carries" profile.carries
        ++ site "private" profile.private
        ++ lib.concatLists (
          lib.mapAttrsToList (reference: granted: site "sharedWith.${reference}" granted) profile.sharedWith
        )
        ++ scopeSites "perHost"
        ++ scopeSites "perTag"
      ) names;

      # A secret name is the last component of the path the provisioner parks the
      # secret at when the entry declares none, and it is also the key read inside
      # the encrypted file. So a name carrying `/` puts the file outside the
      # directory the provisioner manages, and one starting `..` walks out of it.
      # That matters most for `private` and the per-host scopes, which exist so
      # that someone whose declarations are not otherwise trusted can add a secret
      # without touching the catalogue.
      unsafeSecretName = lib.concatMap (
        s:
        lib.optional (!(wellFormedName s.name))
          "flake.safix.users.${s.user}.${s.where} names '${s.name}', which is not [a-z0-9][a-z0-9_-]* and so cannot be the last component of the path the provisioner parks it at"
      ) secretNameSites;

      # Anchors are registry-wide: two declarations naming one anchor for two
      # different keys would have the generated file define it twice and every rule
      # referencing it resolve to whichever definition YAML kept. Each definition
      # carries the declaration it came from, because an organization's custody
      # anchors share the space a person's recovery anchors live in and a refusal
      # naming only the anchor would send the reader to the wrong record.
      anchorDefinitions =
        lib.concatMap (
          user:
          lib.mapAttrsToList (anchor: recovery: {
            inherit anchor;
            inherit (recovery) key;
            where = "flake.safix.users.${user}";
          }) r.users.${user}.recoveryRecipients
          ++ lib.optional (r.users.${user}.recipient != null) {
            anchor = "${user}-safix";
            key = r.users.${user}.recipient;
            where = "flake.safix.users.${user}";
          }
        ) names
        ++ lib.concatMap (
          organization:
          lib.mapAttrsToList (anchor: escrow: {
            inherit anchor;
            inherit (escrow) key;
            where = "flake.safix.organizations.${organization}";
          }) r.organizations.${organization}.custody
        ) (builtins.attrNames r.organizations);

      anchorConflict = lib.concatLists (
        lib.mapAttrsToList (
          anchor: defs:
          lib.optional (builtins.length (lib.unique (map (d: d.key) defs)) > 1) (
            "the declarations give the recipient policy anchor '${anchor}' more than one key, declared by "
            + lib.concatMapStringsSep " and " (d: d.where) defs
          )
        ) (lib.groupBy (d: d.anchor) anchorDefinitions)
      );

      # The owner side of noRecipientKey below. A secret whose owner records no
      # recipient has a file with an empty recipient list, which cannot be written
      # and no key can open.
      #
      # A shared entry is excluded because that sentence is not true of one: its
      # file is encrypted to the carriers who do hold a key, and the defect is
      # that this carrier cannot open it rather than that nobody can.
      # `keylessCarrier` says that instead.
      ownerWithoutRecipient = lib.concatMap (
        user:
        map
          (
            name:
            "flake.safix.users.${user} declares '${name}', but flake.safix.users.${user}.recipient is null, so ${
              audienceFileOf (audienceOf r user name)
            } has no recipient to encrypt it to"
          )
          (
            lib.optionals (r.users.${user}.recipient == null) (
              lib.filter (n: !(isShared r user n)) (ownedNames r.users.${user})
            )
          )
      ) names;

      # The carrier side of noRecipientKey. Carrying a shared entry puts a person
      # in its audience, and an audience member with no key is a member the data
      # key cannot be wrapped for.
      keylessCarrier = lib.concatMap (
        user:
        lib.optionals (r.users.${user}.recipient == null) (
          lib.concatMap (
            name:
            lib.optional (isShared r user name)
              "flake.safix.users.${user}.carries names '${name}', which flake.safix.catalogue.${name} shares, but flake.safix.users.${user}.recipient is null, so no copy can be encrypted to them"
          ) (sortNames (builtins.attrNames r.users.${user}.carries))
        )
      ) names;

      # Two mechanisms declaring one audience. A grant enumerates recipients and
      # `shared` derives them from the carriers; both name who opens the file,
      # they can disagree, and one file has one audience.
      sharedAndGranted = lib.concatMap (
        g:
        lib.optional (isShared r g.owner g.name) (
          "flake.safix.catalogue.${g.name} is shared, so its audience is every user whose carries names it, and ${grantPath g} shares a secret of that name as well; "
          + (
            if isPerson r g.reference then
              "drop the grant and let flake.safix.users.${g.reference}.carries say it"
            else
              "drop the grant: a shared entry's audience is the users whose carries name it, and nothing else can carry"
          )
        )
      ) resolvable;

      # `shared` is a statement about an entry's carriers, and a private entry has
      # none — nobody else can select it. Refused rather than ignored, because an
      # ignored flag reads as sharing and resolves as custody of one.
      sharedPrivateEntry = lib.concatMap (
        user:
        lib.concatMap (
          name:
          lib.optional r.users.${user}.private.${name}.shared
            "flake.safix.users.${user}.private.${name} sets shared = true, but a private entry has no carriers other than its holder; declare it in flake.safix.catalogue and let each carrier's carries select it"
        ) (sortNames (builtins.attrNames r.users.${user}.private))
      ) names;

      # One message per reference rather than per grant, because the defect is the
      # reference: a name nobody declared, a machine nobody declared, or a machine
      # whose record names no owner for the reference to resolve through.
      references = lib.concatMap (
        owner:
        map (reference: { inherit owner reference; }) (builtins.attrNames r.users.${owner}.sharedWith)
      ) names;

      unknownReference = lib.concatMap (
        ref:
        let
          machine = ownerRefMachine ref.reference;
          at = "flake.safix.users.${ref.owner}.sharedWith";
        in
        lib.optional (!(referenceResolves r ref.reference)) (
          if !(isOwnerRef ref.reference) then
            "${at} names '${ref.reference}', which is not a declared subject of ${declaredSubjectPaths}"
          else if !(r.machines ? ${machine}) then
            "${at} names the owner of '${machine}', which is not a declared machine of flake.safix.machines"
          else
            "${at} names the owner of flake.safix.machines.${machine}, which records none, so the grant resolves to nobody"
        )
      ) references;

      # An owner is a person or an organization: the two kinds of subject that hold
      # custody of their own, so the two an `ownerOf` grant can resolve through to
      # a key. Anything else would resolve the grant to something holding no
      # recipient at all.
      holdsCustody = owner: isPerson r owner || isOrganization r owner;

      ownerPaths = "flake.safix.users or an organization of flake.safix.organizations";

      machineOwner = lib.concatMap (
        machine:
        let
          owner = r.machines.${machine}.owner;
        in
        lib.optional (owner != null && !(holdsCustody owner))
          "flake.safix.machines.${machine}.owner names '${owner}', which is not a declared user of ${ownerPaths}"
      ) (builtins.attrNames r.machines);

      # A service's owner is judged on the same ground a machine's is.
      serviceOwner = lib.concatMap (
        service:
        let
          owner = r.services.${service}.owner;
        in
        lib.optional (owner != null && !(holdsCustody owner))
          "flake.safix.services.${service}.owner names '${owner}', which is not a declared user of ${ownerPaths}"
      ) (builtins.attrNames r.services);

      # ── escrow, and what an organization with no custody cannot do ──
      # Consent is the person's declaration, so both refusals about it name the
      # person's own record: an organization nobody declared, and one whose custody
      # would encrypt their files to nobody.
      undeclaredEscrow = lib.concatMap (
        user:
        map (
          organization:
          "flake.safix.users.${user}.escrowedTo names '${organization}', which is not a declared organization of flake.safix.organizations"
        ) (lib.filter (organization: !(isOrganization r organization)) r.users.${user}.escrowedTo)
      ) names;

      emptyCustody =
        organization: isOrganization r organization && r.organizations.${organization}.custody == { };

      escrowToEmptyCustody = lib.concatMap (
        user:
        map (
          organization:
          "flake.safix.users.${user}.escrowedTo names flake.safix.organizations.${organization}, whose custody is empty, so the escrow would add no recipient to any file they hold"
        ) (lib.filter emptyCustody r.users.${user}.escrowedTo)
      ) names;

      # The other two ways an organization is reached: a grant naming it, and a
      # grant resolving through a machine or service it owns. One rule over both,
      # because `reachClause` already says which of the two it was.
      emptyCustodyReach = lib.concatMap (
        g:
        lib.optional (emptyCustody g.leaf) "${grantPath g} shares '${g.name}'${reachClause r g}, but flake.safix.organizations.${g.leaf}.custody is empty, so the file would be encrypted to nobody"
      ) reaches;

      # ── delegation's two dangling references ──
      # Both sides are refused where they are written, and neither is refused for
      # anything else: a delegation adds no recipient to any file, so there is no
      # empty-custody sentence to say about one and an organization that holds no
      # key can still name managers.
      undeclaredManager = lib.concatMap (
        organization:
        map (
          person:
          "flake.safix.organizations.${organization}.managers names '${person}', which is not a declared user of flake.safix.users"
        ) (lib.filter (person: !(isPerson r person)) r.organizations.${organization}.managers)
      ) (builtins.attrNames r.organizations);

      undeclaredManagedBy = lib.concatMap (
        user:
        let
          organization = r.users.${user}.managedBy;
        in
        lib.optional (organization != null && !(isOrganization r organization))
          "flake.safix.users.${user}.managedBy names '${organization}', which is not a declared organization of flake.safix.organizations"
      ) names;

      # A principal is not a member. What an organization holds is custody of its
      # own, so a group containing one would be a people-set with a key in it, and
      # an audience wanting that key names the organization.
      organizationInGroup = lib.concatMap (
        group:
        map (
          member:
          "flake.safix.groups.${group}.members names flake.safix.organizations.${member}, which is a principal rather than a member; an audience wanting its custody names the organization"
        ) (lib.filter (isOrganization r) r.groups.${group}.members)
      ) (builtins.attrNames r.groups);

      # A service names machines the fleet declares. Refused rather than dropped:
      # the machine set is the whole of a service's audience, so a name nobody
      # declared is a recipient silently missing from every file the service reads.
      serviceMachine = lib.concatMap (
        service:
        map (
          machine:
          "flake.safix.services.${service}.machines names '${machine}', which is not a declared machine of flake.safix.machines"
        ) (lib.filter (machine: !(isMachine r machine)) r.services.${service}.machines)
      ) (builtins.attrNames r.services);

      unknownGroupMember = lib.concatMap (
        group:
        map (
          member:
          "flake.safix.groups.${group}.members names '${member}', which is not a declared subject of ${declaredSubjectPaths}"
        ) (lib.filter (member: !(isSubject r member)) r.groups.${group}.members)
      ) (builtins.attrNames r.groups);

      # A group naming itself, directly or through others. Refused by name here so
      # that the one report anybody reads names the participants: `expandSubjects`
      # is bounded rather than recursive, so a cycle would otherwise surface as a
      # group audience quietly missing members.
      groupCycle =
        let
          edges = groupEdges r;
          split = topoSplit edges;
        in
        lib.optional (split.stuck != [ ]) (
          "flake.safix.groups declares a cycle: "
          + lib.concatMapStringsSep " -> " (n: "'${n}'") (firstCycle edges split.stuck)
          + ". A membership that cannot be expanded is not a membership."
        );

      unknownSiloGroup = lib.concatMap (
        silo:
        map (
          group:
          "flake.safix.silos.${silo}.groups names '${group}', which is not a declared group of flake.safix.groups"
        ) (lib.filter (group: !(isGroup r group)) r.silos.${silo}.groups)
      ) (builtins.attrNames r.silos);

      siloRows = lib.concatMap (silo: map (group: { inherit silo group; }) r.silos.${silo}.groups) (
        builtins.attrNames r.silos
      );

      # Sets rather than pairs is what keeps the constraint linear, and this is
      # what that rests on: a group in two sets makes each set's exclusions reach
      # into the other's, so the two sets are one set that was written as two.
      groupInTwoSilos = lib.concatLists (
        lib.mapAttrsToList (
          group: rows:
          lib.optional (builtins.length rows > 1) (
            "flake.safix.groups.${group} is named by more than one silo set, "
            + lib.concatMapStringsSep " and " (row: "flake.safix.silos.${row.silo}") rows
            + ". A group in two sets closes each set's exclusions over the other's, which is one set written as two."
          )
        ) (lib.groupBy (row: row.group) siloRows)
      );

      carriedAndPrivate = lib.concatMap (
        user:
        map (
          name:
          "flake.safix.users.${user} declares '${name}' in both flake.safix.users.${user}.carries and flake.safix.users.${user}.private"
        ) (builtins.attrNames (builtins.intersectAttrs r.users.${user}.carries r.users.${user}.private))
      ) names;

      notHeld = lib.concatMap (
        g:
        lib.optional (!(holds r.users.${g.owner} g.name))
          "${grantPath g} names '${g.name}', which flake.safix.users.${g.owner} declares in neither carries nor private"
      ) resolvable;

      # A grant to a service that runs nowhere. Its own sentence rather than the
      # empty-reach one below, because the defect and the consequence are both more
      # specific: the machine set is the whole of a service's audience, so the file
      # the grant places would have no recipient at all.
      emptyServiceGrant = lib.concatMap (
        g:
        lib.optional (isService r g.reference && r.services.${g.reference}.machines == [ ])
          "${grantPath g} shares '${g.name}' with flake.safix.services.${g.reference}, whose machines is empty, so the file would be encrypted to nobody"
      ) resolvable;

      # A reference that resolves and reaches nobody but the person who wrote it.
      # An empty group and a group whose only member is its own grantor both widen
      # nothing, and a widening that widens nothing is a declaration that reads as
      # sharing and resolves as custody of one.
      #
      # Silent while a group cycle or an undeclared service machine stands, and
      # silent over a service with no machines at all. Each of those already
      # empties the expansion and is reported in its own words, and one fault
      # producing two unrelated sentences is worse than the second sentence is
      # worth.
      emptyReach = lib.optionals (groupCycle == [ ] && serviceMachine == [ ]) (
        lib.concatMap (
          g:
          lib.optional
            (
              !(isService r g.reference && r.services.${g.reference}.machines == [ ])
              && lib.filter (leaf: leaf != g.owner) (leavesOf r g.reference) == [ ]
            )
            "${grantPath g} shares '${g.name}' with ${referenceNoun r g.reference}, which reaches no subject beyond flake.safix.users.${g.owner}, so the grant widens nothing"
        ) resolvable
      );

      # An organization is excluded because the sentence is not true of one: it has
      # no `recipient` field to be null, and `emptyCustodyReach` says what is wrong
      # in the words its own declaration uses.
      noRecipientKey = lib.concatMap (
        g:
        lib.optional (!(isOrganization r g.leaf) && subjectRecipientsOf r g.leaf == [ ])
          "${grantPath g} shares '${g.name}'${reachClause r g}, but ${subjectPath r g.leaf}.recipient is null, so no copy can be encrypted to them"
      ) reaches;

      ownAndShared = lib.concatMap (
        g:
        let
          field = if r.users.${g.leaf}.private ? ${g.name} then "private" else "carries";
        in
        lib.optional (isPerson r g.leaf && holds r.users.${g.leaf} g.name)
          "flake.safix.users.${g.leaf} declares '${g.name}' in flake.safix.users.${g.leaf}.${field}, and ${grantPath g} shares a secret of that name${reachClause r g}"
      ) reaches;

      # Two grants putting one name in one subject's resolved set, where one would
      # silently win — the whole record replaced, its mode, path and key with it.
      #
      # Grouped by the key the entry resolves under rather than by its name, which
      # is what lets two services on one machine hold the same name: they resolve
      # under two keys, so neither is replaced and neither can win. Two grants of one
      # name to one service still collide, and so does a service's name against a
      # machine's own grant of the composed key, because those are one key twice.
      sharedTwice = lib.concatLists (
        lib.mapAttrsToList (
          leaf: received:
          lib.concatLists (
            lib.mapAttrsToList (
              key: from:
              lib.optional (builtins.length from > 1) (
                "${subjectPath r leaf} receives '${key}' from more than one grant: "
                + lib.concatMapStringsSep " and " grantPath from
              )
            ) (lib.groupBy (g: g.key) received)
          )
        ) (lib.groupBy (g: g.leaf) reaches)
      );

      # The silo constraint, judged where audiences are computed. A file whose
      # audience reaches two groups one silo set holds apart is a file both silos
      # can open: one data key, wrapped once per recipient, is the whole reason
      # this cannot be a read-time policy.
      crossSiloAudience = lib.concatMap (
        owner:
        lib.concatMap (
          name:
          let
            audience = audienceOf r owner name;
            leaves = lib.unique (lib.concatMap (e: leavesOf r (refOfElement e)) audience);
            spans = lib.concatMap (leaf: map (m: m // { inherit leaf; }) (siloMembershipsOf r leaf)) leaves;
            reachedBy =
              rows: group:
              sortNames (lib.unique (map (row: row.leaf) (lib.filter (row: row.group == group) rows)));
          in
          lib.concatLists (
            lib.mapAttrsToList (
              silo: rows:
              let
                groups = sortNames (lib.unique (map (row: row.group) rows));
              in
              lib.optional (builtins.length groups > 1) (
                "flake.safix.users.${owner}'s '${name}' resolves an audience spanning silo set flake.safix.silos.${silo}: "
                + lib.concatMapStringsSep " and " (
                  group: "flake.safix.groups.${group} reaches ${lib.concatStringsSep ", " (reachedBy rows group)}"
                ) groups
                + ". ${audienceFileOf audience} is one file with one data key, so it would be readable from both."
              )
            ) (lib.groupBy (row: row.silo) spans)
          )
        ) (ownedNames r.users.${owner})
      ) names;
    in
    unsafeUserName
    ++ unsafeSubjectName
    ++ unsafeAnchorName
    ++ unsafeCustodyAnchorName
    ++ unsafeSecretName
    ++ subjectNameCollision
    ++ anchorConflict
    ++ unknownReference
    ++ machineOwner
    ++ serviceOwner
    ++ serviceMachine
    ++ unknownGroupMember
    ++ organizationInGroup
    ++ groupCycle
    ++ unknownSiloGroup
    ++ groupInTwoSilos
    ++ carriedAndPrivate
    ++ notHeld
    ++ emptyServiceGrant
    ++ emptyReach
    ++ undeclaredEscrow
    ++ escrowToEmptyCustody
    ++ emptyCustodyReach
    ++ undeclaredManager
    ++ undeclaredManagedBy
    ++ noRecipientKey
    ++ ownerWithoutRecipient
    ++ keylessCarrier
    ++ ownAndShared
    ++ sharedTwice
    ++ sharedAndGranted
    ++ sharedPrivateEntry
    ++ crossSiloAudience;

  guard =
    r: value:
    let
      found = violationsIn r;
    in
    if found == [ ] then
      value
    else
      throw (
        "safix custody: ${toString (builtins.length found)} invalid declaration(s)\n"
        + lib.concatMapStrings (m: "  - ${m}\n") found
      );

  catalogueEntry =
    catalogue: user: field: name:
    catalogue.${name}
      or (throw "flake.safix.users.${user}.${field} names '${name}', which is not an entry of flake.safix.catalogue");

  # The record an owner holds for one of their own names, which is also the
  # record a recipient of it receives.
  ownEntry =
    r: user: name:
    let
      profile = r.users.${user};
    in
    if profile.private ? ${name} then
      profile.private.${name}
    else
      applyOverride (catalogueEntry r.catalogue user "carries" name) profile.carries.${name};

  # The grants reaching one subject, as the `shared` half of a resolved set. One
  # function for a person and for a machine, because a grant does not know which
  # kind it reached: what differs is only where the entries land, which is the
  # consuming module's question and not this one.
  #
  # A row reached through a service resolves under the service's key and carries
  # the service's ownership. Those fields are applied as an override, so a service
  # declaring neither leaves the owner's own record standing and resolves with the
  # scope's ordinary placement; a service declaring either replaces it, because the
  # landed file belongs to what runs, not to who declared the secret.
  inboundFor =
    r: subject:
    lib.listToAttrs (
      map (
        g:
        lib.nameValuePair g.key {
          origin = "shared";
          inherit (g) name owner service;
          base = applyOverride (ownEntry r g.owner g.name) (serviceOwnership r g.service);
          override = { };
        }
      ) (lib.filter (g: g.leaf == subject) (reachesOf r))
    );

  # What a service contributes to the entries it is granted, as a partial entry the
  # override algebra applies: its unix account and group, under the names an entry
  # carries them by.
  serviceOwnership =
    r: service:
    if service == null then
      { }
    else
      {
        owner = r.services.${service}.user;
        inherit (r.services.${service}) group;
      };

  # key -> { origin; name; service; owner; base; override; } for one user, over
  # all three sources. `base` is the unadjusted record and `override` the
  # adjustment the base layer of the scope algebra contributes, kept apart so that
  # a later scope layer replaces the override rather than compounding with it.
  #
  # `name` rides beside the key because the two part company for an entry reached
  # through a service: the key is what the provisioner parks the file under, the
  # name is what the audience and the key inside the encrypted file are computed
  # from. For a person the two are always equal, and nothing here branches on
  # which subject it is.
  sourcesIn =
    r: user:
    guard r (
      let
        profile = r.users.${user};

        carried = lib.mapAttrs (name: override: {
          origin = "carries";
          inherit name;
          service = null;
          owner = user;
          base = catalogueEntry r.catalogue user "carries" name;
          inherit override;
        }) profile.carries;

        privately = lib.mapAttrs (name: entry: {
          origin = "private";
          inherit name;
          service = null;
          owner = user;
          base = entry;
          override = { };
        }) profile.private;
      in
      carried // privately // inboundFor r user
    );

  # A machine holds exactly what has been granted to it — to the machine itself,
  # and to every service declared to run on it — with the owner's record unchanged
  # but for the ownership a service contributes. There is no `carries`, no
  # `private` and no scope algebra here: a machine declares nothing of its own, and
  # the per-host and per-tag layers are adjustments to one *person's* resolved set
  # on one host rather than statements about the host.
  machineSourcesIn = r: machine: guard r (inboundFor r machine);

  # Named rather than inline in the `throw` below so that a check can read what a
  # caller sees, and so that this refusal and `safix`'s own `refuse_unknown_user`
  # stay the same sentence: the command and a profile reach the same declarations
  # by different routes, and a person told two different things about one mistake
  # has to work out that they are one mistake.
  unknownUserMessage =
    users: user:
    ''
      safix: '${user}' is not a declared user of flake.safix.users.

      Declared users:
    ''
    + lib.concatMapStrings (u: "  - ${u}\n") (sortNames (builtins.attrNames users))
    + ''

      A profile selects with safix.user, which at user scope defaults to the
      profile's own username, so an account name that differs from the
      declaration key arrives here. Name one of the above, or declare this one in
      flake.safix.users.
    '';

  # The machine-scope counterpart, and separate prose rather than a parameterized
  # sentence: a profile naming a machine nobody declared has made a different
  # mistake from one naming a person nobody declared, and the option it names and
  # the record it should have named are both different.
  unknownMachineMessage =
    machines: machine:
    ''
      safix: '${machine}' is not a declared machine of flake.safix.machines.

      Declared machines:
    ''
    + lib.concatMapStrings (m: "  - ${m}\n") (sortNames (builtins.attrNames machines))
    + ''

      A profile selects a machine with safix.machine, which has no default: a
      machine is a subject an audience names, and safix has no host registry to
      derive one from. Name one of the above, or declare this one in
      flake.safix.machines with the age form of the host identity it already
      decrypts with.
    '';

  # Deny-wins resolution of one subject's effective secrets for one host, over the
  # union of the sources that reach them.
  #
  # The membership guard is first because every path into this file goes through
  # here — the two consumption modules by way of `materializeFor`, and any direct
  # `safix.lib` call — and an unguarded selection reports `attribute '<u>'
  # missing` against a line of this file rather than the option that named a
  # subject nobody declared.
  #
  # `machine` and `user` are alternatives rather than a pair. A profile serves one
  # subject, and which kind it serves is the consuming module's own refusal to
  # make: naming both here would be a resolution of two subjects into one set.
  selectFor =
    {
      users,
      catalogue ? { },
      machines ? { },
      services ? { },
      groups ? { },
      organizations ? { },
      silos ? { },
      root,
      user ? null,
      machine ? null,
      hostname,
      tags,
    }:
    let
      r = registryOf {
        inherit
          users
          catalogue
          machines
          services
          groups
          organizations
          silos
          ;
      };

      # Both branches read the placements of whoever owns each name, so a public
      # output is dropped from a machine's selection for the same reason it is
      # dropped from a person's: there is no ciphertext, no key and no creation
      # rule for the provisioner to decrypt.
      publicOwned = src: (placementsIn r).${src.owner}.${src.name}.public != null;

      forMachine =
        let
          sources = machineSourcesIn r machine;
        in
        lib.mapAttrs (
          key: src:
          let
            entry = src.base;
          in
          if entry.sopsFile != null then
            throw "safix placement: flake.safix.machines.${machine} resolves '${key}' with a sopsFile of its own, but safix derives every entry's file from its audience; drop it and widen the audience through flake.safix.users.${src.owner}.sharedWith instead"
          else
            entry
            // {
              sopsFile = root + "/${audienceFileOf (audienceOf r src.owner src.name)}";

              # The key inside the encrypted file is the entry's own name, which
              # parts company with the name the file is parked under as soon as a
              # service prefixes it. Left null the provisioner would read the
              # prefixed name as the key and find nothing there.
              sopsKey = if entry.sopsKey != null then entry.sopsKey else src.name;
            }
        ) (lib.filterAttrs (_key: src: !(publicOwned src)) sources);

      forUser =
        let
          profile = r.users.${user};
          sources = sourcesIn r user;

          perTagSets = field: map (t: profile.perTag.${t}.${field} or { }) tags;

          addSet = mergeSets (
            [
              (lib.mapAttrs (_n: s: s.override) sources)
              (profile.perHost.${hostname}.add or { })
            ]
            ++ perTagSets "add"
          );

          omitSet = mergeSets ([ (profile.perHost.${hostname}.omit or { }) ] ++ perTagSets "omit");

          forceSet = mergeSets ([ (profile.perHost.${hostname}.force or { }) ] ++ perTagSets "force");

          selected = removeAttrs (
            (removeAttrs addSet (builtins.attrNames omitSet)) // forceSet
          ) publicOutputs;

          # A public output is dropped from the selection, and this is the one
          # place it is dropped, so that `resolveSet`, `resolveNames` and
          # `materializeFor` cannot disagree about what a person resolves.
          #
          # The reason is what selection is for. Every name it returns is handed to
          # the secret provisioner with a `sopsFile` and a key inside it, and the
          # provisioner decrypts at activation. A public output has no ciphertext,
          # no key and no creation rule — that is what declaring it
          # `files.<n>.secret = false` means — so an entry for one is an activation
          # that fails to extract a key which will never exist.
          #
          # It is still the user's output and `flake.safix.lib.placements` still
          # names it, which is where `safix generate`, `list` and `check` read it.
          # A module reads its bytes with `flake.safix.lib.publicValue` at
          # evaluation, or its path with `outputPath`.
          publicOutputs = lib.filter (name: (placementsIn r).${user}.${name}.public != null) (
            builtins.attrNames (placementsIn r).${user}
          );
        in
        lib.mapAttrs (
          name: override:
          let
            entry = applyOverride (
              if sources ? ${name} then
                sources.${name}.base
              else
                catalogueEntry r.catalogue user "perHost/perTag" name
            ) override;

            # A name reaching the scope from nowhere else is this user's own; a name
            # from a grant carries its owner, whose sharedWith is what widens the
            # audience, so both parties derive the same file.
            owner = sources.${name}.owner or user;
          in
          if isShared r user name && !(profile.carries ? ${name}) then
            throw "safix placement: flake.safix.users.${user} resolves '${name}' on '${hostname}' through a perHost or perTag selection, but flake.safix.catalogue.${name} is shared and derives its audience from carries; one file serves every host, so a host-scoped selection would leave them reading a file they are not encrypted to. Name it in flake.safix.users.${user}.carries instead, and use perHost/perTag omit where a host should not resolve it"
          else if entry.sopsFile != null then
            throw "safix placement: flake.safix.users.${user} resolves '${name}' with a sopsFile of its own, but safix derives every entry's file from its audience; drop it and widen the audience through flake.safix.users.${owner}.sharedWith instead"
          else
            entry // { sopsFile = root + "/${audienceFileOf (audienceOf r owner name)}"; }
        ) selected;
    in
    if machine != null then
      if !(machines ? ${machine}) then throw (unknownMachineMessage machines machine) else forMachine
    else if !(users ? ${user}) then
      throw (unknownUserMessage users user)
    else
      forUser;

  # The scoped view materialized into the shape the secret provisioner's option
  # tree takes. Split into a second application because `path` is a function of
  # the configuration being materialized, which exists only inside the module the
  # caller is writing, not at flake level.
  #
  # `scope` is "system" or "user". It changes exactly one thing: whether the
  # ownership fields are carried or refused. Nothing about a declaration names a
  # scope, and nothing else here reads it.
  materializeFor =
    args: cfg:
    let
      resolved = selectFor (builtins.removeAttrs args [ "scope" ]);

      # Whichever subject the selection was made for, for the refusals below. They
      # name a declaration rather than a scope, and a machine's is a different
      # record from a person's.
      subject =
        if args.machine or null != null then
          "flake.safix.machines.${args.machine}"
        else
          "flake.safix.users.${args.user}";

      # Two entries resolving onto one path is unrecoverable rather than untidy —
      # whichever declaration activates second unlinks the first's output — and
      # the defect belongs to the declarations, not to the configuration that
      # happens to import them, so it is refused here for every user and every
      # host rather than only where a check has a table. Entries handed no path
      # park at the consuming scope's default — the system module mints
      # `<symlinkPath>/<name>` in safix's own store, the user scope keeps the
      # provisioner's — and either is a function of the name and so cannot
      # collide; only a declared path can.
      declaredPaths = lib.concatLists (
        lib.mapAttrsToList (
          name: secret:
          lib.optional (secret.path != null) {
            secret = name;
            path = secret.path cfg;
          }
        ) resolved
      );

      collisions = lib.filterAttrs (_p: claimants: builtins.length claimants > 1) (
        lib.groupBy (claim: claim.path) declaredPaths
      );

      refusePathCollisions =
        value:
        if collisions == { } then
          value
        else
          throw (
            "safix paths: ${subject} resolves two secrets onto one path${
              # A machine's resolution is host-independent — it is the host — so
              # naming one here would suggest the collision was a property of
              # where the entries landed rather than of the declarations.
              lib.optionalString (args.machine or null == null) " on '${args.hostname}'"
            }\n"
            + lib.concatStrings (
              lib.mapAttrsToList (
                path: claimants:
                "  - ${path} is claimed by ${lib.concatMapStringsSep " and " (c: "'${c.secret}'") claimants}\n"
              ) collisions
            )
          );

      # Refused rather than dropped. The user-scope provisioner has no ownership
      # axis and runs as the user, so a silently dropped `owner` reads afterwards
      # as an ownership claim that was honoured — and the file it names would sit
      # at the wrong ownership with nothing to say so.
      ownershipFields = [
        "owner"
        "group"
      ];

      # The records the selection was made from, rebuilt here so that a refusal can
      # say which declaration put an ownership field on a resolved entry. The same
      # closed pattern `selectFor` applies, over the same arguments.
      registry = registryOf (
        builtins.removeAttrs args [
          "scope"
          "root"
          "user"
          "machine"
          "hostname"
          "tags"
        ]
      );

      # Resolved key -> the service it was reached through, or null. Empty for a
      # person's resolution: a service's entries land on its machines, and a person
      # is not one.
      serviceOf =
        if args.machine or null == null then
          { }
        else
          lib.mapAttrs (_key: src: src.service) (machineSourcesIn registry args.machine);

      # An entry's ownership field, named as the service declaration spells it. A
      # service's account is `user` where an entry's is `owner`, and a refusal that
      # named the entry's spelling would send the reader to a field the service
      # record does not have.
      serviceAxis = {
        owner = "user";
        group = "group";
      };

      # Where an ownership field on a resolved entry came from. A service declaring
      # it is the case D4 extends the asymmetry to; a service declaring neither
      # leaves the owner's own record standing, and that field is the entry's as it
      # always was.
      declaredBy =
        key: field:
        let
          service = serviceOf.${key} or null;
        in
        if service != null && (serviceOwnership registry service).${field} != null then
          "flake.safix.services.${service} declares ${serviceAxis.${field}}, which '${key}' carries onto ${subject}"
        else
          "'${key}' sets ${field}";

      refuseOwnership =
        value:
        let
          offending = lib.concatLists (
            lib.mapAttrsToList (
              key: secret: map (field: declaredBy key field) (lib.filter (f: secret.${f} != null) ownershipFields)
            ) resolved
          );
        in
        if args.scope != "user" || offending == [ ] then
          value
        else
          throw (
            "safix ownership: ${subject} materializes at user scope, where the provisioner has no ownership axis and runs as the user, but "
            + lib.concatStringsSep " and " offending
            + ". Drop the field, or materialize this entry at system scope."
          );
    in
    refuseOwnership (
      refusePathCollisions (
        lib.mapAttrs (
          _n: secret:
          {
            inherit (secret) mode sopsFile;
          }
          // lib.optionalAttrs (secret.path != null) { path = secret.path cfg; }
          // lib.optionalAttrs (secret.sopsKey != null) { key = secret.sopsKey; }
          // lib.optionalAttrs (args.scope == "system" && secret.owner != null) { inherit (secret) owner; }
          // lib.optionalAttrs (args.scope == "system" && secret.group != null) { inherit (secret) group; }
        ) resolved
      )
    );

  # ── the entry points ──
  # One attrset holding the records, so a fleet that declares no subjects passes
  # none and travels the same algebra. Every pattern is closed: a misspelled
  # record name is refused rather than silently defaulted to the empty one.
  entryPoint = f: args: f (registryOf args);
in
{
  inherit
    nameRegex
    audienceFileOf
    audienceMarkers
    audienceSeparator
    elementOf
    refOfElement
    publicFileOf
    recipientsOf
    custodyOf
    selectFor
    materializeFor
    unknownUserMessage
    unknownMachineMessage
    ;

  violations = entryPoint violationsIn;
  delegationOf = entryPoint delegationIn;
  generatorViolations = entryPoint generatorViolationsIn;
  generatorPlanOf = entryPoint generatorPlanIn;
  audiencesOf = entryPoint audiencesIn;
  placementsOf = entryPoint placementsIn;
  publicPathsOf = entryPoint publicPathsIn;
  audienceOf = entryPoint audienceOf;
  outputPathOf = entryPoint outputPathIn;
  publicValueOf = entryPoint publicValueIn;
  sourcesOf =
    {
      users,
      catalogue ? { },
      machines ? { },
      services ? { },
      groups ? { },
      organizations ? { },
      silos ? { },
      user,
    }:
    sourcesIn (registryOf {
      inherit
        users
        catalogue
        machines
        services
        groups
        organizations
        silos
        ;
    }) user;
}
