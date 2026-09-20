//! What `-h` prints, for the subcommands this binary implements.
//!
//! Help goes to standard error and exits successfully. Diagnostic prose may
//! evolve; stable refusal codes, not exact terminal wording, are the contract.

/// `safix set -h`.
pub const SET: &str = "\
safix set [<user>] <name>

Prompt for a value twice without echoing it, write it into the file the
declarations place <name> in, then stage and commit that file alone.

Values are single-line and stored exactly as typed, with no trailing newline. A
multi-line value is `sops <file>`, or a generator — `generate` stores what a
script produces, newlines and all.

This is the hand-typed case, and it stays separate from `generate` on purpose: a
credential some server already knows cannot be minted, only transcribed.

\u{2500}\u{2500} scripted writes \u{2500}\u{2500}
When standard input is not a terminal the value is read from it instead, whole,
and stored exactly as sent — `echo` pipes a trailing newline and `printf` does
not, and nothing here removes one:

  printf '%s' \"$TOKEN\" | safix set alice grafana-token

This replaces nothing. A terminal still gets the prompt above, unchanged. What
the piped form drops is the confirmation, and that is the point rather than a
concession: the second prompt exists to catch a value mistyped invisibly, and a
piped value has no typist. An empty pipe takes the same refusal an empty prompt
takes, because it is what a failed upstream command leaves behind.
";

/// `safix fix -h`.
pub const FIX: &str = "\
safix fix [--yes] [--vault-rollback]

Regenerate .sops.yaml from the declarations, then re-wrap each governed file's
data key to the audience that policy declares. --yes answers sops' confirmation.
Raw age has no inspectable recipient roster. For those files, --yes explicitly
authorizes replacement with the declared audience; decryption verifies bytes,
not the previous roster.

The order is not interchangeable: re-wrapping first re-wraps to a policy that is
about to change.

The governed set is the union of the files the declarations imply and the ones
named in flake.safix.extraGovernedFiles. A file left out of it is a file a change
of audience reaches for every other file and not for that one.

It does not commit: re-wrapping every governed file is a diff worth reading
first. It does not revoke either. A person removed from an audience has already
read every value in the file, and re-wrapping the data key does not unread it;
revoking means a new value, which is `generate --regenerate` or `sops <file>`.

\u{2500}\u{2500} with a vault declared \u{2500}\u{2500}
Before re-wrapping, this relocates every readable-layout document, public
output and definition record still at the declaration root into its opaque
vault destination: decrypted under your own identity, re-encrypted under the
vault's disposable creation rules, then removed from the declaration root. A
destination already present is left alone, so an interrupted run resumes
where it stopped. This also writes the vault's own .gitignore entry for
.sops-vault-rules.yaml when it is missing. Nothing here commits; a note
names the commit order that follows — the vault first, then the
declaration root with a Safix-Vault: <short-id> trailer — and the lock
bump that comes after that.

--vault-rollback runs the same move the other direction and skips the
re-wrap: every document, output and record still in the vault is decrypted
and re-encrypted into the readable layout at the declaration root, then
removed from the vault. Run it while flake.safix.vault is still declared —
the naming key needed to find a vault-rooted entry's readable name is only
reachable through the still-standing declaration — then remove the
declaration and unset SAFIX_VAULT_ROOT yourself once it finishes.
";

/// `safix audit -h`.
pub const AUDIT: &str = "\
safix audit [clan|keepassxc|pass|bitwarden|1password] [<mapping>...] [--direction <value>]

Compare declared mappings and change nothing. With no target it compares both
clan and keepassxc; naming one narrows to that target's own mappings, and
mapping names after it narrow further, to as many as are named \u{2014} with none
named it acts on every mapping of the target chosen. --direction narrows the
clan target to mappings declared with that value; it is refused on
keepassxc, whose mappings declare a mode instead of a direction.

\u{2500}\u{2500} the clan target \u{2500}\u{2500}
Both sides of each mapping are read and compared, and nothing is written. A
mapping agrees when both sides hold the same bytes, and also when neither
side holds a value yet \u{2014} that is a bridge nothing has bootstrapped rather
than a disagreement. It is a finding when the two sides hold different
values, when one side holds a value the other does not, or when the
comparison could not be made, and each finding names the mapping, its two
endpoints and the command that converges it: safix sync clan <mapping>.

Clan vars no currently declared mapping's clan side accounts for are
reported alongside as information \u{2014} lingering, in the same shape
keepassxc's own report gives it \u{2014} scoped to the machines the selected
mappings name or resolve, and never move the exit status. Nothing here
removes one; a person does that, with clan's own command.

\u{2500}\u{2500} the keepassxc target \u{2500}\u{2500}
Both sides of each mapping are read and compared per its declared mode, and
nothing is written. Each mapping's outcome is reported as agreeing, diverged,
or unjudgeable, and the remedy for a diverged mapping is safix sync
keepassxc <mapping>. Entries under the declared group that no mapping
declares are reported alongside as information \u{2014} lingering, in the same
shape sync's own report gives it \u{2014} and never move the exit status.

\u{2500}\u{2500} the pass target \u{2500}\u{2500}
Both sides of each mapping are read and compared per its declared mode, and
nothing is written \u{2014} no entry is created, no entry is replaced, and no
recipient declaration is touched. Each mapping's outcome is reported as
agreeing, diverged, diverged in named fields, or unjudgeable; a value
divergence takes precedence over a field divergence for the same mapping, so
a cosmetic drift is never mistaken for a secret drift. The remedy for a
diverged mapping is safix sync pass <mapping>. A diverged field is named and
never printed, because a notes body and a field sourced from another entry
are themselves secrets. Entries in the store that no mapping declares are
reported alongside as information \u{2014} lingering, in the same shape sync's own
report gives it \u{2014} and never move the exit status.

\u{2500}\u{2500} the bitwarden target \u{2500}\u{2500}
Both sides of each mapping are read and compared, and nothing is written \u{2014}
no item is created, no item is edited, and no folder is touched. The client
is asked for its own state first and its local copy is refreshed once, so a
comparison is never made against a copy that may predate another device's
change: a locked or unauthenticated client, a declared server that is not
the one reached, and a failed refresh each refuse exactly as they refuse
under sync. Each mapping's outcome is reported as agreeing, diverged,
diverged in named fields, or unjudgeable; a value divergence takes
precedence over a field divergence for the same mapping. A diverged mapping
under a pushing mode is given safix sync bitwarden <mapping> as its remedy;
one declared bitwarden-to-safix is told instead that the declaration is the
author of a field. A diverged field is named and never printed. Items under
a declared folder that no mapping declares are reported alongside as
information \u{2014} lingering \u{2014} and never move the exit status.

\u{2500}\u{2500} the 1password target \u{2500}\u{2500}
Both sides of each mapping are read and compared, and nothing is written \u{2014}
no item is created, no item is edited, and no vault is touched. Each
mapping's outcome is reported as agreeing, diverged, diverged in named
fields, or unjudgeable; a value divergence takes precedence over a field
divergence for the same mapping, so a cosmetic drift is never mistaken for a
secret drift. A diverged mapping under a pushing mode is given safix sync
1password <mapping> as its remedy; one declared 1password-to-safix is told
instead that the declaration is the author, because for that mode converging
toward the declaration is what the mode already means. A diverged field is
named and never printed, because a notes body and a field sourced from
another entry are themselves secrets. Items in a declared vault that no
mapping declares are reported alongside as information \u{2014} lingering \u{2014} and
never move the exit status.

\u{2500}\u{2500} why this is a verb of its own and not four more rows in `check` \u{2500}\u{2500}
`check` decrypts nothing, which is what lets one machine judge files
belonging to people whose keys it does not have, and it needs no clan and no
password database. This comparison needs both of those powers on the clan
target and the database's own password on the keepassxc target. Carrying
them here is what keeps both of `check`'s properties unconditionally true.

