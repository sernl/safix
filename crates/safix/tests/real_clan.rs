//! The bridge driven against the real clan command, over a clan built for it.
//!
//! Every other bridge test drives the stub, and `tests/support/clan-stub.rs`
//! states what that can and cannot establish. What it cannot establish is that
//! the arguments mean to clan what this runtime thinks they mean: the stub
//! answers the argument vector safix sends because it was written to, and would
//! go on answering it after clan changed its command line, its output
//! convention, or its wording. This target is the other half of that sentence.
//!
//! # What is real here and what is not
//!
//! clan is real: the command from `clan-core`'s own `packages.clan-cli`, over a
//! clan with one machine, three `age`-backed generators, an identity minted
//! inside the check and a recipient derived from it. Its store is clan's own,
//! written and read by clan alone, and it commits in its own repository as it
//! goes. `modules/flake/checks/real-clan.nix` says what the three generators
//! differ in and why.
//!
//! sops, age and git on safix's side are real, as everywhere in this suite. Only
//! safix's *evaluator* is the stub, for the reason `harness/mod.rs` gives: what
//! this target is about is the clan boundary, and a real evaluation of a real
//! consumer fleet is `modules/flake/checks/`'s subject rather than this one's.
//!
//! # What this covers
//!
//! The delegation contracts recorded in `openspec/changes/clan-bridge/design.md`
//! under "What the real clan confirmed", each against the real command:
//!
//! - `clan vars get` hands back the raw bytes on a pipe, with no trailing byte
//!   added and no printable rendering substituted.
//! - `clan vars set` fed on standard input succeeds, and the real clan commits
//!   in its own repository.
//! - A second run of either direction writes nothing and commits nothing, which
//!   is the claim the read-first comparison exists for: clan's write is
//!   unconditional and its `age` backend re-encrypts, so an unchanged value
//!   still produces fresh ciphertext and a commit.
//! - A `sync clan` run with no `--direction` converges a clan-to-safix mapping
//!   and a safix-to-clan mapping in the one invocation that used to take two.
//! - The two absent-var states are distinguished by clan's own words — a
//!   declared generator holding nothing is an outcome the run continues past, an
//!   id nothing declares is a refusal naming the triple.
//! - A generator the real `clan vars check` calls stale drives the
//!   safix-to-clan direction's refusal, and `audit clan` reports a real
//!   divergence and reports none when the two sides hold the same bytes.
//! - Nothing safix ran changed clan's repository across a read, which is the
//!   prohibition held against a store that has real files to open.
//!
//! One of these was not designed but found. A generator that declares a
//! validation and has never run is reported stale by clan, because a declared
//! validation with nothing recorded beside it does not match, so the drift
//! refusal fires on a *first* safix-to-clan write into one. No fixture of the
//! stub would have produced that state, and it is what this target is for.
//!
//! # What this defers
//!
//! Anything requiring more than one machine, a shared generator, a prompt, or a
//! backend other than `age`. The out-of-sandbox miniature clan the design note
//! records also established clan's on-disk layout —
//! `secrets/clan-vars/per-machine/<machine>/<generator>/<file>/<file>.age` —
//! and nothing here asserts it, deliberately: that layout is the thing this
//! change exists in order not to depend on, so a test that pinned it would make
//! a virtue into a coupling.
//!
//! Absent rather than trivially green off linux and away from a clan. clan's own
//! `age` vars tests are marked `broken_on_darwin`, and generation runs the
//! generator under bubblewrap, which has no darwin equivalent a build sandbox
//! can reach. `modules/flake/checks/real-clan.nix` therefore declares the check
//! on linux only, and the environment-free half of this file says what it did
//! not do rather than passing silently.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

#[cfg(target_os = "linux")]
mod against_a_real_clan {
    use std::io::Write as _;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    use crate::harness::{ALICE_FILE, Fixture, Run, real_clan, real_clan_seed};

    /// The generator whose value the seed clan holds. It declares a validation,
    /// which is what lets clan be asked whether its definition has moved.
    const MINTED: &str = "ntfy";

    /// The generator the seed clan declares, leaves empty, and gives no
    /// validation — the ordinary safix-to-clan target, and the state every
    /// first write into it lands in.
    const EMPTY: &str = "handover";

    /// The generator that declares a validation and has never run. clan calls
    /// this one stale, because a declared validation with nothing recorded beside
    /// it does not match, and that is what the safix-to-clan direction refuses.
    const SCHEDULED: &str = "scheduled";

    /// The generator no bridge mapping this fixture declares ever names — the
    /// var `audit clan`'s lingering section is expected to report against the
    /// real command's own output shape.
    const ORPHAN: &str = "orphan";

