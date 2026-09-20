---
title: "Command reference"
---

## What this page is for

This page lets you look up any safix subcommand: what it does, what it reads,
what it writes, whether it needs a terminal, and which flags it takes.

The table has one row per subcommand in the order `safix -h` lists them. That
order is the operator-facing one — write, read, converge, bridge, custody — and
`install` sits last because it is the one machine-facing verb: an activation
runs it, and a person never types it.

Refusal codes named below are listed in [Refusal codes](refusals.md).
Variables are listed in [Environment](environment.md).

## The subcommands

| Verb | What it does | What it reads | What it writes | Needs a terminal |
| --- | --- | --- | --- | --- |
| `set` | Stores a value you type or pipe in | The placement map, the target file | The placed file, and a commit naming it | No — a non-terminal standard input is read instead |
| `edit` | Opens your editor on a value | `$VISUAL` or `$EDITOR`, the placement map, the current value | The placed file, and a commit naming it | Yes for the no-name form, which offers a choice |
| `get` | Decrypts one key to standard output | The placed file | Nothing | No |
| `view` | Shows one value, or offers a choice of what to show | The placement map, the placed file, the picker's remembered state | The picker's remembered state | Only to offer a choice; a value goes to standard output without one |
| `list` | Prints every name a person holds | The placement map and the per-value records | Nothing | No |
| `generate` | Mints values from declared generators | The placement map, the generator plan, dependencies' plaintext | The placed files, the public outputs, the per-value records, and a commit per generator | Only to answer a prompt or the cascade confirmation |
| `check` | Reports drift and changes nothing | The committed `.sops.yaml`, each governed file's structure, the per-value records | Nothing | No |
| `fix` | Converges the recipient policy and the ciphertext | The declarations and every governed file | `.sops.yaml` and every governed file; it does not commit | Only for sops' own confirmation, unless `--yes` |
| `audit` | Compares declared mappings and changes nothing | Both sides of each selected mapping | Nothing | Yes where a target needs a password typed |
| `sync` | Converges declared mappings | Both sides of each selected mapping | Whichever side the mode names, and a commit where safix's side moved | Yes where a target needs a password typed |
| `keygen` | Mints a local identity | The existing identity file | The identity file, or the managed GnuPG keyring | Only for the GnuPG profile's pinentry |
| `migrate` | Executes a migration plan, or abandons an interrupted one | The plan, its sources, the target identities, and the journal of an interrupted run | The destinations, the deployment module, the receipt, and a journal while the run is in flight | No |
| `identity` | Backs up or restores the local identity | The local identity and the named recovery identities | The backup package, or the local identity on restore | Only where pinentry is involved |
| `adduser` | Declares a person who holds nothing yet | The declarations | The person's record, `.sops.yaml`, a commit, then the onboarding hook | Only for the confirmation, unless `--yes` |
| `enroll` | Takes a hardware key to a proven recovery identity | Every governed file's recipients, the card | The identity file, the person's record, `.sops.yaml`, every governed file, and a commit | Yes — refused without one, before the card is touched |
| `group` | Edits one group's declared membership | The group's declaration file | That file, `.sops.yaml`, and a commit | No |
| `rotate` | Re-mints a value whose rotation deadline has passed | The placement map, the generator plan, the per-value records | The placed files, the public outputs, the per-value records, and a commit per generator | Only to answer the cascade confirmation, unless `--yes` |
| `rotation` | Edits the rotation policy one entry's declaration names | The declarations and the person's declaration file | That file, and a commit | No |
| `upload` | Seeds a machine's own host identity | The identity you supply, and the key the address presents | A pre-seed tree, or the machine's `/etc/ssh` | No |
| `install` | Installs a manifest — the verb an activation runs | The manifest and the documents it names | A private generation, then the published symlink | No |

## Global options

Two options apply to every subcommand.

```text
--entry <file>          evaluate <file> instead of the repository's flake
--nixpkgs <flake-ref>   generate's sandbox resolves its tools against this
                        flake reference instead of the declaring one
```

`SAFIX_ENTRY` and `SAFIX_NIXPKGS` set the same two. A flag wins over its
variable when both are given.

