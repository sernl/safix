# safix

safix is built for its operator's own fleet; that use case, not general adoption, decides its opinions.

safix is a custody-first secrets manager for nix.
Secrets are declared as flake-parts module options, the encrypted file each secret lives in is derived from the audience that can read it rather than authored by hand, and the `.sops.yaml` recipient policy is generated from those same declarations.
It is tied to no framework and serves NixOS and home-manager alike through sops-nix.

Its headline opinion: declarations may be scattered anywhere across your tree, one per file, because they are mergeable attrsets — but ciphertext placement is never scattered, because the audience picks the file.

## Quick start

Add the input, import the module, declare a person.

```nix
{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  inputs.flake-parts.url = "github:hercules-ci/flake-parts";
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

That is the flake half.
`flake.safix.lib` now holds the audiences, the placements, the generated policy text and the check builders, and `packages.safix` is the command.
Put `safix` in your devshell and run `safix fix` once to write `.sops.yaml`, then `safix set alice-token`.

The profile half is an import and four lines, in whichever module system alice's secrets are to arrive in.

```nix
# alice's home-manager profile
{ inputs, ... }:
{
  imports = [ inputs.safix.homeModules.default ];

  safix.flake = inputs.self;
  safix.user = "alice";
  safix.hostname = "workstation";
  safix.identity.sshKeyPaths = [ "/home/alice/.ssh/id_ed25519" ];
}
```

Every secret alice resolves on that host is now established there.

`nixosModules.default` is the first three of those lines for a system configuration.
The fourth is the user scope's alone: sops-nix's NixOS module defaults its age identity to the ed25519 keys of `config.services.openssh.hostKeys`, and its home-manager module has no per-person equivalent to fall back on, so a profile that resolves secrets and names no identity refuses at evaluation rather than establishing them.
See [Establishing secrets in a profile](#establishing-secrets-in-a-profile) for the rest of the surface, including which of the two module forms to import.

Declarations merge, so the flake block above can live in its own file imported alongside a hundred others; safix reads no path, no filename and no directory structure to find them.

## The one mental model

A secret has three questions: who declares it, who can read it, and where it lands.
Declaring happens in nix and is the label on the box.
Reading is decided by the `.sops.yaml` recipients, which are generated from the declarations and never hand-edited.
Landing means the file a profile reads at activation, by default the secret provisioner's own path for the name.
Everything below is a different answer to the first two questions.

The distinction that does the work is placement versus custody.
Custody is who holds a secret, and it is a property of a subject — a person, a machine, a service, a group of subjects, or an organization: for a person it is the same on every host they log into.
Placement is where the decrypted value shows up, and it is a property of a configuration.
Every refusal safix makes comes from keeping those two apart.

## private: mine alone

```nix
flake.safix.users.alice.private = {
  filen-key = { };
  ssh-personal.mode = "0600";
};
```

```console
$ safix set filen-key    # prompts hidden, encrypts, commits
$ safix get filen-key    # prints the value, for piping
```

Think of it as alice's drawer.
Declaring an entry here is the whole story: there is no catalogue entry and no separate selection step, because a private declaration is its own selection.
Only the holder can read it.

## carries: I take one from the shelf

```nix
# the shelf, declared once
flake.safix.catalogue.cognee-api-key = { };

# alice taking one
flake.safix.users.alice.carries.cognee-api-key = { };
```

Think of it as a shelf of standard items.
The shelf says this thing exists; carrying says I have one.
By default each carrier gets their own copy with their own value: if bob also carries `cognee-api-key`, his value and alice's are unrelated.
Same label, different contents.

## sharedWith: I hand you a copy of my thing

```nix
flake.safix.users.alice.sharedWith.bob = {
  linear-credentials = { };
};
```

```console
$ safix fix
$ sops secrets/safix/shared/alice,bob/secrets.yaml
```

Think of it as a shared drawer between exactly those two.
The directory name is the guest list: `alice,bob` means those two can open it and nobody else can.
`fix` regenerates the rule for the new audience file; moving the value in is a keyholder's act, which is why the second command is `sops` in your hands rather than something automatic.
On the recipient's next rebuild the secret appears in their resolved set at their own path, and they declare nothing.

Revoking is deleting the grant and running `safix fix`.
The recipient has already seen the value, so truly taking it back means minting a new one.
Revocation is not retroactive, and that sentence is written on the `recipient` option, on the grant, and in the generated policy's own header — at each of the three places where someone decides to narrow an audience.

## shared = true: one team key, not copies

```nix
# on the shelf itself
flake.safix.catalogue.team-api-token.shared = true;

# both of them carry it
flake.safix.users.alice.carries.team-api-token = { };
flake.safix.users.bob.carries.team-api-token = { };
```

Think of it as the office wifi password.
There is exactly one value, and everyone who carries the entry reads the same bytes from one audience file.

The contrast with plain `carries`:

| | `carries` (default) | `carries` + `shared = true` |
|---|---|---|
| values | one per person, independent | one, total |
| a person joins | gets their own empty slot | can read the existing value |
| a person leaves | nothing happens to yours | rotation needed — they have seen it |

The last cell is why `safix check` reports a shrunk audience as a revocation rather than as a re-wrap.
The signal is derived from the file's own recipient stanzas — a stanza belonging to someone who is no longer a carrier — so no state file records the former audience.

## perHost and perTag: where it shows up, not who owns it

```nix
flake.safix.users.alice.perHost.builder = {
  omit.filen-key = { };
};
```

Think of it as which rooms my keys follow me into.
The secret is still alice's everywhere; it simply does not land on that host.
This is why a carrier of a shared entry who omits it on one host stays in the audience: omitting is about placement, and custody is about carrying.
Reaching an entry only through a `perHost` or `perTag` `add` is refused, because a host-scoped selection puts nobody in any audience and would leave that person resolving a file they are not encrypted to.

## Subjects: machines, services, groups, silos, ownership, organizations

Everything above is a person sharing with a person.
The set of things that can hold a key and appear in an audience is wider than that, and it is one algebra rather than a second grant surface: a subject is a person, a machine, a service running on machines, a group of subjects, or an organization holding recovery custody.
Nothing in this section changes anything until you declare it, and declaring a machine, a service, a group, a silo or an organization that nothing references generates the same policy, the same rules and the same files, byte for byte.

### A machine is a subject

```nix
flake.safix.machines.deck = {
  recipient = "age1..."; # ssh-to-age of the host's ed25519 key
  owner = "alice";
  tags = [ "laptop" ];
};