    /// The generator declared `share = true`, minted on `MACHINE` alone; a
    /// second real machine in the seed clan declares no generator at all,
    /// which is what lets an addressing search genuinely distinguish "does
    /// not see this generator" from "has not generated it".
    const SHARED: &str = "bothways";
    /// The machine both are declared on.
    const MACHINE: &str = "meridian";

    /// The value the seed clan's generator mints, fixed by its script so that
    /// "safix pulled what clan was holding" is an assertion against a literal
    /// rather than against a second read through the same path.
    const FROM_CLAN: &str = "CANARY-minted-by-clan";

    /// One clan of this test's own, copied out of the seed the check built.
    ///
    /// A copy rather than a share, because these tests write into clan's store
    /// and commit in its repository. The identity travels with the copy — it is
    /// a file inside the clan — so each copy decrypts its own store and no test
    /// can read another's.
    struct Clan {
        flake: PathBuf,
        command: String,
    }

    impl Clan {
        /// The seed clan, copied for one test, or `None` when no check supplied
        /// one.
        fn copied(fixture: &Fixture) -> Option<Self> {
            let command = real_clan()?;
            let seed = real_clan_seed()?;
            let flake = fixture.work.join("clan");
            let copied = Command::new("cp")
                .arg("-r")
                .arg("--no-preserve=mode")
                .arg(&seed)
                .arg(&flake)
                .status()
                .expect("could not copy the seed clan");
            assert!(copied.success(), "the seed clan did not copy");
            Some(Self { flake, command })
        }

        /// The environment a safix run needs to reach this clan: the command, the
        /// identity its store is encrypted to, and a flake cache of its own.
        ///
        /// The cache is per clan rather than shared. clan keys it by flake path
        /// and each copy is its own path, so a shared one would hold an entry per
        /// test and answer none of them faster.
        fn environment(&self) -> Vec<(String, String)> {
            vec![
                ("SAFIX_CLAN".to_owned(), self.command.clone()),
                (
                    "AGE_KEYFILE".to_owned(),
                    self.flake.join(".age/key.txt").display().to_string(),
                ),
                (
                    "CLAN_TEST_FLAKE_CACHE".to_owned(),
                    self.flake.join(".flake-cache").display().to_string(),
                ),
            ]
        }

        /// One clan command, run directly, for the assertions this test makes
        /// about clan's own state rather than about safix's report of it.
        fn run(&self, arguments: &[&str]) -> std::process::Output {
            let mut command = Command::new(&self.command);
            command.arg("vars");
            command.args(arguments);
            command.arg("--flake").arg(&self.flake);
            for (name, value) in self.environment() {
                command.env(name, value);
            }
            command.output().expect("could not run clan")
        }

        /// What clan holds for one var, read through clan.
        ///
        /// Through the command and nowhere else, for the reason the runtime
        /// itself reads it that way: a test that opened clan's files would be
        /// asserting against a layout this change exists in order not to know.
        fn holds(&self, generator: &str, file: &str) -> Option<String> {
            let id = format!("{generator}/{file}");
            let finished = self.run(&["get", MACHINE, &id]);
            finished
                .status
                .success()
                .then(|| String::from_utf8_lossy(&finished.stdout).into_owned())
        }

        /// The commit clan's own repository is on.
        fn head(&self) -> String {
            let finished = Command::new("git")
                .arg("-C")
                .arg(&self.flake)
                .args(["rev-parse", "HEAD"])
                .output()
                .expect("could not read the clan repository's head");
            String::from_utf8_lossy(&finished.stdout).trim().to_owned()
        }

        /// Change the generator's declared validation, which is what clan
        /// records a hash of and what makes it call the generator stale.
        ///
        /// The definition rather than the script: `validationHash` is null unless
        /// the generator declares `validation`, so a script edit alone leaves
        /// clan calling the generator valid. Established against this clan —
        /// changing the script and asking `clan vars check` answered "All vars
        /// are present and valid."
        fn invalidate(&self, generator: &str) {
            let configuration = self
                .flake
                .join("machines")
                .join(MACHINE)
                .join("configuration.json");
            let text = std::fs::read_to_string(&configuration).unwrap();
            let mut declared: serde_json::Value = serde_json::from_str(&text).unwrap();
            declared["clan"]["core"]["vars"]["generators"][generator]["validation"]["revision"] =
                serde_json::json!(2);
            std::fs::write(&configuration, declared.to_string()).unwrap();
            let committed = Command::new("git")
                .arg("-C")
                .arg(&self.flake)
                .args(["commit", "-qam", "the definition moved", "--no-gpg-sign"])
                .status()
                .expect("could not commit in the clan repository");
            assert!(committed.success(), "the definition change did not commit");
        }

