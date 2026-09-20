---
title: "Custody"
---

## What this page is for

This page tells you which things can hold a key, how a public recipient differs
from the private identity that decrypts, and what happens when you take access
away.

## Subjects

The things that can hold a key are one algebra rather than a second grant
surface. Nothing here changes anything until you declare it: a subject nothing
references produces the same policy and the same files, byte for byte.

A **person** is `flake.safix.users.<u>`. A person holds entries through
`private`, through `carries` and through grants, and their custody is the same
on every host they log into.

A **machine** is `flake.safix.machines.<m>`. Its recipient is the age form of the
host identity the system scope already decrypts with. A machine holds nothing of
its own — no `carries`, no `private`, no `sharedWith` — and it needs no
hostname, because it is the host. Its entries arrive in the profile that names it
through [`safix.machine`](../reference/profile-options.md#safixmachine).

A **service** is `flake.safix.services.<s>`. The machines it runs on are the
whole of its recipient set, and its `user` and `group` are the account the landed
file belongs to. A service grant narrows what is declared and what is placed,
not what decrypts: the host identity still opens the file. safix derives where a
unit runs from nothing, so keeping the declared set and the running unit in step
is yours.

A **group** is `flake.safix.groups.<g>.members`, which may name people, machines,
services or other groups. A cycle is refused at evaluation with the participants
named. A group is what makes a membership change a re-wrap of one file rather
than a move to another.

A **silo** is `flake.safix.silos.<s>.groups`, a non-overlap you can prove.
Evaluation refuses any file whose audience would reach subjects of two groups in
one set, naming the file, the subjects and the declaration that forbids it. It is
deliberately not transitive over ownership: one person may own machines in two
silos, and what is refused is a single file readable from both.

An **organization** is `flake.safix.organizations.<o>`. Its `custody` holds its
own recovery identities, each a key with a note, and it can own a machine, be
granted a name, and be reached through `ownerOf`. A group may not contain one,
because a principal is not a member, and an organization whose custody is empty
is refused everywhere it is reached.

## Recipients and identities

A **recipient** is a public key string: native age, an SSH public key, or `pgp:`
followed by a full uppercase GnuPG fingerprint. It is what a file is encrypted
to, and declaring one mints nothing.

An **identity** is the private half, and it lives on the machine or in the
person's hands, never in a declaration. A profile names its identity through
the options under
[`safix.identity`](../reference/profile-options.md#safixidentitykeyfile). A
profile that resolves entries and names no usable identity refuses rather than
establishing them.

The two halves travel separately on purpose. Key generation belongs to the
person who will hold the key: `adduser` mints nothing, and minting somebody
else's identity takes an explicit flag naming what it is — see
[the CLI reference](../reference/cli.md).

A recipient that needs a physical interaction to decrypt is refused as a
person's primary recipient, because activation decrypts without a human present.
Such a key belongs among their recovery identities, where it is additive — see
[Hardware keys](../guides/hardware-keys.md).

## Recovery recipients

`flake.safix.users.<u>.recoveryRecipients` lists further identities of the same
person, each a `key` with a `note`. Every file whose audience includes that
person is wrapped to these as well, so the field widens what they can open and
nothing else.

Leaving it empty keeps their custody independent, at a cost no later edit undoes.
With only their activation key, losing it makes their files unopenable by every
party, the operator included.

An offline master key, or a token the person themselves holds, is the mitigation
that keeps that independence. An operator-held identity buys the same
recoverability at the price of that operator reading everything.

Where the further holder is an organization, `flake.safix.users.<u>.escrowedTo`
is the consent, and it lives in the record of the person whose files it widens.
So nothing an organization declares widens anybody's audience by itself. The
organization can then rotate a custody key in its own declaration, and one
`safix fix` re-wraps every consenting person's files with no person's
declaration changing. See [Identity backup](../guides/identity-backup.md) for
the mechanics of holding a second identity.

## Delegation is not authorization

`flake.safix.organizations.<o>.managers` names the people who scaffold on the
organization's behalf. `flake.safix.users.<u>.managedBy` is the person's own
statement that they are scaffolded for by it.

Neither line places a key in any audience. A manager scaffolds and never reads
by virtue of managing, so the generated policy is byte-identical to what it was
before either line existed.

Where both halves are declared, `safix enroll` and `safix group` accept that
organization's managers and refuse anybody else, naming the delegation and the
person who ran the command. The acting identity is the one the commit will carry,
as the repository resolves `user.name` and `user.email`, and no flag names
somebody else.

The tree is the authorization. Anyone who can commit can edit these declarations
by hand, and evaluation refuses structure rather than people. What delegation
buys you is a reviewable record of who scaffolds for whom, not a permission
check.

## The revocation rule

An encrypted document has one data key, wrapped once per recipient, so everyone
the file names can read all of it.

Narrowing an audience therefore aligns future ciphertext with the new audience,
and it does not take back what an old recipient already read. They have seen the
values in every file they could open.

The remedy is to mint a new value: `safix set` for a typed value,
`safix generate --regenerate` where it has a generator, or `sops <file>` for a
multi-line one. `safix check` reports a shrunk audience as the narrowing it is
and names rotation as the remedy. `safix fix` aligns ciphertext with policy and
is explicitly not that remedy.

Every narrowing is the same shape: removing a grant, a person leaving a group, a
machine leaving a service, a change of owner, a withdrawn escrow consent. Each
re-wraps files and none of them unreads a value.

[After removing access](../guides/after-removing-access.md) walks the sequence,
and [Rotating secrets](../guides/rotating-secrets.md) covers doing it on a
schedule.
