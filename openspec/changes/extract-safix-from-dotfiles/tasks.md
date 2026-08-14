# Tasks: extract-safix-from-dotfiles

`<dotfiles>` names the originating repository, and every path under it is relative to that repository's root.
It is read-only for the whole of this change: nothing here edits, moves, or deletes anything in it.

Two disciplines hold across every section.
No real recipient, no real ciphertext, and no value from the source repository enters this repository at any point, including as a test fixture; fixtures use throwaway keys minted in a scratch directory.
Nothing here deploys, switches, or activates anything; every verification builds or evaluates.

Sections 1 through 4 port the evaluation half, and each ports its test suite before the code that suite judges, so the suite is red first and its passing means something.

## 1. Vocabulary under the new namespace

- [ ] 1.1 Port the type definitions from `<dotfiles>/modules/flake/secrets/_types.nix` into `modules/flake/safix/types.nix`, renaming the entry, override, scope, grant, and profile types onto safix's namespace
- [ ] 1.2 Drop the `backend`, `alsoProvisionedBy`, and `rekeyFile` fields with the enum behind them; record in the commit message that they were a completed retirement's revert path in the source repository and have nothing to point at here
- [ ] 1.3 Keep the field that names an encrypted file, as the field the resolver refuses by name, and rewrite its description onto safix's own placement story
- [ ] 1.4 Add the ownership fields the system scope needs, defaulting to unset, and document that the user-scope materialization refuses rather than drops them
- [ ] 1.5 Rewrite every option description that explains itself by reference to `clan vars`, keeping the substantive facts and dropping the correspondence framing
- [ ] 1.6 Declare `flake.safix.catalogue` and `flake.safix.users` in `modules/flake/safix/options.nix`, both as plain mergeable attribute sets with no default derived from outside the namespace
- [ ] 1.7 Verify: a scratch flake importing the module declares one secret in each of two separate files and evaluates to one record

## 2. Resolution algebra

- [ ] 2.1 Port the custody test suite from `<dotfiles>/modules/flake/secrets/custody.test.nix`, keeping the claims made against synthetic fixtures and dropping the claims made against one particular fleet's declarations
- [ ] 2.2 For each dropped fleet claim, decide whether it carried a property worth keeping and, where it did, restate it as a synthetic fixture exercising the same path; list the dropped claims and the restatements in the commit message
- [ ] 2.3 Port the algebra from `<dotfiles>/modules/flake/secrets/_resolve.nix` into `modules/flake/safix/resolve.nix`, substituting the namespace throughout, until the suite from 2.1 is green
- [ ] 2.4 Confirm every refusal message names a path in safix's namespace and no message names an option path belonging to any consumer
- [ ] 2.5 Confirm the resolver takes the two records as arguments rather than reading configuration, which is what lets the suite exercise its error paths against synthetic fleets
- [ ] 2.6 Port the generator suite from `<dotfiles>/modules/flake/secrets/generators.test.nix` under the same keep-synthetic rule, and make it green
- [ ] 2.7 Verify: every refusal enumerated in design.md D8 has a test that asserts it fires, and each such test fails when the corresponding guard is disabled

## 3. Recipient policy renderer

- [ ] 3.1 Port the policy suite from `<dotfiles>/modules/flake/secrets/sops-yaml.test.nix`, keeping the structured-plan claims and dropping the claims that hold a particular fleet's committed file
- [ ] 3.2 Port the renderer from `<dotfiles>/modules/flake/secrets/_sops-yaml.nix` into `modules/flake/safix/policy.nix`, keeping the split between the structured plan and the rendered text
- [ ] 3.3 Rewrite the generated file's header: keep the one-rule-per-audience explanation, the anchoring rationale, and the revocation-is-not-retroactive statement; replace the agenix and clan examples of what a wide rule would capture with the general statement that a wide rule captures encrypted material this package did not place
- [ ] 3.4 Name the regenerating command as `safix fix` in the header, and confirm the check that holds the committed file to the generated one names the same command in its failure
- [ ] 3.5 Verify: rendering a fixture fleet of three people, one of whom shares one secret with another, produces exactly three rules and the expected anchors

## 4. Flake module wiring

- [ ] 4.1 Port `<dotfiles>/modules/flake/secrets/lib.nix` into `modules/flake/safix/default.nix`, exposing the resolution helpers, the audience map, the placement map, the recipient map, the generator plan, the name pattern, and the policy text
- [ ] 4.2 Drop the helper that enumerates already-committed encrypted files by reading a fixed directory of the consumer's tree; replace it with a computation over the audiences the declarations imply, plus an option through which a consumer names extra files it wants governed
- [ ] 4.3 Export the module as a flake output a consumer imports, and confirm importing it into a scratch flake that declares nothing evaluates green and produces an empty policy with no catch-all
- [ ] 4.4 Verify: `nix flake check --no-build` green

## 5. The command