A mapping this operator cannot decrypt, or a database entry the store's own
command refuses over, is reported rather than skipped, as one that could not
be judged. A report that dropped it would be a report about who ran it, and
a clean one would mean `what I could see agrees` while reading as `the
mappings agree`.

Nothing here writes, on either target. No value, and no digest of one,
reaches the report.
";

/// `safix sync -h`.
pub const SYNC: &str = "\
safix sync [clan|keepassxc|pass|bitwarden|1password] [<mapping>...] [--direction <value>]

Converge declared relationships. With no target it converges every mapping
on both clan and keepassxc, each in its own declared direction or mode;
naming one target narrows to its own mappings, and mapping names after it
narrow further, to as many as are named \u{2014} with none named it acts on every
mapping of the target chosen. There is no all target: the bare form is the
one spelling for everything.

\u{2500}\u{2500} the clan target \u{2500}\u{2500}
Moves values across the boundary to or from clan, one declared mapping at a
time \u{2014} see flake.safix.bridge.mappings. Each mapping converges in its own
declared direction, clan-to-safix, safix-to-clan and two-way mixed freely in
the same run; --direction clan-to-safix, --direction safix-to-clan or
--direction two-way narrows the run to mappings declared with that value,
without overriding any mapping's own direction. import and export do not
exist here: sync clan replaces both, converging every direction a run reaches
in the one invocation that used to take two.

The value is read or written by running clan's own command, on a pipe. safix
reads, writes and parses none of clan's stored files, in either direction, so
the bridge works whatever backend clan uses.

Both sides are read and compared before either is written. A mapping whose
two sides already agree is not written and not committed, so a second run
immediately after a first writes nothing; on the safix-to-clan direction the
comparison is load-bearing rather than an optimisation, because clan's write
is unconditional and a re-encrypting backend produces fresh ciphertext for
an unchanged value.

Two refusals are the safix-to-clan direction's own. A source entry that
holds no value is refused rather than exported as nothing. A mapping whose
clan-side generator clan already considers outdated is refused, because
clan's next routine generation would replace whatever was written without
saying so; there is no option that writes anyway. A two-way mapping's push
toward clan carries the identical refusal, under the identical condition,
with no option that bypasses it either.

Each clan-target mapping is reported as unchanged, updated, absent at
source, or refused. An updated mapping's line names its direction as an
arrow \u{2014} pulled \u{2190} clan or pushed \u{2192} clan. Absent at source is not a
failure: a clan var that has not been generated yet is the ordinary state
during bootstrap. A refused mapping does not stop the others, and the run
exits non-zero.

\u{2500}\u{2500} two-way, across the clan boundary \u{2500}\u{2500}
A two-way mapping remembers the last state both sides agreed on, in a
companion entry it mints beside the mapped one, inside safix's own
sops-encrypted store \u{2014} never in clan's, and never in a plaintext, committed
file. When exactly one side has moved since \u{2014} including a side that has
never held a value, which converges rather than refuses \u{2014} the other
converges to it and the agreement is recorded. When both have moved, or the
two disagree and nothing has ever been recorded, nothing is written and the
finding names the mapping and its remedy: narrow a sync clan run to it with
--direction clan-to-safix or --direction safix-to-clan and run once, then put
the mapping's declared direction back to two-way. Forcing a resolution this
way never remembers the agreement, which is deliberate \u{2014} the same reason
keepassxc-sync's own two-way mode gives.

Each converged two-way mapping is reported as unchanged or converged \u{2014}
converged names no source and no destination, because a two-way convergence
is neither. A conflict or a refusal gets its own paragraph naming the remedy.
The companion's name is the mapped entry's plus -safix-bridge-sync-state,
distinct from keepassxc-sync's dot-prefixed suffix because a companion here
is a safix entry name rather than a database path; evaluation refuses a
hand-declared entry that collides with one.

A shared-placement mapping's clan side names no machine in its declaration.
The machine that answers for it on clan's command line is discovered at run
time by asking clan which machines it has, the same way for every direction,
one-way or two-way alike.

\u{2500}\u{2500} the keepassxc target: the four modes \u{2500}\u{2500}
The mode is declared per mapping, not passed here: a remembered flag on a verb is
exactly the drifting operational knowledge a declaration exists to end.

  safix-to-keepassxc   the database converges to safix's value. A database-side
                       edit to a mapped entry is overwritten, and reported.
  keepassxc-to-safix   safix converges to the database's value, through the same
                       path `safix set` writes through: the same empty-value
                       refusal, the same recipient-drift refusal, the same staged
                       write and rename, and a commit naming the mapping.
  two-way              whichever side changed since the last agreement wins.
                       Both changed is a conflict.
  backup               safix's value is written where the database has none, and
                       a database value that differs is never overwritten \u{2014} the
                       divergence is reported instead.

\u{2500}\u{2500} nothing is ever deleted \u{2500}\u{2500}
No mode deletes an entry, on either side, under any circumstances. Remove a
mapping and its last database value stays where it is until a person removes it;
the report says the entry is there and that no mapping declares it. Filen's
mirror modes do propagate deletions and that is the one part of the model
deliberately not taken: an accidental deletion of a secret is not a state a sync
should be able to reach.

\u{2500}\u{2500} what a conflict is, and is not \u{2500}\u{2500}
A two-way mapping remembers the last state both sides agreed on. When exactly one
side has moved since, the other converges to it. When both have, nothing is
written and the finding names the two one-way modes that each resolve it \u{2014} it
is never resolved by a heuristic, because last-writer-wins over secrets rewards
whichever clock lied best.

That memory is a digest of the agreed value, and it lives in a companion entry
beside the mapped one, inside the encrypted database. It is never in the
repository, and that is a security decision rather than a filing one: a committed
digest of a secret is an oracle for confirming a guessed value offline. The
companion's name is the entry's plus `.safix-sync-state`, which no mapping may
declare \u{2014} evaluation refuses one that tries.

Deleting the companion is safe and converts the mapping to bootstrap semantics:
the next run writes only where one side is empty and reports everything else. The
memory is written only as part of a converging write, so a two-way mapping whose
sides already agreed before safix ever ran has none, and its first divergence is
a conflict rather than a guess.

\u{2500}\u{2500} the database, and the one prompt \u{2500}\u{2500}
flake.safix.keepassxc.database names it, as a string rather than a nix path \u{2014}
a path would be copied into the world-readable store on every evaluation. The
password is asked for once per run and travels standard input; the value of every
entry travels standard input in and a pipe out. No value reaches an argument
vector or an environment variable on any leg.

Without a terminal to ask on, this refuses before reading anything. The session's
secret service is not a second way in: the collection it publishes is KeePassXC's
own exposed group, so it cannot address the group and path a mapping declares,
and a transport that silently landed a secret somewhere else is not one this verb
has.

\u{2500}\u{2500} it manages no keyring \u{2500}\u{2500}
No database is created, no database key is changed or added, and no hardware slot
is touched, under any flag. The store is a store being written, not a keyring
being managed. Writing a challenge-response slot is what would end a database
permanently, and there is nothing here that can. A database may additionally
declare a YubiKey slot, a key file, or both; both open alongside the one
password prompt, and reading a declared slot to answer the database's own
unlock challenge is not the touching this heading forbids.

\u{2500}\u{2500} convergence, and why it is load-bearing \u{2500}\u{2500}
A kdbx save rewrites and re-uploads the whole file. So both sides of every
mapping are read and compared first, every database write of a run is issued
consecutively, and a run over mappings that agree writes nothing anywhere.

A value carrying a newline is refused rather than written: the store's own
command reads an entry's password as one line, so what would land is the bytes
before the first newline. Nothing here strips the byte for you \u{2014} a mirror that
silently trims a value lies about what it holds. `printf` where `echo` minted it.

Each keepassxc-target mapping is reported as unchanged, updated, pulled, conflict,
refused, or not judged, and every declared mapping appears whatever happened to
it. A mapping whose safix side did not decrypt is reported rather than skipped,
because a report that dropped those would be a report about who ran it. The run
exits non-zero when any mapping conflicts, is refused, or could not be judged.

