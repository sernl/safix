# Design: adopting clan's executor, and paying for it honestly

## What was read

clan-core at `/nix/store/skwb0795vlb7ymhl8zkc9cdx2cm3mf9d-source`, specifically `pkgs/clan-cli/clan_lib/vars/generator.py` (the executor), `nixosModules/clanCore/vars/public/in_repo.nix` (the public store and the `.value` accessor), and `clan_lib/vars/set.py` (the write path a bridge would use).

The executor's contract, read off the code rather than the documentation:

- `env["in"]`, `env["out"]` and `env["prompts"]` are set to directories under one temporary root, and the process's working directory is that root.
- `in` and `out` always exist. `prompts` is created *only when the generator declares prompts*, so a script cannot distinguish "no prompts declared" from "prompts directory missing".
- A prompt's answer is written with `write_text(value)` — no newline is added, and none is stripped on read.
- Dependencies are materialized into `in` as `in/<dependency-name>/<file-name>`.
- After the script exits, each declared file is looked for at `out/<name>`. A missing one raises, and the message lists what the output directory *did* contain — a good error and worth copying.
- `file.secret` routes the bytes to the secret store or the public store. Public bytes are written in the clear into the repository.
- The script runs `bash -c` inside a sandbox by default, with the staging root as the only read-write path, and `--no-sandbox` disables it.

## D1. The pipes-only invariant is broken on purpose, and this is the decision to contest

safix 0.1's `secret-generators` spec says a generated value "travels a pipe and never argv, the environment, or a file", and `safix-cli` says values "move through pipes only".
Those are the strongest claims in the project.
Every one of `$out/<name>`, `$in/<dep>/<file>`, `$prompts/<key>` and an editor's buffer is a file.

There is no version of clan compatibility that preserves the pipe.
The contract *is* a filesystem contract; a generator script addresses its inputs and outputs by path, and a path is a file.
Emulating it with named pipes was considered and refused for two reasons that are not close: a FIFO is not seekable and not re-readable, so `head -c 32 $in/dep/key` and any tool that opens its input twice would break in ways whose failure mode is a truncated secret rather than an error; and a directory of FIFOs cannot answer `ls $out` or `[ -f $out/x ]`, which scripts written against clan's contract legitimately do.

So the decision is: adopt the file contract, and replace the absolute with a bounded one.

The replacement is stated as a comparison rather than as reassurance, because the two are not equivalent:

| | pipe (0.1) | tmpfs staging (0.2) |
|---|---|---|
| plaintext at rest on a block device | never | never, unless the acknowledgement flag is passed |
| plaintext in memory | for the transfer | for the run |
| plaintext in swap | possible | possible |
| reachable by another process of the same user | no | yes, for the run's duration |
| reachable by root | yes | yes |
| survives a crash | no | no, if the shred runs; yes, in the window before it |

The row that is genuinely worse is the fourth.
A pipe between two processes safix spawned is reachable by neither a third process nor a shell; a mode-700 directory on `/dev/shm` is reachable by anything running as that user, which on a workstation includes the operator's own editor, shell and every agent process they are running.
That is a real reduction and it is written into the spec as a stated limit, not smoothed over.

What is retained: the pipe requirement is *modified*, not deleted.
Every path where a pipe is still possible keeps it — `set` from standard input, `get` to standard output, and the sops invocation, which stays `Stdio::piped()` throughout and never gains a file argument for a value.
The exception is named and bounded to the generator staging root and the editor buffer.

## D2. Where plaintext is allowed to be

The operator's rule, adopted as written and made executable:

1. The staging root is a directory created mode `0700` inside a tmpfs mount, and every file in it is `0600`.
2. On linux the runtime does not assume `/dev/shm` is tmpfs — it stats the filesystem and requires the tmpfs magic. `/dev/shm` being a tmpfs is the overwhelmingly common case and is not the case this check exists for; the case it exists for is a container or a hardened host where it has been remounted or replaced.
3. If no tmpfs mount is available, the run refuses. It proceeds only when the operator passes `--allow-disk-staging`, which is a flag whose name states what is being accepted rather than one that reads as a convenience.
4. The shred runs on every exit path — return, error, panic, `SIGINT`, `SIGTERM` — through the existing process-wide registry in `scratch.rs`, which already carries a directory list and already registers before creating.