        /// Write a value directly into the real clan, bypassing safix.
        ///
        /// The clan-side counterpart of a hand-set safix write: used to move
        /// clan's side of a two-way mapping without going through the
        /// runtime under test, so that "safix converged toward what clan was
        /// already holding" is a claim about the runtime rather than about a
        /// write the runtime itself made.
        fn set(&self, generator: &str, file: &str, value: &str) {
            let id = format!("{generator}/{file}");
            let mut child = Command::new(&self.command)
                .arg("vars")
                .arg("set")
                .arg("--flake")
                .arg(&self.flake)
                .arg(MACHINE)
                .arg(&id)
                .envs(self.environment())
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("could not spawn clan vars set");
            {
                let mut stdin = child.stdin.take().expect("clan vars set has no stdin");
                stdin
                    .write_all(value.as_bytes())
                    .expect("could not write the value");
            }
            let finished = child
                .wait_with_output()
                .expect("could not wait for clan vars set");
            assert!(
                finished.status.success(),
                "clan vars set refused: {}",
                String::from_utf8_lossy(&finished.stderr)
            );
        }
    }

    /// A fixture and a clan, with one mapping between them.
    ///
    /// `placement` is `"per-machine"` or `"shared"`; `direction` may be
    /// `"two-way"`, in which case the fixture also mints the companion
    /// placement [`Fixture::seed_two_way_mapping`]/`_shared` mints, the way
    /// `crates/safix/tests/bridge_sync.rs`'s own fixtures do.
    ///
    /// `None` when no check supplied a clan, which every test below turns into a
    /// stated absence.
    fn bridged(direction: &str, placement: &str, generator: &str) -> Option<(Fixture, Clan)> {
        let mut fixture = Fixture::new();
        let clan = Clan::copied(&fixture)?;
        fixture.clan_flake_is(&clan.flake);
        match (direction, placement) {
            ("two-way", "shared") => fixture.seed_two_way_mapping_shared(
                "ntfy-token",
                (generator, "token"),
                ("alice", "api-token"),
            ),
            ("two-way", _) => fixture.seed_two_way_mapping(
                "ntfy-token",
                (MACHINE, generator, "token"),
                ("alice", "api-token"),
            ),
            (_, "shared") => fixture.seed_shared_mapping(
                "ntfy-token",
                direction,
                (generator, "token"),
                ("alice", "api-token"),
            ),
            _ => fixture.seed_mapping(
                "ntfy-token",
                direction,
                (MACHINE, generator, "token"),
                ("alice", "api-token"),
            ),
        }
        Some((fixture, clan))
    }

    /// A fixture and a clan, with one mapping of each direction: a
    /// clan-to-safix mapping reading the seed clan's already-minted generator,
    /// and a safix-to-clan mapping writing into its empty one.
    fn bridged_both_directions() -> Option<(Fixture, Clan)> {
        let mut fixture = Fixture::new();
        let clan = Clan::copied(&fixture)?;
        fixture.clan_flake_is(&clan.flake);
        fixture.seed_mapping(
            "down",
            "clan-to-safix",
            (MACHINE, MINTED, "token"),
            ("alice", "api-token"),
        );
        fixture.seed_mapping(
            "up",
            "safix-to-clan",
            (MACHINE, EMPTY, "token"),
            ("alice", "mail-password"),
        );
        Some((fixture, clan))
    }

    /// One safix run against the real clan.
    fn bridge(fixture: &Fixture, clan: &Clan, arguments: &[&str]) -> Run {
        let environment = clan.environment();
        let borrowed: Vec<(&str, &str)> = environment
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        fixture.run_env(arguments, None, &borrowed)
    }

    /// Said once, so that a run without a clan is one line rather than one per
    /// test claiming to have passed.
    fn no_clan_here(claim: &str) {
        eprintln!(
            "no real clan in this environment, so nothing was established about {claim}. \
             The claim is made by `safix-bridge-real-clan`, which exists on linux and \
             only when clan-cli is in the check closure; the delegation itself is held \
             everywhere by `safix-bridge-transfer` against the stub."
        );
    }

    // ── clan to safix ──────────────────────────────────────────────────────