\u{2500}\u{2500} the pass target: the four modes \u{2500}\u{2500}
The mode is declared per mapping, not passed here, for the reason the keepassxc
target's own modes are.

  safix-to-pass        the store converges to safix's value. A store-side edit to
                       a mapped entry is overwritten, and reported.
  pass-to-safix        safix converges to the store's value, through the same
                       path `safix set` writes through: the same empty-value
                       refusal, the same recipient-drift refusal, the same staged
                       write and rename, and a commit naming the mapping.
  two-way              whichever side changed since the last agreement wins.
                       Both changed is a conflict.
  backup               safix's value is written where the store holds no entry,
                       and a store value that differs is never overwritten \u{2014}
                       the divergence is reported instead, and no field is
                       written either.

No mode deletes an entry, and `pass rm` is never run.

\u{2500}\u{2500} the pass target: the record layout is safix's own \u{2500}\u{2500}
The store fixes no metadata schema \u{2014} its own convention is that the value is
the record's first line and further information follows it \u{2014} so safix defines
the layout: the value's bytes, then one blank line, then one `login:`, `url:`,
`notes:` or `tags:` line per declared field, under the spellings the store's
ecosystem already reads. A read takes the field block to be the trailing run of
such lines, consumes one blank line before it as the separator, and takes every
byte above it as the value, verbatim; a body with no such run is a value and
nothing else, so an entry safix wrote with no field is indistinguishable from
one written by hand and `pass -c` still copies the value.

One ambiguity follows and is stated rather than refused: a value whose own
trailing lines are spelled like fields reads back as fields. safix's own writes
never produce it, because they emit the blank separator.

\u{2500}\u{2500} what sync's pass target is \u{2500}\u{2500}
It converges a declared safix entry with one entry inside the declared store,
per mapping and per mode \u{2014} see flake.safix.pass.mappings. The whole record
body, the value and every field together, travels standard input; the argument
vector carries the entry path and the child's environment carries
PASSWORD_STORE_DIR, which is a location rather than a value. Nothing here reads
or decrypts the store's files itself, creates a store, writes a recipient
declaration, or removes an entry.

There is no unlock step and no passphrase prompt: the store shells to gpg, and
the unlock belongs to your own agent, which may answer from its cache, from a
pinentry on your own terminal, or not at all. A decrypt the agent declined is
its own refusal carrying gpg's own words, told apart from an absent entry \u{2014}
absence is answered from the store's own listing of names, so a backup mapping
never writes over an entry it merely could not read. Before any mapping is
read, a declared store that is not one \u{2014} absent, or carrying no recipient
declaration \u{2014} refuses the run, naming the location.

A value carrying newlines is written and read back byte-identically, a trailing
newline included. There is no value-shape refusal for this target, and that is a
property of the transport rather than an omission: keepassxc's exists because
its own command reads a password as one line.

A two-way mapping's last agreement is a companion entry beside the mapped one,
named the mapped path plus `.safix-sync-state`, written as its own second write
strictly after the value's. Evaluation refuses a declared path carrying that
suffix, and each mapping accounts for its own companion, so a mapping's memory
is never reported as an entry nothing declares.

Each pass-target mapping is reported as unchanged, updated, pulled, fields
updated, fields diverged, conflict, refused, or not judged, and every declared
mapping appears whatever happened to it. A diverged field is named and never
printed. SAFIX_PASS points safix at the command to run.

\u{2500}\u{2500} the bitwarden target: the four modes \u{2500}\u{2500}
The mode is declared per mapping, not passed here, for the reason the keepassxc
target's own modes are.

  safix-to-bitwarden   the vault converges to safix's value. A vault-side edit to
                       a mapped item is overwritten, and reported.
  bitwarden-to-safix   safix converges to the vault's value, through the same
                       path `safix set` writes through: the same empty-value
                       refusal, the same recipient-drift refusal, the same staged
                       write and rename, and a commit naming the mapping.
  two-way              whichever side changed since the last agreement wins.
                       Both changed is a conflict.
  backup               safix's value is written where the vault holds no such
                       item, and a vault value that differs is never overwritten
                       \u{2014} the divergence is reported instead.

No mode deletes an item, a folder or a field, under any circumstances.

\u{2500}\u{2500} the bitwarden target: the client is unlocked, never logged in \u{2500}\u{2500}
Before any mapping's side is read, the client is asked for its own state. A
client that is not logged in refuses, naming logging in as your own act: safix
unlocks a vault and never authenticates one. A locked client with a terminal is
unlocked by one master-password prompt, and a locked client with none refuses
rather than prompting into the void. The master password travels the client's
standard input alone \u{2014} never an argument, never the client's own
password-from-environment option, which is not used. A run that unlocked the
client locks it again when it finishes; a run that found it unlocked leaves the
session as it found it.

The session key the unlock returns travels in the environment of the children
that need it, and in no argument vector, no file safix writes and no output. That
is a deliberate narrowing of the rule every other target keeps unconditionally,
and the reason is that this client offers no third channel: an argument vector is
readable by every process on the machine, an environment by the same user alone,
so the key goes in the lesser exposure rather than in none. No mapped value and
no master password is in either, on any invocation.

flake.safix.bitwarden.server is optional: the client already holds that
configuration, so an undeclared server is a working configuration rather than a
missing declaration. A declared server that is not the one the unlocked client
reports reaching refuses before any side is read, naming both \u{2014} a write against
the wrong vault is not correctable by a later run.

\u{2500}\u{2500} the bitwarden target: one refresh, and one item write \u{2500}\u{2500}
The client's reads answer from a local copy, so exactly one refresh runs after
the unlock and before the first read. A failed refresh refuses every mapping on
this target: a stale copy would report agreement that is not there, and a backup
mapping would write into an item that already holds a value.

An item is addressed by an optional folder and its name, never by the vault's own
item identifier, and an address that matches more than one item is refused naming
the count \u{2014} nothing picks between them. An address that matches none is created
by a pushing mode and refused by bitwarden-to-safix.

The value, the declared username, url and notes, and a two-way mapping's recorded
state all cross as one base64 JSON payload on standard input; no payload is ever
a positional argument. A write onto an existing item begins from that item's own
current content, because this client replaces a whole item \u{2014} so a totp, a second
website or your own custom field survives a value repair. Tags are refused at
evaluation rather than approximated: this vault has no tag concept, only folders
and collections, and both are placements rather than labels.

A value carrying newlines is written and read back byte-identically. There is no
value-shape refusal for this target, and that is a property of the transport
rather than an omission.

A two-way mapping's last agreement is a hidden custom field of the mapped item
itself, named safix-sync-state, written in the same item write as the value. So
this target reserves no item name: the companion object keepassxc needs exists
only because its own command cannot write a custom field. An unreadable or
unrecognised recorded state is treated as absent, which takes the mapping to
bootstrap semantics rather than to a refusal.

\u{2500}\u{2500} what sync's bitwarden target is \u{2500}\u{2500}
It converges a declared safix entry with one item in the operator's vault, per
mapping and per mode \u{2014} see flake.safix.bitwarden.mappings. Each mapping is
reported as unchanged, updated, pulled, fields updated, fields diverged,
conflict, refused, or not judged, and every declared mapping appears whatever
happened to it. A diverged field is named and never printed. Items under a
declared folder that no mapping declares are reported as information and never
removed.

No check of this repository runs the real client, and that absence is written
down where this target's checks live: the client cannot authenticate without a
network and a nix build has none. SAFIX_BW points safix at the command to run.

\u{2500}\u{2500} the 1password target: the four modes \u{2500}\u{2500}
The mode is declared per mapping, not passed here, for the reason the keepassxc
target's own modes are.

  safix-to-1password   the item converges to safix's value. An item-side edit to
                       a mapped value is overwritten, and reported.
  1password-to-safix   safix converges to the item's value, through the same
                       path `safix set` writes through: the same empty-value
                       refusal, the same recipient-drift refusal, the same staged
                       write and rename, and a commit naming the mapping.
  two-way              whichever side changed since the last agreement wins.
                       Both changed is a conflict.
  backup               safix's value is written where the item does not exist or
                       holds none, and an item value that differs is never
                       overwritten \u{2014} the divergence is reported instead.

