# Extract safix: the home-secret machinery as a standalone, framework-free package

Throughout this change `<dotfiles>` names the originating repository — the private nix-config flake whose `modules/flake/secrets/` tree holds the machinery being extracted — and every path given under it is relative to that repository's root.

## Why

The machinery works and it is trapped.

`<dotfiles>/modules/flake/secrets/` is a complete secrets manager: a catalogue of declarations, an audience algebra that derives which encrypted file each secret lives in, a generated `.sops.yaml` that is the recipient policy the sops CLI reads off disk, a generator system with dependency chaining and rotation cascade, and one command that is the whole lifecycle.
None of it is reachable by anyone else, because every entry point is spelled in one repository's private vocabulary.
`flake.homeSecrets` is a fleet catalogue with no namespace of its own.
`flake.users.<u>.secrets` hangs off that repository's user registry, so a consumer cannot declare a secret without first adopting a user record carrying `aggregates`, `access`, `contentPrivate`, and a dozen fields that have nothing to do with custody.
The resolver reads `flake.users.<u>.meta.ageRecipient` directly, so the identity model is not a parameter but a hard-coded path into someone else's option tree.

The alternative that exists is `clan vars`, and adopting it means adopting clan.
That is a framework decision made to obtain a secrets decision, and the two are separable.
safix is that separation: the same custody model, the same audience-derived placement, the same fail-closed policy generation, in a package that assumes nothing about how a consumer names its users, describes its hosts, or structures its flake.

The extraction is also the moment the opinions get stated as opinions rather than as incidental behaviour.
Derived placement, one file per audience, no catch-all rule, custody refusals at evaluation, values through pipes only — these are the identity of the package, and inside the source repository they read as local implementation choices that a reader could plausibly take to be adjustable.
A standalone package has to say which of its behaviours are non-negotiable, because a consumer who can configure a thing will eventually configure it wrong.

## What Changes

- A new namespace, owned by safix and decoupled from any user registry: `flake.safix.catalogue.<name>` for what a secret is, and `flake.safix.users.<u>` for who holds what. The second is safix's own user record — `recipient`, `recoveryRecipients`, `carries`, `private`, `sharedWith`, `perHost`, `perTag` — and carries nothing that is not custody. A consumer with its own user vocabulary writes an adapter that projects its records onto this one; the originating repository becomes the first such adapter rather than the shape everything else must copy.
- Dendritic by construction rather than by convention. Every option safix declares is a mergeable attrset on a flake-parts module, so a consumer may scatter declarations one per file anywhere in its tree, and safix imposes no file layout at all. The headline opinion, stated in the README and enforced by the resolver: declarations go wherever the tree wants, values go where the audience says. Scattered declarations good, scattered ciphertext never.
- The command is `safix`, with the subcommand set unchanged: `set`, `get`, `list`, `generate`, `check`, `fix`, `keygen`, `adduser`. `adduser` narrows: upstream it scaffolds safix's own user declaration and regenerates `.sops.yaml`, and nothing else. Attaching a host account is a consumer concern expressed through a hook option, because the source repository's idiom for it — a NixOS module supplying `hashedPasswordFile` — is not portable and refusing hosts that lack it is a refusal about someone else's module tree.
- The non-negotiable opinions become explicit requirements with checks behind them: placement is derived and an authored `sopsFile` is refused; `.sops.yaml` is generated, anchored per audience, terminated on the file extension, scoped to one directory level, and carries no catch-all so an unmatched path fails closed; one file per distinct audience; custody violations — a keyless recipient, a cross-user generator dependency, two mechanisms naming one audience, a name declared twice, two entries claiming one path — are evaluation errors naming the declaration; revocation is not retroactive, documented at each option where that bites; and no secret value ever reaches argv, an environment variable, or a file on the way in or out.
- Both consumption scopes are served. sops-nix has a NixOS module and a home-manager module, and a resolved safix entry materializes into either, so a secret can land on a system service or in a user profile without changing how it was declared.
- The port is staged so that each stage is verifiable: types and resolver first with their test suites, then the policy renderer, then the CLI, then the integration modules, then the documentation the guide grows into.

Not in scope: any change to the source repository, which keeps working exactly as it does until an adapter is written for it in a separate change; any new capability beyond what the machinery already has; and publishing, packaging for nixpkgs, or a release process.

## Capabilities

### New Capabilities

- `secret-catalogue`: what a secret is as a declaration — the entry vocabulary, the name alphabet, and the mergeable-attrset property that makes scattered declaration possible.
- `secret-custody`: who can read a secret — the user record, the audience algebra over `carries`, `private` and `sharedWith`, the placement scopes, and the refusals that make an incoherent custody claim an evaluation error rather than a runtime surprise.
- `recipient-policy`: the generated `.sops.yaml` — derived placement, one rule per audience, anchoring and extension termination, no catch-all, and the fail-closed behaviour of an unmatched path.
- `secret-generators`: how a value can write itself — scripts, prompts, dependencies, multi-output runs, validation, the rotation cascade, and the plaintext-handling boundary the tool can and cannot guarantee.
- `safix-cli`: the command that is the whole lifecycle, its subcommand contract, and the pipe-only value path.
- `consumer-integration`: what a consumer must supply and what safix refuses to assume — the adapter seam, the `adduser` host-attachment hook, and serving NixOS and home-manager alike.

### Modified Capabilities

None.
This repository has no specs yet; this change is its first.

## Impact

Affected code: all of it, since the repository holds only the scaffold.

- New: `modules/flake/safix/` — the option types, the resolution algebra, the policy renderer, and the CLI package, ported from `<dotfiles>/modules/flake/secrets/{_types,_resolve,_sops-yaml,lib}.nix` and `home-secret.{nix,sh}` with the namespace substituted and the fleet-specific parts removed.
- New: the python readers behind the CLI's recipient and key inspection, ported from `<dotfiles>/modules/flake/secrets/sops_{recipients,keys}.py`.
- New: the test suites, ported from `custody.test.nix`, `generators.test.nix`, `sops-yaml.test.nix` and `home-secret.test.nix`, rebased onto synthetic fixtures. Fixtures use throwaway keys minted in a scratch directory and never reference a real recipient.
- New: `flake.nix` gains the flake-parts module output a consumer imports, and the check surface those suites register into.
- Deliberately not ported: the source repository's host idioms — `--host` attachment, the `user-password-vars` refusal, the uid allocation, the `access` record — which become the hook; every clan reference, since safix replaces `clan vars` rather than interoperating with it; the runtime-extract file convention (`ops-tooling.yaml`), which is a repository convention rather than machinery, though the directory-scoped rule that lets such a file ride an existing audience is kept because it is a property of the rule shape; and the fleet-specific check harnesses that assert one particular fleet's declarations, whose synthetic-fixture halves are kept and whose literal-fleet halves are dropped.

Affected checks: the whole check surface is new. Every requirement in the delta specs that is enforceable at evaluation gets a check, and each check gets a severity drill in `tasks.md` — a perturbation that must turn it red — because a check that has never failed has not been shown to be able to.

No runtime behaviour changes anywhere, because nothing consumes safix yet.
The source repository is untouched by this change and continues to serve its own secrets through its own copy.
