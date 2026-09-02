# Design: composite-key unlock for the operator's password database

Every `keepassxc-cli` claim below was run in this session against `keepassxc-cli` 2.7.12, the same version `crates/safix-core/src/store.rs:31` already pins its own measurements to.
Commands and their exact output are reproduced where the wording itself is the evidence.

## Context

`store.rs` reaches the database through exactly five argument-vector constructors: `read_arguments`, `write_arguments`, `group_arguments`, and `listing_arguments` in that file, plus `keepassxc_arguments` in `crates/safix-core/src/enroll/custody.rs`, which builds the entry `safix enroll --mirror-to-store` writes a card's PIN and PUK to.
All five open the same class of thing — an operator's kdbx file, addressed by path, with a password fed on standard input — and none of them accepts anything beyond `--quiet`, the subcommand's own flags, the database path, and (for the entry-addressing three) an entry or group path.
The password is the only unlock factor safix has ever asked for, and it is asked for once per run through `DatabasePassword::database_password` (`custody.rs:113-119`), never through the store's own command.

The gap this closes was named, not discovered.
`openspec/changes/archive/2026-08-18-add-keepassxc-sync/design.md:94` recorded it as a deliberate deferral at the time `sync` was built: "the database open could take the store's other key factors — `-y slot[:serial]`, a key file — through a declaration field, should a prompt-free flow be wanted. Every db-opening verb already accepts them."
That sentence is a measured fact and it still holds:

```
$ keepassxc-cli open --help
Usage: keepassxc-cli open [options] database
  -k, --key-file <path>          Key file of the database.
  -y, --yubikey <slot[:serial]>  Yubikey slot and optional serial used to
                                 access the database (e.g., 1:7370001).
```

The same two flags, worded identically, are present in `show`, `edit`, `add`, `mkdir`, and `ls` — the exact five subcommands `store.rs` and `custody.rs` already invoke.

## Goals / Non-Goals

**Goals:**

Declare a YubiKey challenge-response slot, a key file, or both, once per database, alongside the existing `database` and `group` options.
Thread those factors as additional arguments into every keepassxc-cli invocation safix makes against that database, so `sync` and `enroll --mirror-to-store` open it the same way a person at a terminal would.
Keep the single password prompt exactly as it is: asked once per run, on the same path, never replaced or made conditional on the declared factors.
State plainly, with measured evidence, what a wrong or absent factor looks like next to a wrong password, rather than promising a distinction safix cannot actually make.

**Non-Goals:**

No new sync mode, no change to convergence, deletion, or the report — those are `keepassxc-sync`'s other requirements and this change does not touch them.
No per-mapping key factor: the composite key is a property of the database file itself, at the same granularity `database` and `group` already sit at, not of an individual mapped entry.
No change to `enroll`'s CLI flag surface (`--serial`, `--slot`, `--store-database`, …); the declared factors are read off the existing `flake.safix.keepassxc` declaration, not added as new flags to a command that already has enough of them.
No attempt to make a wrong composite-key factor distinguishable from a wrong password beyond what the vendored command's own stderr already gives for free — see D7.

## Decisions

### D1. The composite key is declared at database granularity, not per mapping

`flake.safix.keepassxc.yubikey` and `flake.safix.keepassxc.keyFile` sit beside `database` and `group` in `options.nix`, not inside `keepassxc.nix`'s `mapping` submodule.
A kdbx file's composite key is one recipe for the whole file — KeePassXC does not let one entry require a YubiKey and another not — so a per-mapping field would let a declaration say something the database itself cannot be true of, and every mapping would have to repeat the same value or be silently ignored.
`keepassxc.nix` itself needs no change: it defines the mapping shape and the refusals local to a mapping's own declaration, and neither is about how the file that holds the mapped entries opens.

### D2. `keyFile` is a string, and the reason is not the reason `database` is one

`modules/flake/safix/options.nix:277-281` already explains why `database` is a string: a nix path would copy a 292 MB encrypted file into the world-readable store on every evaluation.
That reasoning does not apply to a key file, which is typically a few kilobytes, and copying it into the store would not be expensive — it would be wrong for a different reason.
A key file is not the encrypted thing; it is one of the secrets the encryption depends on, structurally the same role `age.keyFile` plays for sops-nix, which the shared program contract's own upstream measurement records as `nullOr pathNotInStore` (`modules/sops/default.nix:338`) specifically so a private key never lands in the store.
`keyFile` is declared `nullOr str` for that reason: a string naming an absolute path on the machine the verb runs on, exactly as `database` is, but because the value it names must stay out of a world-readable place, not because it is large.