No mode deletes an item, a field or a vault, under any circumstances.

\u{2500}\u{2500} the 1password target: one channel, and all four fields \u{2500}\u{2500}
The value and all four declared fields \u{2014} the username, the url, the notes and
the tags \u{2014} travel standard input, as one JSON item payload per command. No
field=value assignment statement is ever spelled: 1Password's own documentation
states such a statement is recorded in shell history and can be visible to other
processes. So no declared field of this target is refused for want of a channel,
and a field whose source is another entry of the mapping's own person is
admissible on every one of the four. A declared url becomes the item's own
autofill website rather than a similarly named custom field, because the custom
field is not what a browser fills from.

A value carrying newlines is written and read back byte-identically. There is no
value-shape refusal for this target, and that is a property of the transport
rather than an omission: the refusal keepassxc has exists because its own
command reads a password as one line.

\u{2500}\u{2500} the 1password target: the session is yours \u{2500}\u{2500}
Before any mapping's side is read, one invocation establishes that the service
answers and the session is authenticated; a signed-out run refuses there, naming
the declared account when one is declared and carrying the command's own words,
with safix's own side of no mapping decrypted. safix signs nothing in, mints,
holds, prints and stores no session token, and never puts one in an argument
vector \u{2014} what the child process inherits is whatever your own environment
carries, a service-account token or a session your own `op signin` established.
flake.safix.onepassword.account is optional: `op` resolves its own default
account and an automation token names one implicitly, so an undeclared account is
a working configuration rather than a missing declaration.

A two-way mapping's last agreement is a concealed custom field of the mapped
item itself, named safix-sync-state. So this target reserves no item name and has
no refusal keeping one out of your reach: the companion object keepassxc needs
exists only because its command cannot write a custom field. An unreadable or
unrecognised recorded state is treated as absent, which converts the mapping to
bootstrap semantics rather than to a refusal.

A write onto an existing item begins from that item's own current content and
replaces only the value, the declared fields and the recorded state; safix never
constructs an item from a template. 1Password's documentation carries an explicit
danger that a template-assembled edit loses the passkeys on an item, and safix
cannot reach that outcome \u{2014} so no refusal about passkey-bearing items exists,
and an ordinary login item carrying one stays mappable.

A failure against the service refuses its own mapping and does not end the run:
every declared mapping still appears in the report, and the run exits non-zero.
Items in a declared vault that no mapping declares are reported as information
and never removed.

No check of this repository runs the real command, and that absence is permanent:
the package is unfree, there is no self-hostable server to point a sandboxed node
at, and every authentication path needs the network. SAFIX_OP points safix at the
command to run.
";

/// `safix get -h`.
pub const GET: &str = "\
safix get [<user>] <name>

Decrypt that one key to stdout. The output is plaintext by design and is meant
for piping. It needs an identity that opens the file, which is the owner's or a
recovery identity theirs names.

`get` is the pipe. `safix view` is the verb that shows a value on a terminal and
can offer a choice of what to show; a script wants this one.
";

/// `safix view -h`.
pub const VIEW: &str = "\
safix view [--no-preview] [<user>] [<name>]
safix view [--no-preview] [<user>]

Decrypt one key to the terminal. `safix get` writes the value to stdout and is
what a pipeline calls; this writes it to the terminal, so a pipeline wants
`get`. With no terminal to write to, the value goes to stdout instead \u{2014} what
needs a terminal here is offering a choice, not writing a value.

With no <name>, every entry <user> holds is offered for selection, showing seven
of the eight columns `safix list` prints: the name, where it came from, whether
one value serves every carrier, whether a generator mints it, the key it is read
under, and when the value was created and last updated. Tab adds the eighth, the
file serving it. Choosing is a way of naming: the run proceeds exactly as though
the chosen name had been given.

The list is drawn from the bottom of the screen upwards, alphabetically, with
the first entry as the bottom row under the cursor: the rows nearest the query
being typed are the ones a query is narrowing towards. Typing narrows without
reordering, so an entry does not move under the cursor between two keystrokes.

A lone argument is a user when flake.safix.users declares one by that name, and
an entry's name otherwise. An entry whose name is also a person's is reachable
by naming both.

\u{2500}\u{2500} the keys \u{2500}\u{2500}
  enter          read the entry under the cursor
  escape, ^C     leave without choosing
  up, down       move the cursor; up is later in the alphabet
  left, right    scroll the columns; neither leaves and neither wraps
  tab            show or hide the FILE column
  ^P             show or hide the value pane
  backspace      edit the query
Every other key is consumed and does nothing, including every function key and
every editing key a terminal spells as an escape sequence: a key this does not
bind cannot put bytes in the query and cannot leave the list.

\u{2500}\u{2500} the query \u{2500}\u{2500}
KeePassXC's syntax. Terms are separated by whitespace and all of them have to
match; \"two words\" is one term. A term may name a column and may be modified:

  token                    a substring of any column, case-insensitive
  name:token, n:token      the same, in that column alone \u{2014} name, origin,
                           shared, generator, key, created, updated, file, each
                           also by its initial
  !token                   entries this term does not match
  +Token                   the whole cell, case included
  *^api-.*$                a regular expression
  api-*, api-toke?         * is any run and ? is one character, whole-cell
  api-token|mail-password  either, whole-cell

A regular expression that does not compile matches nothing rather than refusing
the frame, because every prefix of one being typed is a query this has to answer.

\u{2500}\u{2500} the preview \u{2500}\u{2500}
The value under the cursor is decrypted and shown in the pane once the cursor has
rested, so moving through twelve entries decrypts none of the eleven passed
through. Exactly one decrypted value is held at any moment: the previous one is
dropped and overwritten before the next is read. The rendering is bounded to the
pane, control bytes are shown as placeholders, and a value that is not valid text
is described by its size. It is drawn in a region the terminal clears on exit, so
no value enters scrollback, and nothing is staged: no plaintext of a previewed
value reaches a file at any point. A value that does not decrypt is reported in
the pane and the list stays usable \u{2014} failing to show one value says nothing
about choosing another.

^P and --no-preview both suppress it, and a suppressed preview decrypts nothing
at all until a choice is made \u{2014} for a shared screen, a recording, or a session
whose scrollback you do not control.

\u{2500}\u{2500} what the colours mean \u{2500}\u{2500}
  green    the decrypted value
  yellow   the entry you chose last, as this user
  cyan     the entry created most recently
  dim      a cell with nothing in it

\u{2500}\u{2500} what is remembered \u{2500}\u{2500}
Whether the pane and the extra column were showing, and what you chose last, in
${XDG_STATE_HOME:-$HOME/.local/state}/safix/picker.json, created 0600. It is
written on every toggle and on every choice, and a file that does not parse is
ignored and overwritten rather than refused.

\u{2500}\u{2500} the three refusals \u{2500}\u{2500}
  no terminal to choose on   name the entry, or `safix list` what the user holds
  the user holds nothing     a state of the declarations, not of the session
  left without choosing      one line, safix::selection_cancelled, and exit 1:
                             nothing written, nothing decrypted kept, the
                             terminal as it was found
";

/// `safix list -h`.
pub const LIST: &str = "\
safix list [<user>]

Every name <user> holds, where it came from, whether it has a generator, the key
it is read under, when the value was created and last updated, and the file
serving it.

The GENERATOR column shows a generator's own description when it has one, `yes`
when it has a generator and no description, and `-` when the value can only be
typed or transcribed.

CREATED and UPDATED are read from the record written beside each value, in local
time. They show `-` for a value written before that record existed: no record is
no claim about when a value arrived, and a date invented for one would be worse
than none.
";

/// `safix check -h`.
pub const CHECK: &str = "\
safix check [<user>]

