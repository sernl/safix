# safix

safix is a custody-first secrets manager for nix.
Secrets are declared as attribute sets the nix module system merges, the encrypted file each one lives in is derived from the audience that can read it, and the recipient policy in `.sops.yaml` is generated from the same declarations.
One opinion decides most of the rest: the audience picks the file.
Declarations may scatter anywhere across your tree, one per file, because they are mergeable attribute sets; placement never scatters, because it is computed rather than written.
safix is not a framework, and it is not your user registry.
It serves NixOS and home-manager alike through consumption modules of its own, and installs what they resolve with an installer of its own.

## Install it and declare one secret

### The flake half

```nix
{
  inputs.safix.url = "github:you/safix";

  outputs =
    inputs@{ flake-parts, safix, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      imports = [ safix.flakeModules.default ];

      flake.safix.users.alice = {
        recipient = "age1...";
        private.alice-token = { };
      };
    };
}
```

`flake.safix.lib` now holds the audiences, the placements, the generated policy text and the check builders, and `packages.safix` is the command.
Declarations merge, so that block can live in its own file beside a hundred others; safix finds them through the module system and reads no path, no filename and no directory structure to do it.
Flake-parts is one way to reach that merge rather than the only one — see **Fitting safix to a tree you already have**.

### The profile half

```nix
# alice's home-manager profile
{
  imports = [ inputs.safix.homeModules.default ];

  safix.flake = inputs.self;
  safix.user = "alice";
  safix.hostname = "workstation";
  safix.identity.sshKeyPaths = [ "/home/alice/.ssh/id_ed25519" ];
}
```

Every secret alice resolves on that host is now established there.
`nixosModules.default` is the first three of those lines for a system configuration, with `safix.machine = "deck"` in place of `safix.user`.
The fourth line is the user scope's alone: at system scope safix derives an age identity from the host's own ssh keys, and a person is not a host.
A profile that resolves secrets and names no identity refuses at evaluation instead of establishing them.

### Your first three commands

```console
$ safix fix                 # write .sops.yaml from the declarations
$ safix set alice-token     # hidden prompt, confirmed, encrypted, committed
$ safix get alice-token     # the value on standard output, for piping
```

Every `sops <path>` command in this document uses the default spelling of the encrypted root; that spelling is a default rather than a promise, and if you have renamed the root the path is yours.

## How safix thinks

A secret has three questions: who declares it, who can read it, and where it lands.
Declaring happens in nix and is the label on the box; reading is decided by the recipients in `.sops.yaml`, which are generated from the declarations and never hand-edited.
Landing means the file a profile establishes at activation.

The distinction that does the work is custody against placement, and both words are used in their narrow sense here.
Custody is who holds a secret: a property of a subject — a person, a machine, a service, a group of subjects, or an organization — and for a person it is the same on every host they log into.
Placement is where the decrypted value shows up, and it is a property of a configuration rather than of the holder.
Every refusal safix makes comes from keeping those two apart.

One rule follows from the file format itself, and it is stated here once because every later chapter needs it.
An encrypted document has one data key, wrapped once per recipient, so everyone the file names can read all of it.
Narrowing an audience therefore aligns future ciphertext with the new audience, and it **does not retract** what an old recipient already holds: they have read the values in every file they could open.
The remedy is to rotate the value — mint a new one with `safix set`, or regenerate it where it has a generator.
`safix check` reports a shrunk audience as the narrowing it is and names rotation as the remedy, while `safix fix` aligns ciphertext with policy and is explicitly not that remedy.

## Declaring what someone holds

```nix
flake.safix.users.alice = {
  recipient = "age1...";
  recipientNote = "alice — her workstation's software identity";
};

flake.safix.catalogue.cognee-api-key = { };
```

`flake.safix.users.<u>.recipient` is the age public key every file in that person's audience is wrapped to, and `flake.safix.users.<u>.recipientNote` is carried into the generated policy beside it.
A recipient that needs a touch or a PIN is refused for that field, because activation decrypts with nobody present.
`flake.safix.catalogue` is the set of entries that exist to be carried, and an entry there says the thing exists rather than that anybody holds one.
Every entry, wherever it is declared, carries the same fields.

| field | default | what it is |
|---|---|---|
| `mode` | `"0400"` | the on-disk mode of the decrypted value |
| `path` | `null` | where the value is written, as a function of the configuration materializing it |
| `sopsKey` | the entry's name | which key inside the encrypted document holds the value |
| `generator` | `null` | how `safix generate` mints the value — see **Generators** |
| `shared` | `false` | whether the carriers hold one value between them or one each |
| `owner`, `group` | `null` | the account and group the decrypted file belongs to, at system scope only |
| `sopsFile` | refused | declared so the refusal has a name; placement is derived, never authored |

`path` is a function because the configuration it is relative to differs per scope: one written as `cfg: "${cfg.home.homeDirectory}/…"` will not materialize into a system configuration.

### A drawer of your own: `private`

`flake.safix.users.<u>.private.<name>` declares an entry that exists only for that person, as in `private.filen-key = { }` or `private.ssh-personal.mode = "0600"`.
The audience is one key, so only the holder can read it, and no other person's declaration can widen it.

### A shelf you take from: `carries`

`flake.safix.users.<u>.carries.<name>` selects a catalogue entry for that person: the shelf says the thing exists, and carrying says this person has one.
By default each carrier gets their own file with their own value, so bob's copy and alice's are unrelated values under one label.
Carrying confers no read of anyone else's copy, and says nothing about which hosts the value lands on.

### A copy you hand to someone: `sharedWith`

`flake.safix.users.<u>.sharedWith.<subject>.<name>` is the owner's statement that a name they hold is to reach one other subject.
The audience directory's name is the guest list, so `secrets/safix/shared/alice,bob/secrets.yaml` is readable by those two and by nobody else.
`safix fix` writes the rule for that file; moving the value into it is a keyholder's act, so the second command is `sops <path>` in your hands.
A grant carries no fields of its own: the recipient's copy is the owner's record unchanged, and a recipient-side adjustment belongs in the recipient's own scopes.
Removing a grant narrows the audience and nothing more — see **How safix thinks**.

### One value for everybody: `shared = true`

`flake.safix.catalogue.<e>.shared` makes the entry one value rather than one value per carrier: one ciphertext, wrapped once per recipient, read by every carrier.

| | `carries` (default) | `carries` + `shared = true` |
|---|---|---|
| values | one per person, independent | one, total |
| a person joins | gets their own empty slot | can read the existing value |
| a person leaves | nothing happens to yours | rotation needed — they have seen it |

The last cell is the rule of **How safix thinks**, seen from the shelf, and the signal is derived from the file's own recipient stanzas rather than from a state file.
Two statements of one audience are refused: an entry that is `shared` and also granted through a `sharedWith` has two answers to who reads it.

### Where it lands, not who holds it: `perHost` and `perTag`

```nix
flake.safix.users.alice.perHost.builder.omit.filen-key = { };
flake.safix.users.alice.perTag.portable.force.shelf-item = { };
```

`flake.safix.users.<u>.perHost` selects by the host a profile resolves on, and `flake.safix.users.<u>.perTag` by the tags that host carries.
Each scope has three fields: `add` carries an entry in this scope, `omit` drops one, and `force` re-adds a name `omit` dropped, beating it within the same resolution.
Each of the three may also adjust that entry's mode or its path for the scope alone.
These are placement and never custody, so a carrier of a shared entry who omits it on one host stays in the audience.
An `add` naming an entry that person already holds through `carries`, `private` or a grant adjusts its placement in that scope.
An `add` that is the only route to the entry is refused: a host-scoped selection puts nobody in any audience, so that person would resolve a file they are not encrypted to.

