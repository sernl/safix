# Design

## Context

See proposal.md for why.
Three facts about the current code shape the approach.

- Every `check` question is answered without decryption, from the declarations, the ciphertext's structure and plaintext records under the generator-record root (`crates/safix-core/src/check.rs:1-13`).
  The stamp record (`crates/safix-core/src/stamps.rs`) already dates every write: `set` moves `updated` when the value changed (`crates/safix-core/src/set.rs:208-219`), and `generate` moves every output's stamp to one instant per generator run (`crates/safix-core/src/generate.rs:998-1005`).
- Every finding currently fails the run: `check_command` exits 1 when the finding list is non-empty (`crates/safix/src/main.rs:648-654`), and `render::report` counts every finding in the closing line (`crates/safix/src/render.rs:27-41`).
- Every record path is emitted by the resolver on the placement and never reconstructed by the runtime (`modules/flake/safix/resolve.nix:945-983`, `crates/safix-core/src/definition.rs:129-150`), and vault relocation enumerates each record kind through its `logical*` field (`crates/safix-core/src/relocation.rs:87-124`).

The upstream evidence is the survey's agenix §3.2 and agenix-rekey §3.3 and §3.4 (`docs/notes/research/upstream-survey-2026-11.md:457-466,603-611`).
The age facts below were read from age at `b74dce4` (`~/ghq/github.com/FiloSottile/age`) and agenix at `5e5b88a` (`~/ghq/github.com/ryantm/agenix`).

## Goals / Non-Goals

**Goals:**

- Report a derived value whose input was written after it, from stamps alone.
- Extract every bit of roster evidence a raw-age header discloses without decryption, and say exactly how strong it is.
- Make `fix --yes` idempotent on raw-age files whose audience has not moved.

**Non-Goals:**

- Regenerating anything from `check` or `fix`; both stay as they are on decisions (`crates/safix-core/src/fix.rs:3-7`).
- Verifying X25519, hybrid post-quantum (`mlkem768x25519`), plugin (`piv-p256` and others) or `scrypt` stanzas.
  An X25519 stanza carries only an ephemeral share; a plugin stanza's tag scheme is the plugin's and is not read here.
- An audience record for files named only through `extraGovernedFiles`: they have no placement, so no resolver-emitted path, and keep today's re-wrap-every-run behaviour.
- Changing `set`, `edit`, `generate` or `sync` to write the audience record; see D9.
- Relocating a raw age file itself into or out of a vault.
  Document relocation still goes through sops in `relocate_document` (`crates/safix-core/src/fix.rs`), which reads keyed documents; only the raw age entry's audience record moves with `audience_leaves`, and the vault migration test fixtures the record alone.

## Decisions

### D1. The staleness finding is `DerivedValueStale`, and it fails the run

`Finding::DerivedValueStale { user, generator, dependency, record }`, where `generator` is the entry the generator is declared on and `record` is the dependency's stamp record path.
It fails the run like `DefinitionDrift` and `RotationDue`: it describes a tree that disagrees with what the declarations promise, and a CI gate on `check` exists to catch exactly that.

The question is asked inside `entry_records` (`crates/safix-core/src/check.rs:665-725`), the walk that already reads the definition and stamp records per entry, so the report keeps one order: for each user, for each held entry carrying a generator, after that entry's `RotationDue` and `DefinitionDrift`.

### D2. The remedy names the dependent, never the dependency

The survey proposed `safix generate --regenerate <dependency>`.
That command is wrong in both cases it can meet.
`generate` resolves the named entry to the generator that writes it (`crates/safix-core/src/generate.rs:161-175`), so for a dependency set by hand — the case this finding exists for — it refuses `safix::no_generator`, and for a generated dependency it re-mints the very value the operator just changed and discards it.
`safix generate --regenerate <user> <dependent>` re-runs the dependent's generator and, through `UserPlan::cascade` (`crates/safix-core/src/model.rs:530-562`), everything derived from it, against the dependency as it now stands.
The render therefore prints `safix generate --regenerate {user} {generator}`, where `generator` is the entry the dependent generator is declared on.

### D3. What is compared

For one holder and one generator: the reference is the earliest `updated` among the stamp records of every value that generator writes (`Placements::producer_of` read in reverse: every held entry whose producer is this generator) that has a stamp record.
The earliest, because a generated output re-set by hand moves its own stamp forward, and the oldest output still standing is the one derived from the oldest inputs.
For each name in the generator's `dependencies` that the holder holds, the dependency's `updated` is read from its own placement's stamp record.
A finding is reported when the dependency's `updated` is strictly greater than the reference.

