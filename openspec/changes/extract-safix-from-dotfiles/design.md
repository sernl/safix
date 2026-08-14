# Design: extract safix from the home-secret machinery

## Context

The machinery being extracted is four nix files and one shell script.
`_types.nix` is the vocabulary: what an entry is, what an override is, what a scope is, what a person's profile is.
`_resolve.nix` is the algebra: which secrets a user resolves on a host, whose audience each secret has, which file that audience picks, what order the generators run in, and which declarations are refused.
`_sops-yaml.nix` renders the recipient policy from the same declarations the algebra reads, so a file, its rule, and its ciphertext stanzas cannot disagree except by hand-editing.
`lib.nix` wires those three into flake options and `flake.lib` helpers.
`home-secret.sh` is the operator surface: eight subcommands over the placements the evaluation produces.

Three things bind it to one repository, and only three.
The catalogue option is `flake.homeSecrets`, a name with no namespace.
The user surface is `flake.users.<u>.secrets`, a branch of a foreign registry.
The identity fields are `flake.users.<u>.meta.ageRecipient`, `ageRecipientNote` and `ageRecoveryRecipients`, read by path.
Everything else — the audience computation, the placement derivation, the refusals, the generator graph, the policy renderer — is already a pure function of two records.
`_resolve.nix` is explicitly parameterized by `users` and `catalogue` rather than by `config`, so its error paths can be exercised against a synthetic fleet.
The extraction is therefore mostly a renaming, and the design work is in deciding what the two records should be when nobody else's registry is available to borrow from.

## Goals / Non-Goals

Goals.
Give safix a namespace it owns outright, so that adopting it costs a consumer no vocabulary they do not already want.
Keep every opinion that makes the machinery safe, and state each one as a requirement with a check behind it rather than as prose in a file header.
Make scattered declaration a structural property rather than a style, so a consumer's tree layout is never safix's business.
Serve NixOS and home-manager from one declaration.

Non-Goals.
Interoperating with clan, which safix replaces at this scope rather than integrates with.
Supporting a second encryption backend; sops is the whole of it, and the source repository's `backend` enum was a revert path for a retirement that is already complete.
Migrating anyone, including the originating repository, which is a separate change with its own adapter.
Any new capability: this port adds none, and a feature that looks obvious during the port belongs in a change of its own.

## Decisions

### D1. The namespace is `flake.safix`, with `catalogue` and `users` beneath it

`flake.safix.catalogue.<name>` holds what a secret is: `mode`, `path`, `shared`, `generator`, `sopsKey`.
`flake.safix.users.<u>` holds who holds what: `recipient`, `recoveryRecipients`, `carries`, `private`, `sharedWith`, `perHost`, `perTag`, and the `recipientNote` D2 accounts for separately.

`safix` rather than the generic `secrets` because a flake-parts option tree is a shared namespace with no registry behind it, and a top-level `flake.secrets` is exactly the name a second module reaching for the obvious word would also claim.
A collision there is not a merge, it is two packages disagreeing about what an attribute means with the module system unable to tell them apart.
Naming the namespace after the package also keeps three things in step that a reader has to hold together anyway: the package is safix, the command is `safix`, and the options are `flake.safix.*`.
The cost is four more characters at every declaration site, which is the site that matters and is where the verbosity is paid; it buys a name that cannot be claimed out from under a consumer.

`catalogue` rather than the bare `flake.safix.<name>` because the second slot has to exist beside it, and a namespace whose top level is half free-form names and half reserved words is a namespace where adding a reserved word is a breaking change.
`users` rather than `holders` or `principals` because the thing being named is a person with a key, and inventing a word for it buys nothing but a glossary entry.

The recipient fields lose their `age` prefix and their `meta.` parent.
In the source repository `meta` separates identity from the other dozen things a user record carries; here the record carries nothing else, so the parent has no work to do.
The `age` prefix went with a `backend` field that admitted a second answer; sops with age recipients is the only answer here, so the prefix would name a distinction that no longer exists.

### D2. safix's user record carries only custody, and that is what makes the decoupling real