Report drift and change nothing. Exits non-zero when there is any, and each
finding prints the command that resolves it. Five classes:

  - the committed .sops.yaml against the policy the declarations imply
  - each governed file's recipients against the audience declared for it
  - declared names with no value, saying which have a generator
  - values in a governed file that no declaration claims
  - generated values minted under a generator that has changed since

`fix` handles the first two. The last three are decisions rather than
convergences — a value is minted or typed, an unclaimed one is declared or
deleted, and a value whose generator has changed is either regenerated or the
edit is reverted — so nothing here does them for you.

The last class is answered from the generator-record tree — one plaintext file per
generated value, under `flake.safix.storage.generatorRecords`, which defaults to
state/safix/definitions/. `generate` writes a digest of the definition it minted
under there, in the same commit as the value. A value with no record predates the
record and is not a finding: no record, no claim. A record whose format tag this
version does not write gets the same answer, which is what keeps a change to what
the digest covers from reporting the whole tree as drifted.

It needs no identity for any file it examines: every question above is answered
from the document's structure and from that plaintext record, and nothing on this
path decrypts.
";

/// `safix generate -h`.
pub const GENERATE: &str = "\
safix generate [--regenerate] [--yes] [--allow-disk-staging] [<user>] [<name>]

Run <user>'s generators, in the dependency order the declarations compute, for
every declared secret with no value yet. --regenerate re-runs over values that
already exist, which is the rotation affordance.

With no <name>, every generator that has something to mint runs. Naming a secret
runs the one generator that writes it; naming either half of a multi-output
generator runs the generator that mints both, and both land in one commit.

A single argument that names a declared user selects that user and runs all of
their generators; anything else is read as a secret's name.

\u{2500}\u{2500} --regenerate cascades \u{2500}\u{2500}
Rotating a named generator also re-runs every generator that reads what it
writes, transitively, in the same dependency order. Otherwise a rotation would
leave values derived from the value it replaced, and nothing afterwards can tell
a hash of a retired password from a hash of the current one \u{2014} the tree records
no run that a value came from.

The set is listed before anything runs and confirmed, because each re-run
commits as it goes and declining afterwards takes nothing back out of history.
--yes answers that confirmation in advance. A generator nothing reads is not a
cascade and asks nothing.

\u{2500}\u{2500} what a generator script sees \u{2500}\u{2500}
A staging directory, with the script's working directory at its root:

  $out/<name>              write each declared output here
  $prompts/<name>          one answered prompt per file, when any are declared
  $in/<generator>/<name>   a dependency's plaintext, keyed by its producer

This is the interface clan's own generators are written against, so a script
written for either system runs under the other. Only the dependencies a
generator declares appear under $in \u{2014} clan places every file of the dependency
generator, which would hand a script depending on a keypair's public half the
private half as well.

Every declared output must exist when the script exits. A missing one refuses
the whole run and lists what $out did contain, and nothing is written until all
of them are present. Bytes are stored exactly as written: `echo` leaves a
trailing newline and `printf` does not, and nothing here removes one.

`runtimeInputs` is prepended to PATH, and inside the envelope below it is the
whole of what a script can run: name every tool, because the paths PATH
otherwise carries are not there.

An output declared `files.<name>.secret = false` is written to the repository in
the clear under public/, is given no creation rule, and is readable at
evaluation. That is what a public key or a fingerprint is for.

\u{2500}\u{2500} where the plaintext is \u{2500}\u{2500}
The staging directory is mode 0700 on a filesystem this command asks the kernel
about rather than infers from its name, and it is overwritten and removed
however the run ends \u{2014} on return, on error, on panic, and on interrupt or
terminate. There is no fallback to /tmp: on a host whose /tmp is disk-backed a
silent fallback would put plaintext in free blocks under a code path that looks
like it succeeded. Where no memory-backed filesystem is available the run
refuses, and --allow-disk-staging accepts a disk-backed one. SAFIX_STAGING_DIR
names the mount to use instead of the conventional ones \u{2014} it replaces them
rather than being tried first, so a mount you named and this rejects is a
refusal rather than a silent fall back to somewhere else \u{2014} and it is
verified like any one of them.

What that bounds, and what it does not. Overwriting a page of a memory-backed
filesystem does not reach a copy already written to swap. A mode-0700 directory
is readable by every process running as you for the length of the run, where the
pipe this replaced was readable by neither a third process nor a shell.

What the directory no longer has to carry alone is the script. It runs inside a
sandbox, so a copy of $in/dep/name to somewhere this command does not look fails
rather than succeeding quietly.

\u{2500}\u{2500} the envelope \u{2500}\u{2500}
A script and its validation fragments run inside a sandbox: the staging root is
the only writable path, the nix store is read-only, and there is no network. The
envelope is clan's own \u{2014} bubblewrap on linux, sandbox-exec on darwin \u{2014} so a
fragment meets the same confinement under either system's default executor. A
validation fragment gets it with no writable path at all, because the staging
root has been shredded by the time a candidate is judged; the candidate still
arrives on standard input.

`network = true` on the generator re-shares the network and nothing else, for
the script and the validation alike. The filesystem confinement stays, so what
remains yours to get right is what a granted connection carries.

There is no --no-sandbox and nothing spelled otherwise. Where no backend runs,
this refuses before the first fragment and names what it looked for.

\u{2500}\u{2500} under --entry \u{2500}\u{2500}
The sandbox above resolves its own tools through nix shell, a flake-only
operation. Run under --entry with something to mint and neither --nixpkgs nor
SAFIX_NIXPKGS declared, this refuses before the sandbox is probed, naming both
remedies: drop --entry and run against the declaring flake, or add --nixpkgs
<flake-ref>. A user with nothing to mint is unaffected either way.

Standard error and standard output both reach you, so diagnostics are free and
neither is a value.
";

pub const KEYGEN: &str = "\
safix keygen [--for-someone-else] [<user>]
safix keygen --show
safix keygen --kind age
safix keygen --kind pgp --uid <identity> [--expires 2y]

Mint an age identity and append it to ${XDG_CONFIG_HOME:-$HOME/.config}/sops/age/keys.txt,
then print the public half and what to do with it. The private half is never
printed.

It appends and never truncates: sops tries every identity in that file, so a
second identity beside a first is a working state, and overwriting is how
someone loses the key to everything they hold.

Run it on your own machine, as yourself. Minting another person's identity here
means you hold their private key, which is the opposite of the independent
custody this package rests on, so it takes an explicit --for-someone-else.

An existing ssh key works instead: `ssh-to-age < ~/.ssh/id_ed25519.pub` prints
the age recipient for it, and safix.identity.sshKeyPaths names the private half.

The standalone --kind forms need no repository. Native age is the default key
profile. The GnuPG profile uses an expiring Ed25519 certification primary and
cv25519 encryption subkey; pinentry handles the passphrase. Its protected
keyring is SAFIX_GNUPGHOME or $XDG_DATA_HOME/safix/gnupg (defaulting to
$HOME/.local/share/safix/gnupg). Existing compatible keys are not weakened or
replaced. Use identity backup with an independent recovery identity before
relying on a new key.
SAFIX_AGE_KEY_FILE overrides the managed age file. Key files use mode 0600;
private directories use 0700. Symlinked paths, repository paths and Nix-store
paths are refused.

\u{2500}\u{2500} --show \u{2500}\u{2500}
Prints the public recipient derived from the identity already minted on this
machine, and mints nothing: no identity, no line appended to keys.txt, no
write of any kind. Refused, naming plain keygen as the remedy, when no
identity has been minted here yet.
";

/// `safix migrate -h`.
pub const MIGRATE: &str = "\
safix migrate <plan.json>

Execute a version-1 migration plan. Paths are relative to the plan's directory.
Each entry names source and destination {path, format, key}, public recipients,
and deployment metadata. Supported formats: yaml, json, dotenv, ini, binary,
and age. An empty key selects the whole document.

