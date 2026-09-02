# Design: audit's view of clan's namespace

Revisions are as named in `proposal.md`.
Every clan-core line anchor below was read at that revision, in `/home/sernl/ghq/git.clan.lol/clan/clan-core`.

Amended 2026-09-03 while proposing `rename-transfer-verbs`: vocabulary updated to the sync family; substance unchanged.
`audit` gains `clan` and `keepassxc` targets there; every invocation this design cites as bare `safix audit` or `audit <mapping>` becomes `safix audit clan` / `audit clan <mapping>`, since this change's own lingering report is the clan target's alone.

## Context

`clan vars list <machine>` is `clan_cli/vars/list.py`, registered with no flag of its own beyond the global `--flake` (`clan_cli/cli.py:85-90`, the same flag `vars get`, `vars set` and `vars check` already take in `clan.rs`).
It resolves `Machine(name=machine, flake=flake)`, loads every generator that machine's evaluated exports declare through `get_machine_generators` (`clan_lib/vars/generator.py:229-323`) — which is placement-transparent: a generator reaches a machine's exports whether it is `Shared`, `PerMachine`, or `PerExport`, because all three resolve into one machine's generator set before `list` ever sees them, so D6's three-way placement sum needs no special handling here — and prints one line per var, sorted by the line's own text (`clan_cli/vars/list.py:9-13`).

Each line is `Var.__str__` (`clan_lib/vars/var.py:79-88`): `f"{self.id}: ..."`, where `id` is always `f"{generator}/{file}"` (`clan_lib/vars/generator.py:308`, `clan_lib/vars/generate.py:411`) — exactly the string `Clan::var_id` in `clan.rs:142-144` already builds.
The state after the colon is one of three: `********` for a secret var that exists, the var's own printable value for a public var that exists, or `<not set>` for one that does not.
Critically, the secret branch never reads `self.value` — it returns the literal string `"********"` — so listing a machine's namespace never decrypts anything, matching `bridge-transfer`'s existing rule that reading a clan value happens only when a value is actually needed.
There is no `--json` output anywhere under `clan_cli/vars/` or `clan_lib/vars/` at this revision (checked by grep across both trees), so what `list` gives is a stable, line-oriented, `id`-first text format and not a structured one.

That is enough for this change and no more.
Every var's `id` sits before the first `": "` on its line and an `id` cannot itself contain `": "` — it is `generator/file`, and both halves are nix attribute names — so splitting on the first occurrence of that substring recovers the `id` exactly, and the state half, along with any public value it might carry, is discarded unread.
Nothing this change reports ever needs to know whether a var exists, only whether clan knows its id at all, so the state half being present or not-set is treated identically: a listed id is part of clan's declared namespace either way, whether or not a generator has been run yet.

What this cannot give, and what this change does not need: a machine-independent view of clan's whole namespace.
`clan vars list` names one machine at a time, and there is no sibling verb that lists vars across every machine in one call.
`clan machines list` (`clan_cli/machines/list.py:12-21`) exists and would supply the machine names, but D2 below is why it is deliberately not called.

## Goals / Non-Goals

Goals.
Give `audit` a way to notice a clan var that no currently declared mapping accounts for, so that removing a mapping is a visible act on both sides of the boundary rather than a silent one on the clan side.
Do this with exactly the read-only, delegate-only relationship to clan that `bridge-transfer` already requires of `get`, `set`, and `check`.
Reach parity with `sync::lingering`'s shape and its stated properties — information, not a finding; no mode deletes; reported every run — rather than inventing a second vocabulary for the same idea.

Non-Goals.
Deleting anything, on either side, ever, in this change.
Enumerating a clan machine no currently declared mapping names — see D2.
Anything about `clan secrets users`; `safix-bridge-real-clan` does not exercise it today and this change does not start.
The two-way agreement-memory mechanism `sync-clan-vars-two-way` builds inside safix's own store — it has no clan-side namespace footprint, so it is neither a dependency of this design's mechanism nor something this design's enumeration would ever see.
A machine-readable (`--json`) clan output; none exists at this revision, so the parsing this change does is against the line format described above, held against the real command in `modules/flake/checks/real-clan.nix`.

## Decisions