### Further identities of your own: `recoveryRecipients`

```nix
flake.safix.users.alice.recoveryRecipients.master = {
  key = "age1...";
  note = "alice's offline master identity — held by her, not by the operator";
};
```

`flake.safix.users.<u>.recoveryRecipients` lists further identities of the same person, each a `key` with a `note`.
Every file whose audience includes that person is wrapped to these as well, so the field widens what they can open and nothing else.
Leaving it empty keeps their custody independent, at a cost no later edit undoes: with only their activation key, losing it makes their files unopenable by every party including the operator.
An offline master key or a hardware token the person themselves holds is the mitigation that keeps their independence, and an operator-held identity buys the same recoverability at the price of that operator reading everything.
Where that holder is an organization, `escrowedTo` declares the same trade-off in a reviewable form — see **The subjects that can hold a key**.

## The subjects that can hold a key

The set of things that can hold a key is wider than a person, and it is one algebra rather than a second grant surface.
Nothing here changes anything until you declare it: a machine, a service, a group, a silo or an organization that nothing references generates the same policy and the same files, byte for byte.

### Machines

```nix
flake.safix.machines.deck = {
  recipient = "age1..."; # ssh-to-age of the host's ed25519 key
  recipientNote = "deck — alice's laptop";
  owner = "alice";
  tags = [ "laptop" ];
};

flake.safix.users.alice.sharedWith.deck.fleet-token = { };
```

`flake.safix.machines.<m>.recipient` is the age form of the host identity the system scope already decrypts with, derived from the ed25519 entries of `services.openssh.hostKeys` that lie outside safix's own store.
`flake.safix.machines.<m>.recipientNote` annotates it in the generated policy, `flake.safix.machines.<m>.owner` names the person or organization that holds the machine, and `flake.safix.machines.<m>.tags` are the tags a profile resolving as this machine carries by default.
A machine holds nothing of its own — there is no `carries`, no `private` and no `sharedWith` on one — and it needs no hostname, because it is the host.
Declaring a recipient mints no identity and does not put the matching private half on the machine's disk, which is what `safix upload` is for.
A machine's entries arrive in the profile that names it through `safix.machine`.

### Services

```nix
flake.safix.services.nginx = {
  machines = [ "deck" ];
  owner = "alice";
  user = "nginx";
  group = "nginx";
};
```

`flake.safix.services.<s>.machines` is where the unit runs, and it is the whole of the service's recipient set.
`flake.safix.services.<s>.owner` records who owns it, while `flake.safix.services.<s>.user` and `flake.safix.services.<s>.group` are the account the landed file belongs to.
A `%` marks a service in an audience directory, as in `secrets/safix/shared/%nginx,alice/secrets.yaml`.
A service grant narrows what is declared and what is placed, and not what decrypts: the audience names the service, the landed file belongs to the service's account, and the host identity still opens it.
The entry arrives on each machine the service runs on, keyed under the service's name, so two services granted one name never collide.
A machine joining is a re-wrap of the same file and a machine leaving is a narrowing — see **How safix thinks**.
safix records where a service runs because audiences need it, and derives it from nothing: keeping the declared set and the running unit in step is yours.

### Groups

`flake.safix.groups.<g>.members` may name people, machines, services, or other groups, and a cycle among them is refused at evaluation with the participants named.
An `@` marks a group, as in `secrets/safix/shared/@oncall,alice/secrets.yaml`, and it is what makes a membership change cheap.
Membership confers a read of every file that group's audience names, and nothing else.

### Silos

`flake.safix.silos.<s>.groups` declares non-overlap you can prove.
Evaluation refuses any file whose audience would reach subjects of two groups in one set, naming the file, the subjects and the declaration that forbids it.
It is deliberately not transitive over ownership: one person may own machines in two silos, and what is refused is a single file readable from both.

### Ownership

`flake.safix.users.<u>.sharedWith."ownerOf.<m>"` is a grant audience that resolves through that machine's `owner`.
The audience directory names the reference rather than the person, so a change of owner re-wraps that one file toward the new owner instead of leaving the grant pointed at the old one.
The record confers nothing else: an owner does not thereby read the machine's own entries or manage its users, because a record that silently granted either would be escrow arrived at by accident.
The old owner's loss of future access is a narrowing — see **How safix thinks**.

### Organizations and escrow

```nix
flake.safix.organizations.acme.custody.acme-escrow = {
  key = "age1...";
  note = "acme's escrow — held offline by the operator";
};

flake.safix.users.alice.escrowedTo = [ "acme" ];
```

`flake.safix.organizations.<o>.custody` holds the organization's own recovery identities, each a key with a note.
`flake.safix.users.<u>.escrowedTo` is the consent, and it lives in the record of the person whose files it widens, so nothing an organization declares widens anybody's audience.
acme rotates a custody key in its own declaration, one `safix fix` re-wraps every consenting person's files, and no person's declaration changes.
Withdrawing consent is a narrowing — see **How safix thinks**.
`flake.safix.machines.<m>.owner` may name an organization, `sharedWith.acme.<name>` grants to one, and `ownerOf` resolves through the record to its custody keys exactly as it resolves to a person's own key.
An `=` marks an organization, as in `secrets/safix/shared/=acme,alice/secrets.yaml`, the way `@` marks a group.
A group may not contain one, because a principal is not a member, and an organization whose custody is empty is refused everywhere it is reached.

### Delegation, and what it is not

`flake.safix.organizations.<o>.managers` names the people who scaffold on the organization's behalf, and `flake.safix.users.<u>.managedBy` is the person's own statement that they are scaffolded for by it.
It is not authorization: the tree is the authorization, anyone who can commit can edit these declarations by hand, evaluation refuses structure rather than people, and no delegation record places a key in any audience.
A manager scaffolds and never reads by virtue of managing, so the generated policy is byte-identical to what it was before either line existed.
Where both halves are declared, `safix enroll` and `safix group` accept that organization's managers and refuse anybody else, naming the delegation and the person who ran the command.
The acting identity is the one the commit will carry, as the repository resolves `user.name` and `user.email`, and there is no flag naming somebody else.
A commit identity the declarations do not name is its own refusal, whose remedy is `git config user.name`.
Delegation over a group is the silo set that holds it: a set whose groups reach an organization's managed people is that organization's, so every group in it is its managers' to edit.

## Generators

### A value that writes itself

```nix
flake.safix.users.alice.private.grafana-token.generator = {
  script = ''openssl rand -hex 32 > "$out/grafana-token"'';
  runtimeInputs = [ "openssl" ];
  description = "a grafana service account token";
};
```

`safix generate` mints everything declared but empty, and `safix generate --regenerate <name>` rotates one.
`script` writes files rather than printing a value, and the directories it addresses are clan's.

| | what it holds |
|---|---|
| `$out/<name>` | one file per declared output; the script's working directory is the root above it |
| `$prompts/<name>` | one answered prompt each, present only where prompts are declared |
| `$in/<generator>/<name>` | a dependency's plaintext, keyed by the generator producing it |

One difference is deliberate: only the dependencies a generator declares appear under `$in`, where clan places every file of the dependency generator.
`runtimeInputs` names nixpkgs attributes as strings rather than holding packages, because the whole generator travels to the command as JSON and a derivation cannot cross that boundary.
Strings are unchecked by construction, so each one is resolved against the package set at build time; otherwise a misspelling is discovered at a rotation.
`description` says what the generator mints, and `safix list` and `safix check` print it.
Bytes are stored exactly as written: `echo` leaves a trailing newline and `printf` does not, and nothing removes one, because a convention that took a byte off would corrupt every key whose last byte is a newline.

### Chaining, prompts and multi-output