### D3. `yubikey` is a two-field submodule, not a pre-formatted `slot[:serial]` string

`keepassxc-cli`'s own flag takes one string, `slot[:serial]`, and it would be the smallest possible option to mirror that directly.
It is rejected because it makes the declaration respect the command line's own punctuation rather than nix's: a consumer would have to know that a bare `"1"` means slot 1 with no serial and that `"1:12345678"` adds one, a format nowhere else in this option surface asks a consumer to hand-assemble.
`yubikey = { slot; serial = null; }` states the same two facts as two fields, and the argument-vector constructors — which already have to know keepassxc-cli's exact flag spelling — are the right place to join them back into `slot[:serial]` when `serial` is set and `slot` alone when it is not.

### D4. All five constructors gain the factors, because they open the same store by two different routes

`store.rs`'s four constructors and `custody.rs`'s `keepassxc_arguments` do not share code today, and this change does not merge them — `store.rs`'s header already explains why the two modules keep separate transports (one addresses `<group>/<path>` a mapping declares, the other addresses an attribute-named entry safix itself owns).
What they do share is the database file, and that file has exactly one composite key regardless of which safix code path is asking it to open.
`enroll --mirror-to-store` does not read `flake.safix.keepassxc.database` today — its own database comes from the `--store-database` flag (`crates/safix/src/main.rs:729-733`), named by hand at the terminal, independently of the nix declaration `sync` reads.
That asymmetry is pre-existing and this change does not fix it; it does mean the declared `yubikey`/`keyFile` factors are read off `workspace.keepassxc()` and applied to whichever database `enroll --mirror-to-store` was pointed at, on the assumption that a fleet declaring composite-key factors for its password database and also using `--mirror-to-store` is pointing both at the same file, which is the only configuration this pairing makes sense in.
If an operator ever named a genuinely different database on that flag, the factors would simply fail to open it — the fail-closed direction, not a silent bypass — and that is recorded once, here, rather than solved.

### D5. Reading the slot is the whole of what this does, and it is the opposite of what enroll refuses