`/tmp` is not a fallback. This fleet's `/tmp` is ext4, so a silent fallback to it would be the exact failure the rule exists to prevent, occurring under a code path that looks like it succeeded.

Two limits are recorded in the spec rather than in a comment:

- Overwriting a tmpfs page does not reach a copy of that page that was swapped out before the overwrite. tmpfs bounds plaintext to memory and swap; closing the swap half is an encrypted-swap decision on the host, outside safix.
- What a generator script or an editor does with a value it was handed is the author's to get right. safix shreds the staging root, including whatever the editor left beside the file it was given — but an editor configured to write undo history or backups to a global directory has put plaintext where safix does not look. The spec says this in the same voice `types.nix` already uses for the equivalent 0.1 limit, which is the right precedent: it is a statement of where the boundary is, not an apology.

## D3. The public store lives outside `secrets/`

`files.<name>.secret = false` writes plaintext into the repository.
clan's layout, read off `in_repo.nix`, is:

```
<clan dir>/vars/shared/<generator>/<file>/value
<clan dir>/vars/per-machine/<machine>/<generator>/<file>/value
```

The leaf is a directory named for the file, containing a file literally named `value`.
safix mirrors the *shape* and not the *path*, because safix's placement axis is audience rather than machine:

```
public/safix/users/<user>/<name>/value
public/safix/shared/<audience>/<name>/value
```

The location decision is the contested one, and the alternative was explicit in the request: "e.g. `public/` beside the sops files", meaning `secrets/safix/public/`.

That is refused, and the reason is not aesthetic.
Under `secrets/safix/public/` the plaintext store sits inside the directory every operator, every backup rule, every sync exclusion and every `rg` invocation treats as "the ciphertext tree".
This fleet has already been bitten by exactly that class of error in the other direction — a plaintext identity inside a minutely two-way sync, which is open as group 0 of `one-unlock-bootstrap` — and the lesson generalizes: a path named `secrets` must mean "everything here is encrypted", without qualification, because that is the proposition every tool and every human applies to it.

A top-level `public/` sibling makes the two trees separable by prefix, which is what a `.gitignore`, an `rsync --exclude`, a backup policy and a reviewer all operate on.

Non-interaction with `.sops.yaml` is then *checked* rather than argued.
The generated rules are anchored `^secrets/safix/...` and terminate on `\.yaml$`, so a `value` file under `public/` cannot match either clause — but relying on that is relying on two independent accidents staying true.
The check is a new member of the existing rule-shape family in `checks.nix`, using the `matches` helper already there: for every generated rule and every public path the declarations produce, assert no match, and add the public paths to `catchAllProbes` so a rule that *would* reach them fails the existing catch-all check as well.
Two checks rather than one, because the first asks "does any rule reach the public store" and the second asks "does any rule reach anywhere nothing is placed", and a future refactor that weakens one is unlikely to weaken both.

### `.path` and `.value`

`.path` is available for every output, secret or public. It is a path, not a value.

`.value` is available only when `secret = false`, and reads the file at evaluation.
When the file does not exist yet, clan throws with a message naming the command to run; that is copied, because a nix evaluation failing with "run `safix generate <name>`" is strictly better than one failing with a path that does not exist.

Where safix departs from clan: when `secret = true`, clan leaves `.value` *undefined* (`mkIf (secret == false)`), so reaching for it produces nix's generic "option used but not defined" message.
safix defines it as a `throw` naming the entry, stating that it is a secret, and pointing at `.path`.
The cost is one evaluated thunk; the benefit is that the most likely authoring mistake in the whole surface — reaching for `.value` on a secret because the sibling public output has one — produces a sentence that says what to do.

## D4. `share` is derived, not authored

clan puts `share` on the generator. safix puts `shared` on the entry.

Moving `shared` to the generator was rejected: safix's audience is a property of who carries an entry, and `shared` is what decides whether two carriers hold one value or two. That is entry-level information and the resolver, the policy renderer and the audience directory all read it there.