Every subcommand but `generate` behaves identically under `--entry` as it does
against a flake. `generate` is the exception: under `--entry` with neither
`--nixpkgs` nor `SAFIX_NIXPKGS` set it refuses, naming both remedies, because a
generator's sandbox resolves its own tools through `nix shell` and that is a
flake-only operation.

Neither option changes where a run stages or commits. That root is still the one
git reports for the current directory, or the one `SAFIX_REPO_ROOT` names
instead.

`SAFIX_VAULT_ROOT` names a second, mutable working tree for a declared vault. It
is unset and unread when no vault is declared, and required whenever one is.

A `<user>` argument defaults to `$USER` when the declarations name them, and
otherwise to the sole declared holder where there is exactly one.

## set

```text
safix set [<user>] <name>
```

Prompts for a value twice without echoing it, writes it into the file the
declarations place `<name>` in, then stages and commits that file alone.

Values are single-line and stored exactly as typed, with no trailing newline
added. When standard input is not a terminal the value is read from it whole,
and the confirmation is dropped: the second prompt exists to catch a value
mistyped invisibly, and a piped value has no typist. An empty pipe takes the
same refusal an empty prompt takes.

A multi-line value belongs to `sops <file>`, or to a generator.

## edit

```text
safix edit [--allow-disk-staging] [--no-preview] [<user>] [<name>]
```

Opens `$VISUAL`, or `$EDITOR` when that is unset, on the value. Neither set is a
refusal naming both: this command opens no editor of its own choosing. The
command is split on whitespace and run directly rather than through a shell, so
`EDITOR="code --wait"` works. The staged file's path is an argument; the value is
not.

An entry that holds no value yet opens on an empty buffer, so this authors as
well as amends.

```text
--allow-disk-staging  accept a disk-backed filesystem for the buffer
--no-preview          suppress the value pane in the no-name form
```

An editor exiting non-zero, and a buffer left unchanged, each write nothing. A
buffer emptied is refused. A buffer changed is written through the same path
`set` writes through, and committed.

With no `<name>`, every entry the person holds is offered for selection, less
every public output. A public output is not editable: it is already plaintext in
the repository, and the generator declaring it is what mints it.

## get

```text
safix get [<user>] <name>
```

Decrypts that one key to standard output. The output is plaintext by design and
is meant for piping. It needs an identity that opens the file, which is the
owner's or a recovery identity theirs names.

`get` is the pipe; `view` is the verb that shows a value on a terminal.

## view

```text
safix view [--no-preview] [<user>] [<name>]
safix view [--no-preview] [<user>]
```

Decrypts one key to the terminal. With no terminal to write to, the value goes
to standard output instead: what needs a terminal here is offering a choice, not
writing a value.

```text
--no-preview  open the list with the value pane hidden
```

With no `<name>`, every entry the person holds is offered for selection. A lone
argument is a person when the declarations name one by that name, and an entry's
name otherwise; an entry whose name is also a person's is reachable by naming
both.

The list's keys are printed on its last line:

```text
Enter choose · Esc/^C cancel · ↑↓ move · ←→/Home/End/Del edit · ^←→ scroll · Tab columns · ^P preview
```

The unmodified arrows, Home, End, Backspace and Delete move the caret inside
the query and edit where it stands; Ctrl with an arrow scrolls the columns
sideways, and neither direction leaves or wraps. `^A` and `^E` are the two ends
as well, since terminals disagree about how they spell Home and End. Tab adds
the two columns behind it, `FILE` and `ROTATES`. Every other key is consumed
and does nothing.

The frame is drawn from the bottom of the terminal upwards: the rows first,
then their column titles under them, a full-width rule carrying the value
pane's title, the pane, and the query above the key line. Each run of
characters a query term matched is drawn bold and underlined, which reads the
same on a plain row, a tinted one and the row under the cursor. A row past its
rotation deadline is red, and its countdown ticks while the picker is open.

