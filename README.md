# safix

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

      flake.safix.users.ana = {
        recipient = "age1...";
        private.ana-token = { };
      };
    };
}
```

That is the whole integration.
`flake.safix.lib` now holds the audiences, the placements, the generated policy text and the check builders, and `packages.safix` is the command.
Put `safix` in your devshell and run `safix fix` once to write `.sops.yaml`, then `safix set ana-token`.

Declarations merge, so the block above can live in its own file imported alongside a hundred others; safix reads no path, no filename and no directory structure to find them.

## The one mental model

A secret has three questions: who declares it, who can read it, and where it lands.
Declaring happens in nix and is the label on the box.
Reading is decided by the `.sops.yaml` recipients, which are generated from the declarations and never hand-edited.
Landing means the file a profile reads at activation, by default the secret provisioner's own path for the name.
Everything below is a different answer to the first two questions.

The distinction that does the work is placement versus custody.
Custody is who holds a secret, and it is a property of a person: it is the same on every host they log into.
Placement is where the decrypted value shows up, and it is a property of a configuration.
Every refusal safix makes comes from keeping those two apart.

## private: mine alone

```nix
flake.safix.users.ana.private = {
  filen-key = { };
  ssh-personal.mode = "0600";
};
```

```console
$ safix set filen-key    # prompts hidden, encrypts, commits
$ safix get filen-key    # prints the value, for piping
```

Think of it as ana's drawer.
Declaring an entry here is the whole story: there is no catalogue entry and no separate selection step, because a private declaration is its own selection.
Only the holder can read it.

## carries: I take one from the shelf

```nix
# the shelf, declared once
flake.safix.catalogue.cognee-api-key = { };

# ana taking one
flake.safix.users.ana.carries.cognee-api-key = { };
```

Think of it as a shelf of standard items.
The shelf says this thing exists; carrying says I have one.
By default each carrier gets their own copy with their own value: if bo also carries `cognee-api-key`, his value and ana's are unrelated.
Same label, different contents.

## sharedWith: I hand you a copy of my thing

```nix
flake.safix.users.ana.sharedWith.bo = {
  linear-credentials = { };
};
```

```console
$ safix fix
$ sops secrets/safix/shared/ana,bo/secrets.yaml
```

Think of it as a shared drawer between exactly those two.
The directory name is the guest list: `ana,bo` means those two can open it and nobody else can.
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
flake.safix.users.ana.carries.team-api-token = { };
flake.safix.users.bo.carries.team-api-token = { };
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
flake.safix.users.ana.perHost.builder = {
  omit.filen-key = { };
};
```

Think of it as which rooms my keys follow me into.
The secret is still ana's everywhere; it simply does not land on that host.
This is why a carrier of a shared entry who omits it on one host stays in the audience: omitting is about placement, and custody is about carrying.
Reaching an entry only through a `perHost` or `perTag` `add` is refused, because a host-scoped selection puts nobody in any audience and would leave that person resolving a file they are not encrypted to.

## Generators: the value writes itself

```nix
flake.safix.users.ana.private.grafana-token = {
  generator.script = "openssl rand -hex 32";
  generator.runtimeInputs = [ "openssl" ];
};
```

```console
$ safix generate                              # mints everything declared but empty
$ safix generate --regenerate grafana-token   # rotation: new value, committed
```

Dependencies chain generators.
Think of a recipe that uses another recipe's output.

```nix
flake.safix.users.ana.private = {
  db-password.generator.script = "openssl rand -base64 24";
  db-password-hash.generator = {
    dependencies = [ "db-password" ];
    script = ''mkpasswd -sm bcrypt <"$in_db_password"'';
    runtimeInputs = [ "mkpasswd" ];
  };
};
```

Rotating `db-password` cascades: every generator downstream re-runs, in dependency order, after showing you the list and asking once.
A hash of a retired password would be a lie, which is why the cascade is not optional.
Cycles, self-references, and depending on another person's secret are all refused at evaluation — the last because your machine structurally cannot decrypt someone else's value.

A prompted generator asks instead of computing.

```nix
flake.safix.users.ana.private.upstream-api-key.generator = {
  prompts.token = {
    type = "hidden";
    description = "the API key issued by the provider's console";
  };
  script = ''cat "$in_token"'';
};
```

A multi-output generator mints related values together, each with its own mode.

