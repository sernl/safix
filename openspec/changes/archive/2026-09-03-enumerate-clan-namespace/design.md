# Design: audit's view of clan's namespace

Revisions are as named in `proposal.md`.
Every clan-core line anchor below was read at that revision, in `/home/sernl/ghq/git.clan.lol/clan/clan-core`.

Amended 2026-09-03 while proposing `rename-transfer-verbs`: vocabulary updated to the sync family; substance unchanged.
`audit` gains `clan` and `keepassxc` targets there; every invocation this design cites as bare `safix audit` or `audit <mapping>` becomes `safix audit clan` / `audit clan <mapping>`, since this change's own lingering report is the clan target's alone.

Amended 2026-09-03 while proposing `sync-clan-vars-two-way`: this design originally assumed a mode-like third relationship kind that two-way's own design does not build.
Two-way instead gives `ClanSide` a `placement` (`shared | per-machine | per-export`, defaulting to `per-machine`) and makes `machine` `nullOr str` — null for a shared or per-export mapping, whose addressing machine is discovered at run time (two-way's D3) rather than declared.
D2 and D3 below are rewritten against that model, grounded directly in `clan_cli`/`clan_lib`'s `vars list` implementation at the pinned revision rather than assumed from it; the Context section below is corrected in the same pass, since its original placement-transparency claim does not hold for `PerExport`.

Amended 2026-09-03 while finishing `sync-clan-vars-two-way`: per-export placement is dropped from the domain entirely, confirmed unreachable via `clan vars get`/`set` at clan-core `56e35624d94e4f1ac55d36575ebab97cbd9b9cdd` (`get_machine_generators`, `clan_lib/vars/generator.py:229-351`, only ever constructs `Shared` or `PerMachine` placements, never `PerExport`); `ClanPlacement` now has two variants only.
Every per-export mention this design carried is removed in the same pass: the Context section's `PerExport`/`get_flake_generators` clause and the `export_vars.py` paragraph, D2's title and its per-export paragraph, D3's per-export sentence, and the Risks section's per-export-invisibility paragraph.
The delta spec's `## MODIFIED Requirements` block is re-synced against `sync-clan-vars-two-way`'s own final `specs/bridge-transfer/spec.md` on the epic tip, which already carries this same removal.

## Context

`clan vars list <machine>` is `clan_cli/vars/list.py`, registered with no flag of its own beyond the global `--flake` (`clan_cli/cli.py:85-91`, the same flag `vars get`, `vars set` and `vars check` already take in `clan.rs`).
It resolves `Machine(name=machine, flake=flake)` and loads every generator that machine's own NixOS configuration declares through `get_machine_generators` (`clan_lib/vars/generator.py:229-382`), which queries `<machine>.config.clan.core.vars.generators.*` (`clan_lib/nix_selectors.py:191-193`, `vars_generators_metadata`) — a `Shared`-placed generator is present there for every machine whose module declares or consumes it, and a `PerMachine`-placed one only for the one machine it names.
`clan vars list <machine>` prints one line per var, sorted by the line's own text (`clan_cli/vars/list.py:9-13`).

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

### D2. Enumeration is scoped to the machines the bridge's own mappings name or resolve, not to `clan machines list`

`bridge-surface` already states "one consumer bridges one clan" as a refusal against declaring more than one `clanFlake`.
It has never stated, and this change does not introduce, "one consumer may see every machine of that clan" — and a single clan repository backing several downstream consumers, each bridging a disjoint subset of its machines, is exactly the shape that sentence is agnostic to.
Calling `clan machines list` and enumerating every machine it returns would make one consumer's `audit` report on machines that consumer's own declarations never mention, which is a stronger claim about clan's namespace than this bridge has ever made about anything else in it — every other requirement in `bridge-surface` and `bridge-transfer` is scoped to what a mapping names or resolves.
So the set of machines enumerated is `{ mapping.clan.machine for mapping in the selected set with placement = per-machine } ∪ { addressing_machine(mapping) for mapping in the selected set with placement = shared }`, where `addressing_machine` reuses `sync-clan-vars-two-way`'s own addressing-machine search (its D3: `Clan::machines` plus the `get`/`set` trial), memoized the same way that search already is so a mapping sharing a generator with another does not repeat it.
`clan machines list` is never called directly by this change; the only route to a machine name that is not `mapping.clan.machine` is that reused search.

The consequence, stated rather than discovered later: a machine whose last per-machine mapping is removed, or whose shared mapping's addressing search no longer resolves to it, is no longer named or resolved by anything safix declares, and so drops out of this enumeration along with it.
Its vars do not become permanently invisible to the operator — `clan vars list <machine>` run by hand still shows them — but they stop being carried in `safix audit clan`'s report the same day the mapping that once named or resolved the machine changes.
This is recorded as a Risk below rather than solved, because solving it means declaring a machine independently of any mapping, which is a new piece of surface this change was not asked for and `bridge-surface` does not currently have.

### D3. What counts as "accounted for" is placement-sensitive, and never depends on direction

A per-machine-placement mapping's var is claimed exactly as before: `(mapping.clan.machine, id)` — where `id` is `mapping.clan.generator/mapping.clan.file` — must match a listed `(machine, id)` pair exactly.
A shared-placement mapping's var is claimed by `id` alone, machine-insensitively: enumeration visits only the one addressing machine D2's search happens to resolve to, but the same shared generator's var can legitimately appear, with the identical `id`, in more than one machine's own listing (Context, above), so a listed `id` on any enumerated machine counts as claimed if any selected shared mapping names that generator and file, regardless of which machine listed it.
Nothing about `mapping.direction` enters any of these comparisons, including `two-way`, which still carries one clan machine (or one resolved address), one generator and one file per mapping — the same shape a one-way mapping's clan side always had.
Only a design that let one mapping claim a *set* of clan triples, or claim one dynamically, would need to revisit this, and nothing `sync-clan-vars-two-way` proposes does either.

### D4. The report lands on `audit`'s clan section, not on a new verb, as a field named the same as keepassxc's own

`audit` already is "the report over the same declarations: it compares both sides of every declared mapping and writes nothing" (`audit.rs:1-8`, `usage.rs:111-113`).
Namespace drift is the same kind of fact — something true about the boundary that a person should see and nothing should act on — so it is a field on the clan section of `audit::Report`, the per-target report structure `rename-transfer-verbs` gives `audit` (its task 3.1) alongside its own keepassxc section: `lingering: Vec<String>`, named identically to (but a distinct field from) the keepassxc section's own `lingering`, which `rename-transfer-verbs` adds as that capability's parallel gap-fill (its task 3.2), and holding the same kind of thing this design's own lingering always held: self-describing strings, `"<machine> <generator>/<file>"`, in the exact format `audit::Finding::clan` already uses for a mapping's clan endpoint (`audit.rs:85-86`), so nothing about this report invents a new way to name a clan var.
It does not participate in `Report::is_clean` or change `audit`'s exit status, exactly as `sync::Report::lingering` does not participate in `sync`'s tally or `is_clean` — a run that finds ten vars nothing declares and every declared mapping agreeing still exits zero, because the exit status answers "did the mappings I compared agree," and this is a different question asked and answered alongside it.
A bare `audit` run fills both sections at once, so its report carries both lingering lists side by side; `audit clan` fills the clan section alone, and this change's own lingering list is the only one that ever appears in it.

Considered and rejected: a new verb (`safix enumerate` or similar).
It would need its own help text, its own exit-status story, and its own decision about whether it requires clan and refuses without it — all of which `audit` already has settled, correctly, for exactly this kind of read-only cross-boundary report.
A fourth thing to type and remember for a report that belongs beside the one `audit` already gives is a cost with no offsetting benefit.

### D5. Enumeration is scoped by the named mapping list, matching what an operator narrowing `audit clan` to specific mappings is asking for

`audit`'s clan target narrows to the mapping names given on the command line — zero or more, per `rename-transfer-verbs`'s variadic grammar (its task 3.3's mapping-name-list scoping, reusing the selection logic `sync`'s clan target gained) — and lingering is computed over the same narrowed set of mappings' machines rather than over every declared mapping regardless of that list.
This is the one place this design departs from `sync::lingering`'s literal shape, which always computes over the whole declared `Keepassxc` regardless of a sync run's own selection.
The reason for the departure is that clan's enumeration is a real subprocess call with a real, per-machine failure mode (D6), where keepassxc's is a single already-open database's own index; an operator who names one or more mappings to `audit clan` because clan is only reachable in some restricted way for those machines should not have that narrowed request additionally require reaching every other machine the bridge happens to declare or resolve.
Naming no mapping restores the full behavior: every machine any declared mapping names or resolves is enumerated, exactly as an unnarrowed `sync` run enumerates the whole declared group.

### D6. A machine that cannot be listed stops the whole run, the same way an unreachable clan already does

`audit::run`'s own reasoning for why `Error::NoClanFlake`, `Error::ClanUnavailable`, and `Error::UnknownMapping` all stop the run before the first mapping is compared is stated in its doc comment: "a run that discovered them partway through would already have said 'agrees' about mappings it never looked at."
The same reasoning applies here without alteration: a lingering section that silently dropped one machine's contribution because its listing failed would read as complete while being partial, and "nothing lingers" is exactly the sentence a partial report can print by accident.
So a machine that cannot be listed — clan exits non-zero for a reason other than the ordinary empty case, which for a real machine should not happen, since every machine enumerated was itself named or resolved for a declared mapping and therefore already resolves for `get`/`set`/`check` — raises a new error, `Error::ClanMachineListFailed { machine, output }`, propagated with `?` exactly as `Error::ClanUnavailable` already is, stopping the run before any mapping is compared or any machine is enumerated.
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
