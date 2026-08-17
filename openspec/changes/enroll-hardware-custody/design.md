## Context

See proposal.md — Why.
Measured facts the approach rests on (research 2026-08-17; clan-core at `56e35624`, age-plugin-yubikey 0.5.1, ykman 5.9.2, sops 3.13.3 vendoring age v1.3.1):

- Every ykman PIV access operation is flag-driven and non-interactive, including `change-management-key --protect --generate` (random key, PIN-protected, on-card). Factory defaults: PIN `123456`, PUK `12345678`, standard TDES management key.
- `age-plugin-yubikey --generate` prompts for the PIN on a TTY only; piped stdin submits an empty string (dialoguer/console return `""` off-tty) and fails. A pseudo-terminal is the only programmatic path. Slot selection: retired slots 1-20 (PIV `82`-`95`), first-empty by default; `--serial` mandatory with two cards connected; `Error::MultipleYubiKeys` otherwise. When stdout is not a terminal the recipient is echoed to stderr as `Recipient: age1…` — the scrape point.
- The plugin's own factory-default flow forces a PIN change and sets PUK = PIN. safix pre-provisions with ykman instead, keeping PIN and PUK distinct.
- age sorts native identities before plugin identities at decrypt (`age.go:324-341` in the vendored v1.3.1), so an enrolled card is never touched while a software identity opens the file.
- clan accepts `age1yubikey1…` through `clan secrets users add` (recipient regex matches; identity blocks satisfy the preceding-recipient rule), and `secrets.age.plugins = [ "age-plugin-yubikey" ]` is clan's documented plugin path.
- safix refuses hardware recipients for `recipient` (`adduser.rs`, `HARDWARE_PREFIX`), with prose directing cards to `recoveryRecipients`. `keygen` appends, never truncates, `keys.txt` at `0600`. `adduser` edits `safix/users/<name>.nix`, regenerates policy, commits. Both fleet keys were enrolled by hand on 2026-08-17; the manual trace is the executable spec.
- The KeePassXC database's challenge-response secret lives on OTP slot 2 of both keys; PIV and OTP applets are disjoint, and a slot-2 write permanently locks the database. A PreToolUse guard on this host already refuses `ykman otp` invocations.

## Goals / Non-Goals

Goals: one verb from blank card to proven recovery identity; touch as the interaction ceiling; generated credentials that land in custody, not in a human's head; additive always.