    /// The bytes the real clan minted are the bytes that land in alice's file.
    ///
    /// The end-to-end clan-to-safix claim, and the one that makes the
    /// raw-capture contract real rather than recorded: `clan vars get` writes
    /// the value when its output is not a terminal and a printable rendering
    /// when it is, and a rendering would not decrypt back to this literal.
    #[test]
    fn a_clan_to_safix_run_lands_the_bytes_the_real_clan_minted() {
        let Some((fixture, clan)) = bridged("clan-to-safix", "per-machine", MINTED) else {
            return no_clan_here("converging from a real clan");
        };

        let run = bridge(&fixture, &clan, &["sync", "clan"])
            .expect_success("converging a declared mapping against a real clan");
        run.says("ntfy-token");
        run.says("pulled ntfy-token");
        run.silent_about(FROM_CLAN);

        assert_eq!(
            fixture.value(ALICE_FILE, "api-token"),
            FROM_CLAN,
            "the converged value is not what the real clan was holding"
        );
    }

    /// A second run over the same mapping writes nothing and commits nothing.
    #[test]
    fn a_second_run_leaves_both_repositories_where_they_were() {
        let Some((fixture, clan)) = bridged("clan-to-safix", "per-machine", MINTED) else {
            return no_clan_here("converging a mapping twice against a real clan");
        };

        bridge(&fixture, &clan, &["sync", "clan"]).expect_success("the first run");
        let settled = fixture.head();
        let document = fixture.read(ALICE_FILE);

        let again = bridge(&fixture, &clan, &["sync", "clan"]).expect_success("the second run");
        again.says("0 updated, 1 unchanged");
        assert_eq!(fixture.head(), settled, "the second run committed");
        assert_eq!(
            fixture.read(ALICE_FILE),
            document,
            "the second run rewrote the file, which re-encrypts it for no reason"
        );
    }

    /// A generator the clan declares and has not run is an outcome, not a
    /// failure.
    ///
    /// This is the state every clan is in before its first generation, and a
    /// runtime that read clan's non-zero status as a failure would stop a bridge
    /// run during bootstrap.
    #[test]
    fn a_generator_the_real_clan_has_not_run_is_reported_and_the_run_continues() {
        let Some((fixture, clan)) = bridged("clan-to-safix", "per-machine", EMPTY) else {
            return no_clan_here("converging a var the real clan has not generated");
        };

        let run = bridge(&fixture, &clan, &["sync", "clan"])
            .expect_success("converging a var clan holds nothing for");
        run.says("absent at source");
    }

    /// A triple the real clan has no declaration for is a refusal naming it.
    ///
    /// The other half of the pair above, and clan makes the distinction: one is
    /// "has not been generated yet" and the other is "Couldn't find var". A
    /// runtime treating them alike would refuse every first safix-to-clan write.
    #[test]
    fn a_triple_the_real_clan_declares_nothing_for_is_refused_by_name() {
        let Some((fixture, clan)) =
            bridged("clan-to-safix", "per-machine", "nothing-declares-this")
        else {
            return no_clan_here("converging an undeclared triple");
        };

        let run = bridge(&fixture, &clan, &["sync", "clan"])
            .expect_refusal("converging a triple clan knows nothing about");
        run.says("nothing-declares-this");
        run.says(MACHINE);
    }

    // ── safix to clan ──────────────────────────────────────────────────────

    /// safix's value reaches the real clan's store, and the real clan commits.
    ///
    /// The value goes in on standard input and appears in no argument vector,
    /// which is what `clan vars set` takes and what `safix-bridge-transfer`
    /// asserts from the far side. What is added here is that clan accepted it:
    /// the read afterwards is clan's own, and clan's repository moved.
    #[test]
    fn a_safix_to_clan_run_puts_safixs_value_into_the_real_clan_and_clan_commits_it() {
        let Some((fixture, clan)) = bridged("safix-to-clan", "per-machine", EMPTY) else {
            return no_clan_here("converging into a real clan");
        };
        fixture
            .set("alice", "api-token", "CANARY-exported-for-real")
            .expect_success("seeding the source");
        let clan_was_on = clan.head();
        let safix_was_on = fixture.head();

        let run = bridge(&fixture, &clan, &["sync", "clan"])
            .expect_success("converging into a real clan");
        run.says("ntfy-token");
        run.says("pushed ntfy-token");
        run.silent_about("CANARY-exported-for-real");

        assert_eq!(
            clan.holds(EMPTY, "token").as_deref(),
            Some("CANARY-exported-for-real"),
            "the real clan does not hold what safix was holding"
        );
        assert_ne!(
            clan.head(),
            clan_was_on,
            "the real clan committed nothing, so the comparison below would prove nothing"
        );
        assert_eq!(
            fixture.head(),
            safix_was_on,
            "the run committed in this repository, where nothing changed"
        );
    }