flake.safix.users.alice.sharedWith.deck.fleet-token = { };
```

```console
$ safix fix
$ sops secrets/safix/shared/alice,deck/secrets.yaml
```

Think of it as sharing with the host rather than with its owner: the machine's own service reads the value, and no person has to be logged in.
The recipient is the key the host already decrypts with — sops-nix's NixOS module defaults `sops.age.sshKeyPaths` to the host's ed25519 keys, and `ssh-to-age` of that key is what goes here — so declaring a machine mints no identity and adds no enrollment step.
The hardware-recipient refusal `safix adduser` applies to a person does not transfer: it exists because a card needs a PIN and a touch once per file while an activation decrypts non-interactively, and a host identity decrypts non-interactively by nature.

A machine's entries arrive in the profile that names it:

```nix
safix.machine = "deck"; # instead of safix.user
```

It holds nothing of its own — there is no `carries`, no `private` and no `sharedWith` on a machine — and it needs no hostname, because it is the host.

### A service is a subject whose recipients are its machines'

A service grant narrows what is declared and what is placed, and not what decrypts: the audience names the service, the landed file belongs to the service's unix user and group, and the host identity remains what opens it — so the machine is the trust boundary for everything running on it.

```nix
flake.safix.services.nginx = {
  machines = [ "deck" ];
  owner = "alice";
  user = "nginx";
  group = "nginx";
};

flake.safix.users.alice.sharedWith.nginx.web-token = { };
```

```console
$ sops secrets/safix/shared/%nginx,alice/secrets.yaml
```

The entry arrives on each machine the service runs on, keyed `nginx/web-token`, so the provisioner's own default path nests it under the service and two services granted one name never collide.
At system scope the file lands owned by `nginx:nginx`; a user-scope profile has no ownership axis, so a service declaring one is refused there rather than having the claim dropped, and a service declaring neither resolves with the scope's ordinary placement.

A machine joining the service is a re-wrap of the same file; a machine leaving is reported by `safix check` as the revocation it is, naming the machine, with rotation as the remedy.
safix records where a service runs because audiences need it and derives it from nothing — keeping the declared set and the running unit in step is yours.

### A group is a subject whose recipients are its members'

```nix
flake.safix.groups.oncall.members = [ "alice" "bob" "deck" ];

flake.safix.users.alice.sharedWith.oncall.pager-token = { };
```

```console
$ sops secrets/safix/shared/@oncall,alice/secrets.yaml
```

Think of it as a drawer with a name on it instead of a guest list.
Members may be people, machines, services, or other groups, and a cycle among them is refused at evaluation with the participants named.

The `@` is what makes membership changes cheap.
A guest-list directory moves when its list changes, which is a migration; a group-named directory does not, so adding a member is one `safix fix` that re-wraps one file, and removing one is a narrowing of the same file.
Ad-hoc `sharedWith.bob` keeps the guest-list form — the two answer different questions and both stay derived.

A member who leaves is reported by `safix check` as the revocation it is, with rotation as the remedy and `fix` as only the alignment afterwards.
They have read what the file holds; no re-wrap unreads it.

### A silo is non-overlap you can prove

```nix
flake.safix.silos.corp.groups = [ "staff" "contractors" ];
```

Think of it as two rooms with no door between them.
Evaluation refuses any file whose audience would reach subjects of two groups in one set, naming the file, the subjects and the declaration that forbids it — so a cross-silo file is one that cannot exist rather than one a policy hopes nobody wrote.
Sets rather than pairs is what keeps this linear, and a group named by two sets is itself refused.

It is deliberately not transitive over ownership.
One person may own machines in two silos — the operator administering both sides is the normal case — and what is refused is a single file readable from both.

### Ownership is a record a grant resolves through

```nix
flake.safix.users.alice.sharedWith."ownerOf.deck".wifi-psk = { };
```

Think of it as sharing with whoever holds the host, without having to know who that is.
The grant resolves through `flake.safix.machines.deck.owner`, and the audience directory names the reference rather than the person, so a change of owner re-wraps that one file toward the new owner instead of leaving the grant pointed at the old one.
The old owner's loss of future access is reported with the same disclosure as any narrowing.

The record confers nothing else.
An owner does not thereby read the machine's entries or manage its users, because a record that silently granted either would be escrowed custody arrived at by accident rather than declared — and `escrowedTo` below is the declared form.

### An organization is a principal that holds recovery custody

```nix
flake.safix.organizations.acme.custody.acme-escrow = {
  key = "age1...";
  note = "acme's escrow — held offline by the operator";
};

