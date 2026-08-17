## 1. The card surface

- [x] 1.1 Enumerate connected cards by serial; refuse two cards without `--serial`, naming both and the flag
- [x] 1.2 Probe PIV access state: factory-default versus provisioned, without consuming a retry (the empty-PIN probe)
- [x] 1.3 Drive ykman non-interactively: generated PIN, generated distinct PUK, management key `--protect --generate`; nothing stored, nothing prompted
- [x] 1.4 Refuse every OTP surface: no command path issues an OTP write, and asking is refused with the database-lockout hazard named
- [x] 1.5 Unit-test the command construction for each drive, and the refusals for multiple cards, missing pcscd, and a provisioned card

## 2. The identity

- [x] 2.1 The PTY wrapper: run one command on a pseudo-terminal, answer password prompts by shape with one attempt, surface everything else (the touch instruction included) to the operator's terminal
- [x] 2.2 Drive `age-plugin-yubikey --generate --serial <n>` with first-empty slot, the person-and-serial name, `pin-policy once`, `touch-policy cached`; refuse `touch-policy never`
- [x] 2.3 Capture the identity block and the recipient; append the block to the identity file with `keygen`'s discipline (append, never truncate, `0600`)
- [x] 2.4 Integration-test against a stub plugin on a real PTY: prompt answered, block captured, one-attempt abort on PIN rejection

## 3. The wiring

- [x] 3.1 Add the recipient to the person's `recoveryRecipients` in `safix/users/<name>.nix`, with `adduser`'s edit-and-commit discipline; never remove or replace anything
- [x] 3.2 Regenerate the policy and re-wrap governed files, and assert every file that opened before still opens
- [x] 3.3 Register with clan through clan's own command when a clan is declared; skip silently when none is
- [x] 3.4 Add `flake.safix.enrollHook` beside `onboardingHook`: receives person, serial, recipient; hookless runs succeed having done less and say so
- [x] 3.5 Verify: a fixture enrollment adds exactly one recipient, one identity block, one commit set, and a second enrollment for a backup serial sits beside it untouched

## 4. The proof

- [x] 4.1 Build the isolated identity source: a temporary file holding only the card's stub, the ordinary sops path pointed at it alone
- [x] 4.2 Decrypt one governed file from the person's audience to a pipe and discard; report what was proven, or report the enrollment incomplete with the wiring intact
- [x] 4.3 Verify: with a software key ambient, the proof still exercises the card (the isolation is the test), and a failed proof leaves the additive wiring in place and the report saying incomplete

## 5. The PIN's custody

- [x] 5.1 Register the generated PIN and PUK as a safix secret in the person's own custody by default, named for the serial, through the ordinary write path; `--no-store-pin` opts out (decision: design's second open question, answered default-on)
- [x] 5.2 Values travel pipes and DBus only: assert no argument vector and no environment variable carries them
- [x] 5.3 Mirror the credentials to the password store when the operator wishes: the session secret service when the database is unlocked, `keepassxc-cli` with one password prompt otherwise, and skippable entirely
- [x] 5.4 Verify: each stored copy round-trips (written, then read back through the same path), and nothing reached stdout unbidden

## 6. The record

- [x] 6.1 USER-RUN (answered): confirm design's first open question — operator-custody scope, not per-user escrow — before the verb's prose is written. Confirmed: the operator's own custody, with the card a peer of the software master identities in the same identity file and capable of becoming the sole master after the plaintext retirement; recorded under design's open questions
- [x] 6.2 Usage text: the verb's contract, the touch ceiling, the refusals, in the scaffold order
- [x] 6.3 README: the enrollment story, one section, concise; CHANGELOG under Unreleased
- [x] 6.4 Verify: `openspec validate enroll-hardware-custody --strict` passes