    /// A second run does not move the real clan's repository.
    ///
    /// The load-bearing claim of the whole safix-to-clan direction, and it can
    /// only be made here: clan's write is unconditional and its `age` backend
    /// re-encrypts, so the stub's write count stands in for what this measures
    /// directly — a commit in clan's own history per mapping per run, forever,
    /// each one a fresh ciphertext of an unchanged value.
    #[test]
    fn a_second_run_does_not_move_the_real_clans_history() {
        let Some((fixture, clan)) = bridged("safix-to-clan", "per-machine", EMPTY) else {
            return no_clan_here("converging a mapping twice against a real clan");
        };
        fixture
            .set("alice", "api-token", "CANARY-exported-once")
            .expect_success("seeding the source");

        bridge(&fixture, &clan, &["sync", "clan"]).expect_success("the first run");
        let settled = clan.head();

        let again = bridge(&fixture, &clan, &["sync", "clan"]).expect_success("the second run");
        again.says("unchanged");
        assert_eq!(
            clan.head(),
            settled,
            "the second run committed in the clan repository, so every run would"
        );
    }

    /// A safix-to-clan write into a generator the real `clan vars check` calls
    /// stale is refused.
    ///
    /// The refusal decision two installs, against the command that answers the
    /// question. Nothing here reads or computes a hash: the definition's declared
    /// validation moves, and clan is asked.
    #[test]
    fn a_safix_to_clan_run_refuses_a_generator_the_real_clan_calls_stale() {
        let Some((fixture, clan)) = bridged("safix-to-clan", "per-machine", MINTED) else {
            return no_clan_here("refusing a write into a stale generator");
        };
        fixture
            .set("alice", "api-token", "CANARY-would-be-lost")
            .expect_success("seeding the source");
        clan.invalidate(MINTED);
        let settled = clan.head();

        let run = bridge(&fixture, &clan, &["sync", "clan"])
            .expect_refusal("converging into a generator clan calls stale");
        run.says("outdated");
        run.says("clan-to-safix");
        run.silent_about("CANARY-would-be-lost");

        assert_eq!(
            clan.holds(MINTED, "token").as_deref(),
            Some(FROM_CLAN),
            "the refused run wrote into clan anyway"
        );
        assert_eq!(
            clan.head(),
            settled,
            "the refused run committed in the clan repository"
        );
    }

    // ── one run, both directions ────────────────────────────────────────────

    /// A `sync clan` run with no `--direction` converges a clan-to-safix
    /// mapping and a safix-to-clan mapping against the real clan, in one
    /// invocation.
    ///
    /// The headline claim of the target-scoped grammar, confirmed against the
    /// real command rather than the stub: what used to take two, `import` and
    /// `export`, now happens in one, and each mapping still moves in its own
    /// declared direction.
    #[test]
    fn sync_clan_converges_both_directions_against_a_real_clan_in_one_run() {
        let Some((fixture, clan)) = bridged_both_directions() else {
            return no_clan_here("converging both directions in one run against a real clan");
        };
        fixture
            .set("alice", "mail-password", "CANARY-pushed-for-real")
            .expect_success("seeding the safix-to-clan source");

        let run = bridge(&fixture, &clan, &["sync", "clan"])
            .expect_success("converging both directions in one run");
        run.says("pulled down");
        run.says("pushed up");

        assert_eq!(
            fixture.value(ALICE_FILE, "api-token"),
            FROM_CLAN,
            "the clan-to-safix mapping did not converge against the real clan"
        );
        assert_eq!(
            clan.holds(EMPTY, "token").as_deref(),
            Some("CANARY-pushed-for-real"),
            "the safix-to-clan mapping did not converge against the real clan"
        );
    }

    // ── the report over the same declarations ───────────────────────────────

    /// Two sides holding different bytes is a finding; agreeing is not.
    ///
    /// Both directions of the claim in one test, with a sync between them,
    /// because the audit's claim is that it reports what a sync resolves. A
    /// report still red afterwards would be reporting something else.
    #[test]
    fn audit_clan_finds_a_real_divergence_and_finds_none_once_it_is_resolved() {
        let Some((fixture, clan)) = bridged("clan-to-safix", "per-machine", MINTED) else {
            return no_clan_here("auditing against a real clan");
        };
        fixture
            .set("alice", "api-token", "CANARY-disagrees-with-clan")
            .expect_success("seeding a disagreement");

        let report =
            bridge(&fixture, &clan, &["audit", "clan"]).expect_refusal("auditing a diverged pair");
        report.says("ntfy-token");
        report.says("different values");
        report.silent_about("CANARY-disagrees-with-clan");
        report.silent_about(FROM_CLAN);

        bridge(&fixture, &clan, &["sync", "clan"]).expect_success("resolving the divergence");

        let after = bridge(&fixture, &clan, &["audit", "clan"])
            .expect_success("auditing the resolved pair");
        after.says("no disagreement");
    }