flake.safix.users.alice.escrowedTo = [ "acme" ];
```

Read the consent in alice's own view, because it is her declaration: acme's custody can open everything she holds, and withdrawing it revokes nothing already readable.
That is the trade-off `recoveryRecipients` carries as a warning, written down in the record of the person whose files it widens — and acme cannot establish it from its side, so nothing an organization declares widens anyone's audience.

The keys arrive beside her `recoveryRecipients` rather than inside it, which is what buys the property raw-key escrow never had.
acme rotates a custody key in its own declaration, one `safix fix` re-wraps every consenting person's files, and no person's declaration changes.
Withdrawal is a narrowing like any other: `safix check` reports it as the revocation it is, with rotation as the remedy.

An organization is also an owner and an audience element:

```nix
flake.safix.machines.rack.owner = "acme";
flake.safix.users.alice.sharedWith.acme.corp-token = { };
flake.safix.users.alice.sharedWith."ownerOf.rack".corp-handover = { };
```

```console
$ sops secrets/safix/shared/=acme,alice/secrets.yaml
```

`ownerOf` resolves through the record to acme's custody keys exactly as it resolves to a person's own, and `=` marks the organization the way `@` marks a group.
A group may not contain one — a principal is not a member, and an audience wanting acme's custody names acme.
An organization whose custody is empty is refused everywhere it is reached: by an `escrowedTo`, by a grant, by an ownership resolution.

## Generators: the value writes itself

```nix
flake.safix.users.alice.private.grafana-token = {
  generator.script = ''openssl rand -hex 32 > "$out/grafana-token"'';
  generator.runtimeInputs = [ "openssl" ];
};
```

```console
$ safix generate                              # mints everything declared but empty
$ safix generate --regenerate grafana-token   # rotation: new value, committed
```

A generator script writes files rather than printing a value, and the three directories it addresses are clan's:

| | what it holds |
|---|---|
| `$out/<name>` | one file per declared output; the script's working directory is the root above it |
| `$prompts/<name>` | one answered prompt each, present only when prompts are declared |
| `$in/<generator>/<name>` | a dependency's plaintext, keyed by the generator producing it |

This is the interface clan-core's own generators are written against, so a script written for either system runs under the other.
One difference is deliberate: only the dependencies a generator *declares* appear under `$in`, where clan places every file of the dependency generator — which would hand a script depending on a keypair's public half the private half as well.

Bytes are stored exactly as written.
`echo` leaves a trailing newline and `printf` does not, and nothing removes one, because a convention that took a byte off would corrupt every key whose last byte is a newline while looking like it had tidied one up.

Dependencies chain generators.
Think of a recipe that uses another recipe's output.

```nix
flake.safix.users.alice.private = {
  db-password.generator.script = ''openssl rand -base64 24 > "$out/db-password"'';
  db-password-hash.generator = {
    dependencies = [ "db-password" ];
    script = ''mkpasswd -sm bcrypt <"$in/db-password/db-password" > "$out/db-password-hash"'';
    runtimeInputs = [ "mkpasswd" ];
  };
};
```

Rotating `db-password` cascades: every generator downstream re-runs, in dependency order, after showing you the list and asking once.
A hash of a retired password would be a lie, which is why the cascade is not optional.
Cycles, self-references, and depending on another person's secret are all refused at evaluation — the last because your machine structurally cannot decrypt someone else's value.

A prompted generator asks instead of computing.

```nix
flake.safix.users.alice.private.upstream-api-key.generator = {
  prompts.token = {
    type = "hidden";
    description = "the API key issued by the provider's console";
  };
  script = ''cat "$prompts/token" > "$out/upstream-api-key"'';
};
```

A multi-output generator mints related values together, each with its own mode, and each half may be encrypted or public.

```nix
flake.safix.users.alice.private = {
  wg-private = {
    mode = "0400";
    generator = {
      runtimeInputs = [ "wireguard-tools" ];
      files.wg-public.secret = false;
      script = ''
        wg genkey > "$out/wg-private"
        wg pubkey < "$out/wg-private" > "$out/wg-public"
      '';
    };
  };

  wg-public.mode = "0444";
};
```

Each name a generator writes is a registry entry in its own right, carrying its own mode, path and key; `files` records which generator produces it and whether it is encrypted.
An entry named there may not carry a generator of its own and may not be named by a second generator, both refused at evaluation, because two producers for one value is a race whose winner is whichever ran last.
Both halves land in one commit, because a keypair split across two commits is an incoherent state.
A `validation` script receives the candidate value on stdin, with `$out_name` naming the output under judgement, and refuses the write on a non-zero exit — before anything is written.

### Public outputs, readable at evaluation

`files.<name>.secret = false` writes the value to the repository in the clear, gives it no creation rule, and makes it readable while nix evaluates:

```nix
peers = [ { publicKey = config.flake.safix.lib.publicValue "alice" "wg-public"; } ];
```

That is what a public key, a fingerprint or a derived identifier is for: a module reads it directly rather than through a deployment-time indirection.
`flake.safix.lib.outputPath` answers for every output and is a path, never a value.
Reaching for a value on a secret output fails with a sentence naming the entry and pointing at the path, rather than with nix's generic undefined-option message.

The plaintext store sits under a top-level `public/` prefix rather than inside `secrets/`:

```
public/safix/users/<user>/<name>/value
public/safix/shared/<audience>/<name>/value
```

A path named for secrets has to mean everything under it is encrypted, without qualification, because that is what every backup rule, sync exclusion and reviewer assumes about it.
Two checks hold the trees apart: `safix-public-no-rule` matches every generated creation rule against every public path, and the catch-all check carries the public shape among its probes — together with the definition-record shape described below, for the same reason.

The default is `secret = true`, not clan's `false`.
A mistyped field that leaves a value encrypted is recoverable by fixing the typo; one that publishes a value is not.

`runtimeInputs` names nixpkgs attributes as strings rather than holding packages, because the whole generator travels to the command as JSON and a derivation cannot cross that boundary.
Strings are unchecked by construction, so `safix-generator-tools` resolves each one against the package set at build time; otherwise `opensll` is discovered at a rotation, which is the worst moment to learn a declaration was never right.

### The envelope a fragment runs in

A generator's script and its validation fragments run inside a sandbox.
The staging root is the only writable path, the nix store is readable because that is where `runtimeInputs` resolve to, and there is no network.
A write outside `$out` fails, so a fragment can no longer put plaintext somewhere safix does not look and cannot shred.

The envelope is clan's rather than one of ours — bubblewrap on linux, `sandbox-exec` on darwin — which is the same reason the directory layout is clan's: a fragment written against the shared interface meets the same confinement under either system's default executor.
Two things follow that are worth knowing before you write a fragment.
`runtimeInputs` is now the whole of what a fragment can run, because the paths your `PATH` otherwise names do not exist inside the envelope.
And a validation fragment has no writable path at all, since the staging root has been shredded by the time a candidate is judged; the candidate still arrives on standard input.

One capability can be granted, on the generator itself:

```nix
flake.safix.users.alice.private.acme-account-key.generator = {
  network = true;
  runtimeInputs = [ "lego" ];
  script = ''…'';
};
```

`network = true` re-shares the network and nothing else — the filesystem confinement stays — and it governs the script and the validation fragments alike, because a validation that verifies a minted token against the API that issued it has the same need its script had.
It lives on the declaration rather than on the invocation so that *which generators may reach the network* is a question your tree answers at evaluation, with nothing to run and no flag history to reconstruct.
What travels over a granted connection is outside what safix shreds or observes, which is the reason the grant is a line a reviewer sees.

There is no `--no-sandbox`, and nothing spelled otherwise does the same thing.
Where no backend is available — a kernel that refuses the namespaces bubblewrap is made of, or a platform with neither backend — `safix generate` refuses before the first fragment and names what it looked for.
clan offers the flag because its generators can come from third-party modules and because it chose degradation over refusal; a safix generator is your own declaration, and safix prefers a named refusal to a silent weakening.

### Where the plaintext is

A generator's inputs and outputs are files, so they exist, and this is where.

The staging directory is created mode `0700` on a filesystem safix asks the kernel about with `statfs` rather than infers from its name, and it is overwritten and removed however the run ends — on return, on error, on panic, and from both signal handlers.
There is no fallback to `/tmp`: on a host whose `/tmp` is disk-backed a silent fallback would put plaintext in free blocks under a code path that looks like it succeeded.
Where no memory-backed filesystem is available the run refuses, and `--allow-disk-staging` is what accepts a disk-backed one.
`SAFIX_STAGING_DIR` names the mount to use instead of the conventional ones — it replaces them rather than being tried first, so a mount you named and safix rejects is a refusal rather than a silent fall back to somewhere else.

What that bounds, and what it does not, stated rather than implied.
Overwriting a page of a memory-backed filesystem does not reach a copy already written to swap.
A mode-`0700` directory is readable by every process running as you for the length of the run, where the pipe this replaced was readable by neither a third process nor a shell — that is a real reduction, and the two are not equivalent.
What the directory no longer has to carry alone is the fragment: a script that copies `$in/dep/name` elsewhere fails inside the envelope, so the containment does not rest on the fragment author getting it right.
Where it still rests on them is a granted connection, which no envelope can follow.
Write generators the way you would write any code that holds a credential.

### The definition a value was minted under

A generated value carries nothing saying which declaration produced it, so editing that declaration afterwards is invisible: the value in the file is a function of a generator that no longer exists, and reads exactly like one the current generator would produce.

`safix generate` therefore writes a digest of the declaration it ran, in the same commit as the value:

```
state/safix/definitions/<user>/<name>
state/safix/definitions/shared/<audience>/<name>
```

One plaintext line each — a format tag and a digest — over everything that decides what a mint produces: the script, its `runtimeInputs`, its `network` grant, its prompts, its dependencies, the outputs it writes with their secrecy, and the validation fragment.
No value and no derivative of a value is in it, which is what lets it be committed in the clear.

The grant is in there because it changes what a mint *may* do: the value in the file came from a fragment that could not reach the network, and a declaration that grants one describes a different mint even when the script is identical.
Covering it moved the tag from `v1` to `v2`, and a record carrying the older tag is read as no record at all — the same answer an absent one gets, for the same reason.

A third top-level tree, because neither existing one can hold it.
A path named for secrets has to mean everything under it is encrypted, without qualification; `public/` means declared public outputs a nix module reads at evaluation, and a bookkeeping file there would dilute that into "plaintext things safix wrote".
`state/` says what it is: recorded state about the tree, neither a secret nor an output.

`safix check` reads it back, and reports a value whose declaration has changed since it was minted — naming regeneration and reverting the edit as the two remedies and recommending neither, because the tree holds a value and a declaration that disagree and nothing but you knows which was meant.
A value with no record predates the record and is not a finding: no record, no claim.
A record in a format the running safix does not write is not a finding either, which is what keeps a change to the digest's canonical form from reporting every value in the tree as drifted.

## Editing a value: `safix edit`

```console
$ safix edit alice grafana-token
```

Opens `$VISUAL`, or `$EDITOR` when that is unset, on the entry's decrypted value.
Neither set is a refusal naming both: safix opens no editor of its own choosing, because dropping you into one you did not pick with a secret in the buffer produces either an accidental write or an accidental abandonment, and nothing can tell those apart.

The command is split on whitespace and run directly rather than through a shell, so `EDITOR="code --wait"` works.
The staged file's path is an argument; the value is not.

A non-zero exit writes nothing, an unchanged buffer commits nothing, an emptied buffer takes the same refusal an empty value takes anywhere else, and a changed non-empty buffer goes through the same write path `safix set` uses.
An entry that holds no value yet opens on an empty buffer, so this is an authoring verb as well as an amending one.

The buffer lives in the same private staging directory generators use, and whatever the editor leaves beside it — swap files, backups, undo history — is removed with the directory.
An editor configured to write undo history to a location of its own has put plaintext where safix does not look; that is the limit of the containment, and it is stated rather than left to be discovered.

## Values without declarations: the runtime extract

Not every secret needs to land on disk.
A credential you invoke interactively can stay encrypted and be decrypted on demand by whatever runs it.

```nix
# in your own module, not safix's
settings.credsCommand = ''sops -d --extract '["dns-creds"]' secrets/safix/users/alice/ops-tooling.yaml'';
```

Think of it as reading a note without photocopying it.
Use this shape for credentials only a person invokes; use a declared secret for anything a service reads from a path.

The file such a value lives in is not one safix placed, so it is not in the set `safix fix` re-wraps.
It still rides the audience's rule, because every rule covers one directory level rather than one literal filename — but a change of audience would reach every file safix placed and leave this one behind, encrypted to whoever it was encrypted to when it was written.
Naming it in `flake.safix.extraGovernedFiles` puts it in the set `fix` re-wraps and the checks judge.

## The daily commands

```console
$ safix list       # everything a person holds: origin, file, generator, shared markers
$ safix check      # report drift, change nothing; each finding prints its remedy
$ safix fix        # regenerate .sops.yaml and re-wrap files to match declarations
$ safix set NAME   # write one value (hidden prompt, confirmed, committed)
$ printf '%s' "$TOKEN" | safix set NAME   # the same write, scripted
$ safix get NAME   # read one value to stdout
$ safix generate   # mint whatever has a recipe
$ safix import     # pull declared clan vars into safix
$ safix export     # push declared safix values into clan
$ safix audit      # report which declared mappings' two sides disagree
$ safix sync       # converge declared entries with your password database
$ safix keygen     # run by a person on their machine: mint their identity
$ safix adduser    # run by the operator: scaffold a person
$ safix enroll     # a hardware key, from a blank card to a proven recovery identity
```

Think of `check` and `fix` as `git status` and `git add` for secret policy.
The nix declarations are intent, the encrypted files are reality, `check` diffs the two, and `fix` reconciles what is reconcilable and names what needs a human.
Its fifth finding class is the generator one: a value minted under a declaration that has changed since — see "The definition a value was minted under" above.

`safix set` reads the value from standard input when standard input is not a terminal, which is what makes the second form above work.
It replaces nothing: a terminal still gets the hidden prompt and the confirmation, unchanged.
What the piped form drops is the confirmation, and only where there is nobody to confirm — a piped value has no typist for the second prompt to catch out — while the empty-value refusal and the store-exactly-these-bytes rule both hold.

`upload` does not exist here, and `safix --help` records why: activation already delivers what an upload would.

`import` and `export` are not a plaintext dump and restore.
They move one declared mapping at a time across the clan boundary — see "The bridge to clan" below — and nothing here writes a plaintext tree, because such a tree outlives the migration that justified it.

## Onboarding a person, end to end

The person's part comes first, and the operator never performs it.

```console
# on their machine, as them
$ safix keygen
# prints: age1abc... — the public half, which they hand to the operator
```

`keygen` appends to their own identity file and never prints the private half.
Minting someone else's identity means holding their private key, which is the opposite of the custody this package rests on, so doing it takes an explicit `--for-someone-else`.

The operator's part is a scaffold and nothing more.

```console
# on the operator's machine
$ safix adduser carol age1abc...
# writes safix/users/carol.nix, regenerates .sops.yaml, commits exactly those two
```

`adduser` mints nothing: no age key, no password material, no secret value.
It gives the person nothing to hold either — the scaffold declares no secret, so no audience is computed for them, and the regenerated policy carries their key as an anchor with no creation rule yet.
Their first secret is a name under `private` or `carries`, then `safix fix` to write the rule, then `safix set`.

Everything beyond a custody record is a property of one consumer's module tree — attaching an account on a host, allocating an identifier, editing a host's imports — so `adduser` passes the name and the recipient to `flake.safix.onboardingHook` and makes no assumption about what happens next.
`--host` is passed through to the hook and is refused while no hook is configured, because there is nothing for a hostname to reach.
Running without a hook is a supported configuration: it succeeds, having done less, and says so.

From then on the person works alone.

```console
# on their machine, no operator involved
$ safix set my-vpn-token
```

The custody story in one line: the operator controls who exists and what is on the shelf, each person controls what is in their drawer, and drawers you cannot open you cannot read.

Whether that independence is real is decided by one field, and the disclosure lives on it.
`recoveryRecipients` is where a person lists further identities of their own — an offline master key, a hardware token — and every file whose audience includes them is encrypted to those as well.
Leaving it empty keeps their custody independent and has a cost no later edit undoes: with only their activation key, losing it makes their files unopenable by every party including the operator, because adding a recipient to an existing file requires decrypting it first.
Listing an operator-held identity there instead buys recoverability at the price of that operator reading everything the person holds.
The mitigation that keeps independence is a second recipient the person themselves holds.
Where that operator is an organization, `escrowedTo` is how the same trade-off is declared rather than assembled out of raw keys — the same breadth, named, in the person's own record, and rotated in one place.

## Enrolling a hardware key: `safix enroll`

`recoveryRecipients` is where a hardware token belongs, and getting one in there used to be seven manual steps that proved nothing at the end.
It is now one verb.

```console
$ safix enroll
# 12345678 is factory-fresh. Generating a PIN and a distinct PUK...
# 👆 Please touch the YubiKey
# 12345678 is enrolled for alice.
```

A touch is the only thing you do.
Everything else happens in one run, in this order: the card is selected; its PIV access is provisioned when the card is factory-fresh, with a safix-generated PIN, a distinct safix-generated PUK and a random management key put on the card under the PIN; an age identity is generated in the first empty retired slot, driven under a pseudo-terminal that supplies the PIN; the identity block is appended to the same file `safix keygen` appends to; the card's recipient is added to the person's `recoveryRecipients`; `.sops.yaml` is regenerated, every governed file re-wrapped, and the three committed together; the recipient is registered with clan through clan's own command when a clan is declared, and `flake.safix.enrollHook` receives the person, the serial and the recipient; the generated PIN and PUK become that person's own safix secret, named for the serial.

Then the step the hand ceremony never had.
The card alone opens a governed file in the person's audience, with an identity source holding only the card's stub, exercising the PIN and the touch.
An enrollment whose proof has not passed reports itself incomplete and exits non-zero — nothing is undone, because the identity, the recipient and the re-wrap are additive and correct on their own.

Everything is additive, on every path.
A recipient is appended, an identity block is appended, a name is declared; nothing is removed and nothing is replaced.
A backup key is the same verb run again: each card gets its own identity and its own recipient, and neither run knows about the other.
A re-wrap that dropped a recipient a file had before the run is refused rather than committed.

Three things are refused, and each refusal names why.
No OTP slot is written under any flag — a programmed challenge-response slot is what opens a password database, the database has no record of the secret it was built with, and writing that slot ends it permanently.
`--touch-policy never` is refused, because the touch is the property a card is for.
And a run with no terminal is refused before the card is touched, because somebody has to touch it and somebody has to be told when.

No credential safix generates reaches an argument vector or an environment variable, on any path.
`ykman`'s credential options are omitted so that it prompts, and the prompts are answered on a pseudo-terminal — an argument vector is readable by every process on the machine, and for a PIN that is the whole difference between a credential and a published one.
The two values that do travel as options are the serial and the factory defaults every card ships with.

The management key is stored nowhere: PIN possession is management possession, so a stored copy would be a credential with no reader.
The PIN and PUK land in the person's own custody by default, with an honest caveat — a PIN readable by the software identity adds protection only once that identity is retired or absent, and `--no-store-pin` turns it off.
`--mirror-to-store` writes them to the password store as well: through the session's secret service when it answers, with no prompt at all, and through `keepassxc-cli` with one password prompt when it does not.

The primary `recipient` stays software-only.
Activation decrypts with nobody present, so a card belongs in `recoveryRecipients` and `safix adduser` refuses one for the other field.

## Wiring it to your own user registry

safix's `flake.safix.users` is its own record and carries only custody.
It is deliberately not your user registry and never reads one.
If you already have users declared somewhere, write a projection from yours into safix's; the two are different objects that happen to share a name.

```nix
{ config, lib, ... }:
{
  flake.safix.users = lib.mapAttrs (_name: person: {
    recipient = person.meta.ageRecipient;
    recoveryRecipients = lib.mapAttrs (_anchor: key: { inherit key; }) person.meta.ageRecoveryKeys;
  }) config.flake.users;
}
```

That projection lives in your tree and is sufficient on its own, because safix reads no option path outside `flake.safix`.
safix's own modules are held to that by the `safix-namespace` check: one read of a consumer's registry, a fleet-wide default or a hostname list would turn every adapter into an integration against a shape safix never documented.

Secrets are then declared against the projected names, and the two records stay independent — a person can exist in your registry and hold nothing here, or hold secrets here without your registry knowing.

`flake.safix.machines` takes a projection on the same terms, from a host inventory rather than a user registry: safix has no host record of its own to reconcile with yours, and a machine declared by a `mapAttrs` over your inventory is indistinguishable to the resolver from one written by hand.
`flake.safix.services` is the same again, from whatever record already says which units run where.

## Establishing secrets in a profile

Custody is declared once, at flake level, where every user is visible at the same time.
Arrival is declared per profile, in the module system that profile is written in, through a `safix.*` namespace that sits beside sops-nix's `sops.*` and can select but never declare.

That split is forced rather than stylistic.
An audience is a function of every user's declarations at once — one person's `sharedWith` widens the file another person reads — and `.sops.yaml` is a single repository-global file the sops CLI reads off disk.
A machine's module system sees one machine, so it can compute neither.

### The option surface

Both modules declare the same options, and none of them can add a secret, a recipient, a grant, or an audience.

| option | default | what it is |
|---|---|---|
| `safix.flake` | `null` | your own flake — `inputs.self` — from which `safix.lib` is read |
| `safix.lib` | from `safix.flake` | the resolver projection, settable directly if your flake reaches the profile some other way |
| `safix.user` | `config.home.username`; none at system scope; `null` where `safix.machine` is set | which `flake.safix.users` entry this profile serves |
| `safix.machine` | `null` | which `flake.safix.machines` entry this profile serves instead of a person; its services' entries arrive with it |
| `safix.hostname` | `osConfig.networking.hostName`; `config.networking.hostName` at system scope | which host to resolve on, since `perHost` and `perTag` select by it; not needed for a machine |
| `safix.tags` | the declared tags of `safix.machine`, else `[ ]` | the tags this host carries, against which `perTag` selects |
| `safix.identity.keyFile` | `null`; at user scope one of these two is required | an age key file this machine decrypts with |
| `safix.identity.sshKeyPaths` | `[ ]`; at user scope one of these two is required | ssh private keys this machine decrypts with |
| `safix.enable` | whether anything resolved | the gate the whole module sits behind |
| `safix.identityPreflight` | `true` | user scope only: install the activation guard below |
| `safix.secrets` | read-only | what resolved, in the shape `sops.secrets` takes |

`safix.flake` is the one thing a module cannot derive.
A profile receives `config`, `lib`, `pkgs` and whatever its evaluator put in `extraSpecialArgs` or `specialArgs`; requiring a particular name there would make your evaluation seam part of safix's interface, which is the same assumption safix refuses to make about your user registry.
So it is named once, and pointing it at something that carries no `safix.lib` fails with a message naming the option.

Standalone home-manager cannot derive a hostname — `osConfig` exists only where home-manager is evaluated as a NixOS module — so `safix.hostname` is the fourth line there, and the identity below is the fifth.

Three states follow from what is set, and each is refused or ignored deliberately.
A profile bound to declarations but missing a person or a host refuses at evaluation, naming the option that is unset, and defines nothing in the meantime.
A profile that names a person or a host and is bound to nothing — `safix.flake` omitted and `safix.lib` never set — refuses as well, naming `safix.flake`.
That state is refused rather than tolerated because a null `safix.lib` empties the resolved set and makes every other refusal here vacuously true, so the profile would otherwise build, establish nothing, and report nothing.
A profile that imports the module and sets nothing at all is a no-op, and so is one whose person resolves nothing on that host: no secrets, no identity, no activation entry, no unit.
The last two are told apart by whether a definition for `safix.user` or `safix.hostname` exists, never by its value — at user scope that option defaults to the profile's own username, so every profile has a value for it.
`safix-consumption-refusals` holds both directions, which is what stops the refusal from swallowing the no-op.

Naming a person no `flake.safix.users` entry declares refuses as well, listing the declared users.
That refusal sits in the resolver rather than in either module, so a direct `safix.lib` call and the `safix` command reach the same sentence the profile does — and it is likelier than it looks, since `safix.user` defaults to the profile's own username and an account name need not match its declaration key.

### Which of the two forms to import

Each module ships twice.
`homeModules.default` and `nixosModules.default` import sops-nix along with safix, for a tree that has not got it.
`homeModules.safix` and `nixosModules.safix` declare the same namespace and import nothing, for a tree that already imports sops-nix at a revision of its own.

Import the second if you already import sops-nix anywhere in that profile, because importing two distinct copies of one option-declaring module is not a merge and not a warning:

```
error: The option `sops.secrets' in `/nix/store/…-sops-nix-b/modules/home-manager/sops.nix'
       is already declared in `/nix/store/…-sops-nix-a/modules/home-manager/sops.nix'.
```