The plan names sourceIdentities, targetIdentities, receipt, deploymentOutput,
and deploymentTarget (safix, sops-nix or agenix). Identities are explicit:
ageKeyFile, ageSshKeyPaths and gnupgHome. Ambient keys cannot satisfy target
verification. Every candidate must decrypt byte-identically before any output
is published. Existing outputs are refused. Sources are always retained.
Unsupported target deployment semantics are refused rather than dropped.
Templates use <safix:secret-name> references. Agenix cannot retain templates,
service hooks or early-user metadata. Native sops-nix needs whole binary files
for binary or intentional empty values, not Safix's keyed byte envelope.
Recoverable errors roll back new outputs; a process crash may leave a subset
of verified artifacts. Sources remain the recovery path.
";

/// `safix identity -h`.
pub const IDENTITY: &str = "\
safix identity backup <age|pgp> <destination> --recipient <public-key>... [--age-key-file <recovery-key>] [--gnupg-home <recovery-keyring>]
safix identity restore <backup> [--age-key-file <recovery-key>] [--gnupg-home <recovery-keyring>]

Back up the local managed identity to an encrypted SOPS binary package, then
verify it with explicitly selected independent recovery identities. Public
GnuPG recipients use pgp:FULL_UPPERCASE_FINGERPRINT. A self-only backup is
refused: a key cannot be its own recovery path. No private key is printed or
passed as a command argument. Destinations must not already exist.

Restore validates every private primary and subkey before importing. Existing
age files and colliding GnuPG fingerprints or private keygrips are refused,
including private files absent from the public keyring. Backup refuses shared
recovery keygrips, unaccounted-for private files, hardware stubs and incomplete
primary-key exports.
Protect the recovery identity separately; safix does not escrow it for you.
";

/// `safix adduser -h`.
pub const ADDUSER: &str = "\
safix adduser <name> <recipient> [--host <hostname>]... [--yes]

Declare a person who holds nothing yet: write safix/users/<name>.nix,
regenerate .sops.yaml from the policy that declaration implies, commit the two,
and then hand the name and the recipient to flake.safix.onboardingHook.

  --host H    passed through to the hook, repeatable. Refused when no hook is
              configured, because attaching an account on a host is a property
              of a consumer's module tree and safix has none.
  --yes       skip the confirmation.

<recipient> is theirs: a native age recipient, an SSH public key, or
pgp:FULL_UPPERCASE_FINGERPRINT. Only its shape is checked here; whether anyone
holds the private half is not knowable from this machine. An SSH key with a
comment must be passed as one quoted argument.
A recipient that needs physical interaction is refused for this field:
activation decrypts non-interactively and a card needs a touch, so it belongs in
that person's recoveryRecipients instead, where it is additive.

\u{2500}\u{2500} what this does not do \u{2500}\u{2500}
Mint anything. Key creation is `keygen`, run by them on their own machine.
This command creates no password material and no secret value.

Give them anything to hold. The scaffold declares no secret, so no audience is
computed for them and the regenerated .sops.yaml carries their key as an anchor
with no creation rule yet. Their first secret is a name under `private` or
`carries`, then `safix fix` to write the rule, then `safix set`.

Anything about hosts, accounts, identifiers or groups. Those are one consumer's
module tree, reached through the hook and nowhere else. A hook receives:

    $1  the new person's name
    $2  their recipient
    $3\u{2026} every --host given, in order

and runs after the scaffold and the policy are committed, so whatever it writes
is its own to stage. Its absence is a supported configuration: onboarding
without a hook succeeds, having done less.
";

/// `safix enroll -h`.
pub const ENROLL: &str = "\
safix enroll [<user>] [--serial <n>] [--slot <n>] [--no-store-pin]
             [--mirror-to-store] [--store-database <path>]
             [--pin-policy <p>] [--touch-policy <p>] [--allow-disk-staging]
             [--trust-declared-recipients]

Take one hardware key from a blank card to a proven recovery identity for
<user>, in one verb. A touch is the only thing you do.

Before selecting a card, inspect every governed file and refuse undeclared
recipients. Raw-age files require --trust-declared-recipients: their encrypted
roster is opaque, so this explicitly authorizes the declared roster instead.

What it does, in order:
  1. select the card. One connected is taken; two are refused naming both
     serials and --serial.
  2. provision PIV access when the card is factory-fresh: a generated PIN, a
     generated and distinct PUK, and a random management key put on the card
     under the PIN. The management key is stored nowhere, because PIN
     possession is management possession. A card already provisioned is not
     re-provisioned and its PIN is asked for once, unechoed.
  3. generate an age identity in the first empty retired slot, named for the
     person and the serial, under a pseudo-terminal that supplies the PIN.
     THIS is where you touch the card.
  4. append the identity block to the same identity file `safix keygen`
     appends to. It holds no private key: the key is on the card.
  5. add the card's recipient to <user>'s recoveryRecipients, regenerate
     .sops.yaml, re-wrap every governed file, and commit the three together.
  6. register the recipient with clan, through clan's own command, when
     flake.safix.bridge.clanFlake is set; then run flake.safix.enrollHook.
  7. store the generated PIN and PUK as <user>'s own safix secret, named for
     the serial. --no-store-pin skips this.
  8. prove it: open one governed file in <user>'s audience with the card's
     stub as the only identity reachable. An enrollment without this has no
     evidence beyond a public string having been copied correctly.

\u{2500}\u{2500} the flags \u{2500}\u{2500}
  --serial <n>          which card, required when two are connected
  --slot <n>            a retired slot to use instead of the first empty one
  --no-store-pin        do not store the generated PIN and PUK in safix
  --mirror-to-store     also write them to the password store: the session's
                        secret service when it answers, with no prompt at all
  --store-database <p>  the kdbx to add the entry to when the service does not
                        answer, through keepassxc-cli with one password prompt
  --pin-policy <p>      default once
  --touch-policy <p>    default cached; never is refused
  --allow-disk-staging  accept a disk-backed filesystem for the proof's
                        identity source, where no memory-backed one is found
  --trust-declared-recipients
                        authorize declared audiences for raw-age files; this
                        does not verify their previous recipient rosters

\u{2500}\u{2500} everything here is additive \u{2500}\u{2500}
A recipient is appended, an identity block is appended, a name is declared.
Nothing is removed and nothing is replaced, on any path. A backup key is this
same verb run again: each card gets its own identity and its own recipient,
and neither run knows about the other. A re-wrap that dropped a recipient a
file had before the run is refused rather than committed.

\u{2500}\u{2500} what is refused, and why \u{2500}\u{2500}
No OTP slot is written, under any flag. A programmed challenge-response slot
is what opens a password database, and the database has no record of the
secret it was built with, so writing that slot ends it permanently. Asking is
refused with that named.

touch-policy never is refused. The touch is the property a card is for.

A run with no terminal is refused before the card is touched: somebody has to
touch it and somebody has to be told when.

The primary `recipient` stays software-only. Activation decrypts with nobody
present, so a card belongs in recoveryRecipients and `safix adduser` refuses
one for the other field.

\u{2500}\u{2500} where the PIN ends up, and what that is worth \u{2500}\u{2500}
In <user>'s own custody by default, encrypted to the recipients they already
hold. The honest caveat: a PIN readable by the software identity adds
protection only once that identity is retired or absent. The default is there
to make starting easy, not to claim a property it does not have, and
--no-store-pin turns it off.

The password store is the optional second home and the reason it exists is
that a credential living only inside the thing it unlocks has a cycle in it.
The database opens by challenge-response with no PIV PIN involved, so the
card's PIN is reachable with the card in hand and no self-reference.
";

/// `safix group -h`.
///
/// The last paragraph is [`safix_core::delegation::BOUNDARY`] word for word, and a
/// test holds it to that string: it is the one sentence every delegation surface
/// carries, and a second wording of it here would be a second answer to what the
/// refusals are for.
pub const GROUP: &str = "\
safix group add|remove <group> <subject>

Edit one group's declared membership: one name inserted into or removed from the
`members` list in safix/groups/<group>.nix, parsed before anything is staged,
with .sops.yaml regenerated from the declarations that edit implies and the two
committed together.

