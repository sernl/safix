# Tasks: rewrite-readme-and-examples

Citations are as read while designing this change, on 2026-09-16; re-read the named lines before editing, since implementation may land after sibling changes and line numbers drift.
The heading text, the option name and the symbol name — never the line number — are the anchors.
No real fleet identifier, hostname, or recipient enters this repository; fixtures use `alice`, `bob` and `carol`, synthetic `age1example…` strings, `deck` and `rack` for machines, `web` for a service, `oncall`, `infra` and `contractors` for groups, `corp` for a silo, `acme` for an organization — matching `examples/plain-nix/fleet.nix:1-102`.
Where a task says "hold", add an assertion that fails when the claim stops being true, not a sentence asserting it.
Every example file must be nixfmt-clean, because `modules/flake/treefmt.nix:8-15` sets `flakeCheck = true` and covers `examples/`.

Three commands recur; group 1 records them in `CONTRIBUTING.md` and every later group's `Verify:` line names them by number.
All three were run against `README.md` as it stands on 2026-09-16 and reported 52, 116 and 4 sites respectively; those are the worklists groups 2 to 7 close, and each command must print nothing when they are closed.

- **P1** (no check name in the body): `grep -nE 'safix-[a-z][a-z-]*' README.md` — 52 lines today.
- **P2** (no sentence over forty words), which skips fenced blocks, table rows and headings because they are not sentences:

  ```console
  $ awk '/^```/{f=!f;next} f{next} /^[|#]/{next} {print}' README.md \
      | tr '\n' ' ' | sed 's/\([.!?]\)  */\1\n/g' \
      | awk '{ if (NF > 40) printf "%d words: %.90s\n", NF, $0 }'
  ```

  116 sentences today, the longest 136 words. A semicolon chain counts as one sentence, which is correct: the 136-word one is a semicolon chain.
- **P3** (no count of verbs or targets in prose): `grep -nEi '\b(thirteen|fourteen|fifteen|sixteen|two|three|five) (of )?(safix.s )?(subcommands?|verbs?|targets?|relationship families)' README.md` — 4 lines today (`:1031`, `:1066`, `:1320`, `:1399`).

## 1. `## Status` moves to `CONTRIBUTING.md`, and the prose contract gets its commands

- [ ] 1.1 Cut `README.md`'s `## Status` section (`:1388-1406`) whole and paste it into `CONTRIBUTING.md` as `## Where the suite stands`, placed after `## Running the checks` (`CONTRIBUTING.md:25-40`) and before `## The fixture fleet` (`:41`)
- [ ] 1.2 Rewrite the pasted section for its new audience: keep which checks are platform-conditional and why (Ubuntu user namespaces, darwin `sandbox_apply`), keep what was retired and when, drop the openspec change-directory names, and state that a skipped check is not a passing one
- [ ] 1.3 Add `## Keeping the README honest` to `CONTRIBUTING.md` after `## Where the suite stands`, carrying P1, P2 and P3 verbatim in one console block, each with the one sentence saying what it holds and why it is a command rather than a check (design D7)
- [ ] 1.4 Severity drill: append one 45-word sentence to `README.md`, run P2, confirm it prints that sentence with its word count; delete the sentence and confirm P2 prints nothing. Add `safix-example` to a README paragraph, run P1, confirm it is reported; delete it. Record both transitions in the commit message for this group
- [ ] 1.5 Verify: P1, P2 and P3 all print nothing against `README.md` as it stands before the rewrite begins — they will not, and the output of each is the worklist groups 2 to 7 close. Record the three starting counts in the commit message so the closing run has something to compare against

## 2. README chapters 1 to 4

Each task names the chapter, its budget, and the current sections it absorbs.
Write the chapter from those sections rather than editing them in place; nothing of the old ordering survives (design D1).

- [ ] 2.1 `# safix`, ~12 lines. Absorbs `:1-10`. One paragraph: what it is, the one opinion (the audience picks the file), what it is not (not a framework, not your user registry). Delete the positioning sentences at `:3` ("built for its operator's own fleet; that use case, not general adoption, decides its opinions") and `:9` ("Its headline opinion: …")
- [ ] 2.2 `## Install it and declare one secret`, ~45 lines. Absorbs `## Quick start` (`:11-63`). Three subsections: `### The flake half`, `### The profile half` (home-manager and NixOS, four and three lines), `### Your first three commands` (`safix fix`, `safix set`, `safix get`). Keep the `sops <path>` caveat that the spelling is a default and a rename is the consumer's (`:125`)
- [ ] 2.3 `## How safix thinks`, ~35 lines. Absorbs `## The one mental model` (`:64-76`). Define custody and placement before using either word — `:72`'s "it is a property of a subject" currently precedes the subjects chapter by a hundred lines. State the three questions (who declares, who can read, where it lands). This is the one site of the revocation rule: state that narrowing an audience aligns future ciphertext with the new audience and **does not retract** what an old recipient already has, and state the remedy (rotate the value, or regenerate it where it has a generator). The words "does not retract" are the canonical phrase, used here and nowhere else, so the rule's one statement is findable by `grep -c 'does not retract' README.md` printing 1; every later mention reads "see **How safix thinks**", which is the second canonical phrase
- [ ] 2.4 `## Declaring what someone holds`, ~110 lines. Absorbs `## private` (`:77-94`), `## carries` (`:95-109`), `## sharedWith` (`:110-132`), `## shared = true` (`:133-157`), `## perHost and perTag` (`:158-170`). Keep the three-row shared/carries/sharedWith table at `:136-157` verbatim. Five subsections in that order. Each states the declaration, the resulting audience, and the one sentence on what it does not confer; none restates the revocation rule, and each that needs it names `## How safix thinks`
- [ ] 2.5 In 2.4, state the legal and the refused shapes of `perHost.<h>.add` in one place: an `add` naming an entry the person already holds through `carries`, `private` or a grant adjusts its placement in that scope, and an `add` that is the only route to the entry is refused (`modules/flake/safix/types.nix:429-433`)
- [ ] 2.6 Delete, in the four chapters written above, every "Think of it as …" framing the absorbed range carries — verified on 2026-09-16 at `:91`, `:105`, `:123`, `:144`, `:166`; at most two survive in the whole document (design D4)
- [ ] 2.7 Severity drill: run P2 over the four new chapters alone (pipe the chapter range through P2's pipeline) and confirm it prints nothing, then temporarily re-paste `:211`'s sentence into chapter 4 and confirm P2 reports it. Recorded in the commit message
- [ ] 2.8 Verify: P1 and P2 print nothing for the line range chapters 1 to 4 occupy; every option named in the absorbed sections (`catalogue`, `catalogue.<e>.shared`, `users.<u>.recipient`, `.recipientNote`, `.carries`, `.private`, `.sharedWith`, `.perHost`, `.perTag`) appears exactly once, checked by `grep -c` per option name

## 3. README chapters 5 and 6

- [ ] 3.1 `## The subjects that can hold a key`, ~130 lines. Absorbs `:171-350`. Seven subsections: `### Machines` (`:178-208`), `### Services` (`:209-233`), `### Groups` (`:234-255`), `### Silos` (`:256-268`), `### Ownership` (`:269-281`), `### Organizations and escrow` (`:282-315`), `### Delegation, and what it is not` (`:316-350`). Each: the declaration, the audience directory it produces, one sentence on what it does not confer
- [ ] 3.2 Move `## safix group` (`:351-372`) out of this chapter: the membership algebra stays under `### Groups`, and the verb's own behaviour — what it edits, what it discloses, that a hand edit owes the same disclosures — becomes the `group` row and its short section in chapter 7
- [ ] 3.3 In `### Delegation`, keep the both-consenting-sides statement and the "not authorization" statement, and cut the repetition of "manager scaffolds and never reads" to one occurrence, naming `flake.safix.organizations.<o>.managers` and `flake.safix.users.<u>.managedBy` once each
- [ ] 3.4 In `### Machines`, name `recoveryRecipients` where it belongs — it is a person's option, so state it under `### Ownership`'s neighbour text or in chapter 4; wherever it lands, it appears exactly once, with the difference between an own offline identity and an operator-held one (`modules/flake/safix/types.nix:46-70`)
- [ ] 3.5 `## Generators`, ~100 lines. Absorbs `:373-561`. Six subsections: `### A value that writes itself` (script, `runtimeInputs`), `### Chaining, prompts, multi-output` (`dependencies` `:408-419`, `prompts` `:425-434`, `files` `:297-321` of `types.nix`), `### Public outputs a nix module reads at evaluation` (`:456-485`, including where the reading expression is written — a consumer's own module, not safix's), `### Validation` (`:453-454`), `### What a fragment may do` (`:486-529`, compressed from 44 to ~18 lines: the sandbox, `network = true`, where the plaintext is, `SAFIX_STAGING_DIR`, `--allow-disk-staging`), `### The definition and the stamps a mint records` (`:531-561`)
- [ ] 3.6 In 3.5, document the stamp records beside the definition record: the resolver builds `logicalStamp` as the definition record plus a `.stamps` suffix (`modules/flake/safix/resolve.nix:809`, argued at `:801-808`) and emits `stampRecord` on every placement (`:853-865`), so the stamps live under the same configured root as the definition record. The current document mentions stamps at `:556-560` and `:1378`; the rewrite states it once, here
- [ ] 3.7 Delete the "Think of it as …" framings this range carries — verified at `:195`, `:246`, `:262`, `:275`, `:327`. The eleventh and last is `:652`, in the range group 4 rewrites
- [ ] 3.8 Severity drill: run P1 over the two new chapters; the current `:995`, `:962` and `:967-968` check names must be absent. Temporarily re-add `safix-generate-envelope` to `### What a fragment may do` and confirm P1 reports it. Recorded in the commit message
- [ ] 3.9 Verify: P1 and P2 print nothing over the chapter-5-and-6 range; every generator option (`script`, `runtimeInputs`, `network`, `prompts`, `dependencies`, `files`, `files.<n>.secret`, `share`, `validation`, `description`) named exactly once, checked by `grep -c`

## 4. README chapter 7: one verb table and the sections that expand it

- [ ] 4.1 `## Everyday verbs`, ~90 lines, opening with one table of sixteen rows — `set`, `edit`, `get`, `view`, `list`, `generate`, `check`, `fix`, `audit`, `sync`, `keygen`, `adduser`, `enroll`, `group`, `upload`, `install` — in the order `crates/safix/src/main.rs:121-204` declares them, with columns: verb, what it does, what it reads, what it writes, needs a terminal
- [ ] 4.2 `install` is a row like any other, marked as the one no operator types (`:954`). It is not omitted and not documented only in the profile chapter, which is what produced the four disagreeing counts
- [ ] 4.3 Delete the console block at `:662-677` and the paragraph at `:687` ("`upload` does not exist here, and `safix --help` records why: activation already delivers what an upload would"). The paragraph is false: `upload` is a verb (`main.rs:191-195`) with a section at `:779`
- [ ] 4.4 `### Browsing and editing` absorbs `## Browsing what is there: safix view` (`:562-618`) and `## Editing a value: safix edit` (`:619-641`): the picker's keys, the query syntax (`:589-597`), the four colours (`:599-600`), the CREATED and UPDATED columns the stamps feed (`:602-605`), and the remembered selection
- [ ] 4.5 `### Onboarding a person` absorbs `:692-736`; `### Enrolling a hardware key` absorbs `:737-778`; `### Seeding a machine's host identity` absorbs `:779-806` compressed from 28 to ~10 lines, with all thirteen check names at `:790-805` deleted; `### Editing a group's membership` absorbs `:351-372`
- [ ] 4.6 State the `check`/`fix` relationship once, with the finding classes named as classes rather than counted, so a new finding class does not make a number wrong
- [ ] 4.7 No sentence in the chapter states how many verbs there are. Run P3 over the chapter
- [ ] 4.8 Severity drill: delete one row from the table and confirm the 4.9 coverage command reports the missing verb; restore it. Recorded in the commit message
- [ ] 4.9 Verify: `for v in set edit get view list generate check fix audit sync keygen adduser enroll group upload install; do printf '%s %s\n' "$v" "$(grep -c "^| \`$v\`" README.md)"; done` prints `1` for all sixteen; P1, P2 and P3 print nothing over the chapter's range

## 5. README chapter 8: one sync chapter, five same-shaped subsections

Depends on `add-pass-bridge`, `add-bitwarden-bridge`, `add-onepassword-bridge` and `extend-bridge-fields` (design D13).
Write the shared half first; it is independent of all four.

- [ ] 5.1 `## Syncing to other stores`, ~120 lines. Absorbs `## The bridge to clan` (`:1132-1209`) and `## The mirror in your password database` (`:1211-1269`). Opens with, once for all targets: what a mapping is and that the identifier is the mapping's own name rather than either endpoint's; the four modes (`safix-to-<target>`, `<target>-to-safix`, `two-way`, `backup`) and how a direction is read; how a conflict is judged; what the sync-state companion records and the `safix-sync-v1` format; that nothing is ever deleted on either side
- [ ] 5.2 State the field surface once: `fields = { username; url; notes; tags; }` on a mapping's far side, each either a literal or `{ entry = "<name>"; }` naming another entry of the same person, resolved at run time; fields are declarations, so a pulling mode writes only the value into safix and a pushing mode writes the fields beside it; a field the target cannot carry is refused at evaluation
- [ ] 5.3 `### clan`, absorbing `:1132-1209`: it addresses a machine, a generator and a file inside another flake; declared as `flake.safix.bridge.clanFlake` plus `bridge.mappings.<id>` with `direction` rather than `mode` (`modules/flake/safix/options.nix:454-505`); carries no fields; unlocks nothing of its own; refuses a stale generator and a second clan flake
- [ ] 5.4 `### keepassxc`, absorbing `:1211-1269`: it addresses a path inside an encrypted database; `flake.safix.keepassxc.{database,group,yubikey,keyFile,mappings}` (`options.nix:507-620`); carries `username`, `url` and `notes` as literals only and refuses `tags` and an `{ entry = …; }` source, because its only channel is the argument vector; unlocks with a composite key; refuses a multi-line value and a path carrying the two-way state suffix
- [ ] 5.5 `### pass`: addresses a path under a store root; `flake.safix.pass.{store,mappings}` with `pass.{path,fields}`; carries all four fields through the record body on standard input; needs no unlock of its own because the gpg agent is ambient, and a locked agent is reported as such; refuses a path carrying the `.safix-sync-state` suffix
- [ ] 5.6 `### bitwarden`: addresses a folder plus an item name; `flake.safix.bitwarden.{server,mappings}` with `bitwarden.{folder,item,fields}`; carries `username`, `url` and `notes` and refuses `tags`; unlocks by prompting once and passing the session to child processes in their environment, which is the one place a value-bearing environment variable is accepted and why; refuses two items of one name in one folder
- [ ] 5.7 `### 1password`: addresses a vault plus an item; `flake.safix.onepassword.{account,mappings}` with `onepassword.{vault,item,fields}`; carries all four fields through the JSON template on standard input; inherits an existing session or a service-account token from the operator's environment and never takes one on the command line; refuses an absent vault
- [ ] 5.8 `### safix audit <target>`: what a divergence report says, that it names a diverged field and never its content, and the remedy line each mode prints
- [ ] 5.9 Every subsection carries the same five headings in the same order — what it addresses, how it is declared, what it can carry, how it unlocks, what it refuses — and a target with nothing to say under one keeps the heading and says so (`pass` under unlocking, `clan` under carrying)
- [ ] 5.10 No sentence states how many targets there are. The four current sites are `:671`, `:689`, `:1320` ("the two relationship families") and `:1399` ("`sync`'s two targets"), plus the two chapter headings; all six are gone
- [ ] 5.11 Severity drill: delete `### pass`'s "what it can carry" heading and confirm the 5.12 shape command reports the subsection with four headings instead of five; restore it. Recorded in the commit message
- [ ] 5.12 Verify: `awk '/^## Syncing to other stores/,/^## Where files go/' README.md | grep -c '^#### '` prints exactly 30 (five subsections × six, counting `safix audit` separately if it takes subheadings — adjust the literal to the shape written and state it in the commit message); P1, P2 and P3 print nothing over the chapter's range

## 6. README chapters 9 to 13

- [ ] 6.1 `## Where files go`, ~50 lines. Absorbs `## The three storage roots` (`:1271-1301`), `## Values without declarations: the runtime extract` (`:642-659`) and `## A vault` (`:1075-1131`, compressed from 56 to ~20 lines). Four subsections: `### The three storage roots` (keep the table at `:1274-1300` verbatim), `### Your own ignore, backup and exclusion rules` (this is where `extraGovernedFiles` lands, with `:656-658`'s consequence stated first rather than last), `### Renaming a root`, `### A vault: ciphertext in a second repository`
- [ ] 6.2 `## Establishing secrets in a profile`, ~110 lines. Absorbs `:831-996`. Four subsections: `### The option surface` (keep the table at `:841-884` verbatim — it is the highest-density page in the document), `### Identity, and the activation guard` (`:918-951`), `### The installer safix owns` (`:952-996`, with every check name at `:955`, `:962`, `:967-968`, `:978`, `:983`, `:995` deleted), `### Refusals you may hit, and what each one asks for`
- [ ] 6.3 Reduce `### The two published names` (`:885-905`) to four lines inside `### The option surface`: the four published names, that the two within each scope name one value, that `homeManagerModules` is an alias, and the collision caveat. Delete the illustrative nix error block at `:896-899` and its own disclaimer at `:901`
- [ ] 6.4 In `### The option surface`, name `safix.secrets` as the read-back option once (`:872`, `:956`), and confirm by `grep -n 'safix\.installed' README.md` returning nothing — the option does not exist and the document must not resurrect it
- [ ] 6.5 `## Fitting safix to a tree you already have`, ~60 lines. Absorbs `## Wiring it to your own user registry` (`:807-830`), `## Without flake-parts, or without a flake` (`:1028-1073`), `## The checks safix hands you` (`:1302-1325`) and `### Migrating from the sops-nix-backed surface` (`:997-1026`, moved here, its twelve-row rename table kept verbatim). This chapter carries the one statement of the namespace rule: safix reads no option outside its own namespace, with the reason, and the seven other sites (`:811-816`, `:833-839`, `:922`, `:989-990`, `:1347-1348`) name this chapter instead
- [ ] 6.6 In 6.5, `mkChecks` is documented as a published function a consumer calls, without naming any check it returns and without stating how many it returns — `:1320`'s "eight checks" followed by seven comma-joined clauses is the defect being removed
- [ ] 6.7 `## The opinions safix will not bend`, ~25 lines. Absorbs `:1326-1351`, minus `:1344-1345`'s eighth statement of the revocation rule, which becomes a clause naming `## How safix thinks`
- [ ] 6.8 `## Where the pieces live`, ~25 lines. Absorbs `:1352-1387`, with the file list extended to name one module per sync target and the anchor map from the old headings to the new chapters, so an inbound link that no longer resolves has a replacement
- [ ] 6.9 `## License`, ~10 lines. Absorbs `:1408-1417` unchanged
- [ ] 6.10 In 6.5, the namespace rule's one statement uses the canonical phrase "reads no option outside its own namespace" and nothing else does; every later mention reads "see **Fitting safix to a tree you already have**". Severity drill: add a second statement of the rule to chapter 10 and confirm `grep -c 'reads no option outside its own namespace' README.md` reports 2 rather than 1; remove it. Recorded in the commit message
- [ ] 6.11 Verify: P1, P2 and P3 print nothing over the whole file; `wc -l README.md` is within 100 lines of 850; `grep -c 'does not retract' README.md` is 1, `grep -c 'reads no option outside its own namespace' README.md` is 1, and `grep -c 'see \*\*How safix thinks\*\*' README.md` is at least 4 — the cross-references that replaced the seven restatements

## 7. The deletion sweep, by class

Each class below is closed either by a command whose output must be empty or by a canonical-phrase count, run over the rewritten file.
A class that cannot be closed is a finding to record in the commit message, not a task to tick.
Two classes are paraphrase classes and cannot be closed by grepping the old wording: today `grep -ci 'not retroactive\|already encrypted\|is not revocation' README.md` finds only 2 of the eight revocation sites and `grep -c 'its own namespace' README.md` only 2 of the five namespace sites, because the rest are paraphrases.
Their closure is therefore the canonical phrase 2.3 and 6.10 introduce plus a read of each cited range in the rewritten file.

- [ ] 7.1 Class C1, the revocation rule restated: `:127-131`, `:152-156`, `:231-232`, `:246-248`, `:280`, `:310-311`, `:364-368`, `:1344-1345` — eight sites reduced to one statement plus cross-references. Closed by 6.11's two counts, and by reading each of the eight ranges in the rewritten document to confirm it carries a cross-reference rather than a restatement
- [ ] 7.2 Class C2, the namespace rule restated: `:811-816`, `:833-839`, `:922`, `:989-990`, `:1347-1348` — five sites reduced to one. Closed the same way, against 6.10's canonical phrase
- [ ] 7.3 Class C3, check names in the body: 52 lines carry at least one, verified on 2026-09-16 by P1, worst at `:790-805` (thirteen names in 27 lines), plus `:924`, `:955`, `:962`, `:967-968`, `:978`, `:983`, `:995`, `:1041-1049`, `:1064`, `:1069`, `:1390-1406`. Closed by P1
- [ ] 7.4 Class C4, sentences over forty words: `:211`, `:478`, `:797`, `:923` (~90 words), `:966`, `:989`, `:1031`, `:1064`, `:1067`, `:1128`, `:1195`, `:1399`. Closed by P2
- [ ] 7.5 Class C5, hedging and positioning: `:3`, `:9`, `:885-904`. Closed by 2.1 and 6.3
- [ ] 7.6 Class C6, the "Think of it as …" tic: eleven sites, verified on 2026-09-16 at `:91`, `:105`, `:123`, `:144`, `:166`, `:195`, `:246`, `:262`, `:275`, `:327`, `:652` — reduced to at most two. Closed by `grep -c 'Think of it as' README.md` printing 2 or less, from 11 today
- [ ] 7.7 Class C7, CI narration: `:1388-1406`. Closed by group 1
- [ ] 7.8 Class C8, counts of verbs and targets: `:671`, `:689`, `:1031`, `:1066`, `:1320`, `:1399`, plus `examples/README.md:21`. Closed by P3 and by group 13
- [ ] 7.9 Class C9, the stale contradiction at `:687`. Closed by 4.3
- [ ] 7.10 Severity drill: the sweep's own commands are the drill — each was run against the pre-rewrite file in 1.5 with non-empty output, and against the rewritten file here with empty output. Record both outputs in the commit message; a command that was empty in 1.5 was never holding anything and must be reported as such
- [ ] 7.11 Verify: P1, P2, P3, the C6 count, the C1 and C2 counts, and `grep -n 'safix\.installed\|clan-core' README.md` all empty or at their stated counts

## 8. `examples/plain-nix/fleet.nix` declares the whole surface

One file, the same fleet as today extended.
`examples/plain-nix/` keeps exactly three files and gains none: `entry.nix` (unchanged — it must stay flakeless, per `entry.nix:1-14`), `fleet.nix` (everything below), `hooks.nix` (unchanged in shape; group 9 copies its two values into the dendritic side).
Every declaration added here must be added to `examples/dendritic/modules/` in group 9 in the same evaluation-visible way, or `safix-examples` fails — which is the check working.
Do not declare `flake.safix.storage` or `flake.safix.vault` here; both are fleet-wide and belong to `examples/profiles/relocated.nix` (design D8).
Do not declare an entry `path`: it is `functionTo str` (`modules/flake/safix/types.nix:382-387`) and no placement field carries it (`resolve.nix:810-866`), so it is not comparable here; it is declared and applied in group 10.

- [ ] 8.1 `catalogue.deploy-key = { mode = "0440"; }` — the entry `mode` option, on a catalogue entry both people carry
- [ ] 8.2 `users.alice.carries.deploy-key = { }` and `users.bob.carries.deploy-key = { }`
- [ ] 8.3 `users.alice.recoveryRecipients.master = { key = "age1examplemaster…"; note = "alice's offline master identity — held by her, not by the operator"; }`
- [ ] 8.4 `users.bob.managedBy = "acme"` and `organizations.acme.managers = [ "alice" ]` — the two consenting sides of a delegation (`modules/flake/safix/types.nix:102-135`, `options.nix:182-224`)
- [ ] 8.5 `users.carol = { recipient = "age1examplecarol…"; recipientNote = "carol — example identity, decrypts nothing"; }` — the third person a second silo group needs
- [ ] 8.6 `machines.rack = { recipient = "age1examplerack…"; recipientNote = "rack — a machine acme owns"; owner = "acme"; }` — an organization-owned machine, which is what makes an `ownerOf` grant resolve to custody keys rather than to a person's recipient
- [ ] 8.7 `users.alice.private.corp-handover = { }` and `users.alice.sharedWith."ownerOf.rack".corp-handover = { }` — the `ownerOf.<m>` audience
- [ ] 8.8 `users.alice.private.escrow-note = { }` and `users.alice.sharedWith.acme.escrow-note = { }` — an organization as a grant audience, distinct from `escrowedTo`
- [ ] 8.9 `users.alice.perHost.deck.add.web-token = { mode = "0440"; }` — a legal `add`: `web-token` is already alice's through `private`, so the `add` adjusts its placement rather than being the only route to it
- [ ] 8.10 `users.alice.perTag.portable.force.shelf-item = { }` — `force` beating the `perTag.portable.omit.shelf-item` already declared at `fleet.nix:63`, which is the only way the third field of the scope submodule (`types.nix:516-520`) is exercised
- [ ] 8.11 `groups.infra.members = [ "deck" "web" "oncall" ]` — a machine, a service and a nested group as members, where `groups.oncall` has people only
- [ ] 8.12 `groups.contractors.members = [ "carol" ]` and `silos.corp.groups = [ "oncall" "contractors" ]` — a silo with two groups, which is the only shape in which its non-overlap refusal means anything
- [ ] 8.13 `users.alice.private.wg-key.generator` with `files.wg-private.secret = true` and `files.wg-public.secret = false` — a public output, plus one comment naming where the reading expression is written (a consumer's own module, through `config.flake.safix.lib.publicValue "alice" "wg-public"`)
- [ ] 8.14 `users.alice.private.prompted-token.generator.prompts` — one prompt with a type and a description (`types.nix:46-90`, `:233-254`)
- [ ] 8.15 `users.alice.private.derived-token.generator.dependencies = [ "generated-token" ]` — a chained generator (`types.nix:255-296`)
- [ ] 8.16 `users.alice.private.validated-token.generator.validation` — a validation fragment (`types.nix:344-366`)
- [ ] 8.17 `users.alice.private.fetched-token.generator = { network = true; … }` — the one generator that declares network access, with the comment stating what that widens (`types.nix:198-232`)
- [ ] 8.18 `users.alice.private.{ntfy-token,grafana-password,deploy-token,deploy-username,vpn-password,registry-token} = { }` — the safix side of the five sync mappings group 12 declares. Declare them here in group 8 so that group 12 adds only the target halves
- [ ] 8.19 `extraGovernedFiles = [ "secrets/safix/users/alice/legacy.yaml" ]` at `flake.safix.extraGovernedFiles` (`modules/flake/safix/default.nix:78-83`) — a file that rides an existing rule and that no declaration implies
- [ ] 8.20 Keep the file's header comment current: it names every person, machine, service, group, silo and organization the fleet declares, and it is the one place a reader learns what the fleet is for
- [ ] 8.21 Severity drill: change `users.carol.recipient` in this file alone and confirm `safix-examples` fails on `recipients` and on `policyText` and on nothing else; revert. Recorded in `examples.nix`'s drill commentary
- [ ] 8.22 Verify: `nix build .#checks.x86_64-linux.safix-examples` green after group 9 lands; on its own this group reddens the check, which is expected and is why 8.21's drill is the group's evidence

## 9. `examples/dendritic/modules/` matches it, one declaration per file

The file set below is the whole directory after this group.
Twenty-two files exist today; twenty-six are new.
Each new file declares exactly one thing and reads no path, no filename and no other file.

- [ ] 9.1 Replace `examples/dendritic/flake.nix`'s `imports` list (`flake.nix:13-41`) with `imports = [ safix.flakeModules.default ] ++ nixpkgs.lib.filesystem.listFilesRecursive ./modules;`, reaching `lib` through the flake's own `nixpkgs` input, and keep the comment explaining why the example declares no `systems` and no `perSystem` (`flake.nix:10-12`). No `./modules/` path may remain in the file (design D11)
- [ ] 9.2 New `modules/catalogue/deploy-key.nix` — 8.1
- [ ] 9.3 New `modules/users/alice/carries-deploy-key.nix` and `modules/users/bob/carries-deploy-key.nix` — 8.2
- [ ] 9.4 New `modules/users/alice/recovery-master.nix` — 8.3
- [ ] 9.5 New `modules/users/bob/managed-by-acme.nix` and `modules/organizations/acme-managers.nix` — 8.4
- [ ] 9.6 New `modules/users/carol/profile.nix` — 8.5
- [ ] 9.7 New `modules/machines/rack.nix` — 8.6
- [ ] 9.8 New `modules/users/alice/private-corp-handover.nix` and `modules/users/alice/shared-with-owner-of-rack.nix` — 8.7
- [ ] 9.9 New `modules/users/alice/private-escrow-note.nix` and `modules/users/alice/shared-with-acme.nix` — 8.8
- [ ] 9.10 New `modules/users/alice/per-host-deck-add.nix` — 8.9
- [ ] 9.11 New `modules/users/alice/per-tag-portable-force.nix` — 8.10
- [ ] 9.12 New `modules/groups/infra.nix` and `modules/groups/contractors.nix`; edit `modules/silos/corp.nix` to name both groups — 8.11, 8.12
- [ ] 9.13 New `modules/users/alice/private-wg-key.nix` — 8.13
- [ ] 9.14 New `modules/users/alice/private-prompted-token.nix` — 8.14
- [ ] 9.15 New `modules/users/alice/private-derived-token.nix` — 8.15
- [ ] 9.16 New `modules/users/alice/private-validated-token.nix` — 8.16
- [ ] 9.17 New `modules/users/alice/private-fetched-token.nix` — 8.17
- [ ] 9.18 New `modules/users/alice/private-sync-entries.nix` — 8.18, the six entries the mappings name, in one file because they are one statement: these exist so a mapping has a safix side
- [ ] 9.19 New `modules/extra-governed.nix` — 8.19
- [ ] 9.20 New `modules/hooks.nix` declaring `flake.safix.onboardingHook` and `flake.safix.enrollHook` with the same values `examples/plain-nix/hooks.nix:6-15` carries, so the hooks exist on both sides and group 11 can compare them (design D10)
- [ ] 9.21 Severity drill: change one field in `modules/users/carol/profile.nix` and confirm `safix-examples` fails on exactly the fields that carry it; revert. This is `examples.nix:30-33`'s existing drill, re-observed after the rewrite, and it must still fail on exactly one field
- [ ] 9.22 Severity drill: add a new declaration file under `modules/` and confirm both the example's own evaluation and the check see it with no other edit — the property 9.1 buys; then put a `./modules/one-file.nix` path back into `flake.nix` and confirm group 11's assertion reddens. Revert both
- [ ] 9.23 Verify: `nix build .#checks.x86_64-linux.safix-examples` green; `grep -c './modules/' examples/dendritic/flake.nix` prints 0; the file count under `examples/dendritic/modules/` equals the number of declarations groups 8 and 9 name

## 10. `examples/profiles/` and the new `safix-examples-profiles` check

- [ ] 10.1 New `examples/profiles/nixos.nix`: a system-scope profile serving the machine, setting `safix.enable = true`, `safix.lib = (import ../../lib { inherit lib; }).mkVault { modules = [ ../plain-nix/fleet.nix ]; root = ../plain-nix; }`, `safix.machine = "deck"`, `safix.hostname = "deck"`, `safix.identity.deriveHostKeys = true`, and `safix.installer.{keepGenerations,useTmpfs,log,secretsMountPoint,symlinkPath,useSystemdActivation,afterActivation,afterUnits,environment,agePlugins,validate}` each set to a value that differs from its default so the setting is observable (`modules/consume/installer.nix:283-500`). One comment shows the one-line flake form (`safix.flake = inputs.self;`) as the alternative to setting `safix.lib` directly
- [ ] 10.2 New `examples/profiles/home.nix`: a user-scope profile serving `alice`, with the same `safix.lib` binding, `safix.user = "alice"`, `safix.hostname = "deck"`, `safix.tags = [ "portable" ]`, `safix.identity.keyFile`, `safix.identity.sshKeyPaths`, `safix.identity.generateKey` (`modules/consume/home.nix:341-360`), `safix.identityPreflight` (`home.nix:322-340`), and the user-scope `safix.installer.*` set (`home.nix:361-469`). It declares no ownership field on any entry, and a comment states the refusal that doing so produces
- [ ] 10.3 New `examples/profiles/relocated.nix`: `flake.safix.storage = { encrypted = ".safix/encrypted"; plaintextOutputs = ".safix/plaintext-outputs"; generatorRecords = ".safix/generator-records"; }` and `flake.safix.vault = { root = ./vault; namingKey = "…"; }` (`modules/flake/safix/options.nix:242-357`, `:360-449`), as a module merged beside the shared fleet rather than into it
- [ ] 10.4 New `examples/profiles/README.md`: what each of the three files is, which check evaluates them, and the sentence that they declare no fleet of their own — they consume `../plain-nix/fleet.nix`, which is the one fleet in the repository
- [ ] 10.5 New `modules/flake/checks/examples-profiles.nix` declaring `checks.safix-examples-profiles`, registered in `flake.nix`'s check list between `./modules/flake/checks/examples.nix` and `./modules/flake/checks/exported.nix` (`flake.nix:104-105`). Module header states: what the check reads, that it is separate from `safix-examples` because a materialization under a scope is not a projection field, and its severity drills
- [ ] 10.6 In the check, evaluate `examples/profiles/nixos.nix` through a real `nixosSystem` and `examples/profiles/home.nix` through `inputs.home-manager.lib.homeManagerConfiguration`, following `modules/flake/checks/portability.nix:60-78` for the shape and `modules/flake/checks/installer.nix` for the home-manager call. Supply `safix` in the package set through one overlay (`nixpkgs.overlays = [ (_: _: { safix = config.packages.safix; }) ]`), because the bare consumption modules default `safix.installer.package` to `pkgs.safix` (`flake.nix:36-60` is where the published wrappers supply it instead)
- [ ] 10.7 Assert `systemSecrets`: the resolved `config.safix.secrets` at system scope, as a literal map of name to `{ key, path, mode, owner, group, sopsFile, format, restartUnits, reloadUnits }`, covering every entry the machine `deck` holds — the grants aimed at it plus the service entries that land on it
- [ ] 10.8 Assert `homeSecrets`: the resolved `config.safix.secrets` at user scope for `alice`, as a literal, including `deploy-key`'s `mode = "0440"` (the entry-`mode` feature, invisible to `safix-examples`) and `web-token`'s `"0440"` from the `perHost.deck.add` override
- [ ] 10.9 Assert `entryPath`: one entry declares `path = cfg: "${cfg.home.homeDirectory}/.config/example-app/credentials.toml"` in the shared fleet's carrier and the assertion is the applied string under the home profile's own `homeDirectory`, which is the only place a `functionTo str` path is observable
- [ ] 10.10 Assert `ownershipAxis`: the service-borne entries at system scope carry the service's `user` and `group` (`web`/`web`), and `ownershipRefused`: a deliberately-assembled user-scope profile serving `deck` fails, with the message naming the entry and the field (`modules/flake/safix/types.nix:391-403`). The refused profile is assembled inside the check, never committed as an example file
- [ ] 10.11 Assert `identity`: `config.safix.identity.derivedHostKeys` is non-empty at system scope (`modules/consume/installer.nix:531-552`), and `config.safix.identityPreflight` is the declared value at user scope
- [ ] 10.12 Assert `installerSurface`: the manifest each scope builds carries the options 10.1 and 10.2 set — read `config.system.build.safix-manifest` at system scope (`installer.nix:557`) and `config.safix.installer.manifest` at user scope (`home.nix:469`), and assert `keepGenerations`, `secretsMountPoint`, `symlinkPath` and `useTmpfs` off the parsed JSON
- [ ] 10.13 Assert `relocated`: with `relocated.nix` merged into the fleet evaluation, every `sopsFile` the system profile resolves begins with `.safix/encrypted`, `vaultDeclared` is `true`, and no resolved name is a readable declared name — the vault's opacity observed through what a profile reads
- [ ] 10.14 Severity drill: drop `safix.machine = "deck"` from `nixos.nix` and confirm `systemSecrets` becomes empty and the check fails on that row alone; revert. Recorded in the check's drill commentary
- [ ] 10.15 Severity drill: add `owner = "web"` to an entry the home profile resolves and confirm `ownershipRefused`'s counterpart — the real home profile — now fails to evaluate rather than dropping the field; revert. Recorded in the check's drill commentary
- [ ] 10.16 Severity drill: revert `relocated.nix`'s `storage.encrypted` to the default and confirm `relocated`'s `sopsFile` rows go red while `vaultDeclared` stays green; revert. Recorded in the check's drill commentary
- [ ] 10.17 Verify: `nix build .#checks.x86_64-linux.safix-examples-profiles` green; drills in 10.14, 10.15 and 10.16 observed

## 11. `modules/flake/checks/examples.nix` stops comparing a value with itself

- [ ] 11.1 Rename `tenFields` to `comparedFields` and extend it with `subjects`, `vaultDeclared` and `vaultCreationRulesText`, so the compared set is every attribute `crates/safix-core/src/nix.rs:31-64` names that serializes. Keep the comment listing the function-valued exclusions (`examples.nix:45-49`) and state that the criterion is serializability rather than a chosen ten
- [ ] 11.2 Extend `entryAttrs` (`examples.nix:88-91`) with the three new `safix.lib.*` spellings so the executed side is queried for them too
- [ ] 11.3 Take the expected hooks from the dendritic evaluation rather than from `plainNixHooks`: `dendriticHooks` reads `config.flake.safix.{onboardingHook,enrollHook}` out of the same `lib.evalModules` call that produces `dendriticVault` (`examples.nix:69-76`), and `entryExpected` (`:93-96`) uses it. Delete `plainNixHooks` (`:81`) and its comment
- [ ] 11.4 Add `hooksAreNotEmpty`: one literal row asserting that the onboarding hook contains the fixture's marker text (`onboarded %s (%s)`, from `examples/plain-nix/hooks.nix:9`) and that `enrollHook` is null, so both sides losing their hooks fails instead of comparing empty to empty (design D10)
- [ ] 11.5 Add `elideRoots`, a recursive map over the JSON-shaped value replacing a leading `${plainNixRoot}` or `${dendriticRoot}` in any string with the literal `<example-root>`, applied to both operands before `jq -S` (`examples.nix:127-134`). It replaces only those two prefixes — not store paths in general (design D9)
- [ ] 11.6 Add `noHandList`: assert `examples/dendritic/flake.nix` contains no occurrence of `./modules/`, failing with the offending lines. This is the group-9 glob's guard (design D11)
- [ ] 11.7 Add `coverageProbes`, one row per feature family, asserted against the dendritic projection: the `=acme` organization audience and the `ownerOf.rack` audience appear in `audiences`; `placements` carries `corp-handover` and `escrow-note` for alice; `recipients.alice` contains alice's `master` recovery key; `governedFiles` contains `secrets/safix/users/alice/legacy.yaml`; `delegation` names `alice` as a manager for `bob`; `generatorPlan.alice` carries the prompt, the dependency edge `derived-token` to `generated-token`, the validation fragment, the `wg-public` non-secret file and `fetched-token`'s network flag; `subjects` carries `rack`, `infra` and `contractors`; each target's mapping set is non-empty in its own field (design D12)
- [ ] 11.8 Update the module header: the file count at `examples.nix:25` becomes a statement that the dendritic side is read by directory rather than a number; the severity paragraph (`:30-33`) gains the three new drills; the compared-field comment states the serializability criterion
- [ ] 11.9 Severity drill: point `dendriticHooks` back at `plainNixHooks` and confirm the hooks rows go green with dendritic's `modules/hooks.nix` deleted — the pre-change condition — then restore both and confirm deleting `modules/hooks.nix` now fails. Recorded in `examples.nix`'s drill commentary
- [ ] 11.10 Severity drill: widen `elideRoots` to replace every `/nix/store/…` path and confirm a deliberately divergent `bridge.clanFlake` subdirectory in one example stops being detected; revert to the narrow form and confirm it is detected. Recorded in `examples.nix`'s drill commentary
- [ ] 11.11 Severity drill: delete `sharedWith.acme.escrow-note` from both examples at once and confirm the field-for-field diff stays green while `coverageProbes` goes red — the whole reason 11.7 exists; revert. Recorded in `examples.nix`'s drill commentary
- [ ] 11.12 Verify: `nix build .#checks.x86_64-linux.safix-examples` green; drills in 11.9, 11.10 and 11.11 observed

## 12. The five sync mappings, in both examples and in the comparison

Depends on `extend-bridge-fields`, `add-pass-bridge`, `add-bitwarden-bridge` and `add-onepassword-bridge`.
Mapping identifiers avoid every reserved word — `clan`, `keepassxc`, `pass`, `bitwarden`, `1password`, `all` (`modules/flake/safix/reserved.nix`, created by `extend-bridge-fields`).
Each mapping is one dendritic file under `modules/sync/` and one block in `examples/plain-nix/fleet.nix`.

- [ ] 12.1 `flake.safix.bridge = { clanFlake = ./.; mappings.ntfy-token = { direction = "clan-to-safix"; clan = { machine = "meridian"; generator = "ntfy"; file = "token"; }; safix = { user = "alice"; name = "ntfy-token"; }; }; }` — dendritic file `modules/sync/clan.nix` declares `clanFlake = ../..` so both sides elide to `<example-root>`
- [ ] 12.2 `flake.safix.keepassxc = { database = "/home/alice/.keys/example.kdbx"; group = "safix"; mappings.grafana = { mode = "safix-to-keepassxc"; safix = { user = "alice"; name = "grafana-password"; }; kdbx = { path = "alice/grafana"; fields = { username = "alice@example.com"; url = "https://grafana.example"; notes = "example mapping — no real database"; }; }; }; }` — no `tags` and no `{ entry = …; }` source, both refused for this target
- [ ] 12.3 `flake.safix.pass = { store = "~/.password-store"; mappings.deploy = { mode = "two-way"; safix = { user = "alice"; name = "deploy-token"; }; pass = { path = "alice/deploy"; fields = { username = { entry = "deploy-username"; }; url = "https://deploy.example"; notes = "example mapping"; tags = [ "example" ]; }; }; }; }` — the one mapping exercising an `{ entry = …; }` field source and all four fields, and the path carries no `.safix-sync-state` suffix
- [ ] 12.4 `flake.safix.bitwarden = { server = null; mappings.vpn = { mode = "safix-to-bitwarden"; safix = { user = "alice"; name = "vpn-password"; }; bitwarden = { folder = "safix"; item = "vpn"; fields = { username = "alice"; url = "https://vpn.example"; notes = "example mapping"; }; }; }; }` — no `tags`, refused for this target
- [ ] 12.5 `flake.safix.onepassword = { account = null; mappings.registry = { mode = "backup"; safix = { user = "alice"; name = "registry-token"; }; onepassword = { vault = "Private"; item = "registry"; fields = { username = "alice"; url = "https://registry.example"; notes = "example mapping"; tags = [ "example" ]; }; }; }; }`
- [ ] 12.6 Extend `comparedFields` (11.1) with `pass`, `bitwarden` and `onepassword` — or with whatever `crates/safix-core/src/nix.rs`'s `Attribute` enum names for them after the sibling changes land — and extend `entryAttrs` to match
- [ ] 12.7 Extend `coverageProbes` (11.7) with one row per target asserting its mapping set is non-empty and carries the declared far-side fields
- [ ] 12.8 Severity drill: delete the `fields` block from the pass mapping in the dendritic example alone and confirm `safix-examples` fails on the `pass` field and nothing else; revert. Recorded in `examples.nix`'s drill commentary
- [ ] 12.9 Severity drill: add `tags = [ "x" ]` to the bitwarden mapping's fields and confirm the evaluation refusal the sibling change specifies fires, naming the target and the field; revert. This drill holds that the examples are declared against the real option types
- [ ] 12.10 Verify: `nix build .#checks.x86_64-linux.safix-examples` and `.#checks.x86_64-linux.safix-examples-profiles` green; drills in 12.8 and 12.9 observed

## 13. `examples/README.md`

- [ ] 13.1 Rewrite `examples/README.md`. Three sections, one per example directory, plus `## What each check reads`. Delete `:12`'s claim that `entry.nix` "reaches `lib.mkVault` through `builtins.getFlake`" — it does not (`entry.nix:1-14`) — and `:34`'s claim that every file under both examples is read by the check, which is false for `examples/dendritic/flake.nix`
- [ ] 13.2 Delete `:21`'s "Fourteen of safix's fifteen verbs" and state the `--entry` fact without a count: every verb behaves identically under `--entry`, and `generate` additionally needs `--nixpkgs` or `SAFIX_NIXPKGS` because the generator sandbox resolves its tools through a flake
- [ ] 13.3 Delete `:5`'s and `:27`'s hand-maintained "twenty-two" counts; the dendritic side is read by directory now
- [ ] 13.4 Add the coverage table: one row per feature the examples declare, naming the file that declares it and the check row that holds it — `safix-examples`' field or probe, or `safix-examples-profiles`' row. Every row names an assertion; a feature with no assertion is a gap to close in group 11 or 10, not a row to write
- [ ] 13.5 Severity drill: delete one coverage row's declaration from both examples and confirm the named probe reddens, which is what makes the table evidence rather than a claim; revert. Recorded in `examples.nix`'s drill commentary
- [ ] 13.6 Verify: every path and check name the file states resolves — `for f in $(grep -oE 'examples/[a-z/.-]+\.nix' examples/README.md | sort -u); do test -e "$f" || echo "missing $f"; done` prints nothing

## 14. Roll-up

- [ ] 14.1 Re-run P1, P2 and P3 against the final `README.md`; all three empty
- [ ] 14.2 Re-run the per-verb coverage command from 4.9; sixteen ones
- [ ] 14.3 `nix build .#checks.x86_64-linux.safix-examples .#checks.x86_64-linux.safix-examples-profiles .#checks.x86_64-linux.treefmt` green
- [ ] 14.4 `CHANGELOG.md` `[Unreleased]` gains one entry: the README rewrite, the `## Status` move, the examples' new coverage, the new check, and the three `safix-examples` defects fixed. No option changed, so nothing in it is a migration note
- [ ] 14.5 Severity drill roll-up: re-run the drills in 1.4, 2.7, 3.8, 4.8, 5.11, 6.10, 7.10, 8.21, 9.21, 9.22, 10.14, 10.15, 10.16, 11.9, 11.10, 11.11, 12.8, 12.9 and 13.5 against the landed tree and record each observed red-to-green transition in the drill commentary of the file that carries it, the way `modules/flake/checks/vault.nix` records its own. A drill that does not reproduce is a finding: record why it did not, rather than dropping the row
- [ ] 14.6 Verify: `openspec validate rewrite-readme-and-examples --strict` green, and every box above ticked with its evidence in the commit message that closed it