Which option the error names is a property of the evaluation rather than of the defect, so read the block as illustrative.
A duplicate declaration is detected when an option is merged, not when the module list is built, so it is reported against whichever of the colliding declarations the configuration forces first — `sops.secrets` for a home-manager profile, `sops.gnupg.home` for a NixOS configuration.

`safix-module-collision` holds that fact, against sops-nix's real module rather than a synthetic one, which is why the choice is offered as two imports rather than as an option: `imports` cannot depend on configuration, so no flag could repair it after the fact.
A consumer whose `sops-nix` input `follows` safix's resolves to one store path and is safe either way.

### One declaration, both scopes

The mode, the path and the key are identical in both scopes, and nothing in a declaration names one.
The system scope additionally carries `owner` and `group`; the user scope refuses an entry that sets them rather than dropping it, because the user-scope provisioner has no ownership axis and a dropped ownership field reads afterwards as an ownership claim that was honoured.

Two entries resolving onto one path are refused for either scope, since whichever declaration activates second unlinks the first's output.

What is scope-specific is not the declaration but the configuration an entry's `path` is a function of: a `path` written as `cfg: "${cfg.home.homeDirectory}/…"` is a home-manager expression and will not materialize into a system configuration.

The resolver's refusals surface as safix's own evaluation errors, listing every violation at once, rather than as the first of them raised from inside sops-nix's manifest generation.