The record is seven fields and every one of them is about who can read what.
There is no `aggregates`, no `access`, no uid, no shell, no home directory, no group membership.
An eighth field, `recipientNote`, sits beside them and is not one of them: it holds the prose the generated policy emits above that person's key, and it is called out separately rather than folded into the seven precisely because it decides nothing about who can read what.
It exists because the generated file may not be hand-edited, so what a key is — which device holds the private half, what converts to it — has nowhere else to live.
This is the decoupling: a consumer's user record and safix's user record are different objects that happen to share a name, and safix never reaches into the consumer's.

The consequence is a small duplication that is deliberate.
A consumer with its own users writes `flake.safix.users` entries whose names match theirs, and something has to keep the two sets in step.
That something is the adapter, and it is the consumer's code.

### D3. The adapter is a projection the consumer owns, and safix ships none

An adapter is one `lib.mapAttrs` from the consumer's registry into `flake.safix.users`, plus whatever field renaming their vocabulary needs.
safix ships no adapter, not even for the originating repository, because an adapter shipped upstream is upstream taking a position on a downstream's option tree, and the next consumer's tree will differ.

What safix does ship is the guarantee that makes an adapter sufficient: `flake.safix.users` is a plain mergeable attrset option with no defaults derived from anything outside it, so a module that sets it from a `mapAttrs` and a module that sets it by hand are indistinguishable to the resolver.
The originating repository's adapter is the first one written and is written in that repository, in a change of its own, after this port lands.

The reverse direction — safix reading a consumer's registry through a configurable option path — is rejected.
It makes the option tree of every consumer part of safix's interface, turns every refusal message into a string that names a path safix cannot verify exists, and buys nothing a `mapAttrs` does not already give.

### D4. Dendritic is structural here, not a convention to follow

Every option safix declares is `attrsOf` something, on a flake-parts module.
Attrsets merge, so a consumer may declare one secret per file, scattered anywhere its tree likes, and the resolver sees one record.
safix imposes no directory layout, no naming scheme for the files that hold declarations, and no import order.

The opinion this makes room for is the one worth stating twice: declarations scatter, values do not.
A declaration is a label on a box and duplicating the vocabulary across files costs nothing; ciphertext placement is derived from the audience, so no file in the consumer's tree gets to say where a value lives.
Scattered declarations good, scattered ciphertext never.

### D5. Placement stays derived, and `sopsFile` stays in the vocabulary as the field that is refused

A sops file has one data key wrapped once per recipient, so everyone a file names reads every value in it.
Recipients are a property of the file, never of a key inside it.
Placement therefore follows the audience — a secret's owner plus everyone the owner shares it with — and one distinct audience gets one file.

An authored `sopsFile` would carry recipients that neither the audience computes nor the generated policy writes a rule for, so it is refused.
It stays declared in the vocabulary rather than deleted, because a field that is refused by name tells the author where placement comes from, whereas an unknown-option error tells them only that they were wrong.
The same reasoning retires `rekeyFile` in the source repository; here it is dropped outright, since no agenix past exists in this repository for it to point at.

### D6. The recipient policy is generated, fail-closed, and committed

`.sops.yaml` is read off the filesystem by the sops CLI, not from a nix evaluation, so it must be committed.
It is generated from the same two records the resolver reads, and `safix check` fails while the committed file and the generated one differ.

Three rule-shape properties are load-bearing and each gets its own check.
Every `path_regex` is anchored with `^`, because sops matches unanchored against the path relative to the policy file, so an unanchored rule also matches the same suffix under any prefix.
Every `path_regex` terminates on the literal extension anchored at end of string, because a rule matching files that are not safix's would let a recipient sweep silently rewrite their recipients, which is unrecoverable without the original identities.
Every `path_regex` matches exactly one directory level — `[^/]*`, never `.*` — so a file dropped in a subdirectory fails closed rather than inheriting a person's recipients.

There is no catch-all rule and the generator emits none.
An unmatched path must fail closed with sops's own no-matching-creation-rules error.
A new person is a new `flake.safix.users` entry with a recipient, and that is what produces their rule.

One directory level rather than one literal filename is deliberate: it lets a file placed beside a person's secrets ride the same custody instead of being stranded with no rule at all.
That is the only part of the source repository's runtime-extract convention that survives the port, and it survives as a property of the rule shape rather than as a convention safix knows about.

### D7. Revocation is not retroactive, and the documentation of that sits where it bites

Narrowing an audience stops future encryptions reaching someone.
It takes nothing back: they have already read every value in every file they could open, and only minting a new value revokes it.

