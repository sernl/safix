---
title: "Hardware keys"
---

## What you will have

This page shows you how to take one hardware key from a blank card to a proven recovery identity for a person. At the end the card's recipient is declared, every governed file is wrapped to it, and the card has opened one of those files for real.

A touch is the only thing you do by hand.

## Steps

1. Connect one card and run the verb as the person it is for:

   ```console
   $ safix enroll alice
   ```

   Two connected cards are refused, naming both serials and `--serial`.

2. Touch the card when it asks. That happens while the age identity is generated in the first empty retired slot, under a pseudo-terminal that supplies the PIN.

3. Read what the run committed. Three things land together: the card's recipient added to that person's recovery identities, a regenerated `.sops.yaml`, and every governed file re-wrapped.

4. Check the proof line at the end. The card alone opens one governed file in the person's audience, exercising the PIN and the touch.

## What the run does, in order

The card is selected, then its PIV access is provisioned when it is factory-fresh: a generated PIN, a distinct generated PUK, and a random management key put on the card under the PIN. A card that is already provisioned is not re-provisioned, and its PIN is asked for once, unechoed.

The identity block is appended to the same file `safix keygen` appends to. It holds no private key, because the key is on the card.

Where a clan is declared through [`flake.safix.bridge.clanFlake`](../reference/declarations.md#flakesafixbridgeclanflake), the recipient is registered with clan through clan's own command. Then [`flake.safix.enrollHook`](../reference/declarations.md#flakesafixenrollhook) receives the person, the serial and the recipient.

The generated PIN and PUK are stored as that person's own secret, named for the serial. `--no-store-pin` turns that off, and `--mirror-to-store` writes them to the password store as well.

Everything the verb does is additive. A recipient is appended, an identity block is appended, a name is declared. A backup key is the same verb run again: each card gets its own identity and its own recipient, and neither run knows about the other.

An enrollment whose proof has not passed reports itself incomplete and exits non-zero. Nothing is undone, because every step was additive.

## Why the card is a recovery identity and not the primary one

A card needs a touch, and activation decrypts with nobody present. So the card's recipient goes into that person's recovery identities, where it widens what they can open and changes nothing about how a host decrypts.

The primary [`flake.safix.users.<name>.recipient`](../reference/declarations.md#flakesafixusersnamerecipient) stays software-only for the same reason. A recipient that needs physical interaction is refused for that field.

See [Unattended hosts](unattended-hosts.md) for the other half of that split, and [Identity backup](identity-backup.md) for keeping the software identity recoverable.

## Seeding a machine instead of a person

`safix enroll` provisions people. A machine's own key is `safix upload`, which writes an operator-held private half to a pre-seed tree or to the host itself before its first activation. That is covered in [Unattended hosts](unattended-hosts.md).

## What is refused, and why

No OTP slot is written under any flag. A programmed challenge-response slot is what opens a password database, and the database keeps no record of the secret it was built with, so writing that slot ends it permanently.

Reading such a slot to answer a database's own unlock challenge is a different operation, and that belongs to the sync verb — see [Syncing to password managers](syncing-password-managers.md).

`--touch-policy never` is refused. The touch is the property a card is for.

A run with no terminal is refused before the card is touched: somebody has to touch it, and somebody has to be told when.

No credential the verb generates reaches an argument vector or an environment variable. The card tool's credential options are omitted so that it prompts, and the prompts are answered on a pseudo-terminal. The management key is stored nowhere, because PIN possession is management possession.

A tree holding raw-age files needs `--trust-declared-recipients` before any card is selected. That flag authorizes the declared audience; it cannot prove which recipients the opaque ciphertext held — see [Inspectable recipients](inspectable-recipients.md).

## What can go wrong

- `safix::no_card_connected` and `safix::cards_ambiguous` — no card, or two. The second names both serials.
- `safix::ykman_unavailable` and `safix::pcscd_unavailable` — the card tooling or the smartcard daemon is not reachable.
- `safix::card_command_failed` and `safix::card_pin_rejected` — the card refused an operation or the PIN.
- `safix::otp_refused` — an OTP slot was asked for.
- `safix::touch_policy_never` — `--touch-policy never` was asked for.
- `safix::no_terminal` and `safix::pty_unusable` — there is nobody to touch the card, or no pseudo-terminal to answer prompts on.
- `safix::recipients_lost` — a re-wrap would have dropped a recipient the file had before the run. It is refused rather than committed.
- `safix::no_file_to_prove_with` — the person's audience holds no governed file, so the proof step has nothing to open.
- `safix::hardware_recipient` — an interaction-requiring recipient was offered for the primary recipient field.
- `safix::clan_user_registration_failed` and `safix::enroll_hook_failed` — the clan registration or your hook failed after the additive steps landed.

Every code is listed in [Refusals](../reference/refusals.md).