The query syntax, the colours and the remembered state are printed by
`safix view -h`. Selection state lives in `picker.json` under the state
directory named in [Environment](environment.md#xdg_state_home).

Three refusals are the picker's: no terminal to choose on, the person holds
nothing, and leaving without choosing.

## list

```text
safix list [<user>]
```

Prints every name the person holds, where it came from, whether one value serves
every carrier, whether a generator mints it, and the key it is read under. Three
more columns follow: when the value was created and last updated, the file
serving it, and how long it has left before its rotation deadline.

The generator column shows a generator's own description where it has one, `yes`
where it has a generator and no description, and `-` where the value can only be
typed or transcribed. The date columns show `-` for a value written before the
per-value record existed: no record is no claim about when a value arrived. The
`ROTATES` column reads `12d 04:12:09` for a value with time left, days alone
beyond a year, `due` past the deadline, and `-` for an entry naming no policy.

## generate

```text
safix generate [--regenerate] [--yes] [--allow-disk-staging] [<user>] [<name>]
```

Runs the person's generators, in the dependency order the declarations compute,
for every declared secret with no value yet.

```text
--regenerate           re-run over values that already exist
--yes                  answer the cascade confirmation in advance
--allow-disk-staging   accept a disk-backed staging filesystem
```

With no `<name>`, every generator that has something to mint runs. Naming a
secret runs the one generator that writes it; naming either half of a
multi-output generator runs the generator that mints both, and both land in one
commit. A single argument naming a declared person selects that person.

`--regenerate` over a named generator also re-runs every generator that reads
what it writes, transitively. The set is listed and confirmed before anything
runs, because each re-run commits as it goes.

A script writes its declared outputs into a staging directory, reads answered
prompts and its dependencies' plaintext from there, and runs inside a sandbox
whose only writable path is that directory. Where no memory-backed filesystem is
available the run refuses. See [Generators](../guides/generators.md).

## check

```text
safix check [<user>]
```

Reports drift and changes nothing. It exits non-zero when there is any, and each
finding prints the command that resolves it. Six classes of finding are
reported:

- the committed `.sops.yaml` against the policy the declarations imply
- each governed file's recipients against the audience declared for it
- declared names with no value, saying which have a generator
- values in a governed file that no declaration claims
- generated values minted under a generator that has changed since
- values past the deadline their rotation policy set

`fix` handles the first two. The rest are decisions rather than convergences, so
nothing here makes them for you.

It needs no identity for any file it examines: every question is answered from
the document's structure and from the per-value record, and nothing on this path
decrypts.

## fix

```text
safix fix [--yes] [--vault-rollback]
```

Regenerates `.sops.yaml` from the declarations, then re-wraps each governed
file's data key to the audience that policy declares. The order is not
interchangeable: re-wrapping first re-wraps to a policy that is about to change.

```text
--yes              answer sops' confirmation, and authorize raw-age replacement
--vault-rollback   move vault-rooted files back to the declaration root
```

Raw age has no inspectable recipient roster, so `--yes` explicitly authorizes
replacement with the declared audience there; decryption verifies bytes, not the
previous roster.

The governed set is the union of the files the declarations imply and the ones
named in [`flake.safix.extraGovernedFiles`](declarations.md#flakesafixextragovernedfiles).

It does not commit: re-wrapping every governed file is a diff worth reading
first. It does not revoke either — see the revocation rule in
[Custody](../concepts/custody.md).

With a vault declared, this first relocates every readable-layout document,
public output and definition record still at the declaration root into its
opaque vault destination, then re-wraps. A destination already present is left
alone, so an interrupted run resumes where it stopped. `--vault-rollback` runs
the same move the other direction and skips the re-wrap; run it while the vault
is still declared, because the naming key is only reachable through the standing
declaration. See [The vault](../guides/vault.md).

## audit

```text
safix audit [clan|keepassxc|pass|bitwarden|1password] [<mapping>...] [--direction <value>]
```

Compares declared mappings and changes nothing. With no target it compares the
clan and keepassxc targets; naming one narrows to that target's own mappings,
and mapping names after it narrow further.

```text
--direction <value>  narrow the clan target to mappings declared with that value
```

`--direction` is refused on the other targets, whose mappings declare a mode
instead. Each mapping's outcome is reported as agreeing, diverged, diverged in
named fields, or unjudgeable, and a diverged mapping's remedy is the matching
`sync` run. A diverged field is named and never printed. Entries the mappings do
not declare are reported alongside as lingering and never move the exit status.

This is a verb of its own rather than more rows in `check` because it needs what
`check` refuses: a decryption of safix's side, and the far side's own
credentials. See [Syncing password managers](../guides/syncing-password-managers.md).

## sync

```text
safix sync [clan|keepassxc|pass|bitwarden|1password] [<mapping>...] [--direction <value>]
```

Converges declared relationships. With no target it converges every mapping on
the clan and keepassxc targets, each in its own declared direction or mode.
There is no `all` target: the bare form is the one spelling for everything.

```text
--direction <value>  narrow the clan target to mappings declared with that value
```

Both sides are read and compared before either is written, so a second run
immediately after a first writes nothing. No mode deletes an entry on either
side. A two-way mapping remembers the last state both sides agreed on; when both
sides have moved, nothing is written and the finding names the one-way runs that
each resolve it.

Per-target behaviour lives in
[Syncing password managers](../guides/syncing-password-managers.md).

## keygen

```text
safix keygen [--for-someone-else] [<user>]
safix keygen --show
safix keygen --kind age
safix keygen --kind pgp --uid <identity> [--expires 2y]
```

Mints an age identity and appends it to the managed identity file, then prints
the public half and what to do with it. The private half is never printed. It
appends and never truncates: sops tries every identity in that file, and
overwriting is how someone loses the key to everything they hold.

```text
--for-someone-else   mint an identity for a person who is not you
--show               print the public recipient of the identity already minted here
--kind age|pgp       which key profile to mint; age is the default
--uid <identity>     the GnuPG user id, required by the pgp profile
--expires 2y         the GnuPG primary's expiry
```

Run it on your own machine, as yourself. Minting another person's identity here
means you hold their private key, so it takes an explicit
`--for-someone-else`.

The `--kind` forms need no repository. The GnuPG profile uses an expiring
Ed25519 certification primary and a cv25519 encryption subkey, and pinentry
handles the passphrase. Existing compatible keys are not weakened or replaced.
Key files use mode 0600 and private directories 0700; symlinked paths,
repository paths and Nix-store paths are refused.

`--show` mints nothing at all, and is refused — naming plain `keygen` as the
remedy — where no identity has been minted here yet. See
[Backing up an identity](../guides/identity-backup.md).

## migrate

```text
safix migrate <plan.json>
safix migrate --abandon <plan.json>
```

Executes a version-1 migration plan. Paths are relative to the plan's directory.
Every candidate must decrypt byte-identically before any output is published,
existing outputs are refused, and sources are always retained. Unsupported
target deployment semantics are refused rather than dropped.

```text
--abandon  discard an interrupted run: remove what its journal records
```

Before the first output lands the run writes a journal at `<receipt>.journal`,
and removes it once the receipt is published. Rerunning the same plan while its
journal exists resumes that run instead of refusing its outputs. `--abandon`
takes exactly one plan and removes only the outputs and staging directories the
journal records.

The schema is in [Migration plan](migration-plan.md), the journal's own fields
are in [The journal](migration-plan.md#the-journal), and the walk-through is in
[Migrating from sops-nix and agenix](../guides/migrating-from-sops-nix-and-agenix.md).

## identity

```text
safix identity backup <age|pgp> <destination> --recipient <public-key>... [--age-key-file <recovery-key>] [--gnupg-home <recovery-keyring>]
safix identity restore <backup> [--age-key-file <recovery-key>] [--gnupg-home <recovery-keyring>]
```

Backs up the local managed identity to an encrypted SOPS binary package, then
verifies it with explicitly selected independent recovery identities.

```text
--recipient <public-key>       a recovery recipient; repeatable, required on backup
--age-key-file <path>          the recovery age identity used to verify or restore
--gnupg-home <path>            the recovery keyring used to verify or restore
```

Public GnuPG recipients use `pgp:FULL_UPPERCASE_FINGERPRINT`. A self-only backup
is refused: a key cannot be its own recovery path. No private key is printed or
passed as a command argument, and destinations must not already exist.

Restore validates every private primary and subkey before importing. Existing
age files and colliding GnuPG fingerprints or private keygrips are refused,
including private files absent from the public keyring. Backup refuses shared
recovery keygrips, unaccounted-for private files, hardware stubs and incomplete
primary-key exports. Protect the recovery identity separately; safix does not
escrow it for you.

## adduser

```text
safix adduser <name> <recipient> [--host <hostname>]... [--yes]
```

Declares a person who holds nothing yet: writes their record, regenerates
`.sops.yaml` from the policy that declaration implies, commits the two, and then
hands the name and the recipient to
[`flake.safix.onboardingHook`](declarations.md#flakesafixonboardinghook).

```text
--host <hostname>  passed through to the hook, repeatable
--yes              skip the confirmation
```

`--host` is refused where no hook is configured, because attaching an account on
a host is a property of a consumer's module tree and safix has none.

The recipient is theirs: a native age recipient, an SSH public key, or
`pgp:FULL_UPPERCASE_FINGERPRINT`. Only its shape is checked here. An SSH key
with a comment must be passed as one quoted argument. A recipient that needs a
physical interaction is refused for this field and belongs in that person's
[`recoveryRecipients`](declarations.md#flakesafixusersnamerecoveryrecipients)
instead, where it is additive.

This mints nothing and gives them nothing to hold. Their first secret is a name
under `private` or `carries`, then `fix` to write the rule, then `set`.

## enroll

```text
safix enroll [<user>] [--serial <n>] [--slot <n>] [--no-store-pin]
             [--mirror-to-store] [--store-database <path>]
             [--pin-policy <p>] [--touch-policy <p>] [--allow-disk-staging]
             [--trust-declared-recipients]
```

Takes one hardware key from a blank card to a proven recovery identity for the
person, in one verb. A touch is the only thing you do.

```text
--serial <n>                 which card, required when two are connected
--slot <n>                   a retired slot to use instead of the first empty one
--no-store-pin               do not store the generated PIN and PUK in safix
--mirror-to-store            also write them to the session's secret service
--store-database <path>      the kdbx to add the entry to instead
--pin-policy <p>             default once
--touch-policy <p>           default cached; never is refused
--allow-disk-staging         accept a disk-backed filesystem for the proof
--trust-declared-recipients  authorize declared audiences for raw-age files
```

Before selecting a card it inspects every governed file and refuses undeclared
recipients. Raw-age files require `--trust-declared-recipients`, which authorizes
the declared roster and does not verify a previous one.

Everything it does is additive: a recipient is appended, an identity block is
appended, a name is declared. A re-wrap that dropped a recipient a file had
before the run is refused rather than committed. No OTP slot is written under any
flag, `--touch-policy never` is refused, and a run with no terminal is refused
before the card is touched. See [Hardware keys](../guides/hardware-keys.md).

## group

```text
safix group add|remove <group> <subject>
```

Edits one group's declared membership: one name inserted into or removed from the
`members` list in that group's declaration file, parsed before anything is
staged, with `.sops.yaml` regenerated and the two committed together.

It writes no value, encrypts nothing and re-wraps nothing. A membership change is
a reason to run `fix`, and the report says so. `remove` takes nothing back — see
the revocation rule in [Custody](../concepts/custody.md), and
[After removing access](../guides/after-removing-access.md).

A group or a subject the declarations do not name is refused before anything is
read. An organization is refused as a member. A `members` value this cannot read
is refused rather than compounded. Where a group is covered by an organization's
silo declarations, only that organization's managers may edit it, judged against
the identity the resulting commit will carry.

## rotate

```text
safix rotate [--yes] [--allow-disk-staging] [<user>] <name>
safix rotate --due [--yes] [--allow-disk-staging] [<user>]
```

Mints a new value because the one there is too old. The named form rotates one
entry now; `--due` rotates everything past the deadline its rotation policy set.

```text
--due                  rotate every entry past its deadline, for one person or all
--yes                  answer the cascade confirmation in advance
--allow-disk-staging   permit staging on disk where no memory-backed root is available
```

The value travels exactly the path `generate` writes one on: the same sandbox,
the same pipe, the same per-generator commit, and the same per-value records.
Every generator reading the rotated value re-runs in the plan's own order, and
`--due` merges those cascades into one ordered set announced before the first
commit.

A value no generator declares is not minted here. The named form refuses and
names `safix set`; `--due` lists every due typed entry with the `safix set` each
one needs and exits zero. Nothing due is a quiet success and exits zero. See
[Rotating secrets](../guides/rotating-secrets.md).

## rotation

```text
safix rotation set <user> <name> <policy>
safix rotation unset <user> <name>
```

Edits the rotation policy one entry's declaration names: one `rotation =
"<policy>";` line inserted, replaced or removed in that person's declaration
file, parsed before anything is staged and committed.

It writes no value, encrypts nothing and re-wraps nothing. A deadline places no
key in any audience, so the recipient policy the edit implies is the one already
committed.

An entry or a policy the declarations do not name is refused before anything is
read, because an entry naming an undeclared policy is refused at the next
evaluation. A declaration this cannot read is refused rather than compounded.
What it edits is the entry's own block, or a dotted declaration of one of its
fields; a declaration living elsewhere or computed rather than written is edited
by hand. Where an organization manages the person, only that organization's
managers may edit their entries, judged against the identity the resulting
commit will carry.

## upload

```text
safix upload <machine> --directory DIR --identity PATH
safix upload <machine> --to ADDRESS [--identity PATH] [--force]
```

Seeds a machine's own ed25519 host key before its first activation, so the
audience already wrapped to its declared recipient can be decrypted from the
moment it boots. safix mints no machine identity.

```text
--directory DIR   write a pre-seed tree, touching no network
--to ADDRESS      probe the address, then no-op, write, or refuse
--identity PATH   the private key the operator already holds
--force           overwrite a different key the address already presents
```

`--directory` writes the host key at mode 0600 and its public half at 0644 — the
paths and modes a fresh install's own key generation would produce — and makes no
ssh connection. `--to` reads the key the address currently presents before
writing anything: the declared key already there reports and writes nothing, no
key presented writes given `--identity`, and a different key is refused by
default naming both recipients.

Nothing here triggers a deploy, switch or rebuild. See
[Unattended hosts](../guides/unattended-hosts.md).

## install

```text
safix install <manifest> [--check-mode=off|manifest|document] [--ignore-passwd]
                         [--dry-run]
```

Installs a manifest using raw age or SOPS YAML, JSON, dotenv, INI or binary. It
decrypts with the configured age, SSH or GnuPG identities, renders templates in a
private generation, publishes it, then runs changed-value service hooks.

This is the verb an activation runs, not one an operator types. A NixOS
activation script and a home-manager activation entry each invoke it against a
manifest safix's own nix half built, and the manifest is the whole input —
including whether this is a user-scope install.

```text
--check-mode=off        install, checking each step as far as it needs
--check-mode=manifest   validate the schema, version, modes, owners and groups, then stop
--check-mode=document   also open each distinct document and verify every declared key
--ignore-passwd         skip user, group and keys-group lookups and set ownership to 0
--dry-run               validate, decrypt and render into private staging only
```

Neither check mode decrypts anything and neither needs a key. This build-time
validation does not prove that runtime identities can decrypt the document.
`NIXOS_ACTION=dry-activate` implies `--dry-run`.

## Verbs retired, reserved, or narrower here than in clan

`export` is retired permanently. clan's own vars export writes a machine's whole
vars folder to plaintext on disk, which is the bulk dump safix's design refuses
to build on either side of the boundary. `sync clan` moves one declared mapping
at a time, encrypted, and always did.

`import` is reserved rather than retired. A future, unbuilt feature — ingesting
a value from an external plaintext source one entry at a time — may use the word
later. There is no scaffold and no partial parser for it yet.

`upload` moves only a machine's own host identity, once, before that machine's
first activation. It is not clan's ongoing vars-delivery verb of the same name.
No verb here delivers a secret's value on an ongoing basis: activation already
does, through `install` reading the manifest safix's own nix half built, once a
machine holds the identity `upload` seeds.