    /// The audit's lingering section names a real clan var no bridge mapping
    /// in this fixture claims — `enumerate-clan-namespace`'s own claim, held
    /// against the real command's own `vars list` output rather than only
    /// against the stub's.
    #[test]
    fn audit_clan_reports_a_real_var_no_mapping_names() {
        let Some((fixture, clan)) = bridged("clan-to-safix", MINTED) else {
            return no_clan_here("the lingering report against a real clan");
        };
        bridge(&fixture, &clan, &["sync", "clan"])
            .expect_success("bringing the declared mapping into agreement");

        let report = bridge(&fixture, &clan, &["audit", "clan"])
            .expect_success("an unmapped var alone must not refuse the run");
        report.says(&format!("{MACHINE} {ORPHAN}/token"));
        report.says("no declared mapping accounts for it");
    }

    /// Once a mapping is declared for a var that used to be unclaimed, the
    /// next audit stops naming it — the scope is genuinely computed from the
    /// current declarations against the real command, not cached across runs.
    #[test]
    fn a_newly_mapped_real_var_stops_lingering() {
        let Some((mut fixture, clan)) = bridged("clan-to-safix", MINTED) else {
            return no_clan_here("re-scoping the lingering report against a real clan");
        };
        bridge(&fixture, &clan, &["sync", "clan"])
            .expect_success("bringing the first mapping into agreement");
        bridge(&fixture, &clan, &["audit", "clan"])
            .expect_success("an unmapped var alone must not refuse the run")
            .says(&format!("{MACHINE} {ORPHAN}/token"));

        fixture.seed_mapping(
            "orphan-token",
            "clan-to-safix",
            (MACHINE, ORPHAN, "token"),
            ("alice", "orphan-token"),
        );
        bridge(&fixture, &clan, &["sync", "clan"])
            .expect_success("bringing the second mapping into agreement");

        bridge(&fixture, &clan, &["audit", "clan"])
            .expect_success("both mappings now agree")
            .silent_about("no declared mapping accounts for it");
    }

    /// The value came off a pipe rather than out of a terminal rendering.
    ///
    /// Asserted from this side because there is no far side to ask: the real
    /// clan records nothing. What makes it a claim rather than a hope is the
    /// literal — `clan_cli/vars/get.py` prints `var.printable_value` on the
    /// terminal branch, and that rendering is not these bytes.
    #[test]
    fn the_real_clans_read_gave_bytes_and_not_a_rendering() {
        let Some((fixture, clan)) = bridged("clan-to-safix", "per-machine", MINTED) else {
            return no_clan_here("the raw-capture contract");
        };

        bridge(&fixture, &clan, &["sync", "clan"]).expect_success("converging from a real clan");
        let landed = fixture.value(ALICE_FILE, "api-token");

        assert_eq!(
            landed, FROM_CLAN,
            "what landed is not the generator's own bytes"
        );
        assert!(
            !landed.ends_with('\n'),
            "a trailing byte was added across the boundary, which corrupts a key whose last byte matters"
        );
    }

    /// A generator that declares a validation and has never run is refused.
    ///
    /// Discovered here rather than designed: clan reports a generator whose
    /// declared validation has nothing recorded beside it as having an outdated
    /// invalidation hash, so the drift refusal fires on the *first* safix-to-clan
    /// write into one. That is the right answer — such a generator has not run
    /// and will, and the run would replace whatever the write sent — and no
    /// fixture of the stub would have produced the state, because the stub's
    /// staleness is a switch a test throws rather than a consequence of the
    /// store.
    #[test]
    fn a_safix_to_clan_run_refuses_a_generator_that_declares_a_validation_and_has_never_run() {
        let Some((fixture, clan)) = bridged("safix-to-clan", "per-machine", SCHEDULED) else {
            return no_clan_here("refusing a first write into a scheduled generator");
        };
        fixture
            .set("alice", "api-token", "CANARY-would-be-replaced")
            .expect_success("seeding the source");
        let settled = clan.head();

        let run = bridge(&fixture, &clan, &["sync", "clan"])
            .expect_refusal("converging into a generator that has never run");
        run.says("outdated");
        run.says(SCHEDULED);
        run.silent_about("CANARY-would-be-replaced");
        assert_eq!(
            clan.head(),
            settled,
            "the refused run committed in the clan repository"
        );
    }