### The identity, and the guard

`safix.identity.keyFile` defaults to null, and that default is not a preference.
`sops-install-secrets` treats a set-but-unreadable key file as fatal, and skips a missing ssh key path with a line to stderr, so a non-null default would abort activation on every machine that happens to lack the path.
Both identity options are defined onto `sops.age.*` at normal priority, so a `mkDefault` elsewhere in your tree loses to safix and a plain definition conflicts loudly — the alternative would let a base module's XDG default silently replace the null and re-arm the abort.
`sshKeyPaths` is defined only when you name it, so the system scope keeps sops-nix's own default of the host's ed25519 keys.

At user scope there is no such default to keep, and naming one of the two is therefore not optional.
A profile whose declarations resolve and which names neither refuses at evaluation, with a message naming both options and stating why neither can be defaulted for a person.
It refuses before sops-nix's own key-source assertion is reached, which is the whole reason it exists: that assertion names its own five options and neither of safix's.
`safix-consumption-refusals` holds the refusal, and holds it off a profile evaluated without home-manager's assertion wrapper — a wrapped profile refuses either way, and reports that something refused rather than which module did.

At user scope, safix installs `home.activation.safixIdentityPreflight`.
It reads the configured identity, checks each path for presence and readability, and refuses the switch when none is usable; it decrypts nothing.
It sorts `entryBefore [ "checkLinkTargets" ]`, which is what makes the refusal atomic: no home file linked, no user package installed, no user unit restarted, no secret written.