```nix
flake.safix.users.alice.private = {
  db-password-hash.generator = {
    dependencies = [ "db-password" ];
    script = ''mkpasswd -sm bcrypt <"$in/db-password/db-password" > "$out/db-password-hash"'';
    runtimeInputs = [ "mkpasswd" ];
  };

  wg-private.generator = {
    runtimeInputs = [ "wireguard-tools" ];
    files.wg-public.secret = false;
    script = ''
      wg genkey > "$out/wg-private"
      wg pubkey < "$out/wg-private" > "$out/wg-public"
    '';
  };
};
```

`dependencies` names other entries of the same person whose plaintext this generator reads.
Rotating an upstream value cascades: every generator downstream re-runs, in dependency order, after showing you the list and asking once, because a hash of a retired password would be a lie.
Cycles, self-references, and depending on another person's secret are all refused at evaluation; the last is structural, since your machine holds no identity that opens someone else's file.
`prompts` asks instead of computing, each prompt declaring its own type and description, and the answer arrives as a file under `$prompts`.
`files` names the further outputs of the same person this one generator also writes, and `files.<n>.secret` says whether each is encrypted.
Each name is a registry entry in its own right, carrying its own mode, path and key.
An entry named there may not carry a generator of its own and may not be named by a second generator, both refused at evaluation, because two producers for one value is a race.
Both halves land in one commit, because a keypair split across two commits is an incoherent state.
The default is `secret = true`, unlike clan's: a mistyped field that leaves a value encrypted is recoverable by fixing the typo, and one that publishes a value is not.
`share` is read-only and derived rather than authored — it is true exactly when every entry the generator writes is `shared`, outputs that disagree are refused, and setting it is refused by name.

### Public outputs a nix module reads at evaluation

`files.<n>.secret = false` writes the value to the repository in the clear, gives it no creation rule, and makes it readable while nix evaluates.

```nix
# in a module of your own, not safix's
peers = [ { publicKey = config.flake.safix.lib.publicValue "alice" "wg-public"; } ];
```

`publicValue` is what a public key, a fingerprint or a derived identifier is for, read directly rather than through a deployment-time indirection, and the reading expression belongs in your own module.
`outputPath` answers for every output and is a path, never a value.
Reaching for a value on a secret output fails with a sentence naming the entry and pointing at the path, rather than with nix's generic undefined-option message.
The plaintext store is its own tree — see **Where files go**.

### Validation

`validation` is a shell fragment that judges a candidate before anything is written.
The candidate arrives on standard input, `$out_name` names the output under judgement, and a non-zero exit refuses the whole run while the values are still only in memory.

### What a fragment may do

A generator's script and its validation fragments run inside a sandbox: the staging root is the only writable path, the nix store is readable, and there is no network.
A write outside `$out` fails, so a fragment cannot put plaintext somewhere safix does not look and cannot shred.
`runtimeInputs` is therefore the whole of what a fragment can run, and a validation fragment has no writable path at all, since the staging root is shredded by the time a candidate is judged.
`network = true`, declared on the generator, re-shares the network and nothing else, and it governs the script and the validation fragments alike.
It lives on the declaration rather than the invocation, so which generators may reach the network is a question your tree answers at evaluation, in a line a reviewer sees.
There is no flag that disables the sandbox, and where no backend is available `safix generate` refuses before the first fragment and names what it looked for.
The staging directory is created mode `0700` on a filesystem safix asks the kernel about rather than infers from its name, and it is overwritten and removed however the run ends.
There is no fallback to `/tmp`, because on a host whose `/tmp` is disk-backed that would leave plaintext in free blocks under a code path that looks like it succeeded.
Where no memory-backed filesystem is available the run refuses, and `--allow-disk-staging` is what accepts a disk-backed one.
`SAFIX_STAGING_DIR` names the mount to use instead of the conventional ones, replacing them rather than being tried first, so a mount safix rejects is a refusal rather than a silent fall back.

### The definition and the stamps a mint records

A generated value carries nothing saying which declaration produced it, so `safix generate` writes a digest of the declaration it ran, in the same commit as the value.
That record lives at `<generatorRecords>/<user>/<name>`, or `<generatorRecords>/shared/<audience>/<name>`, as one plaintext line holding a format tag and a digest.
The covered surface is the script, its `runtimeInputs`, its network grant, its prompts, its dependencies, the outputs it writes with their secrecy, and the validation fragment.
No value and no derivative of a value is in it, which is what lets it be committed in the clear.
A record in a format the running command does not write is read as no record at all, which keeps a change to the digest's canonical form from reporting every value as drifted.
`safix check` reads it back and reports a value whose declaration has changed since it was minted, naming regeneration and reverting the edit as the two remedies and recommending neither.
A value with no record predates the record and is not a finding.
Beside each definition record, in the same tree, sits one stamp record per value, named for the value plus a `.stamps` suffix.
It holds the unix seconds the value was first written and the seconds it last changed, written by `set` and by `generate` in the same commit as the value, and it is what `safix view` prints as `CREATED` and `UPDATED`.
A value written before the record existed has none, and shows as `-` rather than as a date nothing recorded.

## Everyday verbs

| verb | what it does | what it reads | what it writes | needs a terminal |
|---|---|---|---|---|
| `set` | write a value you type | the declarations, and the value from a prompt or standard input | one key's ciphertext, and a commit | only to prompt |
| `edit` | author a value in your editor | the declarations and the current value | the same ciphertext, when the buffer changed | yes |
| `get` | decrypt one key to standard output | the declarations and one document | nothing | no |
| `view` | browse and read, with a preview | the declarations, the stamps, one document at a time | the picker's own state file | only to offer a choice |
| `list` | every name a user holds | the declarations and the stamps | nothing | no |
| `generate` | mint values from generators | the declarations, the definition records, the answered prompts | ciphertext, public outputs, records, stamps, and a commit | only to prompt |
| `check` | report drift, change nothing | the declarations, the policy, every governed file's recipient stanzas | nothing | no |
| `fix` | converge policy and ciphertext | the declarations and every governed file | `.sops.yaml`, re-wrapped files, and a commit | no |
| `audit` | report where a mapping's sides disagree | the declarations, one document per mapping, the target's own store | nothing | whatever the target's unlock needs |
| `sync` | converge declared relationships | the same as `audit` | the side that has not moved, and a commit for a write on safix's side | whatever the target's unlock needs |
| `keygen` | mint an age identity for a person | nothing declared | that person's own identity file | no |
| `adduser` | declare a person who holds none | the declarations | one module file, `.sops.yaml`, and a commit | no |
| `enroll` | put a hardware key in a person's hands, proven | the declarations and the card | an identity block, a recovery recipient, re-wrapped files, and a commit | yes |
| `group` | edit a group's declared membership | that group's module file | that file, `.sops.yaml`, and a commit | no |
| `upload` | seed a machine's own host identity | the declarations and an operator-held identity | a pre-seed tree, or the machine's own key paths | no |
| `install` | establish a resolved set on a host | a manifest and the documents it names | the generation store | no, and no operator types it |

`install` is in the table because it exists, and it is marked because an activation runs it rather than a person.
Documenting it only where profiles are discussed is what let the inventory disagree with the binary.
`check` and `fix` are this fleet's `git status` and `git add` for secret policy: the declarations are intent, the encrypted files are reality, and `fix` reconciles what is reconcilable and names what needs a person.
`check` reports by finding class rather than by count, and a class is a shape of disagreement.
Policy that no longer matches the declarations, a governed file the policy no longer names, an audience that has shrunk, a value whose generator declaration has changed since the mint, and a placement two entries collide on are those classes.
A new class is a new row rather than a number that was wrong.
`check` decrypts nothing, which is what lets one machine judge files belonging to people whose keys it does not hold.
`safix set` reads the value from standard input when standard input is not a terminal, as in `printf '%s' "$TOKEN" | safix set alice-token`.
A terminal still gets the hidden prompt and the confirmation; what the piped form drops is the confirmation, and only where there is nobody to confirm.