    // ── two-way convergence ─────────────────────────────────────────────────

    /// Neither side holding a value writes nothing anywhere, and clan's tree
    /// does not move.
    #[test]
    fn a_two_way_run_with_neither_side_moved_writes_nothing() {
        let Some((fixture, clan)) = bridged("two-way", "per-machine", EMPTY) else {
            return no_clan_here("a two-way run against a real clan with neither side moved");
        };
        let before = digest(&clan.flake);

        let run = bridge(&fixture, &clan, &["sync", "clan"])
            .expect_success("converging with neither side holding a value");
        run.says("ntfy-token");
        run.says("unchanged");

        assert_eq!(
            digest(&clan.flake),
            before,
            "an unchanged run moved the real clan's tree"
        );
    }

    /// safix holding a value and clan holding none pushes it into the real
    /// clan, and records the agreement in the companion.
    #[test]
    fn a_two_way_run_with_only_safixs_side_moved_pushes_it_into_the_real_clan() {
        let Some((fixture, clan)) = bridged("two-way", "per-machine", EMPTY) else {
            return no_clan_here("a two-way push against a real clan");
        };
        fixture
            .set("alice", "api-token", "CANARY-two-way-pushed-for-real")
            .expect_success("seeding the safix side");

        let run = bridge(&fixture, &clan, &["sync", "clan"])
            .expect_success("converging toward the real clan");
        run.says("converged ntfy-token");
        run.silent_about("CANARY-two-way-pushed-for-real");

        assert_eq!(
            clan.holds(EMPTY, "token").as_deref(),
            Some("CANARY-two-way-pushed-for-real"),
            "the real clan does not hold what safix pushed"
        );
        assert!(
            fixture
                .value(ALICE_FILE, "api-token-safix-bridge-sync-state")
                .starts_with("safix-bridge-sync-v1 "),
            "the companion does not carry the recorded agreement"
        );
    }

    /// clan holding a value, set directly rather than through the runtime
    /// under test, and safix holding none pulls it in.
    #[test]
    fn a_two_way_run_with_only_clans_side_moved_pulls_it_from_the_real_clan() {
        let Some((fixture, clan)) = bridged("two-way", "per-machine", EMPTY) else {
            return no_clan_here("a two-way pull from a real clan");
        };
        clan.set(EMPTY, "token", "CANARY-two-way-pulled-for-real");

        let run = bridge(&fixture, &clan, &["sync", "clan"])
            .expect_success("converging from the real clan");
        run.says("converged ntfy-token");
        run.silent_about("CANARY-two-way-pulled-for-real");

        assert_eq!(
            fixture.value(ALICE_FILE, "api-token"),
            "CANARY-two-way-pulled-for-real",
            "safix does not hold what the real clan was holding"
        );
    }

    /// Both sides moved, with no agreement recorded yet: a conflict against
    /// a real clan, exactly as against the stub, and clan's tree does not
    /// move.
    #[test]
    fn a_two_way_run_with_both_sides_moved_is_a_conflict_against_a_real_clan() {
        let Some((fixture, clan)) = bridged("two-way", "per-machine", EMPTY) else {
            return no_clan_here("a two-way conflict against a real clan");
        };
        fixture
            .set("alice", "api-token", "CANARY-safix-side-for-real")
            .expect_success("seeding the safix side");
        clan.set(EMPTY, "token", "CANARY-clan-side-for-real");
        let before = digest(&clan.flake);

        let run = bridge(&fixture, &clan, &["sync", "clan"])
            .expect_refusal("a conflict is what makes the run's exit code non-zero");
        run.says("ntfy-token");
        run.says("conflict");
        run.silent_about("CANARY-safix-side-for-real");
        run.silent_about("CANARY-clan-side-for-real");

        assert_eq!(
            digest(&clan.flake),
            before,
            "a conflict moved the real clan's tree"
        );
        assert_eq!(
            fixture.value(ALICE_FILE, "api-token"),
            "CANARY-safix-side-for-real",
            "a conflict overwrote safix's side"
        );
    }

    /// A two-way push into a generator the real clan calls stale is refused
    /// by the identical condition a safix-to-clan write already carries,
    /// reusing
    /// `a_safix_to_clan_run_refuses_a_generator_the_real_clan_calls_stale`'s
    /// own fixture shape with a two-way mapping instead.
    #[test]
    fn a_two_way_push_refuses_a_generator_the_real_clan_calls_stale() {
        let Some((fixture, clan)) = bridged("two-way", "per-machine", MINTED) else {
            return no_clan_here("refusing a two-way push into a generator clan calls stale");
        };
        fixture
            .set("alice", "api-token", "CANARY-would-be-lost-two-way")
            .expect_success("seeding the source");
        clan.invalidate(MINTED);
        let settled = clan.head();

        let run = bridge(&fixture, &clan, &["sync", "clan"])
            .expect_refusal("converging a two-way push into a generator clan calls stale");
        run.says("outdated");
        run.silent_about("CANARY-would-be-lost-two-way");

        assert_eq!(
            clan.head(),
            settled,
            "the refused two-way push committed in the clan repository"
        );
    }