```nix
flake.safix.users.ana.private = {
  deploy-key = {
    mode = "0600";
    generator = {
      files = [ "deploy-key-pub" ];
      runtimeInputs = [
        "openssl"
        "jq"
      ];
      script = ''
        key=$(openssl genpkey -algorithm ed25519)
        pub=$(printf '%s' "$key" | openssl pkey -pubout)
        jq -n --arg k "$key" --arg p "$pub" '{"deploy-key": $k, "deploy-key-pub": $p}'
      '';
    };
  };

  deploy-key-pub.mode = "0444";
};
```

Each name a generator writes is a registry entry in its own right, carrying its own mode, path and key; `files` only records which generator produces it.
An entry named there may not carry a generator of its own and may not be named by a second generator, both refused at evaluation, because two producers for one value is a race whose winner is whichever ran last.
Both keys land in one commit, because a keypair split across two commits is an incoherent state.
A `validation` script receives the candidate value on stdin and refuses the write on a non-zero exit.

`runtimeInputs` names nixpkgs attributes as strings rather than holding packages, because the whole generator travels to the command as JSON and a derivation cannot cross that boundary.
Strings are unchecked by construction, so `safix-generator-tools` resolves each one against the package set at build time; otherwise `opensll` is discovered at a rotation, which is the worst moment to learn a declaration was never right.

One honest limitation, stated on the option itself: the file descriptor is how a dependency or prompt value arrives, not a sandbox.
A generator script that redirects `$in_*` to a file or echoes it to stderr puts plaintext where the tool cannot see it.
Write generators the way you would write any code that holds a credential.

## Values without declarations: the runtime extract

Not every secret needs to land on disk.
A credential you invoke interactively can stay encrypted and be decrypted on demand by whatever runs it.

```nix
# in your own module, not safix's
settings.credsCommand = ''sops -d --extract '["dns-creds"]' secrets/safix/users/ana/ops-tooling.yaml'';
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
$ safix get NAME   # read one value to stdout
$ safix generate   # mint whatever has a recipe
$ safix keygen     # run by a person on their machine: mint their identity
$ safix adduser    # run by the operator: scaffold a person
```

Think of `check` and `fix` as `git status` and `git add` for secret policy.
The nix declarations are intent, the encrypted files are reality, `check` diffs the two, and `fix` reconciles what is reconcilable and names what needs a human.

`upload`, `export`, and `import` do not exist here, and `safix --help` records why.
Activation already delivers what an upload would.
Export and import serve a migration between backends that safix does not have, and a plaintext export tree outlives the migration that justified it.

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
$ safix adduser tama age1abc...
# writes safix/users/tama.nix, regenerates .sops.yaml, commits exactly those two
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

## Materializing into a profile

One declaration serves both scopes, and nothing in it names which.

```nix
# a home-manager module, with the flake reachable through extraSpecialArgs
{ config, flake, ... }:
{
  imports = [ inputs.sops-nix.homeManagerModules.sops ];

  sops.secrets = flake.safix.lib.materialize {
    user = config.home.username;
    hostname = "workstation";   # whatever your module knows the host as
    tags = [ ];
    scope = "user";
  } config;
}
```

The system scope is the same call with `scope = "system"`, against `inputs.sops-nix.nixosModules.sops`.
The mode, the path and the key are identical in both; the system scope additionally carries `owner` and `group`, and the user scope refuses an entry that sets them rather than dropping it, because the user-scope provisioner has no ownership axis and a dropped ownership field reads afterwards as an ownership claim that was honoured.

Two entries resolving onto one path are refused for either scope, since whichever declaration activates second unlinks the first's output.

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
          ana-workstation = /* the attrset your profile materializes */;
        };
      };
    };
}
```

Called with no arguments it returns five checks over your declarations: the custody refusals, the generator runtime tools, the shape of every generated rule, the absence of a catch-all, and the audience separator.
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
| the two records a consumer declares | `modules/flake/safix/options.nix` |
| the option types and their reference documentation | `modules/flake/safix/types.nix` |
| the resolution algebra | `modules/flake/safix/resolve.nix` |
| the recipient policy renderer | `modules/flake/safix/policy.nix` |
| the checks a consumer instantiates | `modules/flake/safix/checks.nix` |
| the module a consumer imports | `modules/flake/safix/default.nix` |
| the command | `modules/flake/safix/safix.sh` |
| recipient policy, in a consumer's tree | `.sops.yaml` — written by `safix fix`, never by hand |
| encrypted values, in a consumer's tree | `secrets/safix/users/<u>/` and `secrets/safix/shared/<audience>/` |

The option reference lives on the types themselves; this document is the narrative companion.

## Status

The evaluation half, the command, the exported checks and the materializations are here and green under `nix flake check`.
The repository has no remote yet, so the GitHub Actions workflow in `.github/workflows/` has never run; it activates on the first push.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