### Browsing and editing

`safix get` writes the value to standard output and is what a pipeline calls, while `safix view` writes it to the terminal and needs one only when it has to offer a choice.
Given a name it writes to standard output where no terminal opens, so a `view` in a pipeline is a working invocation rather than a refusal.
With no name, every entry the user holds is offered in a table of the name, its origin, whether one value serves every carrier, whether a generator mints it, the key it is read under, and the two stamps.
`Tab` adds one more, the file serving it, and `^P` or `--no-preview` suppresses the value pane.
Choosing is a way of naming: the run proceeds as though the chosen name had been an argument, and a lone argument is a user when `flake.safix.users` declares one by that name and an entry's name otherwise.
`Enter` reads the entry under the cursor, `Esc` and `^C` leave without choosing, the arrow keys move the cursor and scroll the columns, and `Backspace` edits the query.
The rows run in reverse alphabetical order from the bottom of the screen upwards, and typing narrows the list without reordering it, which keeps the entry under the cursor from changing identity between two keystrokes.

Terms are separated by whitespace, `"two words"` is one term, and all of them have to match.
A term may name a column — `name:token`, or `n:token`, and likewise `origin`, `shared`, `generator`, `key`, `created`, `updated` and `file`.
`!token` excludes, `+Token` matches the whole cell including its case, `*^api-.*$` is a regular expression, `api-*` and `api-toke?` are whole-cell wildcards, and `api-token|mail-password` matches either.
Four colours, each a fact rather than a decoration: the decrypted value is green, the entry you chose last is yellow, the entry created most recently is cyan, and an empty cell is dim.

It holds one decrypted value at a time, dropping and zeroing the previous one before the next is read, and it is drawn in a region the terminal clears on exit, so no value enters scrollback.
And it stages nothing, so there is no path to hand anybody and no plaintext reaches a file.
A value that does not decrypt is reported in the pane while the list stays usable, and the picker's own state is remembered under `$XDG_STATE_HOME` in a file created `0600`.
Three refusals belong to the picker: no terminal to choose on, which names both remedies; a user who holds nothing, which is a state of the declarations; and leaving without choosing, which exits non-zero.

`safix edit` opens `$VISUAL`, or `$EDITOR` when that is unset, on the entry's decrypted value, and neither set is a refusal naming both.
The command is split on whitespace and run directly rather than through a shell, so `EDITOR="code --wait"` works, and the staged file's path is an argument while the value is not.
A non-zero exit writes nothing, an unchanged buffer commits nothing, an emptied buffer takes the empty-value refusal, and a changed one goes through `safix set`'s own write path.
An entry that holds no value yet opens on an empty buffer, and the buffer lives in the private staging directory generators use, so whatever the editor leaves beside it goes with the directory.
An editor configured to write undo history elsewhere has put plaintext where safix does not look, and that is the limit of the containment.
With no name, `edit` offers `view`'s selection less every public output, because a public value is already plaintext and is not editable, and the editor is settled before the list opens.

### Onboarding a person

The person's part comes first, and the operator never performs it: `safix keygen`, on their own machine, appends to their own identity file and prints only the public half.
Minting someone else's identity means holding their private key, which is the opposite of the custody this package rests on, so it takes an explicit `--for-someone-else`.
`safix adduser carol age1abc...` then writes that person's module file, regenerates `.sops.yaml` and commits exactly those two.
It mints nothing — no age key, no password material, no secret value — and gives the person nothing to hold, because the scaffold declares no secret.
Their first secret is a name under `private` or `carries`, then `safix fix`, then `safix set`.
Everything beyond a custody record is a property of one consumer's module tree, so `adduser` passes the name and the recipient to `flake.safix.onboardingHook` and assumes nothing about what happens next.
`--host` is passed through to the hook and is refused while no hook is configured; running without a hook is supported, and it succeeds having done less and says so.
Whether a person's independence from the operator is real is decided by `recoveryRecipients` — see **Declaring what someone holds**.

### Enrolling a hardware key

A touch is the only thing you do, and everything else happens in one `safix enroll` run.
The card is selected, and its PIV access is provisioned when it is factory-fresh: a generated PIN, a distinct generated PUK, and a random management key put on the card under the PIN.
An age identity is generated in the first empty retired slot, driven under a pseudo-terminal that supplies the PIN, and appended to the file `safix keygen` appends to.
The card's recipient is added to the person's recovery identities, `.sops.yaml` is regenerated, every governed file re-wrapped, and the three committed together.
The recipient is registered with clan through clan's own command where a clan is declared, and `flake.safix.enrollHook` receives the person, the serial and the recipient.
Then the step a hand ceremony never had: the card alone opens a governed file in the person's audience, exercising the PIN and the touch.
An enrollment whose proof has not passed reports itself incomplete and exits non-zero, and nothing is undone, because every step of it was additive.
A backup key is the same verb run again, and a re-wrap that dropped a recipient a file had before the run is refused rather than committed.

No OTP slot is written under any flag, because a programmed challenge-response slot is what opens a password database, and the database has no record of the secret it was built with.
Reading that slot to answer a database's own unlock challenge is a different operation, and that is what the sync verb does where a database declares one.
`--touch-policy never` is refused, because the touch is the property a card is for, and a run with no terminal is refused before the card is touched.
No credential this verb generates reaches an argument vector or an environment variable: `ykman`'s credential options are omitted so that it prompts, and the prompts are answered on a pseudo-terminal.
The management key is stored nowhere, because PIN possession is management possession and a stored copy would be a credential with no reader.
The PIN and PUK land in the person's own custody by default, with an honest caveat: a PIN readable by the software identity adds protection only once that identity is retired or absent.
`--no-store-pin` turns that off, and `--mirror-to-store` writes them to the password store as well.

### Seeding a machine's host identity

A machine's declared recipient is the age form of an ed25519 host key the operator holds, and `safix fix` wraps every audience naming the machine to it as soon as the declaration exists.
Nothing else mints that key or gets its private half onto the machine's own disk, so `safix upload <machine>` closes that one gap, once, before the machine's first activation.
`--directory DIR` writes a pre-seed tree to an operator-named directory and touches no network, at the paths and modes a fresh install's own key generation would produce.
`--to ADDRESS` probes the host's currently presented ed25519 key, unauthenticated, before writing anything, and takes one of three actions.
A target already presenting the declared key gets an honest no-op, even with `--force` and `--identity` both given; a target presenting no key writes given `--identity`, and otherwise refuses.
A target presenting a different key refuses by default, naming both recipients, and proceeds only with `--force` together with `--identity`.
A remote write mirrors clan's own transport: both files travel inside a gzip tarball built in the private staging root, into a fixed destination that is wiped and then extracted into.
This verb provisions machines and never people, so a person's name is refused the way an undeclared machine is; there is no systemd-credentials delivery path, and a pre-seed tree is a plain filesystem tree.
And nothing here triggers a deploy, a switch or a rebuild: the machine's own next rebuild activates what was written, and the command's success output says so.

### Editing a group's membership

`safix group add <group> <subject>` and `safix group remove <group> <subject>` insert or remove one name in the `members` list in that group's own module file, parsed before anything is staged.
`.sops.yaml` is regenerated from the declarations that edit implies, and the two are committed together.
It writes no value and re-wraps nothing: a membership change is a reason to run `safix fix`, and the report says so.
`remove` prints what removing cannot do, because a subject that has been in a group has held the data key of every file that group's audience names — see **How safix thinks**.