Rotation cannot be automatic on revoke, and nothing may pretend otherwise.
A nix evaluation sees only the audience that is declared, never the audience that used to be, so no rebuild can detect a removal.
`safix fix` re-wraps each governed file's data key to the audience now declared, which aligns ciphertext with policy and is explicitly not revocation.

This statement belongs on the `recipient` and `sharedWith` option descriptions, in the generated policy file's header, and in `safix --help`, because those are the places a person is standing when the fact becomes relevant.
`recoveryRecipients` is not one of them: adding or removing a recovery identity of one's own custody is not the narrowing this statement is about, and repeating it there would dilute it at the two option sites where the choice actually is made.
`safix check` does report a shrunk audience as needing rotation rather than a re-wrap, and it derives that from the file's own recipient stanzas — an extra recipient who is no longer in the audience — so no state file records the former audience.

### D8. Custody refusals are evaluation errors, and each one names the declaration

A custody claim that cannot be satisfied must fail at evaluation, not at activation and not at the moment someone runs the command.
The refusals ported unchanged, each throwing with the offending path spelled out:

- a user who owns or is granted a secret while recording no `recipient`, since there is no key to wrap the data key for;
- a carrier of a `shared` entry recording no `recipient`, for the same reason;
- an entry that is both `shared` and named in some owner's `sharedWith`, since two statements of one audience can disagree;
- `shared = true` on a `private` entry, which has no carriers but its holder;
- a name declared in both `carries` and `private`, or granted to someone who already declares it;
- a grant naming a user that does not exist, or naming a secret the owner declares nowhere;
- a `shared` entry reached only through a `perHost` or `perTag` selection, which puts nobody in the audience;
- a generator dependency naming another person's secret, which is structural rather than policy — the machine running the generator holds no identity that opens the other person's file;
- a generator cycle, a self-dependency, and two generators producing one output;
- a generator whose inputs collide under the shell-name mapping, since `-` becomes `_` and the mapping is not injective;
- a name outside `[a-z0-9][a-z0-9_-]*`, for users, anchors, and secrets alike, since a name is interpolated into a path and into a `path_regex`;
- two entries claiming one on-disk path, since sops-nix unlinks whatever occupies a path it manages.

The audience-directory separator keeps its own guard: the separator that joins members of a shared audience into one directory name must lie outside the name alphabet, or two distinct audiences could be joined into one directory and so into one rule.

### D9. `adduser` narrows upstream, and host attachment becomes a hook

Upstream `adduser <name> <recipient>` scaffolds safix's own user declaration and regenerates `.sops.yaml`.
It mints nothing — no age key, which is `keygen` run by the person on their own machine, and no value — and it checks only the shape of the recipient, because whether anyone holds the private half is not knowable from the operator's machine.
A hardware-backed recipient is refused for the primary field and directed to `recoveryRecipients`, since activation decrypts non-interactively and a card needs a touch.

Everything the source repository's `adduser` does beyond that is host attachment: allocating a uid, writing a per-host account module, editing that host's imports, and refusing hosts that do not import a particular NixOS module.
None of it is portable.
It becomes a hook option — a consumer-supplied script or function that `adduser` invokes with the new user's name and recipient after the safix-owned scaffolding is written, whose absence means `adduser` simply does less.
A hook rather than a plugin interface because the surface is one call with two arguments, and the source repository's implementation is the proof that one call is enough.

Three details settled during the port.
`--host` survives as a flag whose only effect is to reach the hook as further arguments, and it is refused with the reason when no hook is configured, because a hostname silently discarded is worse than one that says where it would have gone.
The hook runs after safix's own commit rather than before it, so that whatever the hook writes stays uncommitted and safix's single-intent commit names only what safix did; a hook that wants its work committed commits it.
And the scaffold is written to `safix/users/<name>.nix`, a path under a directory of safix's own rather than a guess at the consumer's tree — declarations merge from anywhere, so the file resolves identically wherever it is moved to, and the epilogue says so.

### D10. Both scopes are served, from one declaration

sops-nix has a NixOS module and a home-manager module, and a resolved entry materializes into either.
The entry vocabulary is already scope-neutral: `mode`, `path`, `sopsKey` mean the same thing on both sides.