### D1. The state half of each listed line is read and discarded, never surfaced

`bridge-transfer`'s existing requirements state, in several places, that no report of this bridge names a value.
`clan vars list`'s line format puts a var's state — masked, printed, or absent — directly beside its id, so the parsing step that recovers the id sees the state too and throws it away rather than never reading it.
That is a deliberate, narrower claim than "never reads a value": for a public var, `list`'s own output already contains the value in plain text before safix ever runs, so safix reading and discarding it changes nothing about what is disclosed, and doing so is what lets one parse recover the id for every var uniformly, secret and public alike, without a branch on secrecy.
What matters, and what is held, is that nothing safix reports — the `lingering` list itself — ever contains anything but an id.

### D2. Enumeration is scoped to the machines the bridge's own mappings name, not to `clan machines list`

`bridge-surface` already states "one consumer bridges one clan" as a refusal against declaring more than one `clanFlake`.
It has never stated, and this change does not introduce, "one consumer may see every machine of that clan" — and a single clan repository backing several downstream consumers, each bridging a disjoint subset of its machines, is exactly the shape that sentence is agnostic to.
Calling `clan machines list` and enumerating every machine it returns would make one consumer's `audit` report on machines that consumer's own declarations never mention, which is a stronger claim about clan's namespace than this bridge has ever made about anything else in it — every other requirement in `bridge-surface` and `bridge-transfer` is scoped to what a mapping names.
So the set of machines enumerated is exactly `{ mapping.clan.machine for mapping in every currently declared mapping }`, drawn from `workspace.bridge()?.mappings` the same way `audit::selected` already does, and `clan machines list` is not called anywhere in this change.

The consequence, stated rather than discovered later: a machine whose last mapping is removed is no longer named by anything safix declares, and so drops out of this enumeration along with it.
Its vars do not become permanently invisible to the operator — `clan vars list <machine>` run by hand still shows them — but they stop being carried in `safix audit clan`'s report the same day the mapping that once named the machine is removed.
This is recorded as a Risk below rather than solved, because solving it means declaring a machine independently of any mapping, which is a new piece of surface this change was not asked for and `bridge-surface` does not currently have.

### D3. What counts as "accounted for" is computed from the clan triple alone, never from direction or mode

A var is claimed by a mapping when `(mapping.clan.machine, mapping.clan.generator, mapping.clan.file)` matches it — nothing about `mapping.direction` enters the comparison.
This is the reason `sync-clan-vars-two-way` landing first costs this change nothing: that change is expected to give `Mapping` a third relationship kind (mirroring keepassxc's `Mode`, per the shared program contract's D2), and a third kind that still carries one clan machine, one generator and one file needs no change to what "claimed" means here.
Only a design that let one mapping claim a *set* of clan triples, or claim one dynamically, would need to revisit this, and nothing proposed for that change does either.

### D4. The report lands on `audit`, not on a new verb, as a new field named the same as keepassxc's

`audit` already is "the report over the same declarations: it compares both sides of every declared mapping and writes nothing" (`audit.rs:1-8`, `usage.rs:111-113`).
Namespace drift is the same kind of fact — something true about the boundary that a person should see and nothing should act on — so it is a field on `audit::Report`, `lingering: Vec<String>`, named identically to `sync::Report::lingering` and holding the same kind of thing: self-describing strings, `"<machine> <generator>/<file>"`, in the exact format `audit::Finding::clan` already uses for a mapping's clan endpoint (`audit.rs:85-86`), so nothing about this report invents a new way to name a clan var.
It does not participate in `Report::is_clean` or change `audit`'s exit status, exactly as `sync::Report::lingering` does not participate in `sync`'s tally or `is_clean` — a run that finds ten vars nothing declares and every declared mapping agreeing still exits zero, because the exit status answers "did the mappings I compared agree," and this is a different question asked and answered alongside it.

Considered and rejected: a new verb (`safix enumerate` or similar).
It would need its own help text, its own exit-status story, and its own decision about whether it requires clan and refuses without it — all of which `audit` already has settled, correctly, for exactly this kind of read-only cross-boundary report.
A fourth thing to type and remember for a report that belongs beside the one `audit` already gives is a cost with no offsetting benefit.