Strictly, because a cascade stamps the dependency and then the dependent from two `stamps::now()` calls that may fall in the same second.
A dependency or a generator with no stamp record makes no claim, by the rule `crates/safix-core/src/stamps.rs:35-41` states for absent stamps; a stamp record that does not parse refuses as it already does (`Error::StampRecordUnparsable`).

One finding per (dependent stamp record, dependency stamp record) pair, deduplicated the way `DefinitionDrift` is deduplicated by record path (`crates/safix-core/src/check.rs:704`), so a shared generator held by two carriers is reported once.

The rejected alternative was the upstream one: comparing file modification times (`apps/generate.nix:120-140` in agenix-rekey), which a git checkout resets.
Comparing git commit dates was rejected for the reason `crates/safix-core/src/stamps.rs:3-8` gives: a commit dates a file, not one key inside it.

### D4. Reading an age header without decrypting

A new `crates/safix-core/src/age_header.rs` parses the header of the age v1 format into its stanzas' type and first argument.

- Binary: the first line is `age-encryption.org/v1`; each stanza starts `-> <type> <args…>` followed by body lines; the header ends at a line starting `---` (`internal/format/format.go:106-109,212` in age).
  Body lines are skipped, not decoded, because nothing here needs them.
- Armored: after optional leading whitespace, the first line is `-----BEGIN AGE ENCRYPTED FILE-----`, the body is padded standard base64 in lines of at most 64 columns, and it ends at `-----END AGE ENCRYPTED FILE-----` (`armor/armor.go:24-25,131-170` in age).
  The decoder decodes the armor, then parses the result as the binary form, which is what agenix does (`pkgs/agenix.sh:312-320`).
- Stanzas whose type ends in `-grease` are dropped, as agenix does (`pkgs/agenix.sh:359-360`).
- Anything else — a missing intro line, armor that does not decode, a header with no `---` line — is `Error::AgeHeaderUnreadable { reason }` (code `safix::age_header_unreadable`), wrapped in `Error::RecipientsUnreadable { file, cause }`, which is how an unreadable SOPS document already refuses (`crates/safix-core/src/check.rs:368-372`).

Only the header is read; the payload after `---` is never touched, and no identity is consulted.

### D5. The SSH tag

age's tag for an SSH recipient is the first four bytes of SHA-256 over the SSH wire-format public key, encoded as unpadded standard base64, six characters (`agessh/agessh.go:35-38` in age: `sha256.Sum256(pk.Marshal())`, `format.EncodeToString(h[:4])`, where `format`'s encoding is `base64.RawStdEncoding`, `internal/format/format.go:31`).
agenix computes the same thing through `ssh-keygen -lf` (`pkgs/agenix.sh:292-300`).

The wire format is exactly the base64 field of an OpenSSH public key line, so the tag of a declared recipient `ssh-ed25519 AAAA… [comment]` is the tag of the decoded second field.
The SHA-256 is `crates/safix-core/src/digest.rs`, which gains `pub(crate) fn sha256(bytes) -> [u8; 32]` with `sha256_hex` rewritten over it.
No dependency is added: the crate forbids `unsafe` and treats each new dependency as a decision (`crates/safix-core/src/digest.rs:18-28`).

Base64 is needed both ways (decode armor and key blobs, encode a tag), and `crates/safix-core/src/bitwarden.rs:1151-1174` already carries a private encoder.
It moves to a new `crates/safix-core/src/base64.rs` with a strict decoder beside it, and `bitwarden.rs` calls it, so there is one implementation.

### D6. Classifying a raw-age file

For each existing raw-age governed file, the declared audience is resolved exactly as `recipients` resolves it for a SOPS file today (`crates/safix-core/src/check.rs:360-366`, including `UngovernableExtra`).
Declared recipients are split by `crate::ciphertext::is_ssh_recipient` (`crates/safix-core/src/ciphertext.rs:634-644`) into SSH and other; an SSH recipient whose key field does not decode counts as other.
Stanzas are split into SSH (`ssh-ed25519`, `ssh-rsa`) and other.

Expected multiplicity is keyed by `(type, tag)` and counts distinct declared keys, so two declared keys whose tags collide expect two stanzas rather than reading as a duplicate.
Then:

- `missing`: declared SSH keys whose `(type, tag)` has fewer stanzas than expected.
- `extra`: stanzas whose `(type, tag)` no declared key of the audience has.
- `duplicate`: stanzas beyond the expected count for a `(type, tag)` the audience does declare.