Two things differ and are handled rather than papered over.
The source registry has no owner or group field because at home scope neither backend has an ownership axis and both run as the user; at system scope sops-nix does expose `owner` and `group`, so the entry gains them as `nullOr` fields that the home-manager materialization refuses rather than silently drops.
And `path` is a function of the consuming configuration in the source registry, which is how a home path reaches `xdg.configHome`; that shape is kept, with the argument being whichever configuration is materializing.

### D11. What is deliberately not ported

The host idioms, in full: `--host`, uid allocation, the per-host account module, the `user-password-vars` refusal, and the `access`-record seam. These are D9's hook.

Every clan reference. The source files carry an extended correspondence to `clan vars` — field-for-field notes, the `share` mapping, the `migrateFact` absence — which was orientation for a reader migrating from clan inside a repository that also ran clan. safix replaces `clan vars` at this scope; a package that explains itself by reference to the thing it replaces has not finished being its own thing. The one substantive fact survives without the framing: `--help` records that `upload`, `export` and `import` do not exist, because activation already delivers what upload would and a plaintext export tree outlives the migration that justified it.

The runtime-extract file convention. `ops-tooling.yaml` is a file a repository keeps beside its declared secrets and decrypts on demand; it is a convention, not machinery, and the only machinery it depends on is the directory-scoped rule shape kept in D6.

The fleet-specific check harnesses. Each source test suite has two halves: claims against synthetic fixtures, and claims against one particular fleet's declarations. The synthetic halves port; the literal-fleet halves do not, and where a fleet claim was carrying a property worth keeping, it is restated as a synthetic fixture that exercises the same path.

The `backend` enum, `alsoProvisionedBy`, and `rekeyFile`. These were the written way back from a completed backend retirement in the source repository, and a package with no such past has no way back to preserve.

### D12. No real key, no real recipient, and no ciphertext enters this repository

Every fixture key is a throwaway minted in a scratch directory at test time or committed as a public recipient with no private half anywhere.
No ciphertext is copied from the source repository, not even as a test fixture, because a fixture that decrypts is a secret and a fixture that does not still names an audience.
The test suites therefore construct their own encrypted files where they need one, from keys they mint themselves.

## Risks / Trade-offs

The namespace is a decision made once and expensive to revisit, since every consumer declaration names it.
`flake.safix` reads longer at every declaration site than the generic `flake.secrets` would, and the verbosity is real and permanent.
Accepted, because the alternative trades a permanent small cost for an unbounded one: a generic top-level name is claimable by any other flake-parts module reaching for the same obvious word, and the module system surfaces that as a type conflict on an attribute neither package agrees about rather than as a name clash a consumer can rename their way out of.

The adapter is real work a consumer must do before they hold a single secret, and safix ships none to copy.
The mitigation is documentation: the README grows a worked adapter example, and the originating repository's adapter, once written, is referenceable.

The port is large and mostly mechanical, which is the shape of change where a rename lands in fifteen places and misses the sixteenth.
The mitigation is that the resolver is already parameterized and already has test suites; the suites port first and stay red until the code they judge is renamed.

Dev-only flake inputs are a cost a consumer pays.
`sops-nix` and `treefmt-nix` are inputs of this flake, and a consumer who does not override them fetches both.
Accepted for now, recorded as an open question, since the standard mitigations — a separate dev flake, or `follows` guidance in the README — are cheap to apply later and premature to choose now.

## Migration Plan

Nothing migrates in this change.
The originating repository keeps its own copy and continues to serve its own secrets, untouched.
Its adapter is a later change in that repository, whose shape is: write `flake.safix.users` from the existing user registry, write `flake.safix.catalogue` from the existing catalogue, verify that the generated policy is byte-identical to the committed one before deleting anything, and only then remove the local copy.
Byte-identical is the gate, because it is the one comparison that proves no audience moved.

## Open Questions

Should the dev-only inputs move out of this flake before the first consumer arrives, and if so into a `dev/` sub-flake or behind `follows` guidance alone?

Resolved during the port: the `adduser` hook is one call.
Rewriting the source repository's host attachment against it revealed no second call site — the identifier allocation, the account module and the imports edit all sit after the same point in the sequence and take the same two facts, so they compose inside one hook body rather than needing a second hook to hang off.
D9 records the three details that settled with it.