It writes no value, encrypts nothing and re-wraps nothing. A membership change is
a reason to run `safix fix`, which is what re-wraps every file the group's
audience names, and the report says so.

\u{2500}\u{2500} what remove does not do \u{2500}\u{2500}
Take anything back. A subject that has been in a group has held the data key of
every file that group's audience names, so they have read every value in them,
and no re-wrap unreads it. Only minting a new value revokes.

`safix check` reports the shrink as the revocation it is, with rotation named as
the remedy. `safix fix` aligns the ciphertext with the policy now declared, which
is worth doing and is explicitly not that remedy.

\u{2500}\u{2500} what is refused \u{2500}\u{2500}
A group or a subject the declarations do not name, before anything is read: a
membership naming either is refused at the next evaluation, so writing one would
commit a tree that no longer resolves. An organization is refused as a member for
the same reason — a principal is not a member, and an audience wanting its
custody names the organization.

A `members` value this cannot read is refused rather than compounded. What it
reads is a list of names, written as one line or as one name per line; a value
computed elsewhere is a declaration to edit by hand.

A membership that would form a cycle among groups is refused at evaluation, with
every participant named. This verb parses what it writes and does not evaluate
the fleet, so that refusal arrives at the next build rather than here.

\u{2500}\u{2500} the delegation \u{2500}\u{2500}
Where a group is covered by an organization's silo declarations, only that
organization's managers may edit it, judged against the identity the resulting
commit will carry. A group no silo set names is covered by nobody and is editable
by whoever can commit, exactly as before.

These refusals bind the cooperative path and are not authorization. The tree is
the authorization: anyone who can commit can edit these declarations by hand,
evaluation refuses structure rather than people, and no delegation record places
a key in any audience. What they buy is that a scaffold and the identity it is
attributed to cannot disagree.
";

/// What a bare invocation says, and what `safix -h` prints.
///
/// The shell runtime's own general usage, word for word: this binary implements
/// every subcommand it lists.
pub const EDIT: &str = "\
safix edit [--allow-disk-staging] [--no-preview] [<user>] [<name>]

Open $VISUAL, or $EDITOR when that is unset, on <name>'s value. Neither set is a
refusal naming both: this command opens no editor of its own choosing, because
dropping you into one you did not pick with a secret in the buffer produces
either an accidental write or an accidental abandonment, and nothing here can
tell those apart.

The command is split on whitespace and run directly rather than through a shell,
so EDITOR=\"code --wait\" works. The staged file's path is an argument; the value
is not.

An entry that holds no value yet opens on an empty buffer, so this is an
authoring verb as well as an amending one.

\u{2500}\u{2500} what each outcome writes \u{2500}\u{2500}
  editor exits non-zero   nothing written, nothing committed
  buffer unchanged        nothing written, nothing committed
  buffer emptied          refused \u{2014} an empty value is what a truncated write
                          leaves behind
  buffer changed          written through the same path `safix set` writes
                          through, and committed

\u{2500}\u{2500} where the buffer is \u{2500}\u{2500}
Inside the same private staging directory `safix generate` uses: mode 0700 on a
filesystem verified to be memory-backed, removed however the run ends.
--allow-disk-staging accepts a disk-backed one where none is available.

What the editor leaves beside the buffer \u{2014} swap files, backups, undo history
\u{2014} goes with it, because what is removed is the directory and not the one file
this command made. An editor configured to write undo history or backups to a
directory of its own has put plaintext where this command does not look. That is
the limit of the containment, and it is stated here rather than left to be
discovered.

A public output is not editable here: it is already plaintext in the repository,
and the generator declaring it is what mints it.

\u{2500}\u{2500} with no <name> \u{2500}\u{2500}
Every entry <user> holds is offered for selection, the way `safix view` offers
them and through the same code, less every public output \u{2014} a public value is
not editable, so it is not among the choices either. The editor is settled
before the list opens: a refusal after you have browsed and had values
decrypted for a preview is a refusal that wasted your time. The keys, the query
syntax, the colours and the remembered state are `safix view`'s, and
--no-preview suppresses the preview here too.

The three refusals are `safix view`'s: no terminal to choose on, naming both
remedies; the user holds nothing editable; and leaving without choosing, which
writes one line, safix::selection_cancelled, and leaves the terminal as it was
found.
";

/// `safix upload -h`.
pub const UPLOAD: &str = "\
safix upload <machine> --directory DIR --identity PATH
safix upload <machine> --to ADDRESS [--identity PATH] [--force]

Seed a machine's own ed25519 host key before its first activation, so the
audience already wrapped to its declared recipient can be decrypted from the
moment it boots. safix mints no machine identity: --identity PATH names a
private key the operator already holds, and the command refuses before
writing anything when its derived recipient does not match the machine's
declared one.

\u{2500}\u{2500} --directory: a pre-seed tree, touching no network \u{2500}\u{2500}
Writes DIR/etc/ssh/ssh_host_ed25519_key at mode 0600 and
DIR/etc/ssh/ssh_host_ed25519_key.pub at mode 0644 \u{2014} the paths and modes a
fresh NixOS install's own sshd-keygen would produce \u{2014} for nixos-anywhere
--extra-files or for hand-copying onto installer media. Creates no other path
under DIR and makes no ssh connection.

\u{2500}\u{2500} --to: probe first, then no-op, write, or refuse \u{2500}\u{2500}
Reads the ed25519 host key ADDRESS currently presents, unauthenticated,
before writing anything:

  already the declared key     reports it and writes nothing, even with
                               --force and --identity both given
  no key presented             writes, given --identity; refused naming the
                               flag otherwise
  a different key presented    refused by default, naming both recipients;
                               --force together with --identity proceeds

A write streams a tarball built inside the same private staging root
generation and editing use \u{2014} files at mode 0400, directories at 0700, as
root \u{2014} and wipes the fixed destination before extracting into it. No value
is decrypted, encrypted or read on this path: what travels is the identity
material the operator supplied, once.

\u{2500}\u{2500} what this does not cover \u{2500}\u{2500}
Provisioning a person's own first identity. A machine name is all this verb
parses, and a person's name is refused the way an undeclared machine is.

A systemd-credentials delivery path for the same material. It does not exist
yet; --directory's output is a plain filesystem tree.

Any deploy, switch or rebuild. Nothing here triggers one: the machine's own
next rebuild is what activates what was written here.
";

/// `safix install -h`.
pub const INSTALL: &str = "\
safix install <manifest> [--check-mode=off|manifest|document] [--ignore-passwd]
                         [--dry-run]

Install a manifest using raw age or SOPS YAML, JSON, dotenv, INI or binary.
Decrypt with configured age, SSH or GnuPG identities, render templates in a
private generation, publish it, then run changed-value service hooks.

This is the verb an activation runs, not one an operator types. A NixOS
activation script and a home-manager activation entry each invoke it against a
manifest safix's own nix half built; nothing about a person's own workflow
reaches it, and the manifest is the whole input \u{2014} including whether this is a
user-scope install, which is a field of the manifest rather than a flag here.

\u{2500}\u{2500} --check-mode: how much of the manifest is verified \u{2500}\u{2500}
  off        the default: install, checking each step as far as it needs
  manifest   validate the schema, the version, every mode's octal parse and
             every owner and group, then stop. Opens no document at all
  document   everything manifest does, and additionally open each distinct
             document and verify that every declared key is in it

Neither check mode decrypts anything, and neither needs a key. The document
mode inspects public ciphertext structure. SOPS keeps mapping keys in the
clear; raw age supports only a whole document, whose header is inspected.
This build-time validation needs no identity. It does not prove that runtime
identities can decrypt the document.

\u{2500}\u{2500} --ignore-passwd \u{2500}\u{2500}
Skip every user, group and keys-group lookup and set ownership to 0. What a
check inside a nix build needs, where none of those users exist, and what a
dry activation gets too.