Three passages already state, independently, why safix never programs, reprograms, or deletes a hardware slot: `crates/safix/src/usage.rs:205-208` ("no hardware slot is touched, under any flag... Writing a challenge-response slot is what would end a database permanently"), `crates/safix-core/src/error/prose.rs:424-438` (`OTP_REFUSED`, "A programmed challenge-response slot is what opens a password database, and the database has no record of the secret it was built with. Writing that slot replaces the factor and the database stops opening — permanently"), and `README.md:686-689` ("No OTP slot is written under any flag").
Every one of those three sentences is about writing.
None of them says the slot may not be read, and reading is the only operation `-y` performs: it is documented, and was measured in this session, to issue a hardware challenge and use the response to unlock — nothing in `keepassxc-cli`'s own flag vocabulary for `open`, `show`, `edit`, `add`, `mkdir`, or `ls` programs a slot; the tool's slot-programming operations (`add-yubikey` in the KeePassXC GUI, `ykman`'s own `otp` subcommands) are different verbs safix never invokes for this feature or any other.
So this change adds a reader for the same slot number space `OTP_REFUSED` already guarantees safix's own `enroll` verb will never write — not a similar-sounding guarantee, the identical slot, on the identical card, in the identical fleet this package was extracted from, where `prose.rs:421-422` already names it: "the fleet's password database is opened by a challenge-response secret on OTP slot 2 of both keys."
Reading that slot to open the very database it opens is the mechanism this change exists to reach; writing it is what would end that database, and nothing here does that.

### D6. `SAFIX_KEEPASSXC_CLI` stays a test-only override, not a composite-key delivery mechanism

`crates/safix-core/src/enroll/custody.rs:43` and `:327-329` define `SAFIX_KEEPASSXC_CLI`, which redirects every keepassxc-cli invocation to a named program — in the test suite, a stub (`crates/safix/tests/harness/mod.rs:2131-2140` asserts every `sync` run sets it, so a run never reaches a real database on a developer's machine).
A wrapper script pointed at by that variable could in principle splice `-y 2` into whatever argv it receives and hand it to the real binary, which would look like a way to get composite-key unlock without touching the option surface at all.
It is rejected as the mechanism rather than merely left alone, for a reason specific to this codebase's shape: the override sits behind five constructors that build five structurally different argument vectors — `open`, `show --attributes Password --show-protected`, `edit`/`add --password-prompt [--username u]`, `mkdir`, `ls -R -f` — and a wrapper splicing in a flag would have to parse each one positionally to know where its own flag belongs, reimplementing the very dispatch `store.rs` and `custody.rs` already do in typed Rust, in shell, keyed to argv shapes that change independently of any interface the wrapper could depend on.
It would also do it silently: the fixture that proves a real database is never reached (`refuse_a_real_database`, `harness/mod.rs:2126-2154`) checks that the override names the stub and that the declared database is under the fixture's scratch directory; it says nothing about what a wrapper does to the argv passing through it, so a wrapper-based composite key would be invisible to the one check built to keep this feature's own tests honest.

### D7. What a wrong composite-key factor looks like, next to a wrong password

`crates/safix-core/src/error/prose.rs:623-631` already states the shape this question has always had: "A wrong password and an unreadable file present the same way here, and the store's own message below is what tells them apart."
That sentence was written for a password-only world with two causes; this change adds two more causes to the same `Error::DatabaseUnreadable` path, and whether the store's own message actually tells them apart was measured, not assumed, against a scratch password-only database created in this session:

```
$ printf 'wrongpass\n' | keepassxc-cli ls -q test.kdbx
(exit 1, no output on stdout or stderr)

$ printf 'testpass\n' | keepassxc-cli ls -q -k /nonexistent.key test.kdbx
(exit 1, no output on stdout or stderr)

$ printf 'testpass\n' | keepassxc-cli ls -q -y 1:99999999 test.kdbx
Failed to issue challenge:  "Could not find hardware key with serial number 99999999. Please connect it to continue."
(exit 1)
```

Three findings follow, and none of them is a safix decision — all three are the vendored tool's own behaviour, forwarded unchanged the way `store_command_failed` and `database_unreadable` already forward every other refusal:

A wrong password and a key file that will not load are indistinguishable from each other under `--quiet`, both producing an empty `output`.
That is not new: a wrong password and a corrupt database file were already indistinguishable before this change, for the same reason — `--quiet` silences the tool's own explanatory text for both, and safix has never parsed the store's exit status further than "zero or not."

A wrong or absent YubiKey slot is not silenced by `--quiet`.
The challenge-issue failure text reaches stderr regardless of the flag, so `Error::DatabaseUnreadable`'s `output` field carries it, and `database_unreadable`'s existing prose — "the store's own message below is what tells them apart" — becomes true for this case specifically, where it was already slightly overstated for the password-only case it was originally written for.

Safix adds no new `Error` variant, no new refusal, and no parsing of the store's stderr to produce this.
The distinction an operator sees is exactly as reliable as `keepassxc-cli`'s own choice of which failures `--quiet` silences, which is outside this package's control and could change in a future release; this design states what is true of 2.7.12 today rather than promising it will stay true.

## Risks / Trade-offs

[A wrong password and a wrong or missing key file read identically under `--quiet`] → both were already indistinguishable from a corrupt database file before this change; named in D7, not solved, because solving it means dropping `--quiet` for every keepassxc-cli invocation safix makes and inheriting whatever interactive-prompt text that reintroduces on the success path, which is a larger and unrelated change to the transport this capability does not need.

[`enroll --mirror-to-store`'s `--store-database` can legitimately name a database other than the one `flake.safix.keepassxc.database` declares, yet receives the same declared factors] → a mismatched factor fails closed with the store's own refusal — never a silent unlock with the wrong recipe — which is the same posture every other refusal in this package already takes; named in D4.

[A consumer declares a YubiKey factor and the card is never present on the machine `sync` runs on] → every run refuses identically to a wrong password, indefinitely, until the declaration is removed or the card is connected; this is the correct behaviour for a factor that is genuinely required, not a defect to work around.

## Migration Plan

Additive only: `yubikey` and `keyFile` both default to `null`, so every existing declaration evaluates and runs exactly as it does today.
Adoption is naming one or both options on a database that already requires them; there is no state to migrate, because no run has ever succeeded against such a database before this change.
Rollback is removing the two declarations, which returns a database to password-only unlock — assuming the operator's own database still opens on the password alone, which is a fact about the database, not about safix.