That ordering is the whole of the guarantee, and it is held by `safix-consumption-ordering`, which topologically sorts a real profile's activation DAG.
The same check holds the other half of the pair: sops-nix's own entry sorts *after* `checkLinkTargets`, which is why the guard exists.
sops-nix registers it as a bare string, so home-manager treats it as `entryAnywhere` and it lands after `linkGeneration` and `reloadSystemd`; pinning it earlier is not available as a fix, because the unit it restarts is materialized by `linkGeneration` and made visible by `reloadSystemd`, so an early restart aborts on the first switch that introduces sops and thereafter restarts the previous generation's unit with no signal.

The guard is narrower than it sounds, twice over, and its own failure message says so.
Where home activation runs as a NixOS host's `home-manager-<user>.service`, systemd starts that unit after system activation has already switched the system generation, so a system switch is not undone by the refusal — only that user's home generation is held back.
And presence and readability are all that were checked: a key that exists and is readable but is not a recipient of these files still fails later, in `sops-install-secrets`.
That sentence is held by `safix-identity-recipiency`, against fixture ciphertext rather than against an activation, which is the one claim on this path an evaluation cannot make.
The identity it drives is shown to open a document it *is* a recipient of before it is shown not to open one it is not, so what the refusal reports is recipiency and not a key file that was simply unusable.

The system scope installs no such guard, and that asymmetry is deliberate.
No atomic refusal point at NixOS activation has been demonstrated, and safix does not document a guarantee that no code enforces.
The failure is also rarer there, because sops-nix's system-scope default identity is the host key.