- [ ] 5.1 Port `<dotfiles>/modules/flake/secrets/sops_recipients.py` and `sops_keys.py`, which read a file's recipients and its populated keys without decrypting
- [ ] 5.2 Port `<dotfiles>/modules/flake/secrets/home-secret.sh` to `modules/flake/safix/safix.sh`, renaming the program and the evaluation attributes it reads, and keeping the subcommand set
- [ ] 5.3 Port the command's self-test from `<dotfiles>/modules/flake/secrets/home-secret-selftest.sh`, rebasing every fixture onto keys minted in a scratch directory at test time
- [ ] 5.4 Confirm the self-test creates its own encrypted fixtures from its own keys and copies no ciphertext from anywhere
- [ ] 5.5 Package the command with its runtime tools pinned into its closure, and run shellcheck over the script as part of the build
- [ ] 5.6 Rewrite the help text: keep the recorded absences of the upload, export, and import verbs and the reasons for them, and drop the framing that explains them as a correspondence to another tool
- [ ] 5.7 Verify: the self-test passes, and a value set through `set` and read back through `get` round-trips byte-for-byte including trailing-newline handling

## 6. Onboarding narrows and the hook appears

- [ ] 6.1 Strip host attachment from `adduser`: the identifier allocation, the per-host account module, the host imports edit, and the refusal of hosts lacking a particular module
- [ ] 6.2 Keep the recipient shape check and the refusal of a recipient requiring physical interaction, with the reason stated in the message
- [ ] 6.3 Add the hook option: a consumer-supplied invocation receiving the new person's name and recipient, called after the safix-owned scaffolding is written
- [ ] 6.4 Confirm `adduser` with no hook configured succeeds and its output names what it did and did not do
- [ ] 6.5 Verify: an onboarding run in a scratch repository writes the person's declarations, regenerates the policy with their anchor and no rule, and mints nothing

## 7. Materialization into both scopes

- [ ] 7.1 Write the user-scope materialization against the secret provisioner's user-scope module, from the resolved entries
- [ ] 7.2 Write the system-scope materialization against the same provisioner's system-scope module, carrying the ownership fields the user scope refuses
- [ ] 7.3 Confirm one declaration materializes into both without any field naming a scope
- [ ] 7.4 Verify: a fixture configuration of each scope builds, and the entry's mode, path, and key are identical in both

## 8. Checks, each with a severity drill

Every check below gets a perturbation that must turn it red.
A check that has never failed has not been shown to be able to.

- [ ] 8.1 Policy drift: the committed policy differs from the generated one. Drill: edit one recipient in the committed file and confirm the check names the regenerating command
- [ ] 8.2 Rule shape: every pattern is start-anchored, extension-terminated, and one directory level. Drill: strip the anchor, then replace the single-level wildcard with an unrestricted one, and confirm each perturbation fails independently and names the rule
- [ ] 8.3 No catch-all: the policy contains no rule matching an unconstrained path. Drill: add one and confirm the check fails
- [ ] 8.4 Path collision: two entries claiming one on-disk path. Drill: point a second entry at an occupied path and confirm the check fails naming both
- [ ] 8.5 Runtime tool resolution: every generator's declared tools resolve against the package set. Drill: misspell one and confirm the build fails naming the generator
- [ ] 8.6 Custody refusals: each refusal in design.md D8 has a check. Drill: for each, construct the offending fixture and confirm evaluation fails naming the declaration
- [ ] 8.7 Audience separator: the separator joining a shared audience's members lies outside the name alphabet. Drill: change it to a character the alphabet admits and confirm the check fails
- [ ] 8.8 Namespace isolation: no module in this repository reads an option path outside safix's namespace. Drill: add such a read and confirm the check fails naming the file
- [ ] 8.9 Register every check in the flake's check surface and confirm each appears in `nix flake show`

## 9. Documentation

- [ ] 9.1 Grow the README from `<dotfiles>/docs/notes/development/home-secret-guide.md`, keeping the mental model, the private-versus-carried-versus-shared explanations, the generator narrative, and the placement-versus-custody distinction; substitute the namespace and drop the fleet-specific file table
- [ ] 9.2 Write the worked adapter example: a consumer with its own user registry projecting it onto safix's user records
- [ ] 9.3 Document the onboarding sequence end to end, with the operator's part and the person's part separated, and the disclosure about independent custody placed where the choice is made
- [ ] 9.4 State the non-negotiable opinions as a section of their own, each with the failure it prevents
- [ ] 9.5 Confirm no statement in the README describes a guarantee whose enforcing code is not already in this repository
- [ ] 9.6 Replace the README's status section, which says the implementation is not here yet

## 10. Verification, build and evaluate only

- [ ] 10.1 `nix flake check` green on the working platform, with every check from section 8 present
- [ ] 10.2 The command's self-test green
- [ ] 10.3 A scratch consumer flake, declaring two people and four secrets across four separate files, evaluates to the expected audiences, placements, and policy
- [ ] 10.4 Confirm no file in this repository contains an encrypted value, and no file contains a recipient belonging to anyone
- [ ] 10.5 Confirm the source repository is byte-identical to its state at the start of this change
