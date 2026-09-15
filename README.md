# safix

safix is built for its operator's own fleet; that use case, not general adoption, decides its opinions.

safix is a custody-first secrets manager for nix.
Secrets are declared as attribute set options merged by the nix module system, the encrypted file each secret lives in is derived from the audience that can read it rather than authored by hand, and the `.sops.yaml` recipient policy is generated from those same declarations.
It is tied to no framework: it serves NixOS and home-manager alike through consumption modules of its own, and installs what they resolve with its own `safix install`.

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
The fourth is the user scope's alone: at system scope safix derives an age identity from the ed25519 keys of `config.services.openssh.hostKeys` that lie outside its own store, and a person is not a host, so there is no per-person equivalent to derive — a profile that resolves secrets and names no identity refuses at evaluation rather than establishing them.
See [Establishing secrets in a profile](#establishing-secrets-in-a-profile) for the rest of the surface.

Declarations merge, so the flake block above can live in its own file imported alongside a hundred others; safix reads no path, no filename and no directory structure to find them.

Flake-parts is one way to reach that merge, not the only one: a consumer with no flake-parts import, or no flake at all, reaches the identical projection through `flake.lib.mkVault` — see [Without flake-parts, or without a flake](#without-flake-parts-or-without-a-flake).

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
Every `sops <path>` command in this document uses the default `flake.safix.storage.encrypted` spelling; if you have renamed that root, the path is yours.
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

## Subjects: machines, services, groups, silos, ownership, organizations, delegation

Everything above is a person sharing with a person.
The set of things that can hold a key and appear in an audience is wider than that, and it is one algebra rather than a second grant surface: a subject is a person, a machine, a service running on machines, a group of subjects, or an organization holding recovery custody.
Nothing in this section changes anything until you declare it, and declaring a machine, a service, a group, a silo or an organization that nothing references generates the same policy, the same rules and the same files, byte for byte.
Delegation, at the end of the section, is inert in a stronger sense: it changes who may run a scaffolding verb and never what any file holds, so a fleet that declares one derives the byte-identical tree.

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
The recipient is the age form of the host identity the system scope already decrypts with — safix derives which key that is from the ed25519 entries of `services.openssh.hostKeys`, excluding only keys inside safix's own secret store, and `ssh-to-age` of that key is what goes here — so declaring a machine mints no identity and adds no enrollment step.
The hardware-recipient refusal `safix adduser` applies to a person does not transfer: it exists because a card needs a PIN and a touch once per file while an activation decrypts non-interactively, and a host identity decrypts non-interactively by nature.

A machine's entries arrive in the profile that names it:

```nix
safix.machine = "deck"; # instead of safix.user
```

It holds nothing of its own — there is no `carries`, no `private` and no `sharedWith` on a machine — and it needs no hostname, because it is the host.

Declaring `recipient` here is necessary but not sufficient: it tells safix which age form to wrap every audience naming the machine to, and the machine still needs the matching private half sitting on its own disk before its first activation can decrypt any of it — `safix upload` (see "Seeding a machine's host identity") is the step that makes the declaration true on disk.

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

### Delegation is a record with two consenting sides, and it is not authorization

Read this one first, because it is what the rest of the section is bounded by.
Delegation binds the cooperative path and is not authorization: the tree is the authorization, anyone who can commit can edit these declarations by hand, evaluation refuses structure rather than people, and no delegation record places a key in any audience.
What it buys is that a scaffold and the identity it is attributed to cannot disagree.

```nix
flake.safix.organizations.acme.managers = [ "alice" ];
flake.safix.users.bob.managedBy = "acme";
```

Think of it as acme saying who scaffolds on its behalf and bob saying he is one of the people they scaffold for.
Both halves are declarations and neither confers a read: a manager scaffolds, never mints, and never reads by virtue of managing — bob's audience is exactly what it was, and the generated policy is byte-identical to what it was before either line existed.
The consent is bob's own, for the reason `escrowedTo` is: nothing acme declares can subject anyone to it, so a review of bob's record shows everything that binds bob.

Where both halves are declared, `safix enroll` and `safix group` accept acme's managers and refuse anybody else:

```console
$ safix enroll bob            # run by alice
safix: alice is a declared manager of acme, which flake.safix.users.bob.managedBy
       names, so this scaffold is recorded as acme's.

$ safix enroll bob            # run by mallory
safix: flake.safix.users.bob is delegated to flake.safix.organizations.acme by
       flake.safix.users.bob.managedBy.
       mallory is not among the managers named there, so nothing about it was
       edited.
```

The acting identity is the one the commit will carry — `user.name` and `user.email` as the repository resolves them — and there is no flag naming somebody else, because a flag would let the check and the attribution disagree.
It is matched to a declared person by name and by nothing else; a commit identity the declarations do not name is its own refusal, whose remedy is `git config user.name` rather than an edit to anybody's `managers`.
A permitted scaffold records the organization in its commit, so history says whose act it was as well as who made it.

A person no organization manages is scaffolded by whoever can commit, exactly as before.

### `safix group`: membership as a verb, with the disclosures a hand edit owes

```console
$ safix group add oncall bob
$ safix group remove oncall bob
```

One name inserted into or removed from the `members` list in `safix/groups/<group>.nix`, parsed before anything is staged, with `.sops.yaml` regenerated from the declarations that edit implies and the two committed together.
It writes no value and re-wraps nothing: a membership change is a reason to run `safix fix`, and the report says so.

`remove` prints what removing cannot do.
A subject that has been in a group has held the data key of every file that group's audience names, so they have read every value in them and no re-wrap unreads it — `safix check` reports the shrink as the revocation it is, with rotation named as the remedy, and `safix fix` aligns ciphertext with policy and is explicitly not that remedy.

Delegation over a group is the silo set that holds it.

```nix
flake.safix.silos.corp.groups = [ "oncall" "contractors" ];
```

A set whose groups reach acme's managed people is acme's, so every group in it — `contractors` included, which may hold none of them — is acme's managers' to edit.
That reuses the one organizational boundary the model already has rather than inventing a per-group owner field, and a group no silo set names is covered by nobody and stays editable by whoever can commit.

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

This is the interface clan's own generators are written against, so a script written for either system runs under the other.
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

The plaintext store is its own tree, named by `flake.safix.storage.plaintextOutputs` and defaulting to `public/safix`:

```
<plaintextOutputs>/users/<user>/<name>/value
<plaintextOutputs>/shared/<audience>/<name>/value
```

A tree named for encryption has to mean everything under it is encrypted, without qualification, because that is what every backup rule, sync exclusion and reviewer assumes about it.
Once the roots are yours to name, that promise is carried by the option's name — `storage.encrypted` — and by your own choice of value, not by a literal compiled into safix.
What stays mechanically enforced is that the trees do not overlap: evaluation refuses any configuration whose roots are equal or nested, for every configuration rather than for the default one.
Two checks hold them apart behaviourally as well: `safix-public-no-rule` matches every generated creation rule against every real public path, and the catch-all check probes each configured tree — together with the definition-record tree described below, for the same reason.

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
<generatorRecords>/<user>/<name>
<generatorRecords>/shared/<audience>/<name>
```

— under `flake.safix.storage.generatorRecords`, which defaults to `state/safix/definitions`.

One plaintext line each — a format tag and a digest — over everything that decides what a mint produces: the script, its `runtimeInputs`, its `network` grant, its prompts, its dependencies, the outputs it writes with their secrecy, and the validation fragment.
No value and no derivative of a value is in it, which is what lets it be committed in the clear.

The grant is in there because it changes what a mint *may* do: the value in the file came from a fragment that could not reach the network, and a declaration that grants one describes a different mint even when the script is identical.
Covering it moved the tag from `v1` to `v2`, and a record carrying the older tag is read as no record at all — the same answer an absent one gets, for the same reason.

Its own tree, because neither of the other two can hold it.
The encrypted tree has to mean everything under it is encrypted, without qualification; the plaintext-output tree means declared public outputs a nix module reads at evaluation, and a bookkeeping file there would dilute that into "plaintext things safix wrote".
The option's name says what this one holds, which is why the tree no longer has to be at the top level or spelled `state/` to say it.

`safix check` reads it back, and reports a value whose declaration has changed since it was minted — naming regeneration and reverting the edit as the two remedies and recommending neither, because the tree holds a value and a declaration that disagree and nothing but you knows which was meant.
A value with no record predates the record and is not a finding: no record, no claim.
A record in a format the running safix does not write is not a finding either, which is what keeps a change to the digest's canonical form from reporting every value in the tree as drifted.

## Browsing what is there: `safix view`

```console
$ safix view alice grafana-token   # that one, on the terminal
$ safix view                       # choose from what alice holds, then read it
```

`safix get` writes the value to standard output and is what a pipeline calls.
`safix view` writes it to the terminal, and needs one only when it has to offer a choice — given a name it writes to standard output where no terminal opens, so a `view` in a pipeline is a working invocation rather than a refusal.

With no name, every entry the user holds is offered with the same six columns `safix list` prints: the name, where it came from, whether one value serves every carrier, whether a generator mints it, the key it is read under, and the file serving it.
Type to narrow the list, move with the arrows or `^P` and `^N`, enter to read the highlighted entry, escape to leave.
Choosing is a way of naming: the run proceeds exactly as though the chosen name had been given as an argument, through the same resolver every other verb uses.
A lone argument is a user when `flake.safix.users` declares one by that name and an entry's name otherwise, so an entry whose name is also a person's is reachable by naming both.

The highlighted entry's value is shown in a preview with four properties, each of them a property a preview of a secret has to have.
It decrypts on a quiet period rather than on a keystroke, so moving through a dozen entries forks one `sops` subprocess instead of twelve and the entries passed through are not decrypted at all.
It holds exactly one decrypted value at a time: the previous one is dropped — and zeroed — before the next is read, so nothing accumulates over a long browse.
It is drawn in a region the terminal clears on exit, so no value enters scrollback, where it would outlive the process and the zeroing.
And it stages nothing: a preview is drawn by safix itself, so there is no path to hand anybody and no plaintext reaches a file at any point.
A value that does not decrypt is reported in the region and the list stays usable, because failing to show one value says nothing about your ability to choose another.

`--no-preview` offers the same list and decrypts nothing until a choice is made, for a shared screen, a recording, or a session whose scrollback you do not control.

Three refusals, each its own:
no terminal to choose on, which names both remedies — name the entry, or `safix list` what the user holds;
the user holds nothing, which is a state of the declarations rather than of the session;
and leaving without choosing, which writes nothing, keeps nothing it decrypted, and leaves the terminal in the state it was found in.

## Editing a value: `safix edit`

```console
$ safix edit alice grafana-token   # that one
$ safix edit                       # choose what to edit, then open it
```

Opens `$VISUAL`, or `$EDITOR` when that is unset, on the entry's decrypted value.
Neither set is a refusal naming both: safix opens no editor of its own choosing, because dropping you into one you did not pick with a secret in the buffer produces either an accidental write or an accidental abandonment, and nothing can tell those apart.

The command is split on whitespace and run directly rather than through a shell, so `EDITOR="code --wait"` works.
The staged file's path is an argument; the value is not.

A non-zero exit writes nothing, an unchanged buffer commits nothing, an emptied buffer takes the same refusal an empty value takes anywhere else, and a changed non-empty buffer goes through the same write path `safix set` uses.
An entry that holds no value yet opens on an empty buffer, so this is an authoring verb as well as an amending one.

The buffer lives in the same private staging directory generators use, and whatever the editor leaves beside it — swap files, backups, undo history — is removed with the directory.
An editor configured to write undo history to a location of its own has put plaintext where safix does not look; that is the limit of the containment, and it is stated rather than left to be discovered.

With no name, `edit` offers the same selection `safix view` offers, through the same code, less every public output: a public value is already plaintext in the repository and is not editable, so it is not among the choices either — a refusal reachable by selection is one the choice should never have offered.
The editor is settled before the list opens, on every form, because a refusal after you have browsed a list and had values decrypted for a preview is a refusal that wasted your time and decrypted values for nothing.
`--no-preview` suppresses the preview here too, and the three refusals are `safix view`'s.

## Values without declarations: the runtime extract

Not every secret needs to land on disk.
A credential you invoke interactively can stay encrypted and be decrypted on demand by whatever runs it.

```nix
# in your own module, not safix's
settings.credsCommand = ''sops -d --extract '["dns-creds"]' secrets/safix/users/alice/ops-tooling.yaml'';
```

Think of it as reading a note without photocopying it.
The path uses the default `flake.safix.storage.encrypted` spelling; rename that root and this one moves with it.
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
$ safix view       # browse what a person holds, preview one, read it
$ safix generate   # mint whatever has a recipe
$ safix audit      # report which declared mappings' two sides disagree
$ safix sync       # converge declared clan and keepassxc relationships
$ safix keygen     # run by a person on their machine: mint their identity
$ safix adduser    # run by the operator: scaffold a person
$ safix enroll     # a hardware key, from a blank card to a proven recovery identity
$ safix group      # add or remove one subject in a group's declared membership
```

Think of `check` and `fix` as `git status` and `git add` for secret policy.
The nix declarations are intent, the encrypted files are reality, `check` diffs the two, and `fix` reconciles what is reconcilable and names what needs a human.
Its fifth finding class is the generator one: a value minted under a declaration that has changed since — see "The definition a value was minted under" above.

`safix set` reads the value from standard input when standard input is not a terminal, which is what makes the second form above work.
It replaces nothing: a terminal still gets the hidden prompt and the confirmation, unchanged.
What the piped form drops is the confirmation, and only where there is nobody to confirm — a piped value has no typist for the second prompt to catch out — while the empty-value refusal and the store-exactly-these-bytes rule both hold.

`upload` does not exist here, and `safix --help` records why: activation already delivers what an upload would.

`sync`'s `clan` target is not a plaintext dump and restore.
It moves one declared mapping at a time across the clan boundary — see "The bridge to clan" below — and nothing here writes a plaintext tree, because such a tree outlives the migration that justified it.

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
Reading that same slot to answer a database's own unlock challenge is a different operation from writing it, and it is what `safix sync` and `safix enroll --store-database` do when a database declares one — this refusal is about programming a slot, never about reading one to unlock what it already opens.
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


## Seeding a machine's host identity: `safix upload`

A machine's declared `recipient` is the age form of an ed25519 host key the operator holds, and `safix fix` wraps every audience naming the machine to it as soon as the declaration exists — independent of whether the machine has ever booted.
Nothing else here mints that key or gets its private half onto the machine's own disk, so a freshly declared machine's first activation has nothing to decrypt with; `safix upload <machine>` closes that one gap, once, before the machine's first activation.

```console
$ safix upload deck --directory ./preseed --identity ~/.ssh/deck_host_key
```

Two write modes, chosen by which flag is given.
`--directory DIR` writes a pre-seed tree straight to an operator-named directory and touches no network — `DIR/etc/ssh/ssh_host_ed25519_key` at mode `0600` and its `.pub` at mode `0644`, the paths and modes a fresh NixOS install's own `sshd-keygen` would produce — for `nixos-anywhere --extra-files` or for hand-copying onto installer media.
`safix-upload-directory` holds the paths and the modes, and `safix-upload-directory-mismatch` and its one-character-off drill `safix-upload-directory-drift-drill` hold the refusal when the supplied identity does not derive to the declared recipient.

`--to ADDRESS` probes the host's currently presented ed25519 key, unauthenticated, before writing anything, and takes exactly one of three actions.
A target already presenting the declared key gets an honest no-op and writes nothing, even with `--force` and `--identity` both given — `safix-upload-remote-match` and `safix-upload-remote-match-force` hold that.
A target presenting no key writes, given `--identity`, and otherwise refuses — `safix-upload-remote-write` and `safix-upload-remote-needs-identity` hold that.
A target presenting a different key refuses by default, naming both recipients, and proceeds only with `--force` together with `--identity` — `safix-upload-remote-mismatch` and `safix-upload-remote-force` hold that.

The honest no-op is the property this verb exists to hold: `safix-upload-remote-match` asserts it against the recorded subprocess invocations rather than against file state alone, so a bug that opened a write-capable session and happened to write nothing would not pass it, and flipping one byte of the declared recipient in the same fixture turns the same probe into the mismatch branch instead (`safix-upload-remote-flip-drill`) — proving the branch follows the comparison rather than a fixture-specific shortcut.

A `--to` write mirrors clan's own transport (`clan_lib/ssh/upload.py`): the two files travel inside a gzip tarball built in the same private staging root generation and editing already use, at mode `0400` for files and `0700` for directories, owned by root in the archive, and the fixed destination `/mnt/etc/ssh` — the path `nixos-anywhere --extra-files` mounts a fresh install's target root at — is wiped and then extracted into.
`safix-upload-tarball-modes` holds the archive's own contents, `safix-upload-destination` holds the wipe-then-extract sequence naming that fixed destination, and `safix-upload-staging-cleanup` holds the staging root's own lifecycle across both a success and a simulated transport failure.

Three absences are named rather than left to be discovered.
This verb provisions machines, never people: a person's name is refused the same way an undeclared machine is (`safix-upload-not-a-machine`), and provisioning a person's own first identity remains `safix keygen`'s and `safix enroll`'s.
No systemd-credentials delivery path exists yet; `--directory`'s output is a plain filesystem tree.
Nothing here triggers a deploy, a switch or a rebuild — the machine's own next rebuild is what activates what was written, and the command's own success output says so.

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
Arrival is declared per profile, in the module system that profile is written in, through a `safix.*` namespace that can select but never declare.
safix reads and defines no option outside that namespace, at either scope, so a tree that runs another secret-management framework for its own secrets keeps every option it set there and safix is unaffected by whichever revision of it that tree pins.

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
| `safix.secrets` | read-only | what resolved; at system scope typed by safix's own entry submodule and carrying the path each entry arrives at |
| `safix.identity.generateKey` | `false` | user scope only: mint `safix.identity.keyFile` at activation when it is absent |
| `safix.installer.package` | safix's own build, from the flake you imported the module from | the safix build whose `safix install` runs |
| `safix.installer.validationPackage` | the build-platform build of the same | what checks the manifest inside its own derivation |
| `safix.installer.validate` | `true` | check the manifest against its documents at build time, and hash them into the manifest |
| `safix.installer.keepGenerations` | `1` | how many generation directories to keep |
| `safix.installer.log` | `[ ]` | which of `keyImport` and `secretChanges` reach the journal |
| `safix.installer.secretsMountPoint` | `/run/safix.d`; `%r/safix.d` at user scope | where the generation directories live |
| `safix.installer.symlinkPath` | `/run/safix`; `%r/safix` at user scope | where the current generation appears, and what an entry's default path is under |
| `safix.installer.useTmpfs`, `.environment`, `.agePlugins` | `false`, `{ }`, `[ ]` | system scope only: a user-mode install mounts nothing and runs no unit environment of its own |

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

### The two published names

Each scope publishes one module, under both its plain name and its default name, and the two name the same value: `homeModules.safix` and `homeModules.default` are one file, as are `nixosModules.safix` and `nixosModules.default`.
`homeManagerModules` is a published alias of `homeModules` — one definition, two names — so `homeManagerModules.safix` and `homeManagerModules.default` are the identical values `homeModules.safix` and `homeModules.default` are.

Both names stay published so that every `imports` line written against the previous surface keeps resolving; this is a collapse rather than a rename.
The split existed for exactly one reason: `imports` cannot depend on an option, so a tree without the secret provisioner needed a form that imported it and a tree pinning its own needed a form that did not.
With no provisioner imported anywhere, every published form imports nothing outside its own file, which is strictly the stronger of the two properties the split used to offer separately.
`safix-module-entrypoints` holds that symmetrically over all four published names, and evaluates `modules/consume/home.nix` and `modules/consume/nixos.nix` bare, with no flake input in scope at all, to prove each still declares `options.safix.lib` on its own — which is what makes either importable with no flake at all rather than merely no flake-parts.

What has not changed is what happens if you import two distinct copies of one option-declaring module, which is not a merge and not a warning:

```
error: The option `safix.secrets' in `/nix/store/…-safix-b/modules/consume/nixos.nix'
       is already declared in `/nix/store/…-safix-a/modules/consume/nixos.nix'.
```

Which option the error names is a property of the evaluation rather than of the defect, so read the block as illustrative: a duplicate declaration is detected when an option is merged, not when the module list is built, so it is reported against whichever of the colliding declarations the configuration forces first.
`safix-module-collision` holds that fact over two distinct store paths of safix's own declaring module, which is why it cannot be repaired by configuration: `imports` cannot depend on configuration, so no flag could fix it after the fact.
A consumer whose `safix` input resolves to one store path everywhere is safe.

### One declaration, both scopes

The mode, the path and the key are identical in both scopes, and nothing in a declaration names one.
The system scope additionally carries `owner` and `group`; the user scope refuses an entry that sets them rather than dropping it, because a user-mode install runs as the person and chowns nothing, and a dropped ownership field reads afterwards as an ownership claim that was honoured.
That axis is read off safix's own entry type, so the refusal depends on no other framework's option declaration being present.

Two entries resolving onto one path are refused for either scope, since whichever declaration activates second unlinks the first's output.

What is scope-specific is not the declaration but the configuration an entry's `path` is a function of: a `path` written as `cfg: "${cfg.home.homeDirectory}/…"` is a home-manager expression and will not materialize into a system configuration.

The resolver's refusals surface as safix's own evaluation errors, listing every violation at once, rather than as the first of them raised from inside a manifest derivation, where the trace would name a build and not the declaration that broke.

### The identity, and the guard

`safix.identity.keyFile` defaults to null, and that default is not a preference.
safix's own installer treats a set-but-unreadable key file as fatal, naming the path, and skips a missing or unconvertible ssh key path with a line to stderr, so a non-null default would abort activation on every machine that happens to lack the path.
Both options are read by safix and defined into nothing else: safix writes no option outside its own namespace, so a tree that also runs sops-nix configures that framework's key sources itself and the two are independent.
At system scope a named identity wins, and otherwise safix derives the ed25519 entries of `services.openssh.hostKeys` that lie outside its own store — its own rule with its own exclusion prefix, because the exclusion exists to avoid decrypting with a key this installer itself deploys, which is a statement about safix's store and not about `/run/secrets`, where a foreign store's keys are exactly the identity to decrypt with.
`safix.identity.deriveHostKeys = false` turns the derivation off, and `safix-installer-identity` holds every case against the built manifest rather than against an intermediate option.

A gnupg configuration counts for nothing at either scope.
`safix.identity` carries a key file and a list of ssh keys, and nothing else is an identity safix can decrypt with, so a gnupg configuration belonging to another framework no longer suppresses safix's own no-identity refusal — which it previously did, on the strength of a configuration safix neither wrote nor could use.

At user scope there is no such default to keep, and naming one of the two is therefore not optional.
A profile whose declarations resolve and which names neither refuses at evaluation, with a message naming both options and stating why neither can be defaulted for a person.
It refuses at evaluation, before anything is applied, which is the whole reason it exists: the installer's own check runs at activation and reports presence and readability of the paths it was handed, which is no help to a profile that named no path at all.
`safix-consumption-refusals` holds the refusal, and holds it off a profile evaluated without home-manager's assertion wrapper — a wrapped profile refuses either way, and reports that something refused rather than which module did.

At user scope, safix installs `home.activation.safixIdentityPreflight`.
It reads the configured identity, checks each path for presence and readability, and refuses the switch when none is usable; it decrypts nothing.
It sorts `entryBefore [ "checkLinkTargets" ]`, which is what makes the refusal atomic: no home file linked, no user package installed, no user unit restarted, no secret written.

That ordering is the whole of the guarantee, and it is held by `safix-consumption-ordering`, which topologically sorts a real profile's activation DAG.
The same check holds the placement of safix's own install entry, which is registered `entryAfter [ "writeBoundary" ]` rather than as a bare string — a bare string becomes `entryAnywhere`, which gives home-manager no ordering to reason about at all.

The guard is narrower than it sounds, twice over, and its own failure message says so.
Where home activation runs as a NixOS host's `home-manager-<user>.service`, systemd starts that unit after system activation has already switched the system generation, so a system switch is not undone by the refusal — only that user's home generation is held back.
And presence and readability are all that were checked: a key that exists and is readable but is not a recipient of these files still fails later, inside safix's own installer, when it decrypts.
That sentence is held by `safix-identity-recipiency`, against fixture ciphertext rather than against an activation, which is the one claim on this path an evaluation cannot make.
The identity it drives is shown to open a document it *is* a recipient of before it is shown not to open one it is not, so what the refusal reports is recipiency and not a key file that was simply unusable.

The system scope installs no such guard, and that asymmetry is deliberate.
No atomic refusal point at NixOS activation has been demonstrated, and safix does not document a guarantee that no code enforces.
The failure is also rarer there, because safix derives a system-scope identity from the host's ssh keys, excluding only the ones inside its own store — and where nothing is derivable, evaluation refuses in safix's own words, since safix is the only installer on that path and nothing later would refuse usefully.
Before decrypting, the installer itself checks each configured identity path and refuses naming the paths and the ordering options; `safix-installer-refusals` holds both refusals.

### The installer safix owns, at both scopes

safix builds its own installer manifest and runs its own program against it: `safix install <manifest>`, a verb of the same command an operator uses for everything else, and the only one no operator types.
`safix-installer-sole` holds that exactly one installer acts on the resolved set, by scanning every activation step's text and every unit's `ExecStart`.
What arrived is read back at `config.safix.secrets`, the one option this package carries at either scope: at system scope it is the resolution typed by safix's own entry submodule, carrying the path each entry arrives at, and it is the exact set the manifest is built from, so what you read and what is installed cannot disagree.

The manifest schema is safix's own, written down once as `Manifest` in `crates/safix-core/src/install.rs`, versioned, and refusing an unknown field rather than ignoring it.
Both scopes emit exactly that shape, and `safix-installer-schema` holds the built manifest against an accepted snapshot so that a field added, removed or renamed on either side of the nix-to-program boundary fails on the commit that makes the change.
It is validated at build time by the same program that will read it, in whichever mode `safix.installer.validate` selects: `document`, which opens each named document and verifies every declared key is in it, or `manifest`, which validates the schema, the version, every mode's octal parse and every owner and group resolution without opening a document at all.
Neither mode decrypts — a sops document carries its mapping keys in the clear, so key presence is readable without an identity, which is what lets the stronger mode be the default inside a build sandbox that holds none.
`safix-installer-roundtrip` runs the document mode over the built manifest and over four single-field mutations — an unknown version, a non-octal mode, a key absent from its document, an unknown top-level field — and asserts one acceptance and four refusals, each naming its field.
With validation on, the manifest also carries one hash over every distinct document it names, so editing a ciphertext file changes the derivation and causes a rebuild; the installer ignores that field entirely, and making the derivation a function of the ciphertext is its whole purpose.

The store is safix's own: generations under `/run/safix.d`, the current one at `/run/safix`, both movable through `safix.installer.secretsMountPoint` and `safix.installer.symlinkPath`.
An entry that declares no path parks at `/run/safix/<name>`, which is the entry type's own default, so the root and that default move together rather than being separately maintained — `safix-installer-store` holds the pair against an oracle independent of the default itself, because the installer symlinks any entry path that is not `<symlinkPath>/<name>` and a root moved without the default writes into the foreign store instead of colliding with it.
Nothing outside safix's store is written, removed, or mounted over, and `safix-installer-coexistence` demonstrates both halves against safix's own binary in a build sandbox: pointed at an ordinary directory it removes what it finds — the branch a live mount turns into `EBUSY` — and pointed at safix's roots it leaves a foreign directory byte-identical.
`safix-installer-vm` boots a machine and reads the installed store, which is the one thing no evaluation and no sandboxed invocation measures: generation `1` after the first activation, each entry at its declared mode and ownership, generation `2` and a pruned store after the second, and a unit named in `restartUnits` observed restarted.

At system scope the installer registers as `system.activationScripts.safixInstallSecrets`, or as `systemd.services.safix-install-secrets` where systemd-sysusers or userborn manage users; the selection follows the host's own options, with `safix.installer.useSystemdActivation` as the override.
The name is what makes ordering expressible at all — two packages defining one `setupSecrets` step merge into a single activation node with no edge to state — and the ordering is yours to name, in whichever mechanism the host uses:

```nix
safix.installer.afterActivation = [ "setupSecrets" ];           # a clan host's activation step
safix.installer.afterUnits = [ "age-decrypt-secrets.service" ]; # its unit, under sysusers/userborn
```

Naming nothing is supported and leaves the installer unordered, and safix reads no option of another secret-management framework to discover its installer, for the same reason it reads no consumer's user registry.
`safix-installer-ordering` holds all three configurations.

At user scope the profile installs rather than delegating: the same manifest shape with `userMode = true`, a `systemd.user.services.safix` unit on linux, and a `home.activation.safixInstall` entry registered `entryAfter [ "writeBoundary" ]` on both platforms.
The roots are runtime-directory-relative — `%r/safix.d` and `%r/safix` — and `%r` is expanded by the installer against `$XDG_RUNTIME_DIR` on linux and the output of `getconf DARWIN_USER_TEMP_DIR` on darwin, with `%%` yielding a literal `%`.
A user-mode install mounts no filesystem, changes no file's ownership, and restarts or reloads no unit: each of the three needs a privilege the scope does not have, and each is a `userMode` branch in the installer rather than a field this scope leaves out.
`safix.identity.generateKey` mints the configured key file at activation when it is absent, defaulting off, because a profile that has never been activated has no other way to get one into place before the first install runs — `safix keygen` is the operator-run alternative and the better one wherever a person can run a command.

Three things are unsupported, and are stated rather than implied by a missing field: no template is rendered, no secret is relocated for early-boot user creation, and no gnupg identity is accepted.
safix has never supported any of the three; what changes is that the workaround closes — an entry safix resolved can no longer be reached by another framework's template or relocation, even on a host where that framework is also installed — and each absence is a refusal rather than an omission, since the manifest's `deny_unknown_fields` rejects a `templates` or `neededForUsers` field and a gnupg-only identity fails evaluation in safix's own words.

Two refusals cover the identity.
Where the resolution is non-empty and nothing is configured or derivable, evaluation fails naming `safix.identity.sshKeyPaths`, `safix.identity.keyFile` and `safix.identity.deriveHostKeys` — nothing else would refuse, because safix is the only installer on this path.
Before decrypting, the installer checks each configured identity path for presence and readability and refuses naming each path and the two ordering options, since a foreign store that has not yet run is the usual cause; presence and readability are all it checked, and decryption is not.
`safix-installer-refusals` holds both.

The limit is stated rather than implied: this coexistence covers safix's own installer.
A consumer who writes another framework's secrets option directly, beside safix, on a host that already runs that framework's store still has the original collision, and safix neither detects nor repairs that.

### Migrating from the sops-nix-backed surface

safix declares no sops-nix input, and reads and defines no `sops.*` option.
`sops` the binary is unchanged and still does every encryption and decryption, as a subprocess: the document format, its MAC, its IV-reuse rule and its key wrapping remain upstream's.

Your whole migration is: change nothing about `.sops.yaml`, the file layout, `safix fix`, or any custody declaration; rename any option you tuned; and, at user scope, update anything that referenced the old secret path.

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
| `~/.config/sops-nix/secrets/<name>` | `$XDG_RUNTIME_DIR/safix/<name>` on linux, `$(getconf DARWIN_USER_TEMP_DIR)/safix/<name>` on darwin |

The whole table is `safix.*` on the right, which is the point: a consumer who also runs sops-nix for their own secrets keeps every `sops.*` value they set, and it now means only what they meant by it.

The last row is the one that breaks something.
A user-scope secret now arrives in a runtime directory rather than a home directory, so it does not survive a reboot without a login, and anything referencing the old path has to move.
Nothing new is required of you here: every published module name carries safix's own build as a `mkDefault` for `safix.installer.package` and the build-platform one for `safix.installer.validationPackage`, so a consumer who imports `nixosModules.default`, `nixosModules.safix`, or either home-scope name names neither option. A tree that imports `modules/consume/nixos.nix` as a plain file path with no flake at all supplies `pkgs.safix` instead — or names the option, which is what the module's own refusal asks for.

## Without flake-parts, or without a flake

Everything above assumes a flake-parts consumer.
safix's evaluation rests on two narrower things: the nix module system, which a bare `lib.evalModules` call satisfies with no flake-parts machinery anywhere, and — for the thirteen attributes under `safix.lib.*` and `safix.*` the command reads, and fifteen of its sixteen verbs — a nix expression to evaluate, which does not have to be a flake output.

`flake.lib.mkVault` is the entrypoint for the first of those.
It is a function of the form `{ modules, root } -> projection`, published at a new top-level `flake.lib.mkVault` rather than inside `flake.safix.lib` — the latter is a resolved value with a fixed shape, not a namespace a function can live inside without changing what every existing reader of it sees.
Calling it evaluates `modules` together with safix's own resolver module through `lib.evalModules`, with `root` handed to `_module.args.self` unchanged, and returns exactly the value a flake-parts consumer's `flake.safix.lib` holds for the same declarations.
`safix-vault-projection` proves that by declaring one fleet twice — once as `flake.safix.*` under `flakeModules.default`, once through `mkVault`'s `modules` — and comparing the two projections field for field.
`root` is read only as a path to concatenate, never inspected for where it came from, so it need not be a flake input; the check itself calls `mkVault` with `root = ""`.
Declarations passed through `modules` scatter and merge exactly as a flake-parts `imports` list would, because both end at the same `lib.evalModules` call: the same check declares one catalogue entry across two fixture modules and asserts it resolves identically to the same declaration in one, in either module order.
A module in `modules` that declares an option outside `flake.safix` is refused — by the module system's own undeclared-option check rather than by `namespace.nix`'s scan, which only covers the flake-parts path — and `safix-vault-projection` holds that refusal too, naming the option.

`mkVault` returns only the `.lib` half.
`onboardingHook` and `enrollHook` are siblings of `flake.safix`, not fields inside `flake.safix.lib`, and a flake-parts consumer's own `flake.safix.lib` never carried them either, so a consumer who wants either hook available to a flakeless CLI declares it directly in the entry file, beside the `lib` field `mkVault` returns:

```nix
{
  safix = {
    lib = (import <safix>).lib.mkVault { modules = [ ./secrets.nix ]; root = ./.; };
    onboardingHook = null; # or a literal shell fragment, set here directly
    enrollHook = null;
  };
}
```

`safix-vault-projection` asserts the returned value carries neither key.

`mkVault` is defined once, in `lib/default.nix`, as a plain function of `{ lib }: { modules, root }: projection`, so reaching it needs no flake reference at all — only a `lib`.
`modules/flake/lib.nix` republishes that same definition at `flake.lib.mkVault`, unchanged, for a flake-parts consumer who already has `inputs.safix` to read it from.
A flakeless entry file has no `inputs`, so it imports `lib/default.nix` directly and supplies `lib` itself, the same way any other non-flake nix expression does: `examples/plain-nix/entry.nix` gets its `lib` from `NIX_PATH` via `(import <nixpkgs> { }).lib`, and a consumer elsewhere can pin the same file with `builtins.fetchTarball` or `builtins.fetchGit` naming a revision explicitly, rather than depending on the flake registry for this one lookup.

`--entry <file>`, and its environment form `SAFIX_ENTRY`, are the second narrowing, in the command rather than in nix.
Given either, the runtime evaluates `nix eval --file <entry> <attribute>` in place of `<root>#<attribute>` — two arguments where a flake target is one, but the same attribute string in the last position either way, so the thirteen `safix.lib.*`/`safix.*` spellings this runtime reads are unchanged by which form it runs.
Both are read as a leading global option ahead of any subcommand, alongside a global `--nixpkgs <flake-ref>` and its environment form `SAFIX_NIXPKGS`; where a flag and its environment variable disagree, the flag wins.
Root discovery does not move: `Workspace::discover` still finds the repository through git, unaffected by `--entry`, and an entry file need not live inside the repository a run stages and commits into.
`safix-cli` evaluates a fixture entry file through all thirteen attribute spellings, both under `--file` and against a flake target, asserting each succeeds either way, and separately asserts the three structured attributes — `generatorPlan`, `bridge`, `keepassxc` — resolve byte-identical between the two; it also asserts `--entry` overriding a conflicting `SAFIX_ENTRY`, and the workspace root staying git-discovered even when the entry file lives outside it.

Fourteen of safix's fifteen verbs — `list`, `get`, `view`, `set`, `edit`, `fix`, `check`, `audit`, `sync`, `keygen`, `adduser`, `enroll`, `group`, `upload` — read only nix values through those thirteen attributes and behave identically under `--entry` as under a flake.
`generate` is the exception, and states why at evaluation rather than leaving it to be discovered: its sandbox resolves its own tools through `nix shell --inputs-from`, which needs a flake, so running it under `--entry` (or `SAFIX_ENTRY`) with neither `--nixpkgs` nor `SAFIX_NIXPKGS` declared refuses before the first fragment runs, naming both remedies — drop `--entry` and run against the declaring flake, or add `--nixpkgs <flake-ref>` (or `SAFIX_NIXPKGS`), which the sandbox then resolves `nixpkgs#<attribute>` against directly instead of through `--inputs-from`.
A user with an empty generator order is unaffected either way, because the refusal sits after the existing empty-order return, not before it.
`safix-cli` holds the refusal's presence, its absence for an empty-order user, and both remedies named in its message.

`examples/plain-nix/` is a working copy of the recipe above: `entry.nix` fetches `mkVault` and assembles the attrset the CLI reads, `fleet.nix` declares the fleet passed through `mkVault`'s `modules`, and `hooks.nix` declares the two hooks beside it, all reachable with `safix --entry examples/plain-nix/entry.nix list`.
`examples/dendritic/` declares the identical fleet again, behind `flakeModules.default` in an ordinary flake-parts flake with one declaration per file.
`modules/flake/checks/examples.nix` evaluates both and asserts they resolve the same fleet field for field, so copy `plain-nix` if your tree has no flake at all or a flake that does not use flake-parts, and `dendritic` if it already does; `examples/README.md` indexes both in more detail.

## A vault: ciphertext in a second repository

Everything above lands in the declaring flake's own tree.
`flake.safix.vault` moves it to a second repository instead — every ciphertext document, every generated public value and every generator definition record, in place of this flake's own source — while the declarations, the recipient policy and everything you have read so far stay exactly where they are.

```nix
{
  inputs.vault.url = "git+ssh://git@example.com/fleet-vault.git";
  inputs.vault.flake = false;

  outputs = inputs@{ safix, vault, ... }: {
    # ... the rest of your flake ...
    flake.safix.vault = {
      root = inputs.vault;
      namingKey = "<64 or more lowercase hexadecimal characters, minted with `openssl rand -hex 32`>";
    };
  };
}
```

`SAFIX_VAULT_ROOT` names the operator's own working tree of that repository, the one a command actually writes and commits into — a different, mutable path from the locked, store-copied one the declaration resolves at evaluation, the same relationship `SAFIX_REPO_ROOT` already has to the declaring flake.
Left unset with no vault declared, nothing here applies: `set`, `edit`, `get`, `generate` and every other verb behave byte for byte as they do today.

**Every vault-rooted name is opaque.**
Without a vault declared, the guest-list directories under [sharedWith](#sharedwith-i-hand-you-a-copy-of-my-thing) rely on a property stated plainly there: the path states who can open the file.
A vault gives that up on purpose.
Four kinds of name move under `root` — a ciphertext document's file name, the key inside it, a public value's file name, and a generator's definition-record path — each replaced by a hash of `namingKey`, a use-specific tag, and today's readable name, so a vault host or a reader holding only the vault learns none of the audience, key or secret names the declaring flake's own tree carries.
The declaring flake still computes both forms and the mapping between them; only a vault-only view loses the readable one.

**A vault document is not browsable by hand.**
`.sops.yaml` never moves — it stays committed at the declaring flake's own source in every case, because the encryption tool reads it from there and because a vault host's own copy would be the richest document this scheme could hide — so a bare `sops <file>` run against a vault-rooted document finds no policy above it and no creation rule to fall back on.
`safix set`, `safix edit` and `safix get` are the tools against a vault document; each renders the disposable creation rules the vault-rooted write needs, uses them, and removes them again before it returns.
Renaming or re-audiencing an entry re-encrypts its leaf, exactly as it always has — a document's key path is bound into what it decrypts against, so a name is fixed at encryption time and a later rename is a fresh wrap, never a free move.

**Two commits, in an order that matters, and a safe-to-re-run refusal.**
A command that touches only the vault — `set`, `generate` — commits there alone.
One that touches both roots — `adduser`, `group`, `enroll` — commits the vault first and the declaration root second, with a trailer on the second naming the first's commit.
The order is a safety property rather than a convention: a declaration committed first could grant an audience the vault's own policy has not yet been re-wrapped for, and a `safix set` run against that gap would silently wrap a value to the *old*, narrower recipients.
Committing the vault first means the opposite failure is the only one reachable — an unreferenced vault commit nobody's declaration points at yet, which costs nothing.
If the vault commit lands and the declaration commit then fails, the run reports the vault commit's id and the pending declaration paths, and states plainly that re-running the same command completes the operation without repeating the vault commit.

**A vault commit discloses the lock bump it needs.**
A commit landing in the vault's own working tree does not update the declaring flake's lock file, so no consuming build sees it until that lock entry is bumped.
Every command that commits to the vault says so afterward, and names the exact remedy — `nix flake lock --update-input <name>` — when the declaring flake's lock file settles on exactly one input matching the vault; otherwise it states the same requirement in words general enough to stay true without guessing a name.

**What a vault host still sees, stated rather than implied.**
Opacity is a property of names, not of shape: a vault host still sees how many documents there are, one per audience as always, how many keys each holds, one per secret; each ciphertext's length, since sops does not pad; every document's recipient public keys, listed in the clear as sops itself requires; and one commit per write, exactly as before.
The naming key itself is not a secret held only by the vault's owner — it is an evaluation-time nix value, visible to anyone who can evaluate the declaring flake, which is every local user of a machine holding that flake in its nix store.
What it withholds is narrower and still real: a vault host, or a reader holding only the vault and not the declaring flake, cannot recover an audience, a secret's name, or a generator's declaration from a name alone.

**Adopting or abandoning a vault is `safix fix`'s job.**
Declaring `flake.safix.vault` on a consumer that already has ciphertext at the declaration root does not move anything by itself; `safix fix` does, as part of its ordinary convergence, decrypting every readable-layout document, public output and definition record under your own identity and re-encrypting each into its opaque vault destination, then removing the readable-layout copy.
An interrupted run is safe to resume: a destination already present is left alone, so a re-run picks up wherever it stopped rather than repeating work or losing anything.
`safix fix --vault-rollback` runs the same move the other direction while the vault is still declared — the naming key needed to recover a vault-rooted entry's readable name is only reachable through that still-standing declaration — after which the declaration and `SAFIX_VAULT_ROOT` are yours to remove.
Rotating the naming key is the identical migration, run again with a new key: every vault-rooted name is a function of the key, so there is no partial or incremental rotation, only a full re-run.
Losing the naming key never loses a secret — the declarations regenerate every name deterministically from it — so keeping it only in your own record, never committed, is recoverable by re-declaring it; only a *changed* key is a rotation rather than a loss.

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

Then `safix sync clan` converges every declared mapping, each moving in its own declared direction — `clan-to-safix` and `safix-to-clan` mixed freely in the same run; naming one or more mappings after `clan` narrows the run to them, and `--direction clan-to-safix` or `--direction safix-to-clan` narrows it to mappings declared with that value instead.

Direction is written as its endpoints rather than as a verb, and that is not pedantry.
`clan vars export` moves values *out of* clan; a `safix-to-clan` mapping's convergence moves a value the opposite way, so a word one tool already uses for its own verb would mean the opposite thing if reused here.
Both are correct relative to the tool that moves them, and a declaration is read by someone with no tool in hand to be relative to, so the endpoints are named instead.

`import` and `export` no longer exist as safix's own verbs, and the two absences are not the same kind.
`export` is retired permanently: the operation clan's own word names, a bulk plaintext dump, is the one safix's design refuses to build on either side of the boundary.
`import` is reserved rather than retired, for a future, unbuilt feature — ingesting a value from an external plaintext source one entry at a time, analogous to clan's own `import-sops` — and `safix --help` records the reservation, so the absence reads as a decision rather than an oversight.

**clan stays the authority on its own store.**
Every read is `clan vars get` and every write is `clan vars set`, run as subprocesses with the value on a pipe.
safix reads, writes, encrypts, decrypts and parses none of clan's stored files, in either direction, so the bridge works over `sops`, `age`, `password-store` and whatever clan adds, with no code here.
The cost is that a consumer without clan-cli cannot reach clan's side of the bridge at all — which is arguably correct, since a consumer with no clan has no clan-side value to reach.

**Every run compares before it writes.**
Each mapping is read on both sides and compared before either is written, so a mapping whose two sides agree is not written and not committed and a second run changes nothing.
On the safix-to-clan direction that comparison is essential rather than an optimisation: clan's write is unconditional and a re-encrypting backend produces fresh ciphertext for an unchanged value, so without it every run would commit in the clan repository for every mapping.

A value moving clan-to-safix goes through the same path a hand-typed one takes, so it acquires the recipient-drift refusal, the staged write and the rename, and lands as its own commit naming the mapping and the direction and never the value.

Half of every mapping lives in another flake, so evaluation refuses only what is local to you: an unresolvable safix side, a clan-to-safix mapping writing into a value a generator also produces, two mappings writing one target, one pair of endpoints declared in both directions, and mappings with no `clanFlake` to reach.
It claims nothing about the clan side.
A clan side that does not resolve is refused when a transfer reaches it, in clan's own words, naming the machine, the generator and the file.

**Two refusals belong to the safix-to-clan direction alone.**
A source entry that holds no value is refused rather than written into clan as nothing — a question evaluation cannot answer, because an entry declares where a value lives rather than that one is there.
And a mapping whose clan-side generator clan already considers outdated is refused, because clan records a validation per generator and its next routine `clan vars generate` would replace whatever was written without saying so.
There is no option that writes anyway: safix has nowhere to record that a var is externally supplied, so the flag would turn a refusal into a silent loss.
The refusal names both remedies — bring clan's side back into agreement, or declare the mapping `clan-to-safix`, which is the right shape when clan's generator is the producer.

That second refusal reaches further than it may look, and the reach is correct.
clan records a validation for a generator only when the generator declares `validation`, and it calls one whose declared validation has nothing recorded beside it outdated — so a generator that declares a validation and has never run is refused at its *first* safix-to-clan write, because it has not run and will, and the run would replace whatever was written.
The generator to write into is therefore one that declares no `validation`: a var clan holds a place for and nothing else.

**`safix audit clan` is the report over the same declarations.**
It compares both sides of every declared mapping, or the ones named, in either direction, and changes nothing on either side of the boundary.
A mapping agrees when both sides hold the same bytes, and also when neither side holds a value yet, which is a bridge nobody has bootstrapped rather than a disagreement.
It is a finding when the two sides hold different values, when one side holds a value the other does not, or when the comparison could not be made — and each finding names the mapping, its two endpoints and the command that converges it, and never a value.
Alongside those findings, it names every clan var on the machines those declarations name or resolve that no currently declared mapping accounts for — a mapping removed from the declarations does not delete the clan var it named, and this is how that stops being silent — reported as information, scoped to the machines currently in play, and never changing the exit status; nothing here removes one, a person does that, with clan's own command.

It is a verb of its own rather than more rows in `check`, and the reason is what `check` is.
`check` decrypts nothing, which is what lets one machine judge files belonging to people whose keys it does not have, and it needs no clan.
Comparing a mapping's two sides needs both of those: it decrypts the safix side, and it runs clan's own command once per mapping.
So the verb that needs them carries them, `check` keeps both of its properties, and a mapping you cannot decrypt is reported as one that could not be judged rather than quietly left out — a report that dropped those would be a report about who ran it.

**A `two-way` mapping converges toward whichever side changed, and never guesses.**
`direction = "two-way"` declares a standing relationship rather than a one-off transfer: `sync clan --direction two-way`, or a bare `sync clan`, reads both sides against the last agreement it remembers and writes the side that has not moved to match the one that has.
When both sides have moved since the last agreement, or neither side has ever agreed and the two now disagree, nothing is written and the finding names the mapping and the two one-way remedies — narrow the run to `--direction clan-to-safix` or `--direction safix-to-clan`, run it once, then declare the mapping `two-way` again.

That memory is a digest, held the way `keepassxc-to-safix`'s own memory is: recorded only after the value it describes has landed, in a companion entry minted beside the mapped one, sharing its file and its audience, and never in clan's own store or in the plaintext definitions tree.
The companion's name is the mapped entry's plus `-safix-bridge-sync-state`, and evaluation refuses a hand-declared entry that collides with it, naming the entry, the mapping, and the suffix.

**A `shared` placement is addressed by asking clan, never by declaring a second field.**
`placement = "shared"` (default `"per-machine"`) says the clan side is one var no machine owns exclusively, so `machine` is refused rather than required; the runtime discovers which machine to reach it through by trying each name `clan machines list` returns until one resolves, and refuses only once every one of them has failed.
A two-way push still carries the identical stale-generator refusal a `safix-to-clan` write already has, with no override.

**What the bridge's evidence is made of.**
Every check that drives the bridge drives a stub of clan's command line, `crates/safix/tests/support/clan-stub.rs`, and no real-clan drill ships.
That is the right instrument for what the claims are about — that a read runs clan's command and takes what came back on the pipe, that a write puts the value on standard input and nowhere else, that clan's refusals reach the operator as clan's words, and that nothing here reads a file clan placed — because a stub can be asked what it saw and a real clan cannot.
What it does not establish is that those argument vectors mean to clan what safix thinks they mean; that rests on review of clan-cli itself, and the stub's own header cites the source file behind each line of the contract it stands in for, so the review is repeatable rather than remembered.

## The mirror in your password database

Some secrets are read by tools and some are also read by a person — typed into a web login, a phone, another machine's prompt.
`safix sync keepassxc` ends the drift between the two, one declared mapping at a time.

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

Then `safix sync keepassxc` converges every mapping declared here, and naming one or more mappings after it — `safix sync keepassxc grafana` — narrows the run to them.

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

**`safix audit keepassxc` compares without writing.**
It reads both sides of every declared mapping, or the ones named, per its declared mode, and changes nothing in the database or in safix's own files.
Each mapping is reported as agreeing, diverged, or unjudgeable, and a diverged mapping's own remedy is named: `safix sync keepassxc <mapping>`.
Entries under the declared group that no mapping declares are reported alongside as lingering information, in the same shape `sync`'s own report already gives it, and never move the exit status.

## The three storage roots

Safix places files in exactly three trees, and each one is a repository-relative path you name:

```nix
flake.safix.storage = {
  encrypted        = ".safix/encrypted";
  plaintextOutputs = ".safix/plaintext-outputs";
  generatorRecords = ".safix/generator-records";
};
```

The defaults are `secrets/safix`, `public/safix` and `state/safix/definitions`, so leaving the option unset changes nothing.
Three independent roots rather than one parent: a consumer who wants one parent writes three strings sharing it and gets one ignore entry, one backup rule and one directory to move, while no single-parent option could express "the public tree lives where a static-site build can read it".

Evaluation refuses a root that is empty, absolute, ends in `/` or carries a `..` component, and refuses any two of the three that are equal or nested — naming both options and both values.
Comparison is on component boundaries, so `secrets/safix` and `secrets/safix-public` are two disjoint trees while `secrets/safix` and `secrets/safix/pub` are one inside the other.

**What your own ignore, backup and exclusion rules should say.**
Safix writes no `.gitignore` at the declaration root, and exactly one at the vault root, covering only the scratch rules file.
Everything else is yours: the encrypted tree is ciphertext without qualification and belongs in a backup; the plaintext-output tree holds values a module reads at evaluation and belongs in the repository; the generator-record tree holds digests with no value in them and belongs in the repository too.
With all three under one parent, an `rsync --exclude` or a backup rule naming that parent covers all of them at once.
One caveat on a hidden root: `rg` and `fd` skip dot-directories by default, so auditing `.safix/…` needs `--hidden`.

**Renaming a root.**
Change the option, `git mv` the tree, run `safix fix`, run `safix check`.
No re-encryption is involved in readable mode: a sops document does not embed its own path, and the in-document key names do not change — `fix` rewrites `.sops.yaml` (new directory, new `path_regex`, new header prose) and nothing else.
A half-finished rename is visible rather than silent: between the option change and the `git mv`, `safix check` reports every governed file the regenerated policy no longer names.

With a vault declared, a rename moves nothing at all inside the vault: a vault-rooted name is a hash of the entry's identity relative to its root, not of the root's spelling.

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

Every rule is start-anchored under `flake.safix.storage.encrypted`, extension-terminated, and one directory level.
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
| recipient policy, in a consumer's tree | `.sops.yaml` — written by `safix fix`, never by hand; stays at the declaration root even with a vault declared |

And the three trees safix places files in, each named by an option:

| tree | option | default | with a vault declared |
|---|---|---|---|
| encrypted values | `flake.safix.storage.encrypted` | `secrets/safix` — `users/<u>/secrets.yaml` and `shared/<audience>/secrets.yaml` below it | `secrets/<opaque-hash>.yaml` at the vault root |
| public outputs | `flake.safix.storage.plaintextOutputs` | `public/safix` — `users/<u>/<name>/value` and `shared/<audience>/<name>/value` below it, no creation rule, readable at evaluation | `public/<opaque-hash>` at the vault root |
| the definition each generated value was minted under | `flake.safix.storage.generatorRecords` | `state/safix/definitions` — `<u>/<name>` and `shared/<audience>/<name>` below it, one plaintext digest per value | `state/<opaque-hash>` at the vault root |

The vault's own three buckets are not configurable: a vault-rooted name is a hash of an entry's identity, so renaming a storage root moves nothing inside a vault.

| concern | file |
|---|---|
| the vault, when `flake.safix.vault` is declared | a second git repository, at `root`; the operator's working tree of it is named by `SAFIX_VAULT_ROOT` |

The option reference lives on the types themselves; this document is the narrative companion.

## Status

The evaluation half, the command, the exported checks, the materializations and the two consumption modules are here and green under `nix flake check`.
Every push and pull request builds the whole surface on x86_64-linux and aarch64-darwin and evaluates it for aarch64-linux, which nothing there builds.
`.github/workflows/check.yml` carries the one thing a linux runner has to be told first: Ubuntu denies unprivileged user namespaces, and the checks that drive a generator are made of them.
darwin has no tmpfs, so `Staging::establish` refuses there and `--allow-disk-staging` is the acknowledgement the runtime documents; the suite runs under it on that platform, the refusal itself is asserted, and the tmpfs guarantee — which needs a memory-backed mount to compare against — is claimed on linux and absent rather than half-made on darwin.
A GitHub macOS runner refuses `sandbox_apply`, so the envelope a generator fragment runs inside cannot be applied there; that leg builds the checks which do not need the integration suite, derived from the store rather than listed, and says in its own summary what it left out.
The narrowing is the runner's and lives in the workflow: a Mac that can apply a sandbox profile gets the whole surface under `nix flake check`.

The runtime is rust, and `packages.safix` is that binary.
`crates/` holds a cargo workspace — `safix-core`, the runtime as an embeddable library, and `safix`, a thin command over it — built, unit-tested, linted, formatted, licence-checked, advisory-scanned and integration-tested under `nix flake check`.
It implements all fifteen subcommands: the read paths `list`, `get`, `view`, `check` and `audit`, the write paths `set`, `edit` and `fix`, the generator graph behind `generate`, `sync`'s two targets converging the clan bridge and the password-database mirror, and the three that touch custody itself, `keygen`, `adduser` and `enroll`, plus `group` for editing a group's declared membership and `upload` for seeding a machine's own host identity.
The nix half was never in scope and did not move; what was replaced is a shell runtime and two python helpers, all three now deleted.

The port ran behind a differential harness comparing every subcommand against the shell runtime; the five places the two differ are recorded as decisions in the changelog's "Known differences".
With the port complete the harness was deleted with the runtime it compared against — 6205 lines — and its claims rewritten as `crates/safix/tests/`, which drives the built binary against throwaway repositories and asserts against literals.
`safix-syscall-proof` (linux-only) observes every plaintext `write` a `set` and a `generate` make and holds each to a pipe; `safix-channel-drills` damages the runtime once per channel and fails unless each damage is caught by the channel that exists to catch it.
`safix-generate-envelope` (linux-only) drives fragments that try to leave the sandbox and holds each attempt to failing, each one drilled against an unconfined run of the same fragment so that an absent file is the envelope's doing rather than the fragment's.
The proposal, the decisions and the staging are in `openspec/changes/rewrite-runtime-in-rust/` for the port and `openspec/changes/rust-only-runtime/` for the retirement.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
