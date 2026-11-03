# Proposal

## Why

Three gaps in `check` and `fix` were found by the 2026-11 upstream survey (`docs/notes/research/upstream-survey-2026-11.md`, agenix §3.2, agenix-rekey §3.3 and §3.4).

A derived value goes stale silently.
`set`, and so `edit` and `sync`, move the stamp of the value they write (`crates/safix-core/src/set.rs:217`) but know nothing about the generators that read it, and `check` reports only a changed definition or a passed deadline (`crates/safix-core/src/check.rs:177,195`).
A hash of a password changed by hand is therefore a value derived from a retired input, which is the exact state the rotation cascade exists to prevent (`crates/safix-core/src/generate.rs:280-287`), and nothing reports it.

The raw-age roster claim is wrong for SSH recipients.
`check` reports every raw-age file as `RecipientRosterUnverifiable` (`crates/safix-core/src/check.rs:348-353`), and `docs/guides/inspectable-recipients.md:15` says the file "records nothing a reader can attribute" and "discloses no roster".
age writes a four-byte SHA-256 tag of the recipient's SSH public key into every `ssh-ed25519` and `ssh-rsa` stanza, and agenix 0.18.0's `--check` recomputes and compares it (`~/ghq/github.com/ryantm/agenix/pkgs/agenix.sh:292-300` `ssh_tag`, `:302-403` `check_file`, at 5e5b88a).

`fix --yes` rewrites every raw-age file on every run.
`rewrap_age` (`crates/safix-core/src/fix.rs:167-231`) decrypts, re-encrypts and renames each file unconditionally, and age output is randomised, so every run changes every `.age` file in git whether or not its audience moved.

## What Changes

- `check` gains `DerivedValueStale`: a generated value one of whose dependencies has an `updated` stamp later than the value's own.
  It decrypts nothing, fails the run like every existing finding, and names `safix generate --regenerate <user> <dependent>` as the remedy.
- `check` reads a raw-age file's header — binary or ASCII-armored — without decrypting it, and recomputes the SSH tag of every SSH recipient the declared audience names.
  - A missing, extra or duplicated SSH stanza is `RecipientTagDrift`, which fails the run and names `safix fix --yes`.
  - A file whose stanzas are all SSH and match the declared SSH recipients exactly is `RecipientRosterTagMatched`, a note that does not fail the run and states that a 32-bit tag match does not establish the keys.
  - A file holding any non-SSH stanza, or whose audience declares any non-SSH recipient, is still `RecipientRosterUnverifiable`.
  - A raw-age file whose header cannot be read refuses with `safix::recipients_unreadable` caused by the new `safix::age_header_unreadable`, as an unreadable SOPS document does today.
- `check`'s exit status is non-zero when at least one finding fails, rather than when there is any finding at all; `RecipientRosterTagMatched` is the only finding that does not fail.
- Each declared raw-age entry gains an audience record under the generator-record root, `safix-age-audience-v1 <audience digest> <ciphertext digest>`, whose path the resolver emits on the placement as `audienceRecord` (and `logicalAudience` in vault mode).
  - `fix` writes it after each raw-age re-wrap and skips a file whose record matches both the declared audience and the file's current bytes, with or without `--yes`.
  - `check` reports nothing for an absent or unreadable record, and reports `AgeAudienceChanged` when the record matches the file's bytes and names an audience other than the declared one.
  - A vault migration and rollback move the record with the value.
- The raw-age sentences in `docs/guides/inspectable-recipients.md`, the `check` and `fix` sections of `docs/reference/cli.md`, the rotation-cascade paragraph of `docs/guides/generators.md`, `docs/concepts/storage-layout.md` and `docs/reference/refusals.md` are corrected and extended.

No declaration changes.
A tree that runs `check` today sees new findings only where a dependency was written after its dependent, or where a raw-age file's SSH stanzas disagree with the declarations; an all-SSH raw-age file that agrees moves from a failing finding to a note.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `safix-cli`: `check` gains three findings and one note, and its exit status follows failing findings only.
- `recipient-policy`: the raw-age roster sentence is narrowed to what age actually hides, and an unchanged raw-age audience, recorded beside the definition records, is no longer rewritten by `fix`.
- `secret-generators`: the dependency requirement states that a dependency written outside a cascade is reported.
- `secret-custody`: the shared-carrier scenario distinguishes SSH-tag evidence from unverifiable raw-age recipients.

## Impact

- `crates/safix-core/src/check.rs`: three new `Finding` variants, one note variant, `Finding::fails`, the raw-age branch of `recipients`, and the staleness and audience-record questions in `entry_records`.
- New modules `crates/safix-core/src/age_header.rs` (header and armor parsing, SSH tag), `crates/safix-core/src/base64.rs` (moved out of `crates/safix-core/src/bitwarden.rs:1151-1174`, plus a decoder) and `crates/safix-core/src/age_audience.rs` (the record).
- `crates/safix-core/src/digest.rs`: a raw-bytes `sha256` beside `sha256_hex`, and its module note's caller list.
- `crates/safix-core/src/fix.rs`: `rewrap_age` reads and writes the record; `relocate` moves it.
- `crates/safix-core/src/relocation.rs`: `audience_leaves`.
- `crates/safix-core/src/model.rs`: `Placement::audience_record` and `Placement::logical_audience`.
- `crates/safix-core/src/error/`: `Error::AgeHeaderUnreadable` and its code.
- `modules/flake/safix/resolve.nix`: `audienceRecord` and `logicalAudience`; `modules/flake/checks/storage.nix` and `modules/flake/checks/vault.nix` rows for them.
- `crates/safix/src/render.rs` and `crates/safix/src/main.rs`: the paragraphs and the exit status.
- `crates/safix/tests/generators.rs`, a new `crates/safix/tests/raw_age_roster.rs`, and their registrations in `modules/flake/checks/cli.nix`.
- No new dependencies: SHA-256 is `crates/safix-core/src/digest.rs`, and base64 is written out for the reason that module gives.