### D5. Enumeration is scoped by `only`, matching what an operator narrowing `audit` to one mapping is asking for

`audit`'s existing `selected()` narrows to one mapping's comparison when `only` names it, and lingering is computed over the same narrowed set of mappings' machines rather than over every declared mapping regardless of `only`.
This is the one place this design departs from `sync::lingering`'s literal shape, which always computes over the whole declared `Keepassxc` regardless of a sync run's own `only`.
The reason for the departure is that clan's enumeration is a real subprocess call with a real, per-machine failure mode (D6), where keepassxc's is a single already-open database's own index; an operator who names one mapping to `audit clan` because clan is only reachable in some restricted way for that one machine should not have that narrowed request additionally require reaching every other machine the bridge happens to declare.
Naming no mapping restores the full behavior: every machine any declared mapping names is enumerated, exactly as an unnarrowed `sync` run enumerates the whole declared group.

### D6. A machine that cannot be listed stops the whole run, the same way an unreachable clan already does

`audit::run`'s own reasoning for why `Error::NoClanFlake`, `Error::ClanUnavailable`, and `Error::UnknownMapping` all stop the run before the first mapping is compared is stated in its doc comment: "a run that discovered them partway through would already have said 'agrees' about mappings it never looked at."
The same reasoning applies here without alteration: a lingering section that silently dropped one machine's contribution because its listing failed would read as complete while being partial, and "nothing lingers" is exactly the sentence a partial report can print by accident.
So a machine that cannot be listed — clan exits non-zero for a reason other than the ordinary empty case, which for a real machine should not happen, since every machine enumerated was itself named by a declared mapping and therefore already resolves for `get`/`set`/`check` — raises a new error, `Error::ClanMachineListFailed { machine, output }`, propagated with `?` exactly as `Error::ClanUnavailable` already is, stopping the run before any mapping is compared or any machine is enumerated.
This is stricter than degrading gracefully, and deliberately so: a mapping whose clan side does not resolve already has a place to be reported, per-mapping, as `Disagreement::Unjudgeable` (`audit.rs:193-203`) — this new error path exists for the case that path does not cover, a machine that lists correctly for `get`/`set` on one triple but fails to enumerate as a whole, which has not been observed and is not expected, and which this design chooses to fail loudly over rather than silently narrow the claim "nothing lingers" makes.

### D7. Reporting is the entire deliverable; nothing here writes

No code path this change adds ever invokes `vars set`, and no code path removes a clan var, a mapping, or a safix entry.
This mirrors `keepassxc-sync`'s own explicit choice — "No mode SHALL delete an entry on either side" — applied to a boundary where clan, not safix, holds the store, which makes the case for never deleting stronger rather than weaker: `bridge-transfer` already establishes that clan is the authority on its own store, and clan's own command line offers no `vars unset` for safix to delegate to even if a mode wanted one.

## Risks / Trade-offs

A machine whose last mapping is removed drops out of this report on the same day, per D2.
Its vars are not lost and remain visible to `clan vars list` run by hand; what is lost is `safix audit clan` continuing to mention them.
Declaring a machine independently of any mapping would close this gap and is out of scope: `bridge-surface` has no such declaration today, and adding one is a larger surface than this change was asked to add.

Enumeration cost scales with the number of distinct machines a bridge declares, one subprocess per machine per `audit` run, on top of the one subprocess per mapping `audit` already spends.
For the fleet sizes this bridge is built for — mappings measured in the tens, not thousands — this is the same order of cost `audit` already pays, and no batching across machines is attempted.

A machine-listing failure stopping the whole run (D6) means one broken machine can prevent `audit` from reporting on every other, otherwise-healthy mapping in the same run, including ones on unrelated machines.
This is the accepted cost of not silently narrowing what "nothing lingers" claims; an operator in that state still has `audit clan <mapping>` to narrow past the broken machine, per D5.

## Migration Plan

Additive to `audit`'s output and to `Clan`'s and `Error`'s public surface; no existing field, verb, or exit-status contract changes.
A consumer parsing `safix audit clan`'s (or bare `safix audit`'s) text output for its own tooling gains a new section when at least one machine has a lingering var; one does not appear when none does, matching every other empty-case rendering already in `render.rs`.