So `share` is added to the generator as a derived, read-only field: a generator is shared exactly when every entry it writes is shared, and a generator whose outputs disagree is refused at evaluation naming both sides.

Two things fall out, and the second is the reason to prefer this over simply computing `share` at bridge time.

First, the bridge gets the field it must compare, in the shape clan compares it in, without a second authoring surface for the same fact.

Second, a generator's outputs now always resolve to one audience, therefore one file, therefore one rename.
0.1's multi-output write already stages per distinct file and renames each, and `generate.rs`'s own module note records the partial state a crash between renames leaves: "generators that already committed stay".
With outputs constrained to one audience that window closes for the multi-output case that motivated it — a keypair's private and public halves — because there is one rename.
It does not close in general, since a `--regenerate` cascade still commits per generator, and that remains true and remains recorded.

The refusal is a genuine restriction on what a consumer can declare, and it is worth being explicit that it forbids something previously legal: a generator writing one private entry for `ana` alone and one shared entry for `ana+bo`. If a consumer wants that, they write two generators and make the second depend on the first.

## D5. The v1 interface is removed, and the break is loud at evaluation

Recommendation: clean break. No compatibility mode.

The prior offered was that 0.2 is pre-adoption and a clean break with a loud error is right.
That prior is correct, and there is a stronger argument for it than pre-adoption, which is worth having because pre-adoption expires and this reason does not.

The two interfaces do not differ in spelling. They differ in custody.
`$in_<name>` is a read-once descriptor that never materializes a file. `$in/<dep>/<file>` is a real file in the staging root.
A compatibility mode means a single run can contain generators of both kinds, which means the staging root exists whenever *any* generator in the run is v2 — and a v1 generator's descriptor-only guarantee is then a guarantee about that generator's own inputs while the run as a whole is staging plaintext on a filesystem.
The weaker property governs the run, and the operator reading `secret-generators` would find a requirement that is true per-generator and false per-run.
That is precisely the shape of hazard the rust rewrite existed to eliminate: a guarantee held by which declaration you happened to read.

Detection is cheap, total, and available before anything executes, because the script is a nix string:

- A script mentioning `$in_` or `${in_` is v1. Refuse, naming the change, and give the rewrite: `$in_<name>` becomes `$prompts/<name>` for a prompt and `$in/<name>/<file>` for a dependency.
- A script mentioning `$out_name` is v1 validation. Refuse; `$out_name` becomes the name of the file currently under judgement, passed the same way.
- A script that never mentions `$out` produces no output file under v2 and would be refused at runtime with "did not generate a file for '<name>'", which names the symptom rather than the cause. Refuse at evaluation instead, naming the cause.
- A generator declaring `files` as a list rather than an attribute set is v1 by type and fails at evaluation already; the type error is augmented with the same pointer.

Note that `bash -euo pipefail` would catch `$in_foo` as an unbound variable at runtime anyway. The evaluation-time refusal is kept regardless, and the reason is recorded: "unbound variable" names a symptom in a script the operator did not just write, while the refusal names the interface change and the rewrite. Both firing is not redundancy worth removing.

The refusals are retained permanently rather than deleted after the fleet migrates. They cost a string match during evaluation, and the failure they prevent — a v1 generator silently producing no output, or reading an empty input — is one whose runtime symptom is a truncated or absent secret.

## D6. `edit` is a subcommand, not a flag

Precedents, in order of relevance:

- **sops**: `sops <file>` with no verb opens `$EDITOR` on the whole decrypted document. The editor is the *default* action, and every other operation is a flag away from it.
- **pass**: `pass edit <name>` — a subcommand, on the entry rather than the store.
- **git**: `git commit` opens the editor; `-m` bypasses it. The editor is the default and the flag is the escape.
- **clan**: no editor for vars at all. `clan vars set <machine> <var_id>` reads standard input.

Decision: `safix edit <name>`, a subcommand. `set` is unchanged.

The reasons, strongest first:

1. `set` and `edit` have different custody profiles, and folding them into one verb would make custody a function of a flag. `set` never reads the existing value; it takes a new one from a stream and writes it. `edit` must decrypt the existing value, materialize it, hand it to a program safix does not control, and read it back. Those are different enough that they deserve different refusals, and a `--editor` flag would give one command two.
2. `set`'s contract is "the value arrives on a stream and never touches a filesystem", which is a requirement in the current spec. A flag that inverts a requirement is worse than a verb that has a different one.
3. sops' editor is over a whole document; safix's unit is one entry under one key. `edit` matching `get` and `set`'s addressing — by name, never by file — is the consistency that matters, and it is the first requirement in `safix-cli`.
4. Discoverability. `safix --help` lists verbs. A flag is invisible until someone reads `safix set --help`, and the operator asked for editor input as a way of working, not as an option.

Against, and recorded because it is the real cost: git's precedent points the other way — editor as default, flag as bypass — and an operator used to `git commit` may expect `safix set <name>` with no stdin to open an editor. It does not; it prompts, as it does today. The `set` prompt's existence is what makes the git analogy inapplicable, since there is already a non-editor interactive path and changing what it does would be the breaking change.

Editor selection: `$VISUAL`, then `$EDITOR`, then refuse naming both.
No fallback to `vi`, `vim` or `nano`.
sops falls back; that is refused here because a fallback launches a program the operator did not choose, over their plaintext, and the most likely outcome of dropping a person who has never used `vi` into `vi` with a secret in the buffer is an accidental write or an accidental abandonment — and the operator cannot tell which happened.
Refusing names both variables and exits without staging anything.

The editor command is split on whitespace so `EDITOR="code --wait"` works, and executed directly rather than through a shell.
The staged path is an argument, which is fine and is stated: the *path* reaches argv; the *value* does not.

Outcomes:

- editor exits non-zero: nothing is written, the staging root is shredded, the refusal names the exit status.
- file unchanged: nothing is written and nothing is committed, matching the idempotent-rerun behaviour `set` already has.
- file emptied: refused, reusing the existing empty-value refusal, because an empty value is the state a truncated write leaves behind.
- the entry does not exist yet: `edit` opens an empty buffer and behaves as a `set` from there, so `edit` is usable for authoring rather than only for amendment.

## D7. `validation` stays; `validationHash` is not adopted

clan's `validationHash` answers "has the *definition* changed such that this value is stale".
safix's `validation` answers "is this *candidate value* acceptable before it is written".

They are not alternatives and neither subsumes the other.
`types.nix` already records why safix has the second and not the first: safix writes into git, and a value committed is a value distributed, so the failure to prevent is a bad value reaching a committed file rather than a stale one persisting.

`validationHash` becomes relevant exactly once — when the bridge imports a value into clan and clan's next `vars generate` must not immediately overwrite it, which is the case `clan_lib/vars/import_vars.py` handles by restoring the hash alongside the value.
That is `clan-bridge`'s problem and is stated there, not here.

What does change here: `$out_name` in a validation fragment becomes the name of the output currently under judgement passed the same way it is today, and the candidate arrives on standard input as it does today — validation keeps its pipe, because nothing about the clan contract requires otherwise.

## Open question for the operator

clan runs generators inside a sandbox by default (`sandbox_cmd` with the staging root as the only writable path) and refuses when sandboxing is unavailable unless `--no-sandbox` is passed.
safix 0.1 runs the fragment with the caller's filesystem and network and says so explicitly in `types.nix`.

Adopting clan's sandbox would be a second material change to what a generator may do, would break existing generators that reach the network (an ACME or API-token generator, for instance), and is separable from the interface change.
It is **not** in this change, and the question is whether it should be its own 0.2 change or is deliberately out of scope for safix.

Answered by the operator: its own change rather than out of scope, and opened immediately as `adopt-generator-sandbox`.
The operator's position is that generators behave securely by default.
Two facts make now the cheap moment: the fleet declares no network-reaching generator today, so the default changes while breaking nothing, and the interop this change establishes is incomplete without the envelope — clan runs the shared fragment interface sandboxed by default, so a fragment written against safix's open executor does not in fact run under the other system unmodified.
Any escape rides on the generator's declaration rather than on the invocation, for D6's reason; the shape itself is `adopt-generator-sandbox`'s to design.