## The bridge to clan

If your fleet also runs [clan](https://clan.lol), values can move between clan's vars and safix's entries in either direction.
The relationship is declared rather than passed as arguments, because a bridge is a standing relationship and a declaration is diffable, repeatable, checkable and enumerable where a remembered command line is none of those.

```nix
{
  flake.safix.bridge.clanFlake = ./.;

  flake.safix.bridge.mappings.ntfy-token = {
    direction = "clan-to-safix";
    clan = { machine = "meridian"; generator = "ntfy"; file = "token"; };
    safix = { user = "alice"; name = "ntfy-token"; };
  };
}
```

Then `safix import` moves every `clan-to-safix` mapping and `safix export` every `safix-to-clan` one; naming a mapping narrows the run to it.

Direction is written as its endpoints rather than as `import` or `export`, and that is not pedantry.
`clan vars export` moves values *out of* clan; `safix export` moves them *into* clan.
Both words are correct relative to the tool that says them, and a declaration is read by someone with no tool in hand to be relative to.
The verbs stay `import` and `export` because they sit on safix's own command line, where safix is the frame every other verb already assumes.

**clan stays the authority on its own store.**
Every read is `clan vars get` and every write is `clan vars set`, run as subprocesses with the value on a pipe.
safix reads, writes, encrypts, decrypts and parses none of clan's stored files, in either direction, so the bridge works over `sops`, `age`, `password-store` and whatever clan adds, with no code here.
The cost is that a consumer without clan-cli cannot import either — which is arguably correct, since a consumer with no clan has nothing to import from.

**Both verbs converge.**
Each reads both sides and compares before writing either, so a mapping whose two sides agree is not written and not committed and a second run changes nothing.
On the export side that comparison is load-bearing rather than an optimisation: clan's write is unconditional and a re-encrypting backend produces fresh ciphertext for an unchanged value, so without it every run would commit in the clan repository for every mapping.

An imported value goes through the same path a hand-typed one takes, so it acquires the recipient-drift refusal, the staged write and the rename, and lands as its own commit naming the mapping and the direction and never the value.

Half of every mapping lives in another flake, so evaluation refuses only what is local to you: an unresolvable safix side, an import target a generator also produces, two mappings writing one target, one pair of endpoints declared in both directions, and mappings with no `clanFlake` to reach.
It claims nothing about the clan side.
A clan side that does not resolve is refused when a transfer reaches it, in clan's own words, naming the machine, the generator and the file.

Two refusals belong to export alone.
A source entry that holds no value is refused rather than exported as nothing — a question evaluation cannot answer, because an entry declares where a value lives rather than that one is there.
And a mapping whose clan-side generator clan already considers outdated is refused, because clan records a validation per generator and its next routine `clan vars generate` would replace whatever was exported without saying so.
There is no option that exports anyway: safix has nowhere to record that a var is externally supplied, so the flag would turn a refusal into a silent loss.
The refusal names both remedies — bring clan's side back into agreement, or declare the mapping `clan-to-safix`, which is the right shape when clan's generator is the producer.

That second refusal reaches further than it may look, and the reach is correct.
clan records a validation for a generator only when the generator declares `validation`, and it calls one whose declared validation has nothing recorded beside it outdated — so a generator that declares a validation and has never run is refused at its *first* export, because it has not run and will, and the run would replace whatever was exported.
The generator to export into is therefore one that declares no `validation`: a var clan holds a place for and nothing else.

**`safix audit` is the report over the same declarations.**
It compares both sides of every declared mapping, or the one named in either direction, and changes nothing on either side of the boundary.
A mapping agrees when both sides hold the same bytes, and also when neither side holds a value yet, which is a bridge nobody has bootstrapped rather than a disagreement.
It is a finding when the two sides hold different values, when one side holds a value the other does not, or when the comparison could not be made — and each finding names the mapping, its two endpoints and the command that converges it, and never a value.

It is a verb of its own rather than more rows in `check`, and the reason is what `check` is.
`check` decrypts nothing, which is what lets one machine judge files belonging to people whose keys it does not have, and it needs no clan.
Comparing a mapping's two sides needs both of those: it decrypts the safix side, and it runs clan's own command once per mapping.
So the verb that needs them carries them, `check` keeps both of its properties, and a mapping you cannot decrypt is reported as one that could not be judged rather than quietly left out — a report that dropped those would be a report about who ran it.

## The mirror in your password database

Some secrets are read by tools and some are also read by a person — typed into a web login, a phone, another machine's prompt.
`safix sync` ends the drift between the two, one declared mapping at a time.

```nix
{
  flake.safix.keepassxc = {
    database = "/home/alice/.keys/master.kdbx";
    group = "safix";

    mappings.grafana = {
      mode = "safix-to-keepassxc";
      safix = { user = "alice"; name = "grafana-password"; };
      kdbx = { path = "alice/grafana"; username = "alice@example.com"; };
    };
  };
}
```

Then `safix sync` converges every mapping and `safix sync grafana` narrows the run to one.

**The mode is declared, not passed.**
`safix-to-keepassxc` makes the database follow safix and reports the database-side edit it overwrote.
`keepassxc-to-safix` makes safix follow the database, through the same path a hand-set value takes — the same recipient-drift refusal, the same staged write, a commit naming the mapping and never the value.
`two-way` converges toward whichever side changed since the last agreement.
`backup` writes safix's value where the database has none and never overwrites one that differs.
The vocabulary is the one this fleet's file-sync declaration already uses for pairs, and the mode lives in the declaration because a remembered flag on a verb is exactly the drifting operational knowledge a declaration exists to end.

**Nothing is ever deleted, in any mode.**
Remove a mapping and its last database value stays until a person removes it; the report says the entry is there and that nothing declares it.
Deletion propagation is the one part of the sync model deliberately not taken: an accidental deletion of a secret is not a state a sync should be able to reach.

**A conflict is a finding, never a guess.**
A two-way mapping remembers the last state both sides agreed on; when both have moved since, nothing is written and the report names the two one-way modes that each resolve it.
Last-writer-wins over secrets rewards whichever clock lied best.

That memory is a digest of the agreed value, and it lives in a companion entry beside the mapped one, inside the encrypted database — never in the repository.
That is a security decision rather than a filing one: a committed digest of a secret confirms a guessed value offline, for anyone who has the tree.
The companion's name is the entry's plus `.safix-sync-state`, and evaluation refuses a mapping that tries to declare one.
Deleting it is safe and takes the mapping back to bootstrap semantics: write where one side is empty, report everything else.

**One database, one prompt, and a bounded cost.**
`database` is a string rather than a nix path, because a path is copied into the world-readable store on every evaluation and this file is 292 MB.
The password is asked for once per run and travels standard input; so does every value, and no value reaches an argument vector or an environment variable on any leg.
Without a terminal to ask on, the run refuses before reading anything.

A kdbx save rewrites the whole file, so both sides of every mapping are read and compared first, every database write of a run is issued consecutively, and a run over mappings that agree writes nothing anywhere.
A value carrying a newline is refused rather than written: the store's own command reads an entry's password as one line, and nothing here trims the byte for you — `printf` where `echo` minted it.

The session's secret service is not a second way in, and the reason is worth stating: the collection KeePassXC publishes is its own *exposed group*, so an entry found or created through it lives where your exposure setting says rather than where the declaration says.
`safix enroll --mirror-to-store` does use it, and correctly — that entry is safix's own and is addressed by an attribute, so the exposed group is the right home for it.

`sync` manages no keyring: no database is created, no database key is changed, and no hardware slot is touched under any flag.

## The checks safix hands you

```nix
{ config, ... }:
{
  perSystem =
    { pkgs, ... }:
    {
      checks = config.flake.safix.lib.mkChecks pkgs {
        committedPolicy = ./.sops.yaml;
        materializations = {
          alice-workstation = /* the attrset your profile materializes */;
        };
      };
    };
}
```

Called with no arguments it returns eight checks over your declarations: the custody refusals, the generator runtime tools, the shape of every generated rule, the absence of a catch-all — whose probes carry the public store's shape and the definition record's, so a rule reaching either fails — the non-interaction between the rules and the public store, the audience separator, and the two relationship families, which are silent until you hand them your own records: `bridge = config.flake.safix.lib.bridge` and `keepassxc = config.flake.safix.lib.keepassxc`.
`committedPolicy` adds the drift check, which fails while the committed `.sops.yaml` and the generated one differ and whose failure names `safix fix`.
`materializations` adds the path-collision check, which forces the materializations you hand it so that the refusal reaches the hosts nobody has built this week.

Every one of them is instantiated in this repository over a fixture fleet, and every one has a perturbation that turns it red.

## The opinions safix will not bend

Placement is derived from the audience and never authored.
An entry carrying a `sopsFile` of its own is refused, because such a file's recipients are outside the computation that produced the policy, and the value would then be encrypted to an audience nothing checked.

There is no catch-all rule and the generator emits none.
An unmatched path must fail closed with sops' own "no matching creation rules found" rather than silently acquiring a default recipient set.

Every rule is start-anchored, extension-terminated, and one directory level.
Without the anchor a rule also matches its own suffix under any prefix; without the extension it reaches encrypted material safix did not place, and a `sops updatekeys` sweep would then rewrite that material's recipients — unrecoverable without the original identities.

The recipient policy is generated and committed, never hand-edited.
The sops CLI reads the committed file off disk, so that is the version deciding what a new file is encrypted to, and a check holds it to the declarations.

Narrowing an audience is not revocation.
It stops future encryptions reaching someone and takes nothing back, so the code says so at each place where the choice is made rather than once in a document.

safix reads nothing outside its own namespace.
That is what makes an adapter a projection you write rather than an integration you maintain.

Key generation belongs to the person who will hold the key.
`adduser` mints nothing, and minting someone else's identity takes an explicit flag naming what it is.

A recipient that needs a physical interaction to decrypt is refused for the primary `recipient` field.
Activation decrypts non-interactively and a card needs a touch, so such an identity belongs in `recoveryRecipients`, where it is additive.

## Where the pieces live

| concern | file |
|---|---|
| the records a consumer declares | `modules/flake/safix/options.nix` |
| the option types and their reference documentation | `modules/flake/safix/types.nix` |
| the resolution algebra | `modules/flake/safix/resolve.nix` |
| the clan bridge's mappings and their refusals | `modules/flake/safix/bridge.nix` |
| the password-database mirror's mappings and their refusals | `modules/flake/safix/keepassxc.nix` |
| the recipient policy renderer | `modules/flake/safix/policy.nix` |
| the checks a consumer instantiates | `modules/flake/safix/checks.nix` |
| the flake module a consumer imports | `modules/flake/safix/default.nix` |
| the consumption options both scopes share | `modules/consume/common.nix` |
| the home-manager module and its activation guard | `modules/consume/home.nix` |
| the NixOS module | `modules/consume/nixos.nix` |
| the runtime as a library | `crates/safix-core/` |
| the command, exposed as `packages.safix` | `crates/safix/` |
| the integration suite the command is held to | `crates/safix/tests/` |
| recipient policy, in a consumer's tree | `.sops.yaml` — written by `safix fix`, never by hand |
| encrypted values, in a consumer's tree | `secrets/safix/users/<u>/` and `secrets/safix/shared/<audience>/` |
| public outputs, in a consumer's tree | `public/safix/` — plaintext, no creation rule, readable at evaluation |
| the definition each generated value was minted under | `state/safix/definitions/` — plaintext, one digest per value |

The option reference lives on the types themselves; this document is the narrative companion.

## Status

The evaluation half, the command, the exported checks, the materializations and the two consumption modules are here and green under `nix flake check`.

The runtime is rust, and `packages.safix` is that binary.
`crates/` holds a cargo workspace — `safix-core`, the runtime as an embeddable library, and `safix`, a thin command over it — built, unit-tested, linted, formatted, licence-checked, advisory-scanned and integration-tested under `nix flake check`.
It implements all twelve subcommands: the read paths `list`, `get`, `check` and `audit`, the write paths `set`, `edit` and `fix`, the generator graph behind `generate`, the bridge pair `import` and `export`, and the two that touch custody itself, `keygen` and `adduser`.
The nix half was never in scope and did not move; what was replaced is a shell runtime and two python helpers, all three now deleted.

The port ran behind a differential harness comparing every subcommand against the shell runtime; the five places the two differ are recorded as decisions in the changelog's "Known differences".
With the port complete the harness was deleted with the runtime it compared against — 6205 lines — and its claims rewritten as `crates/safix/tests/`, which drives the built binary against throwaway repositories and asserts against literals.
`safix-syscall-proof` (linux-only) observes every plaintext `write` a `set` and a `generate` make and holds each to a pipe; `safix-channel-drills` damages the runtime once per channel and fails unless each damage is caught by the channel that exists to catch it.
`safix-bridge-real-clan` (linux-only) drives the real clan command over a clan it builds inside the check — one machine, three `age`-backed generators, an identity minted per run — because every other bridge check drives a stub, and a stub goes on answering safix's arguments after clan has changed what they mean.
`safix-generate-envelope` (linux-only) drives fragments that try to leave the sandbox and holds each attempt to failing, each one drilled against an unconfined run of the same fragment so that an absent file is the envelope's doing rather than the fragment's.
The proposal, the decisions and the staging are in `openspec/changes/rewrite-runtime-in-rust/` for the port and `openspec/changes/rust-only-runtime/` for the retirement.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
