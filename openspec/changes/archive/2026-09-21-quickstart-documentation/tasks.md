# Tasks

## 1. The checked example

- [x] 1.1 Create `examples/quickstart/{flake.nix,secrets.nix,hosts/web.nix,home/alice.nix}` with region markers for every snippet the README will quote; verify `nix eval` of both profiles through `mkVault` succeeds outside any check.
- [x] 1.2 Add the `examples-quickstart` check evaluating the host and home profiles and asserting the materialized entries, template and rotation option; verify it builds.
- [x] 1.3 Add the `readme-snippets` check that resolves each README nix block's `title` to a file or region and diffs bytes, failing on an untitled nix block; verify it builds against the new README and fails when a snippet is edited.
- [x] 1.4 Add the fourth example to `examples/README.md` with what each check holds; verify the examples document names only what a check evaluates.

## 2. README

- [x] 2.1 Rewrite `README.md` as the quickstart in D6's order, every nix block lifted from the example by title, prose under forty words per sentence, links into `docs/` for every unexpanded mention; verify `readme-snippets` passes and a sentence-length script reports none over forty words.
- [x] 2.2 Add the flake-parts section stating full support and pointing at the guide and `examples/dendritic`; verify the link targets exist.

## 3. Docs tree

- [x] 3.1 Create `docs/reference/{cli,declarations,profile-options,migration-plan,environment,refusals}.md` from the option declarations and the verb table, each option and verb exactly once; verify a script lists every `flake.safix.*` and `safix.*` option and every verb and finds each in exactly one reference page.
- [x] 3.2 Create `docs/concepts/{model,custody,storage-layout}.md`, stating the revocation rule and the namespace rule once; verify a grep finds each rule's statement in one file and only references elsewhere.
- [x] 3.3 Create the guides in D4 including the three custody guides and the sync page with its five same-shaped subsections; verify the sync subsections carry identical headings in order.
- [x] 3.4 Create `docs/tutorials/first-secret.md` and `docs/index.md` mapping the tree; verify every page has a front-matter title and every internal link resolves.
- [x] 3.5 Add the snippet-check paragraph to `CONTRIBUTING.md` and move the platform-conditional check narration there if the old README still carried any; verify the README names no check.

## 4. Verification

- [x] 4.1 Run the examples, snippet and consumption checks; verify all pass.
- [x] 4.2 Read the README top to bottom as a new NixOS user, following each snippet into the example; verify each step's code is the next step's input and record the read in the change.

## Observed

### The README read as a new NixOS user

Read top to bottom against `examples/quickstart/`, checking at each step that the code the previous step produced is what the next step consumes.

- `flake.nix#inputs` adds the input. The two consumption modules are named in prose and appear literally in the two `#bind` blocks further down, so nothing is promised that is not later shown.
- `secrets.nix#declare` declares `users.alice` and `machines.web`. Both recipients are placeholders, and the first command step is what replaces them.
- `flake.nix#outputs` names `./secrets.nix`, so the file written in the previous step is the file the binding reads. It also publishes `safix.lib`, which is the attribute every verb in the next step evaluates.
- `safix keygen` prints the recipient the reader pastes into `flake.safix.users.alice.recipient`, and `ssh-to-age` supplies the machine's. `safix fix` then has a complete audience to write policy from, which is why it precedes `safix set`. `safix set alice grafana-admin-password` names the entry declared two steps earlier, and `safix list alice` and `safix view alice` read what it wrote.
- `hosts/web.nix#bind` reaches the `safix.lib` output through `safix.flake`, and `safix.machine = "web"` names the machine declared in `secrets.nix#declare`.
- `hosts/web.nix#service` reads `config.safix.secrets."grafana-admin-password".path`, which exists at this scope only because of the `sharedWith.web` line in the declaration block.
- The rebuild step explains the `restartUnits` already visible in the declaration block rather than introducing a new field.
- `secrets.nix#generator` adds the second entry and grants it onward the same way. `secrets.nix#policy` declares the interval the generator block's `rotation = "quarterly"` names, and `home/alice.nix#rotation` is the timer that runs `rotate --due` on a schedule.
- `hosts/web.nix#template` renders both entries' placeholders, which resolve only because both were granted to the machine in the two declaration blocks. Dropping either grant makes the host profile fail to evaluate, which the example check's first drill records.
- `home/alice.nix#bind` reuses the same declarations at user scope and names the key file `safix keygen` appended to in the command step.

No step depends on code the reader has not yet written, and no block introduces an option the page had not reached. The sentence-length command in `CONTRIBUTING.md` reports nothing over forty words, its check-name command reports nothing, and every relative link in the document resolves to a file that exists.