## Syncing to other stores

A value a program reads is often also a value a person reads, and the copies drift.
`safix sync` converges a declared secret with one entry in another store, and `safix audit` reports on the same declarations without writing.
A mapping is that declared relationship: it names one safix entry, one address in the target's own store, and the mode the two converge under.

```nix
flake.safix.keepassxc.mappings.grafana = {
  mode = "two-way";
  safix = { user = "alice"; name = "grafana-password"; };
  kdbx = {
    path = "alice/grafana";
    fields = {
      username = "alice@example.com";
      url = "https://grafana.example";
      notes = "where this credential came from";
    };
  };
};
```

The mapping's identifier is its own name and is neither endpoint's: it is the word you pass to narrow a run, and an identifier colliding with a target's own keyword is refused at evaluation.
A mode is written as its two endpoints, in the order the value moves, and the endpoints are this package's own name and the target's word.
So `clan-to-safix` moves a value out of clan and into a safix entry, and the same pair with the endpoints swapped moves it the other way.
`two-way` converges toward whichever side changed since the last agreement, and `backup` writes safix's value where the target holds none and never overwrites one that differs.
Every run reads both sides before it writes either, so a mapping whose sides agree is not written and not committed.
A conflict is a finding and never a guess: where both sides have moved since the last agreement, nothing is written, and the report names the two one-way modes that each resolve it.
The last agreement is remembered beside the mapped entry, as one line holding a format tag and a fingerprint of the agreed value, and the fingerprint never reaches safix's own plaintext trees.
Where the target can hold a hidden field on the item itself, the memory lives there; where it cannot, it is a companion entry named for the mapped one plus a reserved suffix.
Evaluation refuses a declared address carrying that suffix, and deleting a companion is safe and returns the mapping to bootstrap.
Nothing is ever deleted on either side, in any mode: remove a mapping and its last value on the target stays until a person removes it, and the report says nothing declares it.

The far side also carries fields, which is everything beside the value that a store shows a person.
Each field is either a literal string or `{ entry = "<name>"; }` naming another entry of the mapping's own person, decrypted at run time rather than interpolated at evaluation.
Fields are declarations, so they have one author: a pulling mode writes only the value into safix, because a safix entry is a placement with no slot for a URL.
A pushing mode writes the declared fields beside the value in the same write, `backup` writes them only where it writes a value, and a two-way mapping's fields are push-only.
A field the target cannot carry is refused at evaluation, naming the target and the field, rather than approximated or dropped.
A field whose source is another entry is a secret value, so a target whose only channel for that field is an argument vector refuses it as well.

### clan

#### What it addresses

A var inside another flake, by the machine that owns it, the generator that produces it, and the file within that generator.

#### How it is declared

`flake.safix.bridge.clanFlake` holds the other side, and each `flake.safix.bridge.mappings.<id>` names `clan.machine`, `clan.generator` and `clan.file`.
This target spells its mode `direction`, which predates the shared vocabulary and is the one place the word differs.
`placement = "shared"` says the clan side is one var no machine owns, so `machine` is refused and the runtime tries each machine clan lists until one resolves.
`safix sync clan` converges every mapping; naming mappings narrows the run, and `--direction` narrows it to mappings declared with that value.

#### What it can carry

Nothing beside the value, and the heading stays to say so: a clan var is a file's bytes, and clan's own command offers no field beside it.

#### How it unlocks

Nothing of its own, because every read and every write is clan's own command with the value on a pipe, so clan's credentials and backends apply unchanged.
safix reads, writes, encrypts, decrypts and parses none of clan's stored files, and a consumer without clan's command cannot reach clan's side at all.

#### What it refuses

Locally: an unresolvable safix side, a pull into a generator-produced value, two mappings writing one target, one pair of endpoints declared both ways, and a mapping with no clan flake.
At transfer time: a clan side that does not resolve, refused in clan's own words, and a push out of an entry holding no value.
A push into a generator clan considers outdated is refused with no override, because clan's next routine generation would replace what was written without saying so; the refusal names both remedies.

### keepassxc

#### What it addresses

A path inside an encrypted database on this machine, under a declared group.

#### How it is declared

`flake.safix.keepassxc.database` is a string rather than a nix path, because a path is copied into the world-readable store on every evaluation and this file is large.
`flake.safix.keepassxc.group` is the group entries live under, `flake.safix.keepassxc.yubikey` names a challenge-response slot, and `flake.safix.keepassxc.keyFile` names a key file.
Each `flake.safix.keepassxc.mappings.<id>` addresses its entry through `kdbx.path` and declares its far side under `kdbx.fields`.

#### What it can carry

`username`, `url` and `notes`, as literals only.
`tags` is refused, because this store's command has no flag for one, and a field sourced from another entry is refused because the only channel here is an argument vector.

#### How it unlocks

With a composite key, asked for once per run on the terminal, and the run refuses before reading anything where there is none.
The password travels standard input, and so does every value; the session's secret service is not a second way in, because the collection it publishes is its own exposed group.

#### What it refuses

A value carrying a newline, because the store's command reads a password as one line and nothing here trims the byte for you.
A declared path carrying the companion suffix, a mapping whose safix side does not resolve, a pull into a generator-produced entry, and two mappings naming one entry.
No database is created, no database key is changed, and no hardware slot is written under any flag.

### pass

#### What it addresses

A path under a store root, which is one gpg-encrypted file per entry.

#### How it is declared

`flake.safix.pass.store` is the store root, a string with `~` expanded by the runtime, and it reaches the store's command in the child's environment as a path and never a value.
Each `flake.safix.pass.mappings.<id>` names `pass.path` and its `pass.fields`.

#### What it can carry

All four fields, including a field sourced from another entry, because the whole record crosses on one pipe as a value followed by a trailing block of fields.
A multi-line value crosses whole, because this store's read is byte-exact and imposes no one-line rule.

#### How it unlocks

Nothing of its own, and the heading stays to say so: the store shells to gpg, so the unlock belongs to the ambient agent.
The preflight is that the store exists, and a locked or refusing agent is reported as exactly that rather than as a generic command failure.

#### What it refuses

A declared path carrying the companion suffix, a mapping whose safix side does not resolve, a pull into a generator-produced entry, and two mappings naming one path.
Nothing here initialises a store or manages its recipients, because a store's own recipient file is its audience declaration.

### bitwarden

#### What it addresses

An item in a personal vault, by an optional folder and the item's own name rather than by the store's item id, because an opaque identifier is not a reviewable declaration.

#### How it is declared

`flake.safix.bitwarden.server` names a self-hosted server, and `null` means whatever server the operator's own client is configured against.
Each `flake.safix.bitwarden.mappings.<id>` names `bitwarden.folder`, `bitwarden.item` and `bitwarden.fields`, and the last agreement lives in a hidden custom field on the item.

#### What it can carry

`username`, `url` and `notes`, each as a literal or sourced from another entry, crossing as JSON on standard input in both directions.
`tags` is refused, because this store has no tag concept: folders and collections are the only grouping, and both are placements rather than labels.

#### How it unlocks

By prompting once for the master password, which travels the child's standard input, and the session key it returns travels to every later child in that child's environment and nowhere else.
That is the one place a value-bearing environment variable is accepted: an argument vector is world-readable through `/proc`, where an environment variable is readable by the same account alone.
No master password and no mapped value is in an argument vector or an environment on any invocation, safix never logs a vault in, and an unauthenticated client is reported as locked.

#### What it refuses

Two items of one name in one folder, reported as ambiguous rather than guessed at, and a declared server differing from the one the unlocked session reached.
A failed pre-read synchronisation, since a stale local copy would be compared as though it were the vault.
An absent item under a mode that needs one, an unresolvable safix side, a pull into a generator-produced entry, and two mappings naming one item.