    /// A shared-placement two-way mapping's clan side is reached by a
    /// machine discovered from clan's own `machines list`, skipping a real
    /// second machine that declares no generator at all — the property
    /// `crates/safix/tests/bridge_sync.rs`'s own stubbed test defers to this
    /// file, because its stub cannot tell "does not declare this generator"
    /// apart from "globally unknown" for a machine it was not told to name.
    #[test]
    fn a_shared_placements_machine_is_discovered_from_a_real_clan_skipping_an_unrelated_one() {
        let Some((fixture, clan)) = bridged("two-way", "shared", SHARED) else {
            return no_clan_here("discovering a shared mapping's machine against a real clan");
        };

        let run = bridge(&fixture, &clan, &["sync", "clan"])
            .expect_success("a shared mapping's machine is discovered rather than declared");
        run.says("converged ntfy-token");

        assert_eq!(
            fixture.value(ALICE_FILE, "api-token"),
            "CANARY-shared-and-real",
            "the shared mapping did not converge through the discovered machine"
        );
    }

    /// Nothing safix ran opened a file the real clan placed.
    ///
    /// The prohibition decision one states, held against the clan that has real
    /// files to open. A run that reached past the command would find clan's
    /// store here — it is a real one, in its documented layout — so this is the
    /// one place the claim can fail rather than being true for want of a target.
    ///
    /// Measured after the sides are brought into agreement, because the reads
    /// under test are the ones that change nothing: a converging run that
    /// transfers writes on safix's side, and asserting that clan's tree did not
    /// move across it would be asserting about the wrong run.
    #[test]
    fn the_runtime_reached_clans_store_only_through_clans_command() {
        let Some((mut fixture, clan)) = bridged("clan-to-safix", "per-machine", MINTED) else {
            return no_clan_here("the prohibition on reading clan's store");
        };
        fixture.seed_two_way_mapping(
            "handover-token",
            (MACHINE, EMPTY, "token"),
            ("alice", "handover-token"),
        );
        bridge(&fixture, &clan, &["sync", "clan"]).expect_success(
            "bringing the two sides into step, including the two-way mapping's own convergence",
        );

        let before = digest(&clan.flake);
        bridge(&fixture, &clan, &["audit", "clan"]).expect_success("auditing an agreeing pair");
        bridge(&fixture, &clan, &["sync", "clan"]).expect_success("converging an unchanged value");
        let after = digest(&clan.flake);

        assert_eq!(
            before, after,
            "a read of clan's side changed clan's repository, and no read writes"
        );
    }

    /// Every path under a tree, with its bytes, as one sorted listing.
    fn digest(tree: &Path) -> String {
        let mut lines: Vec<String> = Vec::new();
        collect(tree, tree, &mut lines);
        lines.sort();
        lines.join("\n")
    }

    fn collect(root: &Path, directory: &Path, lines: &mut Vec<String>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().display().to_string();
            // clan's flake cache and nix's own scratch live under the clan and
            // move on every read, which is not a change to the repository.
            if relative.starts_with(".flake-cache") {
                continue;
            }
            if path.is_dir() {
                collect(root, &path, lines);
            } else {
                let bytes = std::fs::read(&path).unwrap_or_default();
                lines.push(format!("{relative} {}", bytes.len()));
            }
        }
    }
}

/// What was not established here, said out loud.
///
/// An absent attribute is cleaner than a check that passes having done nothing,
/// and `modules/flake/checks/real-clan.nix` is where the attribute is absent.
/// This is the compiled suite's own half of the same statement: `cargo test` in
/// a devshell has no clan and no throwaway clan, and a target that quietly did
/// nothing there is how a claim stops being made without anybody deciding to
/// stop making it.
#[cfg(not(target_os = "linux"))]
#[test]
fn the_real_clan_check_needs_linux_and_was_not_made_here() {
    eprintln!(
        "the real-clan check generates a var, which runs the generator under \
         bubblewrap, and clan's own age vars tests are marked broken on darwin. \
         Nothing was established about the real command on this platform. The \
         delegation itself is held everywhere by `safix-bridge-transfer` against \
         the stub."
    );
}
