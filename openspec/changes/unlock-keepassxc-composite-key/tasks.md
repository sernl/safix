# Tasks: unlock-keepassxc-composite-key

Every `keepassxc-cli` behaviour cited in `design.md` was measured against version 2.7.12 in-session; no task below re-derives it, and no fixture in this change opens a real password database — every argv assertion is against a literal string, following `modules/flake/checks/keepassxc.nix`'s own existing discipline of never depending on a database being in reach.

## 1. The declaration

- [x] 1.1 Add `flake.safix.keepassxc.yubikey` to `modules/flake/safix/options.nix`, a `nullOr` submodule with `slot` (`str`) and `serial` (`nullOr str`, default `null`), defaulting to `null`, documented per D1/D3 of `design.md`; verify with `nix eval .#flake.safix.keepassxc.yubikey --apply builtins.typeOf` (or the fixture-driven equivalent used elsewhere in this file) evaluating to a type rather than throwing
- [x] 1.2 Add `flake.safix.keepassxc.keyFile` to the same file, `nullOr str`, defaulting to `null`, documented per D2 of `design.md` — a string for a different reason than `database` is one, and the description states that reason rather than pointing at `database`'s
- [x] 1.3 Project both fields unchanged in `modules/flake/safix/default.nix:296-299`, alongside `database` and `group`; verify by evaluating `flake.safix.lib.keepassxc` over a fixture declaring both options and asserting the projected attribute set carries them byte-for-byte
- [x] 1.4 Verify: `nix eval` (or the portability check already proving `mkVault`'s mechanism) confirms a fixture declaring neither option still evaluates `flake.safix.keepassxc.yubikey` and `.keyFile` as `null`, so every existing declaration is unaffected

## 2. The nix check

- [x] 2.1 In `modules/flake/checks/keepassxc.nix`, extend the fixture builder to accept `yubikey` and `keyFile`, and add a case asserting a declared `{ slot = "1"; serial = "12345678"; }` and a declared key-file path both reach the projection unchanged
- [x] 2.2 Assert that declaring `keyFile` as a nix path (rather than a string) is a type error at evaluation, mirroring however `database`'s string-not-path constraint is or would be asserted in this file, so the two options carry the same guarantee by the same mechanism
- [x] 2.3 Severity drill: loosening `keyFile`'s type to `nullOr (either str path)` turns 2.2 red
- [x] 2.4 Verify: `nix build .#checks.x86_64-linux.safix-keepassxc` (the actual check name; this file's suite is not `safix-keepassxc-mirror`) green, and the drill in 2.3 observed red before the type is restored

## 3. The Rust model

- [ ] 3.1 Add `pub struct Yubikey { pub slot: String, pub serial: Option<String> }` to `crates/safix-core/src/model.rs`, `#[derive(Debug, Clone, Deserialize)]` with `#[serde(deny_unknown_fields)]`, beside `KdbxSide` and `SyncMapping`
- [ ] 3.2 Add `yubikey: Option<Yubikey>` and `key_file: Option<String>` fields to `Keepassxc` (`model.rs:827-835`), keeping `#[serde(deny_unknown_fields)]` in force
- [ ] 3.3 Extend the existing `Keepassxc`/`SyncMapping` deserialize test (or add one beside it) asserting a literal JSON payload naming both fields deserializes field-for-field, and a payload naming neither deserializes with both `None`, matching the measured-facts discipline the shared program contract already applies to the other seven struct shapes
- [ ] 3.4 Verify: `cargo test -p safix-core model::` green

## 4. The four sync argument-vector constructors

- [ ] 4.1 Add a private helper in `store.rs` that appends `-y <slot[:serial]>` when a `Yubikey` is given and `-k <path>` when a key file is given, formatting `slot[:serial]` per D3 of `design.md` (bare slot when `serial` is `None`, `slot:serial` otherwise)
- [ ] 4.2 Extend `read_arguments`, `write_arguments`, `group_arguments`, and `listing_arguments` (`store.rs:333-427`) to take `yubikey: Option<&Yubikey>, key_file: Option<&str>` and splice the helper's output in after `--quiet` in each
- [ ] 4.3 Extend `Database` (`store.rs:98-107`) to hold the two factors, populate them from the `Keepassxc` the caller already has at `Database::open` (`store.rs:121-141`), and pass them at every call site of the four constructors (`store.rs:186`, `233`ish, `408`, `419` as renumbered)
- [ ] 4.4 Extend `no_argument_vector_can_carry_a_value` (`store.rs:459-481`) with cases carrying a `Yubikey` and a key file, asserting the forbidden list (`--password=`, `--value`, `--generate`) still never appears, and that the *file's contents* never appear (there is nothing to assert of the contents in an argv test other than that the value passed to the helper is a path, not a `Secret`, which the type signature from 4.1 already enforces at compile time — record that as the reason no runtime assertion is added for it)
- [ ] 4.5 Extend `a_write_adds_what_is_absent_and_edits_what_is_there`, `a_username_reaches_argv_and_its_absence_leaves_the_field_alone`, `the_read_asks_for_the_protected_password_attribute_and_nothing_else`, and `the_listing_is_recursive_and_flat` (`store.rs:483-519`) with a factor-bearing case each, pinning the new literal alongside the old
- [ ] 4.6 Severity drill: reverting 4.2's splice on any one of the four constructors turns that constructor's extended test in 4.4/4.5 red while leaving the other three green, which is the evidence the four are independent rather than sharing one code path that happens to pass
- [ ] 4.7 Verify: `cargo test -p safix-core store::tests` green, and the drill in 4.6 observed red on each of the four constructors in turn before the splice is restored

## 5. The enrollment mirror

- [ ] 5.1 Extend `keepassxc_arguments` (`custody.rs:186-193`) with the same two parameters and the same helper from 4.1 (moved to a shared location both modules can reach, or duplicated with a comment cross-referencing the other copy — pick one and record which and why in the commit)
- [ ] 5.2 Extend `Transport::PasswordStore` (`custody.rs:63-66`) to carry `yubikey: Option<Yubikey>` and `key_file: Option<String>`, and thread them through `choose` (`custody.rs:89-106`) and `write` (`custody.rs:202-231`) to the extended `keepassxc_arguments`
- [ ] 5.3 In `crates/safix/src/main.rs`'s `enroll_command`, after `Workspace::discover()` (`main.rs:762`), read `workspace.keepassxc()` and populate the `Transport::PasswordStore` factors from its `yubikey`/`key_file` fields whenever `--store-database` was named, recording D4 of `design.md`'s assumption (same file, different route) as a comment at the call site
- [ ] 5.4 Extend `custody.rs`'s existing `keepassxc_arguments`/`write` tests (around `custody.rs:397-400`) with a factor-bearing case pinning the new literal
- [ ] 5.5 Extend `crates/safix/tests/support/card-stubs.rs`'s `keepassxc` stub dispatcher (`card-stubs.rs:373-375`) to match argv carrying the new flags, so a fixture exercising `--mirror-to-store` with declared factors runs against the stub rather than falling through to an unmatched-argv panic
- [ ] 5.6 Verify: `cargo test -p safix-core enroll::custody::` and the relevant `crates/safix/tests/enrollment.rs` cases green

## 6. Documentation

- [ ] 6.1 Add one sentence to `crates/safix/src/usage.rs`'s `sync` prose block (`usage.rs:194-208`) stating that a database may additionally declare a YubiKey slot and/or a key file, opened alongside the one password prompt, and reusing this file's own "it manages no keyring" heading to state that reading the slot is not the thing that heading forbids
- [ ] 6.2 Add one sentence to `README.md:686-689`'s "three things are refused" paragraph distinguishing programming a slot (refused, unconditionally, by `enroll`) from reading one to unlock a database (what this change does), so the two nearby claims about the same slot number space do not read as contradictory to someone who has not read `design.md`
- [ ] 6.3 Verify: `rg -n 'yubikey|key.file' README.md crates/safix/src/usage.rs` shows the new sentences, and every guarantee either states names a check in groups 1-5 that holds it

## 7. Verification

- [ ] 7.1 `openspec validate unlock-keepassxc-composite-key --strict`
- [ ] 7.2 `openspec validate --all --strict`
- [ ] 7.3 `cargo test` (whole workspace) green, confirming no other suite regressed from the `Keepassxc`/`Database`/`Transport::PasswordStore` shape changes
- [ ] 7.4 `nix flake check` green