Any of the three non-empty is `Finding::RecipientTagDrift { file, missing, extra, duplicate, narrowed, mints }`, failing, remedy `safix fix --yes`, and printed with the not-revocation sentence and the mint commands `RecipientDrift` prints (`crates/safix-core/src/check.rs:66-93`).
An extra stanza is named as the full declared key when exactly one declared SSH recipient anywhere in `workspace.recipients()` has that `(type, tag)`, and as `<type> <tag>` otherwise; that recovery is agenix's (`pkgs/agenix.sh:336-346`), and the recovered keys feed `Holders::holders_of` for `narrowed`.

Independently of drift: if any stanza or any declared recipient is non-SSH, `RecipientRosterUnverifiable { file }` is reported as today.
If neither drift nor a non-SSH element exists, `Finding::RecipientRosterTagMatched { file, matched }` is reported.
A non-existent raw-age file produces nothing, as today (`crates/safix-core/src/check.rs:351`).

### D7. A tag match is a note, not a pass

The project owner rejected treating an exact match as silence.
A tag is 32 bits, so anyone able to write the file can mint SSH keys until one collides with a declared key's tag — on the order of 2^32 key generations — and substitute a stanza the comparison cannot tell apart.
A SOPS document's metadata names the full recipient, so the two are not equal evidence.

`Finding::fails(&self) -> bool` is added to `check.rs`, true for every variant except `RecipientRosterTagMatched`.
`check_command` exits non-zero when any finding fails.
`render::report` prints notes like findings, counts only failing findings in the `{N} finding(s).` closing line, and when only notes were printed closes with `safix: no drift; the note(s) above qualify that.` after a blank line.
The note's headline is `Recipient roster matched by tag only: {file}`, and its detail states that each of the {matched} stanzas matches a declared SSH key by a four-byte tag, which anyone can collide with, so the match does not establish which keys the file is wrapped to; its remedy is the SOPS-container sentence `RecipientRosterUnverifiable` already prints (`crates/safix/src/render.rs:96-106`).

### D8. The audience record

`crates/safix-core/src/age_audience.rs` owns one line:

```text
safix-age-audience-v1 <audience digest> <ciphertext digest>
```

- The audience digest is SHA-256 hex over a canonical form of the declared recipients: sorted, deduplicated, each written length-prefixed with a count in front, the discipline `crate::definition::canonical` uses (`crates/safix-core/src/definition.rs:152-214`).
  Sorted because the order of `--recipient` arguments does not change who can open the file.
- The ciphertext digest is SHA-256 hex over the file's bytes as `fix` renamed them into place.
- The tag `safix-age-audience-v1` moves whenever either canonical form moves, and a record with any other tag is unreadable, by the rule `crates/safix-core/src/definition.rs:79-94` states.

The record carries no value and no derivative of one: one digest is over recipient strings already in the declarations, the other over ciphertext already committed.

Its path is the resolver's.
`resolve.nix` emits `audienceRecord` on every placement: `null` unless `format == "age"`; otherwise `"${logicalRecord}.audience"` with no vault, and `"state/${opaqueOf r.namingKey "audience" (relativeTo r.storage.generatorRecords logicalAudience)}"` in vault mode; `logicalAudience` carries the readable form in vault mode only, beside `logicalStamp` (`modules/flake/safix/resolve.nix:914-922,970-983`).
The `.audience` suffix needs no injectivity argument of its own, for the reason `.stamps` needs none: `.` is outside the declared-name alphabet.
Its vault tag is `"audience"`, so it cannot hash to the definition record's or the stamp's opaque name.
`Placement` gains `audience_record: Option<String>` and `logical_audience: Option<String>`; both are `Option`, so every existing fixture that omits them still deserializes under `deny_unknown_fields`.

Per entry rather than per file, because a raw-age entry is always one file of its own (`docs/concepts/storage-layout.md:70-85`), and because the placement is where the resolver already emits every path.

Rejected alternatives:

- Extending the stamp record: its tag is refused when it moves (`crates/safix-core/src/stamps.rs:54-60`), so changing it would turn every existing stamp into `safix::stamp_record_unparsable`.
- An audience digest alone, without the ciphertext digest: a file re-encrypted by `age -r` or `agenix -e` to some other roster would still match the record, and `fix` would skip a file whose roster it has never seen.
- Keying the record per governed file in the `governedFiles` attribute: it would cover `extraGovernedFiles`, but those paths are the consumer's own and have no readable/opaque pair the relocation machinery can move.

### D9. Only `fix` writes the record

`set`, `edit`, `generate` and `sync` write raw age through `crate::ciphertext::write` to the declared audience (`crates/safix-core/src/ciphertext.rs:986-1053`) and leave the record alone.
Their write changes the file's bytes, so the ciphertext digest no longer matches and the next `fix --yes` re-wraps that one file once and records it.
That is one extra re-wrap per content write, against teaching four writers a record whose only reader is `fix`; the cost is accepted and documented.

