# Tasks

## 1. The checked example

- [ ] 1.1 Create `examples/quickstart/{flake.nix,secrets.nix,hosts/web.nix,home/alice.nix}` with region markers for every snippet the README will quote; verify `nix eval` of both profiles through `mkVault` succeeds outside any check.
- [ ] 1.2 Add the `examples-quickstart` check evaluating the host and home profiles and asserting the materialized entries, template and rotation option; verify it builds.
- [ ] 1.3 Add the `readme-snippets` check that resolves each README nix block's `title` to a file or region and diffs bytes, failing on an untitled nix block; verify it builds against the new README and fails when a snippet is edited.
- [ ] 1.4 Add the fourth example to `examples/README.md` with what each check holds; verify the examples document names only what a check evaluates.

## 2. README

- [ ] 2.1 Rewrite `README.md` as the quickstart in D6's order, every nix block lifted from the example by title, prose under forty words per sentence, links into `docs/` for every unexpanded mention; verify `readme-snippets` passes and a sentence-length script reports none over forty words.
- [ ] 2.2 Add the flake-parts section stating full support and pointing at the guide and `examples/dendritic`; verify the link targets exist.

## 3. Docs tree

- [ ] 3.1 Create `docs/reference/{cli,declarations,profile-options,migration-plan,environment,refusals}.md` from the option declarations and the verb table, each option and verb exactly once; verify a script lists every `flake.safix.*` and `safix.*` option and every verb and finds each in exactly one reference page.
- [ ] 3.2 Create `docs/concepts/{model,custody,storage-layout}.md`, stating the revocation rule and the namespace rule once; verify a grep finds each rule's statement in one file and only references elsewhere.
- [ ] 3.3 Create the guides in D4 including the three custody guides and the sync page with its five same-shaped subsections; verify the sync subsections carry identical headings in order.
- [ ] 3.4 Create `docs/tutorials/first-secret.md` and `docs/index.md` mapping the tree; verify every page has a front-matter title and every internal link resolves.
- [ ] 3.5 Add the snippet-check paragraph to `CONTRIBUTING.md` and move the platform-conditional check narration there if the old README still carried any; verify the README names no check.

## 4. Verification

- [ ] 4.1 Run the examples, snippet and consumption checks; verify all pass.
- [ ] 4.2 Read the README top to bottom as a new NixOS user, following each snippet into the example; verify each step's code is the next step's input and record the read in the change.
