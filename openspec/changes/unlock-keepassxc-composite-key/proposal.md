# keepassxc's own password database gains composite-key unlock

Read against `keepassxc-cli` 2.7.12, the version this package's own `store.rs` measurements are pinned to and the version installed on this workstation; every claim about the tool's flags and messages below was run against it in this session, not assumed.

## Why

`flake.safix.keepassxc.database` opens on a password alone.
A database whose own composite key additionally requires a YubiKey challenge-response slot, a key file, or both cannot be reached by `safix sync` or by `safix enroll --mirror-to-store` at all: safix has no declaration for either factor and no argument vector that carries them, so the store's own command refuses every invocation before the password is even wrong.

This was not an oversight; it was named and deferred on purpose.
`openspec/changes/archive/2026-08-18-add-keepassxc-sync/design.md:94` records it: "the database open could take the store's other key factors — `-y slot[:serial]`, a key file — through a declaration field, should a prompt-free flow be wanted. Every db-opening verb already accepts them. It changes nothing normative here."
That is still true today, and this change builds exactly what was recorded and nothing else.

## What Changes

- `flake.safix.keepassxc` gains two new database-level options, `yubikey` (`{ slot; serial; }`, serial optional) and `keyFile` (a string, never a nix path), both defaulting to none.
- Every keepassxc-cli invocation safix issues against that database — `sync`'s read, write, group-creation, and listing, and `enroll --mirror-to-store`'s write — carries the declared factors as additional command-line flags (`-y slot[:serial]`, `-k <path>`), alongside the single password prompt those commands already ask for. The password prompt is unchanged: it is asked once per run, on the same terminal or stdin path as today, never replaced or skipped by a declared factor.
- A database that will not open on its declared factors is refused the same way any other unreadable database already is, naming the database. Safix makes no attempt to say which factor was at fault; whether that distinction is even visible depends on the store's own stderr, which `design.md` records with the exact measured behaviour.
- Nothing about slot programming changes. The slot a declared YubiKey factor names is read, once, to answer a challenge that unlocks the database; safix issues no command anywhere that creates, reprograms, or deletes it. That is a factual observation about the existing OTP refusal (`crates/safix-core/src/error/prose.rs:424-438`, `crates/safix/src/usage.rs:205-208`, `README.md:686-689`), not a relaxation of it — see `design.md` for the citation-by-citation argument.

Not in scope, and deliberately so: sync modes, deletion propagation, report shape, per-mapping key factors (the composite key is a property of the database, at the same granularity `database` and `group` already are), and any change to `enroll`'s own CLI surface beyond reading the declared factors it already has access to.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `keepassxc-sync`: the requirement "The database is a store being written, never a keyring being managed" is amended to distinguish reading a hardware slot to unlock (permitted, and now exercised) from programming, reprogramming, or deleting one (still forbidden absolutely), and to state which values may travel the store's own argument vector now that two more public identifiers — a slot/serial pair and a key-file path — join the database path there. A new requirement is added for the composite-key declaration itself and where it applies. No other requirement in the capability changes: modes, the two-way memory, and the report are untouched.

## Impact

Affected code:

- `modules/flake/safix/options.nix:268-342` — two new options under `flake.safix.keepassxc`.
- `modules/flake/safix/default.nix:296-299` — the projection carries the two new fields through unchanged.
- `crates/safix-core/src/model.rs:824-835` — `Keepassxc` gains `yubikey: Option<Yubikey>` and `key_file: Option<String>`; a new `Yubikey { slot, serial }` struct.
- `crates/safix-core/src/store.rs:333-519` — the four argv constructors (`read_arguments`, `write_arguments`, `group_arguments`, `listing_arguments`) and `Database` gain the two factors; the argv-pinning tests in the same file change with them.
- `crates/safix-core/src/enroll/custody.rs:43, 186-231, 327-329` — the enrollment mirror's own argv constructor and its `Transport::PasswordStore` variant gain the same two factors, because it opens the same operator password store by a different route (a CLI flag rather than a nix declaration).
- `crates/safix/src/main.rs` (`enroll_command`) — reads the declared factors off `workspace.keepassxc()` alongside the existing `--store-database` flag.
- `modules/flake/checks/keepassxc.nix` — gains fixtures asserting the two new options round-trip into the projection unchanged, and that `keyFile` rejects a nix path the way `database` already does.
- `crates/safix/src/usage.rs:194-218`, `README.md:686-698` — the prose describing the single password prompt and the "no hardware slot touched" guarantee gains one sentence each stating what now travels alongside the prompt and why reading a slot is not the thing those sentences forbid.

This stays inside the option modules under `modules/flake/safix/`, which is the only surface `flake.safix.keepassxc` is declared on today; nothing here introduces a `perSystem` or any other flake-parts-specific facility, so the option surface stays producible without flake-parts, unchanged from before this change (D1 of the shared program contract; `modules/flake/checks/namespace.nix` already forbids anything else under that directory).

No change to `crates/safix/tests/harness/mod.rs`'s `SAFIX_KEEPASSXC_CLI` override or to `card-stubs.rs`'s stub dispatcher beyond recognising the two new flags in the argv it already matches on — `design.md` records why a wrapper script pointed at by that variable was considered and rejected as the mechanism itself, rather than merely as untouched.