### 1password

#### What it addresses

An item in a named vault.

#### How it is declared

`flake.safix.onepassword.account` is an account shorthand or sign-in address, and `null` means whatever account the command itself resolves.
Each `flake.safix.onepassword.mappings.<id>` names `onepassword.vault`, `onepassword.item` and `onepassword.fields`, and the last agreement lives in a concealed field on the item.

#### What it can carry

All four fields, including a field sourced from another entry, because the whole item crosses as JSON on standard input.
safix never spells a `field=value` argument word, since this store's own documentation records that such assignments are logged in shell history.
An edit is a round-trip rather than a template: a write starts from the item's own JSON, replaces only the declared fields, the value and the memory, and writes the whole object back.

#### How it unlocks

By inheriting a session the operator already established, or a service-account token, from safix's own environment.
safix runs no sign-in of its own, passes no session token in an argument vector, and a signed-out run is refused before any mapping's safix side has been decrypted.

#### What it refuses

An absent vault, named as such rather than reported as a failed command.
An absent item under a mode that needs one, an unresolvable safix side, a pull into a generator-produced entry, and two mappings naming one item.
No verb this target issues deletes anything.

### Auditing a target's two sides

`safix audit <target>` compares both sides of every declared mapping, or of the ones named after the target, and changes nothing on either side.
A mapping agrees when both sides hold the same bytes, and also when neither side holds a value yet, which is a relationship nobody has bootstrapped.
A divergence names the mapping, its two endpoints and the command that converges it, and never a value.
Where the values agree and a declared field does not, the report names the diverged field and not its content, because a note may itself be sensitive and a resolved field is a secret.
A mapping that could not be judged is reported as such rather than quietly left out, since a report that dropped those would be a report about who ran it.
Alongside the findings it names every entry under the declared address space that no mapping accounts for, as information that never moves the exit status.
It is a verb of its own rather than more rows in `check`, because `check` decrypts nothing and needs no target, while comparing a mapping's sides decrypts safix's side and runs the target's own command.

## Where files go

### The three storage roots

```nix
flake.safix.storage = {
  encrypted        = ".safix/encrypted";
  plaintextOutputs = ".safix/plaintext-outputs";
  generatorRecords = ".safix/generator-records";
};
```

safix places files in exactly three trees, each a repository-relative path you name, defaulting to `secrets/safix`, `public/safix` and `state/safix/definitions`.
Evaluation refuses a root that is empty, absolute, ends in `/` or carries a `..` component, and refuses any two of the three that are equal or nested, naming both options and both values.
Comparison is on component boundaries, so `secrets/fleet` and `secrets/fleet-public` are disjoint while `secrets/fleet` and `secrets/fleet/pub` are one inside the other.

### Your own ignore, backup and exclusion rules

A file safix did not place is not in the set `safix fix` re-wraps, though it still rides its audience's rule, because every rule covers one directory level rather than one literal filename.
So a change of audience reaches every file safix placed and leaves that one behind, encrypted to whoever it was encrypted to when it was written.
`flake.safix.extraGovernedFiles` is a list of such paths, and naming one there puts it into the set `fix` re-wraps and the checks judge.
The case it exists for is a value with no declaration of its own, decrypted on demand by whatever invokes it — a credentials command in a module of your own, reading one key with `sops -d --extract`.
Use that shape for a credential only a person invokes, and a declared secret for anything a service reads from a path.
safix writes no `.gitignore` at the declaration root, and exactly one at a vault root, covering only the scratch rules file; everything else is yours.
The encrypted tree is ciphertext without qualification and belongs in a backup, while the plaintext-output and generator-record trees hold values a module reads and digests with no value in them, and belong in the repository.

### Renaming a root

Change the option, `git mv` the tree, run `safix fix`, run `safix check`.
No re-encryption is involved: a sops document does not embed its own path, the in-document key names do not change, and `fix` rewrites `.sops.yaml` and nothing else.
A half-finished rename is visible rather than silent, because between the option change and the `git mv`, `safix check` reports every governed file the regenerated policy no longer names.

### A vault: ciphertext in a second repository

```nix
flake.safix.vault = {
  root = inputs.vault; # a second repository, taken as a non-flake input
  namingKey = "<64 or more lowercase hexadecimal characters>";
};
```

`flake.safix.vault.root` moves every ciphertext document, every public value and every definition record to that repository, in place of this flake's own source.
`flake.safix.vault.namingKey` makes every vault-rooted name opaque: a document's file name, the key inside it, a public value's file name and a definition record's path are each a hash of the key, a use-specific tag and the readable name.
A vault host learns none of the audience, key or secret names the declaring flake's tree carries, while the declaring flake still computes both forms and the mapping between them.
`SAFIX_VAULT_ROOT` names the operator's own working tree of that repository, the one a command writes and commits into, and nothing here applies while no vault is declared.
A vault document is not browsable by hand, because `.sops.yaml` never moves and a bare `sops` run against a vault-rooted document finds no creation rule above it.
`set`, `edit` and `get` are the tools there; each renders the disposable rules the write needs, uses them, and removes them again.
A command touching both roots commits the vault first and the declaration root second, with a trailer naming the first commit.
Opacity is a property of names rather than of shape: a vault host still sees how many documents there are, how many keys each holds, each ciphertext's length, every document's recipient keys, and one commit per write.
Adopting or abandoning a vault is `safix fix`'s job: it decrypts each readable-layout file under your own identity, re-encrypts it into its opaque destination, and removes the readable copy.
`safix fix --vault-rollback` runs the same move the other way while the vault is still declared, and rotating the naming key is the identical migration run again.

## Establishing secrets in a profile

Custody is declared once, at flake level, where every user is visible at the same time.
Arrival is declared per profile, in the module system that profile is written in, through a `safix.*` namespace that can select but never declare.

### The option surface

Both modules declare the same options, and none of them can add a secret, a recipient, a grant or an audience.

| option | default | what it is |
|---|---|---|
| `safix.enable` | whether anything resolved | the gate the whole module sits behind |
| `safix.flake` | `null` | your own flake — `inputs.self` — from which `safix.lib` is read |
| `safix.lib` | from `safix.flake` | the resolver projection, settable directly where your flake reaches the profile some other way |
| `safix.user` | the profile's own username; none at system scope | which declared person this profile serves |
| `safix.machine` | `null` | which declared machine this profile serves instead of a person; its services' entries arrive with it |
| `safix.hostname` | the host's own name | which host to resolve on, since `perHost` and `perTag` select by it |
| `safix.tags` | the declared tags of `safix.machine`, else `[ ]` | the tags this host carries, against which `perTag` selects |
| `safix.secrets` | read-only | what resolved, typed by safix's own entry submodule |
| `safix.identity.keyFile` | `null` | an age key file this scope decrypts with |
| `safix.identity.sshKeyPaths` | `[ ]` | ssh private keys this scope decrypts with |
| `safix.identity.generateKey` | `false` | user scope only: mint `safix.identity.keyFile` at activation when it is absent |
| `safix.identity.deriveHostKeys` | `true` | system scope only: derive an identity from the host's own ssh keys |
| `safix.identity.derivedHostKeys` | read-only | which keys that derivation chose |
| `safix.identityPreflight` | `true` | user scope only: install the activation guard below |
| `safix.installer.package` | safix's own build | the build whose installer runs |
| `safix.installer.validationPackage` | the build-platform build of the same | what checks the manifest inside its own derivation |
| `safix.installer.validate` | `true` | check the manifest against its documents at build time, and hash them into it |
| `safix.installer.keepGenerations` | `1` | how many generation directories to keep |
| `safix.installer.log` | `[ ]` | which of `keyImport` and `secretChanges` reach the journal |
| `safix.installer.secretsMountPoint` | `/run/safix.d`; runtime-relative at user scope | where the generation directories live |
| `safix.installer.symlinkPath` | `/run/safix`; runtime-relative at user scope | where the current generation appears, and what an entry's default path is under |
| `safix.installer.useTmpfs` | `false` | system scope only: mount the generation store on a tmpfs |
| `safix.installer.environment` | `{ }` | system scope only: the unit environment the installer runs under |
| `safix.installer.agePlugins` | `[ ]` | system scope only: age plugins the installer makes available |
| `safix.installer.useSystemdActivation` | follows the host | register as a unit rather than as an activation script |
| `safix.installer.afterActivation` | `[ ]` | activation steps this install is ordered after, as in `[ "setupSecrets" ]` |
| `safix.installer.afterUnits` | `[ ]` | units this install is ordered after, as in `[ "age-decrypt-secrets.service" ]` |
| `safix.installer.manifest` | read-only | user scope only: the manifest this profile built |