\u{2500}\u{2500} --dry-run \u{2500}\u{2500}
Validate, decrypt and render into private staging without publishing live or
external links, pruning generations or invoking service hooks. The candidate
is removed afterwards. NIXOS_ACTION=dry-activate implies this mode.
";

pub const SCAFFOLD: &str = "\
safix \u{2014} secrets by declaration name, with explicit migration and key custody.

  safix set      [<user>] <name>                    write a value you type
  safix edit     [<user>] [<name>]                  author a value in your editor
  safix get      [<user>] <name>                    decrypt one key to stdout
  safix view     [<user>] [<name>]                  browse and read, with a preview
  safix list     [<user>]                           every name a user holds
  safix generate [--regenerate] [--yes] [<user>] [<name>]
                                                    mint values from generators
  safix check    [<user>]                           report drift, change nothing
  safix fix      [--yes]                            converge policy and ciphertext
  safix audit    [<target>] [<mapping>...]          report bridge or mirror drift
  safix sync     [<target>] [<mapping>...]          converge declared relationships
                 <target> is clan, keepassxc, pass, bitwarden or 1password
  safix keygen   [--kind age|pgp] [...]             create a protected local identity
  safix migrate <plan.json>                        convert and verify before publishing
  safix identity backup|restore [...]              verified independent recovery
  safix adduser  <name> <recipient> [...]           declare a person who holds none
  safix enroll   [<user>] [--serial <n>] [...]      a hardware key, proven
  safix group    add|remove <group> <subject>       edit a group's membership
  safix upload   <machine> --directory DIR | --to ADDRESS
                                                    seed a machine's host identity
  safix install  <manifest> [--check-mode=...] [--dry-run]
                                                    what an activation runs

\u{2500}\u{2500} global options \u{2500}\u{2500}
  --entry <file>          evaluate <file> instead of the repository's flake
  --nixpkgs <flake-ref>   generate's sandbox resolves its tools against this
                          flake reference instead of the declaring one

SAFIX_ENTRY and SAFIX_NIXPKGS set the same two, and --entry and --nixpkgs win
when both a flag and its variable are given. Every subcommand but generate
behaves identically under --entry as against a flake; generate is
the exception and refuses under --entry with neither --nixpkgs nor
SAFIX_NIXPKGS set, naming both remedies. Neither option changes where a run
stages or commits: that root is still the one git reports for the current
directory, or the one SAFIX_REPO_ROOT names instead.

SAFIX_VAULT_ROOT names a second, mutable working tree for a declared
flake.safix.vault: unset and unread when no vault is declared, and
required whenever one is \u{2014} every ciphertext write, public output and
generator definition record lands there instead of the declaration root.

<user> defaults to $USER when flake.safix.users declares them, and otherwise to
the sole declared holder when there is exactly one.

The file, the key inside it and the recipients all come from
flake.safix.lib.placements. A name no declaration covers is refused rather than
given a destination.

`safix <subcommand> -h` explains one of them.

\u{2500}\u{2500} narrowing an audience is not revocation \u{2500}\u{2500}
Removing someone from an audience stops future encryptions reaching them. It
takes nothing back: they have already read every value in every file they could
open, and only minting a new value revokes it. `safix fix` re-wraps each
governed file's data key to the audience now declared, which aligns ciphertext
with policy and is explicitly not revocation.

\u{2500}\u{2500} who may scaffold for whom \u{2500}\u{2500}
`flake.safix.organizations.<o>.managers` names the people who scaffold for an
organization, and a person's own `flake.safix.users.<u>.managedBy` consents to
that. Where both are declared, `safix enroll` and `safix group` accept those
managers and refuse anybody else, judged against the identity the resulting
commit will carry — there is no flag naming somebody else, because a scaffold and
its attribution must not be able to disagree.

Those refusals bind the cooperative path and are not authorization. The tree is
the authorization: anyone who can commit can edit these declarations by hand,
evaluation refuses structure rather than people, and no delegation record places a
key in any audience. A person no organization manages is scaffolded by whoever can
commit, exactly as before.

\u{2500}\u{2500} what sync's clan target is, and is not \u{2500}\u{2500}
`sync clan` moves values across the clan boundary, one declared mapping at a
time, each in its own declared direction. It is not a plaintext dump and
restore: clan's own `vars export` writes a machine's whole vars folder to
plaintext on disk, and that tree outlives the migration that justified it,
on a disk \u{2014} the shape this command exists to avoid. There is nothing here
that writes one.

`safix audit` is the report over the same declarations: it compares both
sides of every mapping, on either target, and writes nothing. It is separate
from `check` because it needs what `check` refuses \u{2014} a decryption of the
safix side, and either clan or the password database's own password.

\u{2500}\u{2500} what sync's keepassxc target is, and how it differs from the clan target \u{2500}\u{2500}
`sync keepassxc` converges declared safix entries with entries in your
password database, which is where a credential a person types goes. Each
mapping declares its own mode \u{2014} one-way in either direction, two-way, or
backup \u{2014} so one run can push some entries and pull others. No mode deletes
anything on either side.

It is not a second bridge with a different far end. The clan target moves
values between two tools that both hold them for programs; this target ends
the drift between a value a program reads and the same value a person reads.
Its report is the same `audit`/`sync` verbs' own, over the second target,
rather than more rows in `check`.

\u{2500}\u{2500} what sync's bitwarden target is \u{2500}\u{2500}
`sync bitwarden` converges declared safix entries with items in the vault your
people read on a phone. Each mapping declares its own folder, item name, mode
and fields, so one run can push some items and pull others; no mode deletes an
item, a folder or a field.

The client is unlocked and never logged in: a client that is not logged in
refuses, naming logging in as your own act. The master password travels standard
input alone, and the session key the unlock returns travels the environment of
the children that need it and nothing else \u{2014} a deliberate narrowing, because
this client offers no third channel and an argument vector is the worse of the
two exposures. Exactly one refresh precedes the first read, and a failed one
refuses every mapping rather than comparing against a copy that may predate
another device's change.

An item is addressed by folder and name, never by the vault's own identifier, and
an address matching two items is refused rather than resolved. The two-way memory
is a hidden custom field of the mapped item itself, written in the same item
write as the value, so nothing is reserved and any item name is yours to declare.
Tags are refused at evaluation: this vault has no tag concept.

No check of this repository drives a real `bw`. The client cannot authenticate
without a network and a nix build has none, and that absence is recorded where
this target's checks live rather than left silent.

\u{2500}\u{2500} what sync's 1password target is \u{2500}\u{2500}
`sync 1password` converges declared safix entries with items in your 1Password
vaults \u{2014} the other place a person-read credential lives. Each mapping declares
its own vault, item, mode and fields, so one run can push some items and pull
others; no mode deletes an item, a field or a vault.

The value and all four fields travel standard input as one item payload, never
an argument vector. The two-way memory is a concealed field of the mapped item
itself, so nothing is reserved and any item name is yours to declare. The
session is the operator's: safix signs nothing in. A failure against the service
refuses its own mapping and does not end the run.

No check of this repository drives a real `op`. The package is unfree, there is
no self-hostable server, and every authentication path needs the network; each
of the three alone is sufficient, and none of them expires.

\u{2500}\u{2500} verbs retired, reserved, or narrower here than in clan \u{2500}\u{2500}
  export   retired permanently. clan's own vars export writes a machine's whole
           vars folder to plaintext on disk, which is the bulk dump safix's
           design refuses to build on either side of the boundary; sync clan
           moves one declared mapping at a time, encrypted, and always did.
  import   reserved rather than retired: a future, unbuilt feature \u{2014} ingesting
           a value from an external plaintext source one entry at a time,
           analogous to clan's own import-sops \u{2014} may use this word later.
           There is no scaffold and no partial parser for it yet.
  upload   moves only a machine's own host identity, once, before that
           machine's first activation \u{2014} not clan's ongoing vars-delivery verb
           of the same name. No verb here delivers a secret's value on an
           ongoing basis: activation already does, through `safix install`
           reading the manifest safix's own nix half built, once a machine
           holds the identity this verb seeds.
";