Non-goals:
- Writing OTP slots, ever — including the challenge-response cloning that would extend the KeePassXC factor to a new card. That is a deliberate manual act with the database's life at stake, and it is GUI-adjacent anyway (KeePassXC's own enrollment of a challenge-response key is GUI-only). Recorded here so the boundary is a decision, not an omission.
- Making a card the primary `recipient`. Activation decrypts non-interactively; the refusal stands.
- Unattended enrollment. The touch is the point; a card enrolled with `touch-policy never` to make CI pass would be a smartcard emulating a file.
- Retiring any recipient (the plaintext master retirement is `one-unlock-bootstrap`'s Phase 2 decision, made after proofs, by the operator).

## Decisions

### D1. safix provisions access with ykman first, then drives the plugin; the plugin's own onboarding is bypassed

The plugin's factory-default flow collapses PUK into PIN and prompts twice.
ykman sets PIN, PUK (distinct, both generated), and a protected on-card management key with flags alone; the plugin then sees a provisioned card, asks once for the PIN, and the pseudo-terminal supplies it.
The management key is deliberately unsaved: protected mode means PIN possession is management possession, and a stored management key would be a credential with no reader.

### D2. The pseudo-terminal is a narrow, owned mechanism

One PTY wrapper, in safix-core, that runs one command, writes one line when prompted, and surfaces everything else to the operator's real terminal — used for `--generate` and for the proof's PIN entry.
`expect` as a runtime dependency was rejected: the interaction is two prompts, not a protocol, and a dependency that scripts arbitrary TTYs is a bigger surface than the twenty lines that answer these two.

### D3. The PIN and PUK are auto-registered in safix, and mirrored to the password store when the operator wishes

The operator's direction is that credential custody is safix's job first, so the generated PIN and PUK land as a safix secret in the person's own custody by default — named for the serial, encrypted to the person's audience like anything else they hold — and `--no-store-pin` opts out.
The honest caveat is stated rather than hidden: a PIN readable by the software identity adds protection only once that identity is retired or absent; the default exists to make starting easy — the focus on secrets rather than on operational process — not to claim a property it does not have.
The password-store mirror is the optional second home: written through the session's secret service when the database is unlocked, with no prompt at all; through `keepassxc-cli` with one password prompt when it is not; skippable entirely.
The database opens by challenge-response with no PIV PIN involved, so whichever copies exist, the card's PIN is reachable with the card in hand and no self-reference.

### D4. The proof isolates the card by construction, not by hope

A temporary identity file holding only the enrolled card's stub, `SOPS_AGE_KEY_FILE` pointed at it, one governed file from the person's audience decrypted through the ordinary sops path.
Native-first ordering means an ambient `keys.txt` would silently satisfy the decrypt with a software key; isolation is what makes the proof about the card.
Success prints what was proven; failure leaves the enrollment reported incomplete with the identity block in place, because the wiring is additive and correct even while the proof is outstanding.

### D5. clan registration is delegated, and the hook carries what delegation cannot

`clan secrets users add` (or `add-key` when the person exists) runs as a subprocess when a clan is declared — the bridge's symmetric-delegation rule, applied to enrollment.
What clan's command cannot know — this fleet wires recipients into `modules/clan/vars.nix` lists — is the consumer's shape, so `enrollHook` receives `(user, serial, recipient)` exactly as `onboardingHook` receives its arguments, and the hookless run succeeds having done less.

### D6. Policies default to the fleet's measured choice

`pin-policy once`, `touch-policy cached` — what the enrolled keys already carry — overridable per run.
`touch-policy never` is refused rather than accepted, for the non-goal's reason.

## Risks / Trade-offs

- [The PTY drive depends on the plugin's prompt strings] → the wrapper answers password prompts by shape, not by text; a plugin upgrade that changes wording still gets the PIN. The version is pinned by nix either way.
- [A wrong generated PIN fed thrice could burn retries] → the wrapper feeds one attempt and aborts on rejection; retries remain at the card's counter minus one, and the refusal says so. The plugin's own empty-PIN probe is known not to consume a retry.
- [pcscd absent or another agent holds the card] → the refusal names `services.pcscd.enable` and the 15-second insertion timeout is respected rather than raced.
- [Secret service writes land in the database's exposed group] → that is where session secrets go on this fleet by design (the boundary travels inside the kdbx); the entry is labelled as PIV access for the serial, and the `keepassxc-cli` fallback writes outside the session path when the operator prefers.
- [The proof decrypts a real secret to prove access] → it decrypts to a pipe and discards; nothing lands on disk, matching the value-pipe discipline the runtime already proves for itself.

## Migration Plan

Additive: a new verb, a new hook, no change to any existing surface.
The fleet's two enrolled keys need nothing; `safix enroll` on either reports the identity present and runs the missing proof — which is exactly the open item `one-unlock-bootstrap` D1 names, closable the day this lands.
Rollback is removing the verb.

## Open Questions

- USER-RUN: "master key" scope. This design reads the direction as the operator's own custody — the card in the operator's `recoveryRecipients`, registered with clan — not as an escrow key listed in every user's `recoveryRecipients`, which safix's own prose warns buys recoverability at the price of the operator reading everything everyone holds. Confirm, or name the escrow variant and it becomes its own decision with that trade-off stated.
  Answered: the operator's own custody, confirmed and sharpened.
  The card enrolls into the same identity file the software master identities live in — the `keys.txt` that sops, agenix and clan's admin path all read — as a peer of those identities, and is capable of becoming the sole master once the plaintext identities retire.
  That retirement stays `one-unlock-bootstrap`'s own Phase 2 act, made after the decrypt proof, not this change's.
- USER-RUN: whether the optional safix-side PIN copy (`--store-pin`) should exist at all, given D3's argument that it adds nothing while the software key exists. It is in the design because the operator asked that safix handle PIN custody "for itself first and foremost"; it is off by default because the argument against is real.
  Answered: it exists and is the default, with `--no-store-pin` the opt-out; the password-store mirror is the optional wish.
  D3 is rewritten accordingly, with the caveat kept beside the default it qualifies.