`safix.secrets.<name>` carries the resolution of one entry, and the installer reads every field of it.
`name` and `key` say what to write and which key inside the document holds it, while `path` and `mode` say where it lands and with which bits.
`owner` and `group` name the account, with `uid` and `gid` used where those are null; `sopsFile` and `format` name the document and its shape.
`restartUnits` and `reloadUnits` are what the installer acts on when an entry is new or its value changed.
Each scope publishes one module under both its plain name and its default name, and the two name one value.
So `homeModules.safix` and `homeModules.default` are one file, as are `nixosModules.safix` and `nixosModules.default`, and `homeManagerModules` is a published alias of `homeModules`.
Both names stay published so that every `imports` line written against the previous surface keeps resolving, and importing two distinct store paths of one option-declaring module is still an error rather than a merge.
`safix.flake` is the one thing a module cannot derive, because requiring a particular name in a profile's arguments would make your evaluation seam part of safix's interface.
Standalone home-manager cannot derive a hostname either, so `safix.hostname` is the fourth line there and the identity is the fifth.
The mode, the path and the key are identical in both scopes, and nothing in a declaration names a scope; the system scope additionally carries the ownership axis.
A user-scope profile refuses an entry that sets `owner` or `group` rather than dropping it, because a dropped ownership field would read afterwards as an ownership claim that was honoured.
Two entries resolving onto one path are refused for either scope, since whichever declaration activates second unlinks the first's output.

### Identity, and the activation guard

`safix.identity.keyFile` defaults to null, and that default is not a preference: the installer treats a set-but-unreadable key file as fatal, so a non-null default would abort activation on every machine lacking the path.
At system scope a named identity wins, and otherwise safix derives the ed25519 entries of the host's declared ssh keys that lie outside its own store.
A gnupg configuration counts for nothing at either scope, because a key file and a list of ssh keys are the only identities safix can decrypt with.
At user scope there is no such default to keep, so naming one of the two is not optional.
At user scope safix also installs an activation guard: it reads the configured identity, checks each path for presence and readability, refuses the switch when none is usable, and decrypts nothing.
It is ordered before home-manager links any file, which is what makes the refusal atomic: no home file linked, no user package installed, no user unit restarted, no secret written.
Where home activation runs as a host's own per-user unit, the system generation has already switched, so only that user's home generation is held back.
And presence and readability are all that were checked: a key that is readable but is not a recipient of these files still fails later, inside the installer, when it decrypts.
The system scope installs no such guard, because no atomic refusal point at system activation has been demonstrated, and safix does not document a guarantee that nothing enforces.

### The installer safix owns

safix builds its own manifest and runs its own program against it, which is the one verb no operator types.
What arrived is read back at `safix.secrets`, the resolution the manifest is built from, so what you read and what is installed cannot disagree.
The manifest schema is safix's own, written down once in the runtime, versioned, and refusing an unknown field rather than ignoring it.
The document mode opens each named document and verifies every declared key is in it, while the manifest mode validates the schema, the version, every mode's octal parse and every owner and group resolution.
Neither mode decrypts, because a sops document carries its mapping keys in the clear.
With validation on, the manifest also carries one hash over every distinct document it names, so editing a ciphertext file changes the derivation and causes a rebuild.
The store is safix's own: generations under one root, the current one at a symlink, both movable through their options, and nothing outside it is written, removed or mounted over.
At system scope the installer registers as an activation script, or as a unit where the host manages users through systemd's own mechanisms, and ordering is yours to name.
safix reads no option of another secret-management framework to discover its installer — see **Fitting safix to a tree you already have**.
At user scope the profile installs rather than delegating, with the same manifest shape under a user-mode flag, a user unit on linux, and an activation entry on both platforms.
A user-mode install mounts no filesystem, changes no file's ownership, and restarts or reloads no unit, because each of the three needs a privilege the scope does not have.
`safix.identity.generateKey` mints the configured key file at activation when it is absent, defaulting off, because a profile that has never been activated has no other way to get one into place.

### Refusals you may hit, and what each one asks for

A profile bound to declarations but naming neither a person nor a host refuses, naming the option that is unset, and one that names either and is bound to nothing refuses naming `safix.flake`.
That state is refused rather than tolerated, because an empty resolver would make every other refusal here vacuously true, and the profile would build, establish nothing and report nothing.
A profile that imports the module and sets nothing at all is a no-op, and so is one whose person resolves nothing on that host; the two are told apart by whether a definition exists, never by its value.
Naming a person no declaration declares refuses and lists the declared people, which is likelier than it looks, since the option defaults to the profile's own username.
Where a resolution is non-empty and no identity is configured or derivable, evaluation fails naming all three identity options.
Before decrypting, the installer checks each configured identity path for presence and readability and refuses naming each path and the two ordering options.
Three things are unsupported and refused rather than silently omitted: no template is rendered, no secret is relocated for early-boot user creation, and no gnupg identity is accepted.
The limit of the coexistence is stated too: it covers safix's own installer, and a consumer who writes another framework's secrets option directly still has that framework's own collisions.

## Fitting safix to a tree you already have

safix reads no option outside its own namespace.
That is what makes an adapter a projection you write rather than an integration you maintain.
A tree that runs another secret-management framework keeps every option it set there, and safix is unaffected by whichever revision of it that tree pins.
safix defines nothing outside its namespace either, and it holds no record of your users, hosts or units to reconcile with yours.
What it costs you is one projection, written once, in your own tree.

```nix
{ config, lib, ... }:
{
  flake.safix.users = lib.mapAttrs (_name: person: {
    recipient = person.meta.ageRecipient;
    recoveryRecipients = lib.mapAttrs (_anchor: key: { inherit key; }) person.meta.ageRecoveryKeys;
  }) config.flake.users;
}
```

`flake.safix.users` is safix's own record and carries only custody, so a person can exist in your registry and hold nothing here.
`flake.safix.machines` takes a projection on the same terms, from a host inventory, and `flake.safix.services` from whatever record already says which units run where.
Everything above assumes a flake-parts consumer, and safix's evaluation rests on two narrower things.

```nix
# an entry file, reachable with safix --entry
{
  safix = {
    lib = (import <safix>).lib.mkVault { modules = [ ./secrets.nix ]; root = ./.; };
    onboardingHook = null; # or a literal shell fragment, set here directly
    enrollHook = null;
  };
}
```