### D10. `fix` reads the record before it asks for `--yes`

`rewrap_age` (`crates/safix-core/src/fix.rs:167-231`) finds the placement whose `file` equals the governed path and whose `audience_record` is set, reads the record, and skips the file when the record reads, its audience digest equals the declared audience's and its ciphertext digest equals the file's current bytes.
The skip is logged as `safix: {relative} is already wrapped to the declared audience; left alone.`
It happens before the `assume_yes` refusal at `fix.rs:177-179`, because a skipped file replaces nothing and so needs no authorization.

Otherwise the re-wrap proceeds as today, and after the rename at `fix.rs:220` the record is written through a candidate beside its target and a rename, registered with `scratch`, as `stamps::touch` writes a stamp (`crates/safix-core/src/stamps.rs:195-229`).
`fix` does not commit, so the record lands uncommitted beside the file it describes.
The log line at `fix.rs:224-229` and the refusal text at `fix.rs:178` are reworded to say the previous roster's non-SSH recipients cannot be shown, rather than that nothing can.

Vault relocation gains `relocation::audience_leaves`, gated on `logical_audience` the way `stamp_leaves` is gated on `logical_stamp` (`crates/safix-core/src/relocation.rs:107-124`), chained into `check.rs`'s `vault_relocation` (`:290-291`) and `fix.rs`'s `relocate` (`:354-356`).

### D11. What `check` says about the record

`check` reads the record in the raw-age branch of `recipients`.
Absent, in another format, or unparsable: nothing, because a record this version cannot read says nothing (`crates/safix-core/src/definition.rs:108-116`).
Present with a ciphertext digest other than the file's: nothing, because the record describes bytes that are no longer there.
Present, ciphertext digest equal, audience digest different: `Finding::AgeAudienceChanged { file, record }`, failing, remedy `safix fix --yes` and the not-revocation sentence.
This is the one case in which a raw-age file's X25519 recipients are known to disagree with the declarations, and it is reported in addition to whatever D6 reports.

## Risks / Trade-offs

- [Stamps from machines with skewed clocks can invert the comparison] → A false finding is cleared by its remedy; a missed one is no worse than today. The guide states that the stamps are wall-clock seconds from whichever machine wrote them.
- [A tag match can be forged] → It is never a pass (D7); the note says so in its own text.
- [A collision between two honestly declared SSH keys' tags] → Expected multiplicity counts distinct keys, so collisions neither hide a stanza nor fabricate a duplicate; an extra stanza with a colliding tag is named by type and tag rather than attributed.
- [An audience record edited by hand to match] → The record sits beside the declarations in one repository, the trust position `crates/safix-core/src/digest.rs:22-26` describes for the definition record; anybody who can edit it can edit the ciphertext. A forged record makes `fix` skip a file and `check` stay silent about its audience, which is today's behaviour for every raw-age file.
- [The exit-status change is visible to CI] → A tree whose only raw-age files are all-SSH and agree moves from exit 1 to exit 0; a tree with any SSH tag disagreement or staleness moves from exit 0 or 1 to exit 1. Both are the point of the change and are named in the proposal.

## Migration Plan

No declaration changes.
Existing raw-age files have no audience record, so the first `fix --yes` after upgrading re-wraps each of them once, as every run does today, and writes the records; the operator commits them with the re-wrapped files.
From then on an unchanged audience is skipped.
Rolling back to an earlier safix leaves the records in the tree; an earlier `deny_unknown_fields` `Placement` would refuse an evaluation carrying `audienceRecord`, so the resolver and the runtime ship together, as every placement field has.

## Owner decisions recorded here

- The staleness remedy names the dependent (D2).
- An exact SSH tag match is a non-failing note, never a silent pass; mismatches fail; any non-SSH element keeps `RecipientRosterUnverifiable` (D6, D7).
- The audience record format and `AgeAudienceChanged` are as D8 and D11 state.
- A duplicated SSH stanza is named by the audience's own key when the audience declares exactly one key with that `(type, tag)`, and by `<type> <tag>` otherwise; an extra stanza keeps the D6 recovery over every declared key.
- The base64 decoder refuses a final group whose unused bits are not zero, as well as every byte outside the standard alphabet, so each input it accepts encodes exactly one byte string.
- The raw-age branch of `recipients` reports `UngovernableExtra` for a raw age file no audience or covering directory declares, as the SOPS branch does (D6), before its header is read.