`mkVault` is the entrypoint for the first of those, published at `flake.lib.mkVault` and defined as a plain function of `{ lib }` in the repository's own `lib/`.
Calling it evaluates `modules` together with safix's own resolver, hands `root` to `_module.args.self` unchanged, and returns exactly what a flake-parts consumer's `flake.safix.lib` holds.
Declarations passed through `modules` scatter and merge exactly as a flake-parts `imports` list would, and a module declaring an option outside safix's namespace is refused.
`mkVault` returns only the resolver half, so `flake.safix.onboardingHook` and `flake.safix.enrollHook` are declared beside it, as above; they are siblings of the projection rather than fields inside it.
`--entry <file>`, and its environment form `SAFIX_ENTRY`, are the second narrowing, in the command rather than in nix.
Both are read as a leading global option ahead of any subcommand, alongside a global `--nixpkgs <flake-ref>` and its form `SAFIX_NIXPKGS`, and a flag beats its own variable.
Root discovery does not move: the repository is still found through git, and an entry file need not live inside it.
`generate`'s sandbox resolves its own tools through a flake, so running it under `--entry` with neither nixpkgs form declared refuses before the first fragment, naming both remedies.
`examples/plain-nix/` is a working copy of that recipe, `examples/dendritic/` declares the same fleet behind flake-parts, and `examples/README.md` indexes both.

```nix
checks = config.flake.safix.lib.mkChecks pkgs {
  committedPolicy = ./.sops.yaml;
  materializations.alice-workstation = /* what your profile materializes */;
};
```

`mkChecks` is a published function you call, and with no arguments it returns checks over your declarations alone.
Those cover the custody refusals, the generator runtime tools, the shape of every generated rule, the absence of a catch-all, the non-interaction between the rules and the public store, the audience separator, and each sync target's own mappings.
`committedPolicy` adds the drift check, which fails while the committed and the generated policy differ, and whose failure names `safix fix`.
`materializations` adds the path-collision check, and forces the materializations you hand it so that the refusal reaches the hosts nobody has built this week.

A consumer arriving from the sops-nix-backed surface renames options and nothing else.
safix declares no sops-nix input and reads and defines no option of it, and the `sops` binary is unchanged and still does every encryption and decryption as a subprocess.
Change nothing about `.sops.yaml`, the file layout, `safix fix` or any custody declaration; rename any option you tuned; and, at user scope, update anything that referenced the old secret path.

| was | becomes |
|---|---|
| `sops.package` | `safix.installer.package` |
| `sops.validationPackage` | `safix.installer.validationPackage` |
| `sops.validateSopsFiles` | `safix.installer.validate` |
| `sops.keepGenerations` | `safix.installer.keepGenerations` |
| `sops.useTmpfs` | `safix.installer.useTmpfs` |
| `sops.log` | `safix.installer.log` |
| `sops.environment` | `safix.installer.environment` |
| `sops.age.plugins` | `safix.installer.agePlugins` |
| `sops.age.keyFile` | `safix.identity.keyFile` (already existed; safix stops defining the `sops` one from it) |
| `sops.age.sshKeyPaths` | `safix.identity.sshKeyPaths` (already existed; likewise) |
| `sops.age.generateKey` | `safix.identity.generateKey`, home scope only |
| `sops.gnupg.home`, `sops.gnupg.sshKeyPaths`, `sops.gnupg.qubes-split-gpg.enable` | no equivalent; gnupg is not an identity safix accepts |
| `sops.defaultSopsFile`, `sops.templates`, `sops.placeholder`, `sops.useSystemdActivation` | never read by safix; a consumer setting them was configuring their own sops-nix, which is unaffected |
| `imports = [ safix.nixosModules.default ]` | unchanged; `.default` and `.safix` are now one value |
| `~/.config/sops-nix/secrets/<name>` | the runtime directory under `safix.installer.symlinkPath` |

The last row is the one that breaks something: a user-scope secret now arrives in a runtime directory rather than a home directory, so it does not survive a reboot without a login.

## The opinions safix will not bend

Placement is derived from the audience and never authored: an entry carrying a document of its own is refused, because such a file's recipients are outside the computation that produced the policy.

There is no catch-all rule and the generator emits none, so an unmatched path fails closed with the encryption tool's own "no matching creation rules found" rather than acquiring a default recipient set.

Every rule is start-anchored under the encrypted root, extension-terminated, and one directory level.

The recipient policy is generated and committed, never hand-edited, because the encryption tool reads the committed file off disk and that is the version deciding what a new file is encrypted to.

Narrowing an audience is not revocation — see **How safix thinks**.

safix reads nothing outside its own namespace — see **Fitting safix to a tree you already have**.

Key generation belongs to the person who will hold the key: `adduser` mints nothing, and minting someone else's identity takes an explicit flag naming what it is.

A recipient that needs a physical interaction to decrypt is refused for a person's primary recipient, because activation decrypts non-interactively; such an identity belongs among their recovery identities, where it is additive.

## Where the pieces live

| concern | file |
|---|---|
| the records a consumer declares | `modules/flake/safix/options.nix` |
| the option types and their reference documentation | `modules/flake/safix/types.nix` |
| the resolution algebra | `modules/flake/safix/resolve.nix` |
| the fields a mapping's far side carries | `modules/flake/safix/fields.nix` |
| the mapping identifiers no consumer may reuse | `modules/flake/safix/reserved.nix` |
| the clan bridge's mappings and their refusals | `modules/flake/safix/bridge.nix` |
| the password database's mappings and their refusals | `modules/flake/safix/keepassxc.nix` |
| the pass store's mappings and their refusals | `modules/flake/safix/pass.nix` |
| the Bitwarden vault's mappings and their refusals | `modules/flake/safix/bitwarden.nix` |
| the 1Password vault's mappings and their refusals | `modules/flake/safix/onepassword.nix` |
| the recipient policy renderer | `modules/flake/safix/policy.nix` |
| the checks a consumer instantiates | `modules/flake/safix/checks.nix` |
| the flake module a consumer imports | `modules/flake/safix/default.nix` |
| the consumption options both scopes share | `modules/consume/common.nix` |
| the home-manager module and its activation guard | `modules/consume/home.nix` |
| the NixOS module | `modules/consume/nixos.nix` |
| the runtime, as a library and as the command `packages.safix` | `crates/` |
| the worked consumers | `examples/`, indexed by `examples/README.md` |
| how to work on this repository | `CONTRIBUTING.md` |

| tree | option | default | with a vault declared |
|---|---|---|---|
| encrypted values | `flake.safix.storage.encrypted` | `secrets/safix` — `users/<u>/secrets.yaml` and `shared/<audience>/secrets.yaml` below it | `secrets/<opaque-hash>.yaml` at the vault root |
| public outputs | `flake.safix.storage.plaintextOutputs` | `public/safix` — `users/<u>/<name>/value` and `shared/<audience>/<name>/value` below it | `public/<opaque-hash>` at the vault root |
| the per-value records | `flake.safix.storage.generatorRecords` | `state/safix/definitions` — the definition record and its stamps, one plaintext line per file | `state/<opaque-hash>` at the vault root |

`.sops.yaml` is written by `safix fix`, never by hand, and stays at the declaration root even with a vault declared.
`private`, `carries`, `sharedWith`, `shared = true` and `perHost and perTag` are now **Declaring what someone holds**.
`Subjects` is **The subjects that can hold a key**, and `The one mental model` is **How safix thinks**.
`Browsing what is there`, `Editing a value`, `Onboarding a person`, `Enrolling a hardware key` and `safix group` are under **Everyday verbs**.
`The bridge to clan` and `The mirror in your password database` are subsections of **Syncing to other stores**.
`The three storage roots`, `Values without declarations` and `A vault` are **Where files go**.
`Wiring it to your own user registry`, `Without flake-parts`, `The checks safix hands you` and `Migrating from the sops-nix-backed surface` are **Fitting safix to a tree you already have**.
`Status` moved to `CONTRIBUTING.md`, as `Where the suite stands`.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
