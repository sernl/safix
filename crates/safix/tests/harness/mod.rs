//! A throwaway repository, throwaway keys, and the built binary driven against
//! them.
//!
//! Every claim this suite makes is made against the tools that will run at an
//! operator's terminal: the real sops, the real age, the real git. Only `nix` is
//! stubbed, and the stub asserts what it was asked for — see
//! `tests/support/nix-stub.rs`. Standing a stub in for sops is what would let a
//! check stay green over a command calling something the tree no longer
//! contains, and is not done.
//!
//! Every key here is minted inside the fixture's own scratch directory, every
//! user name is a fixture name, and no recipient, ciphertext or value from
//! anywhere else appears in this file or in anything it writes.
//!
//! # Where the plaintext goes
//!
//! Values reach disk here — a fixture value, a canary, an age identity — so the
//! scratch directory is a mode-700 directory on tmpfs, verified as tmpfs at
//! runtime rather than assumed, and removed on every exit path. `/tmp` on the
//! machines this runs on is disk-backed, which is why this is enforced rather
//! than preferred. A platform with no tmpfs refuses unless
//! `SAFIX_TEST_DISK_STAGING` says the caller accepts disk-backed staging.
//!
//! The drills that exercise the disk-staging refusal are the one exception, and
//! they are deliberate: a run under `--allow-disk-staging` has to stage
//! somewhere the kernel calls disk-backed or it is not the drill. That directory
//! is [`Fixture::disk_staging_dir`], made per fixture and removed on drop, so
//! the acknowledged run's plaintext is neither in a directory the rest of the
//! machine shares nor left behind.
//!
//! # Why some runs go through `setsid`
//!
//! A generator's prompt and a confirmation are read from `/dev/tty` when it opens
//! and from standard input when it does not. A build sandbox has no controlling
//! terminal, so the stdin branch is what the checks exercise; a developer's
//! terminal has one, so a run whose answers arrive on standard input is detached
//! into its own session first. Runs that are meant to block — the interrupted
//! ones — are not detached, because the signal has to reach the process itself.
//!
//! `set` is no longer among them and is worth stating separately: it chooses its
//! source by asking whether *standard input* is a terminal, so a run fed by a pipe
//! takes the stream source whatever terminal the machine has. Reaching its prompt
//! path therefore takes a real terminal on standard input, which is
//! [`Fixture::set_on_a_terminal`].

// A test's failure is the point, so the panicking constructions the workspace
// denies are what an assertion here is made of. This is `clippy.toml`'s
// `allow-unwrap-in-tests` reasoning, spelled at the target level because that
// setting reaches `#[test]` functions and not the helpers they call.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::missing_panics_doc
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::{Value, json};

/// alice's own file: the audience is one person, and the creation rule grants
/// her alone.
pub const ALICE_FILE: &str = "secrets/safix/users/alice/secrets.yaml";

/// `^D`, the character a terminal in canonical mode reads as the end of input.
///
/// The default `VEOF`, which is what a person presses and therefore what a
/// pseudoterminal-driven run sends. Not a value this suite chose: it is the
/// terminal discipline's own, and a run reading past the lines it was given has to
/// see the end rather than block.
const END_OF_INPUT: u8 = 0x04;

/// The file the pair shares, in the audience directory named for both in sorted
/// order.
pub const SHARED_FILE: &str = "secrets/safix/shared/alice,bob/secrets.yaml";

/// The built binary under test.
pub fn safix() -> &'static str {
    static PATH: OnceLock<String> = OnceLock::new();
    PATH.get_or_init(|| located("SAFIX_TEST_BINARY", env!("CARGO_BIN_EXE_safix")))
        .as_str()
}

/// The evaluator the suite answers with.
fn nix_stub() -> &'static str {
    static PATH: OnceLock<String> = OnceLock::new();
    PATH.get_or_init(|| located("SAFIX_TEST_NIX_STUB", env!("CARGO_BIN_EXE_safix-nix-stub")))
        .as_str()
}

/// The shim the residue and drill checks put in the runtime's way.
pub fn shim() -> &'static str {
    static PATH: OnceLock<String> = OnceLock::new();
    PATH.get_or_init(|| located("SAFIX_TEST_SHIM", env!("CARGO_BIN_EXE_safix-test-shim")))
        .as_str()
}

/// The card surface the enrollment tests are driven against.
///
/// One binary with four roles. Nothing in this suite runs the real `ykman`, the
/// real age plugin or a real password store, and that is a safety property rather
/// than a convenience: see the head of `tests/support/card-stubs.rs`.
pub fn card_stub() -> &'static str {
    static PATH: OnceLock<String> = OnceLock::new();
    PATH.get_or_init(|| {
        located(
            "SAFIX_TEST_CARD_STUB",
            env!("CARGO_BIN_EXE_safix-card-stub"),
        )
    })
    .as_str()
}

/// The clan the bridge tests delegate across.
pub fn clan_stub() -> &'static str {
    static PATH: OnceLock<String> = OnceLock::new();
    PATH.get_or_init(|| {
        located(
            "SAFIX_TEST_CLAN_STUB",
            env!("CARGO_BIN_EXE_safix-clan-stub"),
        )
    })
    .as_str()
}

/// The ssh-adjacent tools `safix upload` shells out to: `ssh-keygen`,
/// `ssh-to-age`, `ssh-keyscan` and `ssh`, one stub binary with four roles for
/// the reason `card_stub` is: the claims this suite drives against it are
/// about the delegation — that the identity conversion runs the named tool
/// and reaches no network, that a probe reads only what is offered, and that
/// a write streams the tarball and nothing besides — not about a real
/// network's behaviour, which no check in this repository reaches. See the
/// head of `tests/support/transport-stub.rs`.
pub fn transport_stub() -> &'static str {
    static PATH: OnceLock<String> = OnceLock::new();
    PATH.get_or_init(|| {
        located(
            "SAFIX_TEST_TRANSPORT_STUB",
            env!("CARGO_BIN_EXE_safix-transport-stub"),
        )
    })
    .as_str()
}

/// The real clan command, where a check put one in the environment.
///
/// Nothing falls back here, and that is the point: there is no compiled-in path
/// to a real clan, and a `None` is what `real_clan.rs` turns into a stated
/// absence rather than into a green test.
pub fn real_clan() -> Option<String> {
    named_program("SAFIX_TEST_REAL_CLAN")
}

/// The throwaway clan a check built for the real command to answer out of.
pub fn real_clan_seed() -> Option<String> {
    named_program("SAFIX_TEST_REAL_CLAN_SEED")
}

fn named_program(variable: &str) -> Option<String> {
    std::env::var(variable)
        .ok()
        .filter(|value| !value.is_empty())
}

/// Where one of the three programs the suite drives is.
///
/// `CARGO_BIN_EXE_*` is an absolute path fixed when the test was compiled, and
/// it points inside the build directory of whatever compiled it. A check that
/// builds the suite once and then runs one test of it per attribute therefore
/// has to say where the three programs went, and these variables are how it
/// says so. The compiled-in path remains the answer when nothing says
/// otherwise, so a developer's `cargo test` needs no environment at all.
fn located(variable: &str, built: &str) -> String {
    std::env::var(variable).unwrap_or_else(|_| built.to_owned())
}

/// What a run left on each stream, and what it exited with.
pub struct Run {
    /// The exit status, or `None` when the process was ended by a signal.
    pub code: Option<i32>,
    /// Standard output, unmodified: for `get` it is the value.
    pub stdout: Vec<u8>,
    /// Standard error, which is where every refusal and every progress line
    /// goes.
    pub stderr: String,
}

impl Run {
    /// Whether the run succeeded.
    pub fn succeeded(&self) -> bool {
        self.code == Some(0)
    }

    /// Standard output as text.
    pub fn output(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    /// Both streams, for the assertions that do not care which one carried a
    /// line.
    pub fn combined(&self) -> String {
        format!("{}{}", self.output(), self.stderr)
    }

    /// Assert the run succeeded, showing what it said when it did not.
    pub fn expect_success(self, what: &str) -> Self {
        assert!(
            self.succeeded(),
            "{what} exited {:?}\n{}",
            self.code,
            self.combined()
        );
        self
    }

    /// Assert the run was refused, showing what it said when it was not.
    pub fn expect_refusal(self, what: &str) -> Self {
        assert!(
            !self.succeeded(),
            "{what} was accepted\n{}",
            self.combined()
        );
        self
    }

    /// Assert a phrase appears in what the operator was told.
    pub fn says(&self, phrase: &str) -> &Self {
        assert!(
            self.combined().contains(phrase),
            "the run does not say {phrase:?}\n{}",
            self.combined()
        );
        self
    }

    /// Assert a phrase does not appear in what the operator was told.
    pub fn silent_about(&self, phrase: &str) -> &Self {
        assert!(
            !self.combined().contains(phrase),
            "the run says {phrase:?} and must not\n{}",
            self.combined()
        );
        self
    }

    /// The refusal code the graphical reporter names, as `safix::<code>`.
    ///
    /// Only a run made with [`Fixture::run_graphical`] carries one: the plain
    /// reporter is the shell runtime's shape and prints prose alone.
    pub fn refusal_code(&self) -> String {
        self.stderr
            .lines()
            .find_map(|line| line.trim().strip_prefix("safix::"))
            .unwrap_or_else(|| panic!("no refusal code in:\n{}", self.stderr))
            .to_owned()
    }
}

/// A repository the suite owns for the length of one test.
pub struct Fixture {
    /// The scratch directory everything lives under, on tmpfs.
    pub work: PathBuf,
    /// The repository the command operates on.
    pub repo: PathBuf,
    /// alice's recipient, minted here.
    pub alice: String,
    /// bob's recipient, minted here. His private half is destroyed as soon as the
    /// recipient is derived, which is also how the fixture shows that writing to
    /// a shared file needs no key but the writer's own.
    pub bob: String,
    /// A disk-backed directory this fixture alone writes into, where one is
    /// reachable — see [`Fixture::disk_staging_dir`].
    disk_staging: Option<PathBuf>,
    key_file: PathBuf,
    placements: Value,
    audiences: Value,
    /// `subject -> every key that subject can open a file with`, which is what
    /// names the custody behind a stanza the declared audience no longer covers.
    /// A machine belongs here beside a person: it holds a key, so a report that
    /// found its stanza can name it.
    subjects: Value,
    /// The subject records `flake.safix.lib.subjects` projects — every
    /// declared machine, service and group — which `upload` reads to tell a
    /// declared machine apart from an undeclared name or a person's. Distinct
    /// from [`Fixture::subjects`] above: that one is the recipients map, this
    /// one is the record set `safix upload` looks a machine name up in.
    subject_records: Value,
    /// The managers each organization declares, the consent each managed person
    /// declares, and each group's membership with the organizations covering it.
    /// `subjects` is not held here: [`Fixture::write_fixtures`] derives it the way
    /// the nix half does, so a fixture cannot declare a group whose members the
    /// name space does not carry.
    delegation: Value,
    genplan: Value,
    bridge: Value,
    keepassxc: Value,
    clan_flake: Option<PathBuf>,
    extras: Vec<String>,
}

impl Fixture {
    /// A repository with the fixture policy committed, two people declared in
    /// the placements, and nothing minted.
    pub fn new() -> Self {
        let work = scratch();
        let repo = work.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::create_dir_all(work.join("tmp")).unwrap();
        let staging = work.join("staging");
        std::fs::create_dir_all(&staging).unwrap();
        permit_owner_only(&staging);

        let key_file = work.join("age-key.txt");
        run_to_success(
            Command::new("age-keygen").arg("-o").arg(&key_file),
            "minting alice's identity",
        );
        let alice = capture(Command::new("age-keygen").arg("-y").arg(&key_file));

        // Two distinct recipients rather than one key under two anchors, so
        // "the created file took its recipients from the creation rule" is a
        // claim a file encrypted to the writer alone would fail.
        let bob_key = work.join("bob-key.txt");
        run_to_success(
            Command::new("age-keygen").arg("-o").arg(&bob_key),
            "minting bob's identity",
        );
        let bob = capture(Command::new("age-keygen").arg("-y").arg(&bob_key));
        std::fs::remove_file(&bob_key).unwrap();

        let fixture = Self {
            work,
            repo,
            alice: alice.clone(),
            bob: bob.clone(),
            disk_staging: disk_backed_scratch(),
            key_file,
            placements: json!({
                "alice": {
                    "api-token":      placement(ALICE_FILE, "api-token", "carries", "alice"),
                    "mail-password":  placement(ALICE_FILE, "mail-password", "private", "alice"),
                    "aliased-secret": placement(ALICE_FILE, "custom-key", "private", "alice"),
                    "wifi-psk":       placement(SHARED_FILE, "wifi-psk", "shared", "bob"),
                    "no-rule-secret": placement(
                        "secrets/safix/users/carol/secrets.yaml", "no-rule-secret", "private", "alice"),
                    "not-yaml":       placement(
                        "secrets/safix/users/alice/secret.age", "not-yaml", "private", "alice"),
                },
                "bob": {
                    "wifi-psk": placement(SHARED_FILE, "wifi-psk", "private", "bob"),
                },
            }),
            audiences: json!({
                ALICE_FILE: {
                    "audience": ["alice"],
                    "dir": "secrets/safix/users/alice",
                    "recipients": [alice],
                },
                SHARED_FILE: {
                    "audience": ["alice", "bob"],
                    "dir": "secrets/safix/shared/alice,bob",
                    "recipients": [alice, bob],
                },
            }),
            // A consumer who has never heard of clan evaluates exactly this,
            // and every test that does not declare a mapping drives it: the
            // bridge verbs have to be silent about an empty bridge rather than
            // refuse, and that is asserted by every other test's fixture being
            // this one.
            bridge: json!({ "clanFlake": null, "mappings": [] }),
            // The same statement about the mirror that `bridge` above makes about
            // clan: a consumer who has never heard of this evaluates exactly
            // this, and every test that declares no mapping drives it, so `sync`
            // has to be silent about an empty mirror rather than refuse.
            keepassxc: json!({ "database": null, "group": "safix", "mappings": [] }),
            subjects: json!({ "alice": [alice.clone()], "bob": [bob.clone()] }),
            subject_records: json!({ "machines": {}, "services": {}, "groups": {} }),
            // What a fleet that has never heard of delegation evaluates to, and
            // every test that declares none drives it: the verbs have to consult
            // nothing for a target no delegation covers, which is the whole of the
            // compatibility promise.
            delegation: json!({ "managers": {}, "managedBy": {}, "groups": {} }),
            clan_flake: None,
            genplan: json!({
                "alice": { "order": [], "outputs": {}, "inputs": {} },
                "bob":   { "order": [], "outputs": {}, "inputs": {} },
            }),
            extras: Vec::new(),
        };

        fixture.write_policy(&["alice", "bob"]);
        fixture.git(&["init", "-q"]);
        fixture.git(&["config", "user.email", "selftest@example.com"]);
        fixture.git(&["config", "user.name", "selftest"]);
        fixture.git(&["add", "-A"]);
        fixture.git(&["commit", "-q", "-m", "fixture: recipient policy"]);
        fixture.write_fixtures();
        fixture
    }

    // ── the fixture's own state ────────────────────────────────────────────

    /// The recipient policy, with the shared audience's rule granting the named
    /// anchors.
    ///
    /// One definition rather than one per caller: a rule granting fewer anchors
    /// than the declared audience is exactly the stale `.sops.yaml` a new file
    /// would be created through, and the narrowing test says only which anchors
    /// it grants.
    /// The anchor lines are rendered the way `tests/support/nix-stub.rs` renders
    /// the regenerated ones — one `  - &name key` per key a person holds — because
    /// the two documents are compared: a committed policy whose anchors are shaped
    /// differently from the regenerated one is drift `check` would report against a
    /// tree nothing moved. The recovery anchors are absent here and present there,
    /// and that asymmetry is the point rather than a mismatch: this file is written
    /// before any card exists, and the enrollment's edit is what adds one.
    pub fn write_policy(&self, shared_anchors: &[&str]) {
        let mut policy = String::from("keys:\n");
        write!(policy, "  - &alice {}\n  - &bob {}\n", self.alice, self.bob).unwrap();
        policy.push_str(&rules_block(shared_anchors));
        std::fs::write(self.repo.join(".sops.yaml"), policy).unwrap();
        std::fs::write(self.work.join("rules.txt"), rules_block(&["alice", "bob"])).unwrap();
    }

    /// Declare a placement for a name with no generator.
    pub fn seed_output(&mut self, name: &str, file: &str) {
        self.placements["alice"][name] = placement(file, name, "private", "alice");
        self.write_fixtures();
    }

    /// Declare a placement for a public output: a path in the plaintext store
    /// rather than a key inside an encrypted document.
    ///
    /// `file` and `key` stay populated because the resolver populates them for
    /// every entry; what makes this public is `public` naming a path, which is
    /// the one field the runtime branches on.
    pub fn seed_public_output(&mut self, name: &str, path: &str) {
        self.placements["alice"][name] = json!({
            "file": ALICE_FILE, "key": name, "origin": "private",
            "owner": "alice", "shared": false, "generator": null,
            "public": path,
        });
        self.write_fixtures();
    }

    /// The plaintext a public output holds, read straight off the repository.
    pub fn public_value(&self, path: &str) -> String {
        std::fs::read_to_string(self.repo.join(path))
            .unwrap_or_else(|cause| panic!("{path} is not readable: {cause}"))
    }

    /// Declare an entry both people carry and `shared = true` makes one value
    /// of: two placements, each with its carrier as owner, both naming one file
    /// and one key.
    pub fn seed_shared(&mut self, name: &str, file: &str) {
        for user in ["alice", "bob"] {
            self.placements[user][name] = json!({
                "file": file, "key": name, "origin": "carries",
                "owner": user, "shared": true, "generator": null, "public": null,
            });
        }
        self.write_fixtures();
    }

    /// Drop bob from a shared entry, leaving the remaining carrier's own file
    /// as its placement. The ciphertext is left exactly where it was, which is
    /// the state a revocation is discovered in.
    pub fn unshare_from(&mut self, name: &str, remaining: &str, file: &str) {
        if let Some(bob) = self.placements["bob"].as_object_mut() {
            bob.remove(name);
        }
        self.placements[remaining][name]["file"] = json!(file);
        self.write_fixtures();
    }

    /// Declare one file's audience, leaving the ciphertext exactly where it was.
    ///
    /// Narrowed, this is what every narrowing looks like to the runtime: a member
    /// removed from a group, a grant dropped, a machine's owner changed. The
    /// declarations record only the audience that is, so which of the three it
    /// was is not knowable from here — what is knowable is that a key on the file
    /// answers to a subject the audience no longer names.
    ///
    /// Widened, it is the other half of the same fact: the file does not move, so
    /// the convergence is a re-wrap of the file that is there.
    pub fn set_audience(&mut self, file: &str, audience: &[&str], keys: &[&str]) {
        self.audiences[file] = json!({
            "audience": audience,
            "dir": Path::new(file).parent().unwrap().to_str().unwrap(),
            "recipients": keys,
        });
        self.write_fixtures();
    }

    /// Declare a subject's own keys, which is what turns a stanza on a file into a
    /// name a report can print.
    ///
    /// One method for a person and for a machine, because the projection this
    /// stands for is one map over both: a group is absent from it by construction —
    /// its recipients are its members' — and so is a service, whose recipients are
    /// its machines'.
    pub fn declare_subject(&mut self, subject: &str, keys: &[&str]) {
        self.subjects[subject] = json!(keys);
        self.write_fixtures();
    }

    /// Declare a machine in `flake.safix.lib.subjects.machines`, the record set
    /// `safix upload` resolves a name against, and — when `recipient` is given —
    /// its recipient in the same map [`Fixture::declare_subject`] writes,
    /// exactly as `modules/flake/safix/default.nix`'s own `recipients`
    /// projection carries an empty list for a machine with none.
    pub fn declare_machine(&mut self, machine: &str, recipient: Option<&str>) {
        self.subject_records["machines"][machine] = json!({ "owner": null, "tags": [] });
        let keys: Vec<&str> = recipient.into_iter().collect();
        self.subjects[machine] = json!(keys);
        self.write_fixtures();
    }

    /// Declare one more person in the tree the evaluation reads, with a recipient.
    ///
    /// The recipient policy's keys block is derived from the tracked declarations
    /// rather than from the fixture, so a rule that grants a new anchor needs the
    /// person behind it declared here or the regenerated policy references an
    /// anchor it never defines.
    pub fn declare_person(&self, user: &str, recipient: &str) {
        self.write_declaration(user, recipient);
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", &format!("fixture: declare {user}")]);
    }

    /// One person's declaration file, uncommitted.
    ///
    /// The template is here rather than at each caller because the nix stub reads
    /// a recipient back out of it by matching `recipient = "…"`, so the shape is
    /// part of what the stub answers rather than incidental formatting.
    fn write_declaration(&self, user: &str, recipient: &str) {
        std::fs::create_dir_all(self.repo.join("safix/users")).unwrap();
        let declaration = format!(
            "{{\n  flake.safix.users.{user} = {{\n    recipient = \"{recipient}\";\n    carries = {{ }};\n    private = {{ }};\n  }};\n}}\n"
        );
        std::fs::write(
            self.repo.join(format!("safix/users/{user}.nix")),
            declaration,
        )
        .unwrap();
    }

    /// The recipient policy, committed and generated alike, over one audience.
    ///
    /// [`Fixture::write_policy`] deliberately leaves the generated half naming
    /// alice and bob whatever the committed half names, which is how a stale
    /// artifact is fixtured. A test about a re-wrap needs the two to agree, or
    /// what `fix` writes and what it re-wraps to disagree by construction.
    pub fn write_policy_agreeing(&self, shared_anchors: &[&str]) {
        self.write_policy(shared_anchors);
        std::fs::write(self.work.join("rules.txt"), rules_block(shared_anchors)).unwrap();
    }

    /// Declare a generator and derive its run-plan entry from the same record
    /// the command reads.
    ///
    /// `inputs` is computed the way `modules/flake/safix/resolve.nix` computes
    /// it — prompts and dependencies in one name space, hyphens mapped to
    /// underscores — so a change to that mapping on one side and not the other
    /// fails here.
    pub fn seed_generator(&mut self, name: &str, file: &str, outputs: &[&str], record: &Value) {
        self.placements["alice"][name] = json!({
            "file": file, "key": name, "origin": "private",
            "owner": "alice", "shared": false, "generator": record.clone(),
            "public": null,
        });

        // Keyed by the declared name, which is how `resolve.nix` emits the plan
        // now that the script addresses its inputs by path rather than through a
        // shell identifier.
        let mut inputs = serde_json::Map::new();
        if let Some(prompts) = self.placements["alice"][name]["generator"]["prompts"].as_object() {
            for prompt in prompts.keys() {
                inputs.insert(prompt.clone(), json!({ "kind": "prompt", "name": prompt }));
            }
        }
        if let Some(dependencies) =
            self.placements["alice"][name]["generator"]["dependencies"].as_array()
        {
            for dependency in dependencies.iter().filter_map(Value::as_str) {
                inputs.insert(
                    dependency.to_owned(),
                    json!({ "kind": "dependency", "name": dependency }),
                );
            }
        }

        let mut declared = vec![name.to_owned()];
        declared.extend(outputs.iter().map(|output| (*output).to_owned()));
        self.genplan["alice"]["order"]
            .as_array_mut()
            .unwrap()
            .push(json!(name));
        self.genplan["alice"]["outputs"][name] = json!(declared);
        self.genplan["alice"]["inputs"][name] = Value::Object(inputs);
        self.write_fixtures();
    }

    /// Replace a declared generator's record, leaving its run-plan entry alone.
    ///
    /// What an operator editing a declaration does: the generator still writes the
    /// same outputs and still sits where it sat in the order, and what changed is
    /// the record the runtime reads. [`Fixture::seed_generator`] appends to the
    /// order, so calling it twice for one name declares that generator twice —
    /// which is a fleet the resolver refuses rather than an edit.
    pub fn edit_generator(&mut self, name: &str, record: &Value) {
        self.placements["alice"][name]["generator"] = record.clone();
        self.write_fixtures();
    }

    /// Declare the entry an enrollment stores a card's PIN and PUK under.
    ///
    /// The stand-in for what an evaluation computes once `safix enroll` has added
    /// the name to the person's `private` set: the placements this stub answers
    /// with are documents the harness writes, and the run cannot make an
    /// evaluation happen. The claim that the name reaches the declaration is
    /// asserted against the declaration itself; this is what lets the write path
    /// resolve it afterwards.
    pub fn seed_card_custody(&mut self, serial: &str) {
        let name = format!("card-{serial}-piv-access");
        self.placements["alice"][&name] = placement(ALICE_FILE, &name, "private", "alice");
        self.write_fixtures();
    }

    /// Declare one organization's managers, and every person consenting to its
    /// management.
    ///
    /// Both halves at once because both are needed for a scaffold to be judged at
    /// all: an organization's managers reach nobody until somebody's own record
    /// names it.
    pub fn delegate(&mut self, organization: &str, managers: &[&str], managed: &[&str]) {
        self.delegation["managers"][organization] = json!(managers);
        for person in managed {
            self.delegation["managedBy"][*person] = json!(organization);
        }
        self.write_fixtures();
    }

    /// Declare one group: its membership, and the organizations whose silo
    /// declarations cover it.
    ///
    /// The coverage arrives as data because that is how the runtime reads it: the
    /// silo sets and the expansion that decides it live in `resolve.nix`, and
    /// `modules/flake/checks/subjects.nix` is where that computation is held to
    /// literals.
    pub fn declare_group(&mut self, group: &str, members: &[&str], organizations: &[&str]) {
        self.delegation["groups"][group] = json!({
            "members": members,
            "organizations": organizations,
        });
        self.write_fixtures();
    }

    /// Declare a person who holds nothing, as the placements carry one.
    ///
    /// Every declared person has a row there, empty when they hold no secret, and
    /// that row is what makes them a name the runtime can match a commit's
    /// identity against.
    pub fn seed_holder_of_nothing(&mut self, user: &str) {
        self.placements[user] = json!({});
        self.write_fixtures();
    }

    /// One group's declaration file, tracked, exactly as given.
    ///
    /// The text is the caller's because the shapes a declaration can be in are
    /// what the editor's claims are about: a list on one line, a name per line, or
    /// a value computed elsewhere that it must refuse rather than compound.
    pub fn write_group_declaration(&self, group: &str, declaration: &str) {
        std::fs::create_dir_all(self.repo.join("safix/groups")).unwrap();
        std::fs::write(
            self.repo.join(format!("safix/groups/{group}.nix")),
            declaration,
        )
        .unwrap();
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", &format!("fixture: declare {group}")]);
    }

    /// The identity every commit this fixture's runs make will carry.
    ///
    /// Written into the repository's own configuration rather than into the
    /// environment, because that is what the delegation check reads and what
    /// `git commit` will write: the two cannot be made to disagree here any more
    /// than they can at an operator's terminal.
    pub fn commit_as(&self, name: &str, email: &str) {
        self.git(&["config", "user.name", name]);
        self.git(&["config", "user.email", email]);
    }

    /// Name a file the consumer governs without declaring a secret in it.
    pub fn govern_extra(&mut self, path: &str) {
        self.extras = vec![path.to_owned()];
        self.write_fixtures();
    }

    /// Configure the onboarding hook, as the shell fragment a consumer declares.
    pub fn set_hook(&self, script: Option<&str>) {
        let hook = script.map_or(Value::Null, |text| json!(text));
        std::fs::write(self.work.join("hook.json"), hook.to_string()).unwrap();
    }

    /// Configure the enrollment hook, which is the onboarding one's counterpart.
    pub fn set_enroll_hook(&self, script: Option<&str>) {
        let hook = script.map_or(Value::Null, |text| json!(text));
        std::fs::write(self.work.join("enroll-hook.json"), hook.to_string()).unwrap();
    }

    /// Name the flake the mappings' clan side lives in.
    ///
    /// Every mapping declares one `clanFlake`, and a stubbed clan does not read
    /// it, so the fixture repository is the harmless default. A run against a
    /// real clan needs the real thing, and `real_clan.rs` is where that matters:
    /// clan resolves the flake, evaluates the machine, and answers out of the
    /// store it finds there. Call this before [`Fixture::seed_mapping`].
    pub fn clan_flake_is(&mut self, flake: &Path) {
        self.clan_flake = Some(flake.to_owned());
        self.bridge["clanFlake"] = json!(flake.to_string_lossy());
        self.write_fixtures();
    }

    /// Declare one shared-placement bridge mapping: the clan side names a
    /// generator and a file but no machine, the way a `placement = "shared"`
    /// declaration resolves.
    pub fn seed_shared_mapping(
        &mut self,
        id: &str,
        direction: &str,
        clan: (&str, &str),
        safix: (&str, &str),
    ) {
        let (generator, file) = clan;

        let (user, name) = safix;
        self.bridge["clanFlake"] = json!(
            self.clan_flake
                .clone()
                .unwrap_or_else(|| self.repo.clone())
                .to_string_lossy()
        );
        self.bridge["mappings"].as_array_mut().unwrap().push(json!({
            "id": id,
            "direction": direction,
            "clan": {
                "placement": "shared",
                "machine": null,
                "generator": generator,
                "file": file,
            },
            "safix": { "user": user, "name": name },
        }));
        self.write_fixtures();
    }

    /// Declare one two-way bridge mapping, and mint its companion placement
    /// beside the mapped entry's own \u{2014} the fixture's own hand-built
    /// counterpart to `bridge.nix`'s `companionsOf`: the same file, owner and
    /// shared-ness as the mapped entry already carries, a key suffixed
    /// `-safix-bridge-sync-state`, and no generator of its own. Without this
    /// the companion's own `set::run_committing` write has no placement to
    /// resolve against and refuses with `UnknownName`.
    pub fn seed_two_way_mapping(
        &mut self,
        id: &str,
        clan: (&str, &str, &str),
        safix: (&str, &str),
    ) {
        self.seed_mapping(id, "two-way", clan, safix);

        let (user, name) = safix;
        let mapped = self.placements[user][name].clone();
        let file = mapped["file"].as_str().unwrap_or(ALICE_FILE).to_owned();
        let owner = mapped["owner"].as_str().unwrap_or(user).to_owned();
        let shared = mapped["shared"].as_bool().unwrap_or(false);
        let companion = format!("{name}-safix-bridge-sync-state");
        self.placements[user][companion.as_str()] = json!({
            "file": file, "key": companion, "origin": "private",
            "owner": owner, "shared": shared, "generator": null, "public": null,
        });
        self.write_fixtures();
    }

    /// Declare one shared-placement two-way bridge mapping, and mint its
    /// companion placement the same way [`Self::seed_two_way_mapping`] does
    /// for a per-machine one.
    pub fn seed_two_way_mapping_shared(
        &mut self,
        id: &str,
        clan: (&str, &str),
        safix: (&str, &str),
    ) {
        self.seed_shared_mapping(id, "two-way", clan, safix);

        let (user, name) = safix;
        let mapped = self.placements[user][name].clone();
        let file = mapped["file"].as_str().unwrap_or(ALICE_FILE).to_owned();
        let owner = mapped["owner"].as_str().unwrap_or(user).to_owned();
        let shared = mapped["shared"].as_bool().unwrap_or(false);
        let companion = format!("{name}-safix-bridge-sync-state");
        self.placements[user][companion.as_str()] = json!({
            "file": file, "key": companion, "origin": "private",
            "owner": owner, "shared": shared, "generator": null, "public": null,
        });
        self.write_fixtures();
    }

    /// Remove a previously declared bridge mapping by id, as if it had never
    /// been declared this run.
    ///
    /// The fixture's own way of simulating a mapping removed from the
    /// declarations between two runs against the same clan: nothing here
    /// touches the stubbed clan's own store, only the declarations the next
    /// run reads.
    pub fn forget_mapping(&mut self, id: &str) {
        if let Some(mappings) = self.bridge["mappings"].as_array_mut() {
            mappings.retain(|mapping| mapping["id"] != json!(id));
        }
        self.write_fixtures();
    }
    /// Declare one keepassxc mapping, as `flake.safix.lib.keepassxc` resolves it.
    ///
    /// The shape is the one `modules/flake/safix/default.nix` projects: the
    /// database and the group once for the consumer, and one record per mapping
    /// carrying the attribute name it was declared under. Built rather than
    /// pasted, so a field added on the nix side has to be added here too and the
    /// fixture cannot drift into answering an older schema.
    pub fn seed_sync_mapping(
        &mut self,
        id: &str,
        mode: &str,
        safix: (&str, &str),
        path: &str,
        username: Option<&str>,
    ) {
        let (user, name) = safix;
        self.keepassxc["database"] = json!(self.kdbx().to_string_lossy());
        self.keepassxc["mappings"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id": id,
                "mode": mode,
                "safix": { "user": user, "name": name },
                "kdbx": {
                    "path": path,
                    "username": username.map_or(Value::Null, |name| json!(name)),
                },
            }));
        self.write_fixtures();
    }

    /// Declare the group every mapping's entry path is relative to.
    pub fn sync_group_is(&mut self, group: &str) {
        self.keepassxc["group"] = json!(group);
        self.write_fixtures();
    }

    /// Declare mappings and no database, which is the state `sync` refuses in.
    pub fn forget_the_database(&mut self) {
        self.keepassxc["database"] = Value::Null;
        self.write_fixtures();
    }

    /// The database path the fixture declares.
    ///
    /// Inside the fixture's own scratch directory, which is on tmpfs and removed
    /// on every exit path. Nothing in this suite ever names a database of the
    /// operator's, and `refuse_a_real_database` is the structural guard that makes
    /// that a property rather than a habit.
    pub fn kdbx(&self) -> PathBuf {
        self.work.join("fixture.kdbx")
    }

    /// What the modelled database holds for one entry, if it holds anything.
    pub fn store_holds(&self, entry: &str) -> Option<String> {
        from_hex(
            &std::fs::read_to_string(
                self.card_spool()
                    .join(format!("kdbx-{}", entry.replace('/', "%"))),
            )
            .ok()?,
        )
    }

    /// Put a value into the modelled database without going through safix.
    ///
    /// The person's own edit, which is what every divergence fixture is: it
    /// creates the groups the path needs, then the entry, exactly as the model's
    /// own `add` would.
    pub fn store_seed(&self, entry: &str, value: &str) {
        let spool = self.card_spool();
        std::fs::create_dir_all(&spool).unwrap();

        let mut groups: Vec<String> = read_lines(&spool.join("kdbx-groups"));
        let mut path = String::new();
        let mut segments: Vec<&str> = entry.split('/').collect();
        segments.pop();
        for segment in segments {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(segment);
            if !groups.contains(&path) {
                groups.push(path.clone());
            }
        }
        write_lines(&spool.join("kdbx-groups"), &groups);

        let mut entries: Vec<String> = read_lines(&spool.join("kdbx-entries"));
        let line = format!("{entry} ");
        if !entries.iter().any(|held| held.starts_with(&line)) {
            entries.push(line);
        }
        write_lines(&spool.join("kdbx-entries"), &entries);

        std::fs::write(
            spool.join(format!("kdbx-{}", entry.replace('/', "%"))),
            to_hex(value),
        )
        .unwrap();
    }

    /// Every entry the modelled database holds, in path order.
    pub fn store_entries(&self) -> Vec<String> {
        read_lines(&self.card_spool().join("kdbx-entries"))
            .into_iter()
            .filter_map(|line| line.split_once(' ').map(|(name, _)| name.to_owned()))
            .collect()
    }

    /// The username the modelled database holds for one entry.
    pub fn store_username(&self, entry: &str) -> String {
        read_lines(&self.card_spool().join("kdbx-entries"))
            .into_iter()
            .find_map(|line| {
                line.strip_prefix(&format!("{entry} "))
                    .map(str::trim)
                    .map(str::to_owned)
            })
            .unwrap_or_default()
    }

    /// Every argument vector the store's own command was invoked with, in order.
    ///
    /// What the burst discipline is asserted against: the words of each
    /// invocation, so a read between two writes is visible as a line.
    pub fn store_invocations(&self) -> Vec<String> {
        self.card_recorded("kdbx-argv")
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// The environment a sync run needs: the store's own command pointed at the
    /// stub, and the spool it records into.
    ///
    /// Two variables rather than one for the reason [`Fixture::card_env`] has
    /// four: the runtime reaches the tool by its own override, and a single one
    /// would let a rename go unnoticed.
    pub fn store_env(&self) -> Vec<(String, String)> {
        vec![
            ("SAFIX_KEEPASSXC_CLI".to_owned(), card_stub().to_owned()),
            (
                "SAFIX_CARD_STUB_SPOOL".to_owned(),
                self.card_spool().to_string_lossy().into_owned(),
            ),
        ]
    }

    /// Where the stubbed clan keeps its store, its spool and its switches.
    pub fn clan_spool(&self) -> PathBuf {
        self.work.join("clan-spool")
    }

    /// What the stubbed clan holds for one var, if it holds anything.
    ///
    /// Read out of the stub's own layout rather than clan's, which is the point
    /// of the stub having one: a runtime that reached past the command would
    /// find nothing here, because there is nothing shaped like clan's store to
    /// find.
    pub fn clan_holds(&self, machine: &str, id: &str) -> Option<String> {
        let text = std::fs::read_to_string(
            self.clan_spool()
                .join("store")
                .join(machine)
                .join(id.replace('/', "%")),
        )
        .ok()?;
        from_hex(&text)
    }

    /// Put a value into the stubbed clan without going through safix.
    pub fn clan_seed(&self, machine: &str, id: &str, value: &str) {
        let directory = self.clan_spool().join("store").join(machine);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join(id.replace('/', "%")), to_hex(value)).unwrap();
    }

    /// One line of what the stubbed clan recorded, or the empty string.
    pub fn clan_recorded(&self, name: &str) -> String {
        std::fs::read_to_string(self.clan_spool().join(name)).unwrap_or_default()
    }

    /// How many times clan was asked to write.
    ///
    /// The number convergence is a claim about. clan's write is unconditional
    /// and commits what it wrote, so "the second run changed nothing" is only
    /// true if this does not move.
    pub fn clan_writes(&self) -> u64 {
        self.clan_recorded("writes").trim().parse().unwrap_or(0)
    }

    /// Where the card stubs record what they were handed.
    pub fn card_spool(&self) -> PathBuf {
        self.work.join("card-spool")
    }

    /// One line of what a card stub recorded, or the empty string.
    pub fn card_recorded(&self, name: &str) -> String {
        std::fs::read_to_string(self.card_spool().join(name)).unwrap_or_default()
    }

    /// What a store stub holds under one name, if it holds anything.
    pub fn card_holds(&self, name: &str) -> Option<String> {
        from_hex(&std::fs::read_to_string(self.card_spool().join(name)).ok()?)
    }

    /// The environment an enrollment run needs: every tool pointed at the card
    /// stub, and the spool they all record into.
    ///
    /// Four variables rather than one, because the runtime reaches each tool by
    /// its own override and a single one would let a rename go unnoticed.
    pub fn card_env(&self) -> Vec<(String, String)> {
        let stub = card_stub().to_owned();
        vec![
            ("SAFIX_YKMAN".to_owned(), stub.clone()),
            ("SAFIX_AGE_PLUGIN_YUBIKEY".to_owned(), stub.clone()),
            ("SAFIX_SECRET_TOOL".to_owned(), stub.clone()),
            ("SAFIX_KEEPASSXC_CLI".to_owned(), stub),
            (
                "SAFIX_CARD_STUB_SPOOL".to_owned(),
                self.card_spool().to_string_lossy().into_owned(),
            ),
            // Named rather than left to $HOME/.config, so a test can read what
            // enrollment appended without depending on where sops looks. It is
            // the same variable `safix keygen` honours, and enrollment appends to
            // the same file for the same reason: the card's stub and the software
            // identities are peers.
            (
                "SAFIX_AGE_KEY_FILE".to_owned(),
                self.card_identity_file().to_string_lossy().into_owned(),
            ),
        ]
    }

    /// The identity file an enrollment run appends the card's stub to.
    pub fn card_identity_file(&self) -> PathBuf {
        self.work.join("card-keys.txt")
    }

    /// What that file holds, or the empty string when nothing wrote it.
    pub fn card_identity(&self) -> String {
        std::fs::read_to_string(self.card_identity_file()).unwrap_or_default()
    }

    /// The environment a bridge run needs: the stubbed clan, and its spool.
    pub fn clan_env(&self) -> Vec<(String, String)> {
        vec![
            ("SAFIX_CLAN".to_owned(), clan_stub().to_owned()),
            (
                "SAFIX_CLAN_STUB_SPOOL".to_owned(),
                self.clan_spool().to_string_lossy().into_owned(),
            ),
        ]
    }

    /// Where the transport stub records every role it played.
    pub fn transport_spool(&self) -> PathBuf {
        self.work.join("transport-spool")
    }

    /// The environment an upload run needs: `ssh-keygen`, `ssh-to-age`,
    /// `ssh-keyscan` and `ssh` each pointed at the transport stub, and the
    /// spool it all records into.
    ///
    /// Four variables rather than one, for the reason [`Fixture::card_env`]
    /// has four: the runtime reaches each tool by its own override, and a
    /// single one would let a rename of any of them go unnoticed.
    pub fn transport_env(&self) -> Vec<(String, String)> {
        let stub = transport_stub().to_owned();
        vec![
            ("SAFIX_SSH_KEYGEN".to_owned(), stub.clone()),
            ("SAFIX_SSH_TO_AGE".to_owned(), stub.clone()),
            ("SAFIX_SSH_KEYSCAN".to_owned(), stub.clone()),
            ("SAFIX_SSH".to_owned(), stub),
            (
                "SAFIX_TRANSPORT_STUB_SPOOL".to_owned(),
                self.transport_spool().to_string_lossy().into_owned(),
            ),
        ]
    }

    /// One file the transport stub recorded, or the empty string.
    pub fn transport_recorded(&self, name: &str) -> String {
        std::fs::read_to_string(self.transport_spool().join(name)).unwrap_or_default()
    }

    /// Every role the transport stub played, in the order it played them —
    /// `ssh-keygen`, `ssh-to-age`, `ssh-keyscan` or `ssh` — one per line.
    ///
    /// What task 2.5's and 3.6's claims are asserted against: a
    /// `--directory` run's list holds no `ssh-keyscan` and no `ssh`, and a
    /// matching remote probe's holds no `ssh` at all.
    pub fn transport_invocations(&self) -> Vec<String> {
        self.transport_recorded("roles")
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// Write every fixture document the stubbed evaluator answers with.
    ///
    /// `governedFiles` is computed here from the audiences and the extras, as
    /// `modules/flake/safix/default.nix` computes it, so a test that names an
    /// extra changes all three of `required`, `extra` and `managed` the way an
    /// evaluation would.
    fn write_fixtures(&self) {
        let required: Vec<&String> = self
            .audiences
            .as_object()
            .unwrap()
            .keys()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut managed: Vec<String> = required.iter().map(|file| (*file).clone()).collect();
        for extra in &self.extras {
            if !managed.contains(extra) {
                managed.push(extra.clone());
            }
        }
        managed.sort();

        let governed = json!({
            "required": required,
            "extra": self.extras,
            "managed": managed,
        });

        write_json(&self.work.join("placements.json"), &self.placements);
        write_json(&self.work.join("audiences.json"), &self.audiences);
        write_json(&self.work.join("genplan.json"), &self.genplan);
        write_json(&self.work.join("bridge.json"), &self.bridge);
        write_json(&self.work.join("keepassxc.json"), &self.keepassxc);
        write_json(&self.work.join("recipients.json"), &self.subjects);
        write_json(&self.work.join("subjects-lib.json"), &self.subject_records);
        write_json(&self.work.join("governed.json"), &governed);

        // The subject name space, derived here the way `resolve.nix` derives it:
        // every declared person, every declared group, and every organization the
        // delegation names. A fixture that maintained it by hand could declare a
        // group whose members the name space does not carry, which is a fleet
        // evaluation refuses and so a state the runtime never meets.
        let mut subjects: Vec<String> = self
            .placements
            .as_object()
            .map(|held| held.keys().cloned().collect())
            .unwrap_or_default();
        for named in [&self.delegation["groups"], &self.delegation["managers"]] {
            if let Some(declared) = named.as_object() {
                subjects.extend(declared.keys().cloned());
            }
        }
        subjects.sort();
        subjects.dedup();

        let mut delegation = self.delegation.clone();
        delegation["subjects"] = json!(subjects);
        write_json(&self.work.join("delegation.json"), &delegation);
        if !self.work.join("hook.json").exists() {
            self.set_hook(None);
        }
        if !self.work.join("enroll-hook.json").exists() {
            self.set_enroll_hook(None);
        }
    }

    // ── driving the command ────────────────────────────────────────────────

    /// One invocation, with nothing on standard input.
    pub fn run(&self, arguments: &[&str]) -> Run {
        self.invoke(safix(), arguments, None, Reporter::Plain, &[])
    }

    /// One invocation, with the given bytes on standard input.
    pub fn run_with(&self, arguments: &[&str], stdin: &str) -> Run {
        self.invoke(safix(), arguments, Some(stdin), Reporter::Plain, &[])
    }

    /// One invocation under the graphical reporter, which is where a refusal's
    /// code is rendered.
    pub fn run_graphical(&self, arguments: &[&str]) -> Run {
        self.invoke(safix(), arguments, None, Reporter::Graphical, &[])
    }

    /// One invocation under the graphical reporter, with standard input.
    pub fn run_graphical_with(&self, arguments: &[&str], stdin: &str) -> Run {
        self.invoke(safix(), arguments, Some(stdin), Reporter::Graphical, &[])
    }

    /// One invocation with something in its environment the fixture does not
    /// set — a backend that fails, a temporary directory of its own.
    pub fn run_env(&self, arguments: &[&str], stdin: Option<&str>, extra: &[(&str, &str)]) -> Run {
        self.invoke(safix(), arguments, stdin, Reporter::Plain, extra)
    }

    /// One invocation whose standard input and standard error are a terminal.
    ///
    /// `safix enroll` refuses without one, and that refusal is a real property
    /// rather than a thing to be switched off for the suite: a card has to be
    /// touched and somebody has to be told when. So the runs that are meant to
    /// get past it are given a pseudo-terminal, and the one that is meant to be
    /// refused goes through [`Fixture::run_env`], which gives it pipes.
    ///
    /// Standard output stays a pipe, which is what lets a test say that a value
    /// did not reach it: on a terminal both streams are one and the claim could
    /// not be made. `feed` is written to the terminal after the child is running,
    /// which is how a prompt the run makes of the operator is answered.
    pub fn run_on_terminal(&self, arguments: &[&str], feed: &str, extra: &[(&str, &str)]) -> Run {
        self.on_terminal(arguments, feed, extra, false)
    }

    /// The same, in a session of its own.
    ///
    /// `safix sync` refuses without a terminal and reads the database's password
    /// from `/dev/tty` when that opens, so a run on a developer's machine would
    /// ask *their* terminal for it and wait. Detaching leaves the run with no
    /// controlling terminal, so `/dev/tty` does not open and the prompt falls back
    /// to standard input — which is the pseudo-terminal this allocates, and is
    /// what the feed answers. In a build sandbox there is no controlling terminal
    /// to begin with and this is the same run.
    pub fn run_sync(&self, arguments: &[&str], feed: &str, extra: &[(&str, &str)]) -> Run {
        self.on_terminal(arguments, feed, extra, true)
    }

    fn on_terminal(
        &self,
        arguments: &[&str],
        feed: &str,
        extra: &[(&str, &str)],
        detach: bool,
    ) -> Run {
        use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};

        refuse_a_real_card(arguments, extra);
        refuse_a_real_database(self, arguments, extra);
        let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).expect("no pseudo-terminal");
        grantpt(&master).expect("could not grant the pseudo-terminal");
        unlockpt(&master).expect("could not unlock the pseudo-terminal");
        let name = ptsname(&master, Vec::new()).expect("the pair has no name");
        let slave = std::fs::File::options()
            .read(true)
            .write(true)
            .open(String::from_utf8_lossy(name.as_bytes()).into_owned())
            .expect("the slave end does not open");

        let mut command = match (detach, detached()) {
            (true, Some(setsid)) => {
                let mut command = Command::new(setsid);
                command.arg("-w").arg(safix());
                command
            }
            _ => Command::new(safix()),
        };
        command.args(acknowledged(arguments));
        self.environment(&mut command, Reporter::Plain);
        for (variable, value) in extra {
            command.env(variable, value);
        }
        command
            .stdin(Stdio::from(
                slave.try_clone().expect("the slave cannot be duplicated"),
            ))
            .stdout(Stdio::piped())
            .stderr(Stdio::from(slave));
        let mut child = command.spawn().expect("could not spawn the command");

        let mut terminal =
            std::fs::File::from(master.try_clone().expect("the master cannot be duplicated"));
        if !feed.is_empty() {
            terminal
                .write_all(feed.as_bytes())
                .expect("the feed cannot be written");
            terminal.flush().expect("the feed cannot be flushed");
        }

        // Drained on a thread of its own, because a run that fills the terminal's
        // buffer while this process waits for it to exit is a deadlock rather than
        // a failure. The read ends when the child does — `command` is dropped
        // first so the only descriptions of the slave left are the child's.
        drop(command);
        let reader = std::thread::spawn(move || {
            let mut seen = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                match rustix::io::read(&master, &mut buffer) {
                    Ok(0) | Err(rustix::io::Errno::IO) => break,
                    Ok(read) => seen.extend_from_slice(&buffer[..read]),
                    Err(rustix::io::Errno::INTR) => (),
                    Err(_) => break,
                }
            }
            seen
        });

        let mut stdout = Vec::new();
        if let Some(mut pipe) = child.stdout.take() {
            let _ = pipe.read_to_end(&mut stdout);
        }
        let status = child.wait().expect("the command did not finish");
        drop(terminal);
        let seen = reader.join().unwrap_or_default();

        Run {
            code: status.code(),
            stdout,
            stderr: String::from_utf8_lossy(&seen).into_owned(),
        }
    }

    /// One invocation under the graphical reporter with an environment of its
    /// own, which is how a staging drill reads the code of a refusal only a
    /// disk-backed mount produces.
    pub fn run_graphical_env(&self, arguments: &[&str], extra: &[(&str, &str)]) -> Run {
        self.invoke(safix(), arguments, None, Reporter::Graphical, extra)
    }

    /// A disk-backed directory this fixture's runs alone write into, where the
    /// machine has one.
    ///
    /// The disk-backed counterpart of [`Fixture::staging_dir`], and it exists
    /// for the same reason. The drills that refuse disk staging used to point at
    /// whichever disk-backed directory the candidate search returned first —
    /// `/tmp`, the checkout — and then assert that no staging root was left
    /// under it. Two of those drills running at once each saw the other's root
    /// in flight and read it as its own residue, which is a suite that fails on
    /// a schedule rather than on a defect.
    ///
    /// Scoped here, the residue claim is the strong one again: after this run,
    /// this directory holds no staging root, and nothing else was ever going to
    /// put one there.
    ///
    /// `None` where every candidate is memory-backed — a build sandbox whose
    /// whole tree is a tmpfs is one — which is the same condition under which
    /// [`disk_backed_directory`] finds nothing, so a drill handed `None` here is
    /// a drill with nothing to point at either way.
    pub fn disk_staging_dir(&self) -> Option<&Path> {
        self.disk_staging.as_deref()
    }

    /// The directory every run of this fixture stages plaintext in.
    ///
    /// A tmpfs directory belonging to this fixture alone, named to every run
    /// through `SAFIX_STAGING_DIR` — which *replaces* the conventional
    /// candidates rather than preceding them, so a run of this fixture cannot
    /// stage anywhere else.
    ///
    /// This is what makes the residue assertions mean something under
    /// concurrency. They used to snapshot `/dev/shm` and `/run/user`, which are
    /// shared with every other test in this binary and with everything else the
    /// machine is running: a root another test held in flight was
    /// indistinguishable from residue this run left, so the assertions were
    /// written as a before-and-after comparison — and a comparison against a
    /// baseline that is itself moving is a comparison that passes for the wrong
    /// reason as often as it fails for one. Scoped here, the claim is the strong
    /// one: after the run, this directory is empty.
    pub fn staging_dir(&self) -> PathBuf {
        self.work.join("staging")
    }

    /// Every staging root this fixture's runs left behind.
    ///
    /// Named by the prefix `staging.rs` gives them, and looked for only where
    /// this fixture's runs can put them — see [`Fixture::staging_dir`].
    pub fn staging_roots(&self) -> Vec<PathBuf> {
        let mut found = roots_under(&self.staging_dir());
        found.sort();
        found
    }

    /// Every staging root directly under a directory a drill pointed a run at.
    ///
    /// For the drills that override `SAFIX_STAGING_DIR` themselves, whose
    /// residue is therefore not under [`Fixture::staging_dir`].
    pub fn roots_in(directory: &Path) -> Vec<PathBuf> {
        let mut found = roots_under(directory);
        found.sort();
        found
    }

    /// One invocation signalled after a delay, with the signal reaching the
    /// command's own process and nothing else.
    ///
    /// Distinct from [`Fixture::interrupt_after`], which runs under `timeout`
    /// and so signals the whole process group — every descendant included. That
    /// is the right model for a keyboard interrupt and the wrong one for
    /// asking what the *runtime* does when a signal arrives while a child of its
    /// own is still running: with the group signalled, the child dies at the
    /// same moment and there is no such window to observe.
    ///
    /// Here the child keeps running, so the window is the child's whole
    /// remaining lifetime and a fixture can put an assertion inside it.
    pub fn interrupt_command_after(
        &self,
        delay: std::time::Duration,
        signal: rustix::process::Signal,
        arguments: &[&str],
        extra: &[(&str, &str)],
    ) -> Run {
        let mut command = Command::new(safix());
        command.args(acknowledged(arguments));
        self.environment(&mut command, Reporter::Plain);
        for (name, value) in extra {
            command.env(name, value);
        }
        command.stdin(Stdio::null());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        let child = command.spawn().expect("could not spawn the command");

        let pid = rustix::process::Pid::from_raw(
            i32::try_from(child.id()).expect("a pid fits in an i32"),
        )
        .expect("the spawned command has a pid");
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            let _ = rustix::process::kill_process(pid, signal);
        });

        finish(child, None)
    }

    /// One invocation of a program standing in for the binary, which is how a
    /// drill puts a deliberately damaged runtime in its place.
    pub fn run_program(
        &self,
        program: &str,
        arguments: &[&str],
        stdin: Option<&str>,
        extra: &[(&str, &str)],
    ) -> Run {
        self.invoke(program, arguments, stdin, Reporter::Plain, extra)
    }

    /// The temporary directory a run stages in, which is where a value must
    /// never be found afterwards.
    pub fn tmpdir(&self) -> PathBuf {
        self.work.join("tmp")
    }

    /// A path under the fixture's scratch directory, for a spool a shim writes.
    pub fn scratch(&self, name: &str) -> PathBuf {
        self.work.join(name)
    }

    /// `set`, with the value piped.
    ///
    /// Standard input is a pipe here and never a terminal, so this is the stream
    /// source: the bytes given are the bytes stored, and nothing prompts. It used
    /// to write the value twice, because a pipe took the prompt path and read two
    /// lines from it; a caller wanting that path now says so with
    /// [`Fixture::set_on_a_terminal`].
    pub fn set(&self, user: &str, name: &str, value: &str) -> Run {
        self.run_with(&["set", user, name], value)
    }

    /// `set` with a real terminal on standard input, and the two lines typed into
    /// it.
    ///
    /// A pseudoterminal rather than a pipe, because the fork under test is the
    /// terminal test on standard input: a pipe now takes the stream source, and
    /// there is no other way left to reach the prompt path. The pair the command
    /// reads is written as two lines, which is what a person typing does.
    ///
    /// `setsid` sits above the command where this process has a controlling
    /// terminal, for the reason [`detached`] gives and one more of its own: the
    /// prompt prefers `/dev/tty` over standard input, so a run that kept this
    /// process's terminal would read the two lines from the developer's keyboard
    /// rather than from the pseudoterminal. Detached, the command has no
    /// controlling terminal, `/dev/tty` does not open, and the reads land on the
    /// pseudoterminal that standard input is.
    pub fn set_on_a_terminal(&self, arguments: &[&str], typed: &str) -> Run {
        let terminal = Pty::open();
        let mut command = match detached() {
            Some(setsid) => {
                let mut command = Command::new(setsid);
                command.arg("-w").arg(safix());
                command
            }
            None => Command::new(safix()),
        };
        command.args(acknowledged(arguments));
        self.environment(&mut command, Reporter::Plain);
        command.stdin(Stdio::from(terminal.slave));
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        let child = command.spawn().expect("could not spawn the command");

        // The typed lines, then the end-of-input character. A pipe signals the end
        // of its input by being closed; a pseudoterminal has no writer to close —
        // closing the master leaves the slave's reads failing with EIO rather than
        // reporting the end — so the end is signalled the way a person signals it,
        // with the terminal's own EOF at the start of a line. Without it a run that
        // is given fewer lines than it reads waits forever instead of being
        // refused.
        let mut master = std::fs::File::from(terminal.master);
        master.write_all(typed.as_bytes()).unwrap();
        master.write_all(&[END_OF_INPUT]).unwrap();
        master.flush().unwrap();

        let output = child
            .wait_with_output()
            .expect("the command did not finish");
        drop(master);
        Run {
            code: output.status.code(),
            stdout: output.stdout,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// The command's own environment, ready for a caller that needs to spawn it
    /// itself.
    pub fn command(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new(safix());
        command.args(acknowledged(arguments));
        self.environment(&mut command, Reporter::Plain);
        command
    }

    /// A run interrupted in one of the windows it has.
    ///
    /// Standard input is a pipe this process keeps the write end of, so a read
    /// that is not answered blocks rather than seeing end of input — the
    /// difference between a run that was interrupted and one that ran out of
    /// input, which exit differently and must not be confused. `timeout` sends
    /// the signal because it sends it to the process it spawned;
    /// `--preserve-status` is what lets the runtime's own 130 or 143 be
    /// observed rather than timeout's 124.
    ///
    /// The whole chain is detached where this process has a terminal, because a
    /// run that found one would read its value from `/dev/tty` and wait at the
    /// first prompt whatever the fixture fed it — so the window named by the
    /// test would not be the window the signal arrived in. `timeout` stays the
    /// signal's sender either way: it signals the process it spawned, which is
    /// the runtime, and `setsid` sits above it.
    pub fn interrupt_after(
        &self,
        seconds: &str,
        signal: &str,
        arguments: &[&str],
        feed: &str,
        extra: &[(&str, &str)],
    ) -> Run {
        let (reader, writer) = rustix::pipe::pipe().expect("could not open a pipe");
        let mut command = match detached() {
            Some(setsid) => {
                let mut command = Command::new(setsid);
                command.arg("-w").arg("timeout");
                command
            }
            None => Command::new("timeout"),
        };
        command
            .arg("--preserve-status")
            .arg("-s")
            .arg(signal)
            .arg(seconds)
            .arg(safix());
        command.args(acknowledged(arguments));
        self.environment(&mut command, Reporter::Plain);
        for (name, value) in extra {
            command.env(name, value);
        }
        command.stdin(Stdio::from(reader));
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        let child = command.spawn().expect("could not spawn the command");

        let mut held = std::fs::File::from(writer);
        if !feed.is_empty() {
            held.write_all(feed.as_bytes()).unwrap();
            held.flush().unwrap();
        }
        let output = child
            .wait_with_output()
            .expect("the command did not finish");
        drop(held);
        Run {
            code: output.status.code(),
            stdout: output.stdout,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    fn invoke(
        &self,
        program: &str,
        arguments: &[&str],
        stdin: Option<&str>,
        reporter: Reporter,
        extra: &[(&str, &str)],
    ) -> Run {
        refuse_a_real_card(arguments, extra);
        refuse_a_real_database(self, arguments, extra);
        let mut command = match (stdin, detached()) {
            (Some(_), Some(setsid)) => {
                let mut command = Command::new(setsid);
                command.arg("-w").arg(program);
                command
            }
            _ => Command::new(program),
        };
        // `run_program` sends this at `strace` and at the shim as well as at
        // safix, and neither of those knows the acknowledgement.
        if program == safix() {
            command.args(acknowledged(arguments));
        } else {
            command.args(arguments);
        }
        self.environment(&mut command, reporter);
        for (name, value) in extra {
            command.env(name, value);
        }
        command.stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        let child = command.spawn().expect("could not spawn the command");
        finish(child, stdin)
    }

    fn environment(&self, command: &mut Command, reporter: Reporter) {
        command
            .current_dir(&self.repo)
            .env("HOME", &self.work)
            .env("TMPDIR", self.work.join("tmp"))
            .env("USER", "alice")
            .env("SOPS_AGE_KEY_FILE", &self.key_file)
            .env("SAFIX_REPO_ROOT", &self.repo)
            .env("SAFIX_STAGING_DIR", self.staging_dir())
            .env("SAFIX_NIX", nix_stub())
            .env(
                "SAFIX_FIXTURE_PLACEMENTS",
                self.work.join("placements.json"),
            )
            .env("SAFIX_FIXTURE_AUDIENCES", self.work.join("audiences.json"))
            .env("SAFIX_FIXTURE_GOVERNED", self.work.join("governed.json"))
            .env(
                "SAFIX_FIXTURE_RECIPIENTS",
                self.work.join("recipients.json"),
            )
            .env(
                "SAFIX_FIXTURE_DELEGATION",
                self.work.join("delegation.json"),
            )
            .env("SAFIX_FIXTURE_GENPLAN", self.work.join("genplan.json"))
            .env("SAFIX_FIXTURE_BRIDGE", self.work.join("bridge.json"))
            .env("SAFIX_FIXTURE_KEEPASSXC", self.work.join("keepassxc.json"))
            .env("SAFIX_FIXTURE_HOOK", self.work.join("hook.json"))
            .env(
                "SAFIX_FIXTURE_ENROLL_HOOK",
                self.work.join("enroll-hook.json"),
            )
            .env(
                "SAFIX_FIXTURE_SUBJECTS",
                self.work.join("subjects-lib.json"),
            )
            .env("SAFIX_FIXTURE_RULES", self.work.join("rules.txt"))
            // The two the developer's own shell almost certainly sets. A test
            // that inherited them would open whoever is running it into their
            // real editor, on a fixture value, and wait; and one that passed
            // would have proved something about that machine. Every editor test
            // names the one it wants through `run_env`, which is applied after
            // this.
            .env_remove("VISUAL")
            .env_remove("EDITOR")
            // The identity a commit carries is resolved by git, which reads these
            // before it reads the repository's configuration. A run that inherited
            // them would be judged against whoever is running the suite, so a
            // delegation test would pass or fail by a property of that machine;
            // removed, `git config user.name` in the fixture repository is the
            // whole of what decides it.
            .env_remove("GIT_AUTHOR_NAME")
            .env_remove("GIT_AUTHOR_EMAIL")
            .env_remove("GIT_COMMITTER_NAME")
            .env_remove("GIT_COMMITTER_EMAIL")
            .env_remove("EMAIL");
        match reporter {
            Reporter::Plain => command.env("SAFIX_ERROR_FORMAT", "plain"),
            Reporter::Graphical => command.env_remove("SAFIX_ERROR_FORMAT"),
        };
    }

    // ── what a run did to the repository ───────────────────────────────────

    /// One git question, answered in the fixture repository.
    pub fn git(&self, arguments: &[&str]) -> String {
        let mut command = Command::new("git");
        command.arg("-C").arg(&self.repo).args(arguments);
        command.env("HOME", &self.work);
        let output = command.output().expect("could not run git");
        assert!(
            output.status.success(),
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout)
            .trim_end_matches('\n')
            .to_owned()
    }

    /// The commit the repository is on.
    pub fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"])
    }

    /// The subject of one commit.
    pub fn subject(&self, revision: &str) -> String {
        self.git(&["log", "-1", "--format=%s", revision])
    }

    /// The whole message of one commit, subject and body.
    pub fn message(&self, revision: &str) -> String {
        self.git(&["log", "-1", "--format=%s%n%b", revision])
    }

    /// The paths in one commit, sorted. This is the projection the retired
    /// comparative harness canonicalized, expressed as something to assert
    /// against.
    pub fn paths_in(&self, revision: &str) -> Vec<String> {
        let mut paths: Vec<String> = self
            .git(&["show", "--name-only", "--format=", revision])
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect();
        paths.sort();
        paths
    }

    /// The commit naming this phrase in its subject, if there is one.
    pub fn commit_matching(&self, phrase: &str) -> String {
        self.git(&["log", "--format=%H", &format!("--grep={phrase}"), "-1"])
    }

    /// Whether the working tree and index are clean.
    pub fn status(&self) -> String {
        self.git(&["status", "--porcelain"])
    }

    /// The paths staged but not committed.
    pub fn staged(&self) -> Vec<String> {
        self.git(&["diff", "--cached", "--name-only"])
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// Every key's ciphertext line in a sops file, by key.
    ///
    /// Comparing two of these across a write is how a bystander key is shown to
    /// be byte-identical without its value ever being rendered.
    pub fn ciphertext_lines(&self, relative: &str) -> BTreeMap<String, String> {
        let text = std::fs::read_to_string(self.repo.join(relative)).unwrap();
        text.lines()
            .filter_map(|line| {
                let (key, rest) = line.split_once(": ")?;
                (!key.starts_with(' ') && rest.starts_with("ENC["))
                    .then(|| (key.to_owned(), line.to_owned()))
            })
            .collect()
    }

    /// One key's value, decrypted with the identity minted for this fixture.
    pub fn value(&self, relative: &str, key: &str) -> String {
        let mut command = Command::new("sops");
        command
            .arg("decrypt")
            .arg("--extract")
            .arg(format!("[\"{key}\"]"))
            .arg(relative)
            .current_dir(&self.repo)
            .env("SOPS_AGE_KEY_FILE", &self.key_file);
        capture(&mut command)
    }

    /// Whether a file exists in the repository.
    pub fn exists(&self, relative: &str) -> bool {
        self.repo.join(relative).exists()
    }

    /// The bytes of a repository file.
    pub fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.repo.join(relative)).unwrap()
    }

    /// Write a file into the repository.
    pub fn write(&self, relative: &str, contents: &str) {
        let path = self.repo.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    /// Whether any path under the repository, or under the run's `TMPDIR`,
    /// holds this text.
    ///
    /// The value a refused run read must survive in neither.
    pub fn holds_anywhere(&self, needle: &str) -> Option<PathBuf> {
        [self.repo.clone(), self.work.join("tmp")]
            .into_iter()
            .find_map(|root| search(&root, needle))
    }

    /// Whether a scratch file was left beside a secret.
    pub fn scratch_files(&self) -> Vec<PathBuf> {
        let mut found = Vec::new();
        collect(&self.repo, &mut found, &|name| name.contains("safix-tmp"));
        found
    }

    // ── building the subject ───────────────────────────────────────────────

    /// A real multi-key sops file at this path, carrying exactly these keys,
    /// written through the fixture policy and committed.
    pub fn make_sops_file(&self, relative: &str, keys: &[&str]) {
        let plain = self.work.join("plain.yaml");
        let mut text = String::new();
        for key in keys {
            writeln!(text, "{key}: \"fixture-value-for-{key}\"").unwrap();
        }
        std::fs::write(&plain, text).unwrap();
        if let Some(parent) = self.repo.join(relative).parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut command = Command::new("sops");
        command
            .arg("encrypt")
            .arg("--filename-override")
            .arg(relative)
            .arg("--input-type")
            .arg("yaml")
            .arg("--output-type")
            .arg("yaml")
            .arg(&plain)
            .current_dir(&self.repo)
            .env("SOPS_AGE_KEY_FILE", &self.key_file);
        let encrypted = capture_bytes(&mut command);
        std::fs::write(self.repo.join(relative), encrypted).unwrap();
        self.git(&["add", "--", relative]);
        self.git(&["commit", "-q", "-m", &format!("fixture: {relative}")]);
    }

    /// A file encrypted straight to the named recipients, going around the
    /// creation rules.
    ///
    /// `--config /dev/null` because the recipients must come from `--age` and
    /// from nothing else: sops otherwise searches upward for a `.sops.yaml` and
    /// the fixture would depend on which directory the test was invoked from.
    pub fn encrypt_to(&self, relative: &str, recipients: &[&str], contents: &str) {
        let plain = self.work.join("straight.yaml");
        std::fs::write(&plain, contents).unwrap();
        if let Some(parent) = self.repo.join(relative).parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut command = Command::new("sops");
        command
            .arg("--config")
            .arg("/dev/null")
            .arg("encrypt")
            .arg("--age")
            .arg(recipients.join(","))
            .arg("--input-type")
            .arg("yaml")
            .arg("--output-type")
            .arg("yaml")
            .arg(&plain)
            .current_dir(&self.repo)
            .env("SOPS_AGE_KEY_FILE", &self.key_file);
        let encrypted = capture_bytes(&mut command);
        std::fs::write(self.repo.join(relative), encrypted).unwrap();
    }

    /// Re-wrap a file to the creation rule that covers it, which is what
    /// `safix fix` runs.
    pub fn updatekeys(&self, relative: &str) {
        let mut command = Command::new("sops");
        command
            .arg("updatekeys")
            .arg("-y")
            .arg(relative)
            .current_dir(&self.repo)
            .env("SOPS_AGE_KEY_FILE", &self.key_file);
        run_to_success(&mut command, "re-wrapping the drifted file");
    }

    /// The two people the fixture policy anchors, declared as `adduser` writes
    /// them and tracked, because the policy the stub renders follows what git
    /// tracks.
    pub fn seed_declarations(&self) {
        for (user, recipient) in [("alice", self.alice.clone()), ("bob", self.bob.clone())] {
            self.write_declaration(user, &recipient);
        }
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", "fixture: two declared people"]);
    }

    /// A scratch directory of this fixture's own, made rather than named.
    pub fn scratch_dir(&self, name: &str) -> PathBuf {
        let path = self.work.join(name);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    /// A recipient minted here, for a person the fixture does not declare.
    pub fn new_recipient(&self) -> String {
        let path = self.work.join("stranger-key.txt");
        run_to_success(
            Command::new("age-keygen").arg("-o").arg(&path),
            "minting a stranger's identity",
        );
        let recipient = capture(Command::new("age-keygen").arg("-y").arg(&path));
        std::fs::remove_file(&path).unwrap();
        recipient
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Every plaintext this test wrote is under here, so it goes on every
        // exit path including a panicking one. On tmpfs the pages are freed with
        // the directory.
        let _ = std::fs::remove_dir_all(&self.work);
        if let Some(disk_staging) = &self.disk_staging {
            let _ = std::fs::remove_dir_all(disk_staging);
        }
    }
}

/// A freshly allocated pseudoterminal pair.
///
/// The master is what a test types into and the slave is what the command's
/// standard input is. Opened here rather than by standing a program such as
/// `script` in front of the command, so the suite's dependency set does not grow
/// and the environment reaches the command the way every other run's does.
struct Pty {
    master: std::os::fd::OwnedFd,
    slave: std::os::fd::OwnedFd,
}

impl Pty {
    fn open() -> Self {
        use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};

        let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY)
            .expect("this platform has no pseudoterminals");
        grantpt(&master).expect("the pseudoterminal could not be granted");
        unlockpt(&master).expect("the pseudoterminal could not be unlocked");
        let name = ptsname(&master, Vec::new()).expect("the pseudoterminal has no name");
        let path = PathBuf::from(String::from_utf8_lossy(name.as_bytes()).into_owned());

        let slave = std::fs::File::options()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap_or_else(|cause| panic!("{} could not be opened: {cause}", path.display()));

        Self {
            master,
            slave: slave.into(),
        }
    }
}

/// Which reporter a run is made under.
#[derive(Clone, Copy)]
enum Reporter {
    /// The shell runtime's shape: one paragraph, no code.
    Plain,
    /// The graphical renderer, which names the refusal's code.
    Graphical,
}

/// The filesystem type mounted at the deepest mount point covering this path,
/// read from `/proc/mounts`.
///
/// An oracle independent of the runtime, and that independence is the whole
/// reason it exists. `staging::memory_backed` is the code under test wherever a
/// drill needs a disk-backed directory, and a drill that *selected* its fixture
/// with the function under test cannot fail when that function is defeated: a
/// probe that answered "memory-backed" for everything would make the selection
/// find nothing, and a drill that finds nothing to drill reports that it was
/// skipped and passes. So the selection is made here, from the kernel's own
/// mount table, and the runtime's probe is then held against it.
///
/// The deepest covering mount point rather than an exact match, because the
/// paths a drill hands this are directories inside a mount rather than mount
/// points themselves.
#[must_use]
pub fn mounted_filesystem(path: &Path) -> Option<String> {
    let target = std::fs::canonicalize(path).ok()?;
    let mounts = std::fs::read_to_string("/proc/mounts").ok()?;
    let mut best: Option<(usize, String)> = None;
    for line in mounts.lines() {
        let mut fields = line.split_whitespace();
        let (_, point, kind) = (fields.next()?, fields.next()?, fields.next()?);
        // `/proc/mounts` octal-escapes a space in a mount point; a path holding
        // one is not a path this suite makes, and treating the escape as
        // literal would only ever fail to match.
        let point = PathBuf::from(point);
        if target.starts_with(&point) {
            let depth = point.components().count();
            if best.as_ref().is_none_or(|(seen, _)| depth > *seen) {
                best = Some((depth, kind.to_owned()));
            }
        }
    }
    best.map(|(_, kind)| kind)
}

/// Filesystems the kernel keeps entirely in memory.
///
/// `tmpfs` and `ramfs` are what `staging.rs` admits by magic number. `devtmpfs`
/// is here as well and is not a widening: it is a tmpfs instance the kernel
/// mounts for `/dev`, and it reports the tmpfs magic to `statfs`, so a table
/// reading that called it disk-backed would disagree with the probe about a
/// mount both are right about.
const MEMORY_FILESYSTEMS: &[&str] = &["tmpfs", "ramfs", "devtmpfs"];

/// Whether the kernel says this path's filesystem keeps its pages in memory.
///
/// Answered from the mount table rather than from `statfs`, so this reading and
/// the runtime's share no code.
#[must_use]
pub fn kernel_says_memory_backed(path: &Path) -> Option<bool> {
    let kind = mounted_filesystem(path)?;
    Some(MEMORY_FILESYSTEMS.contains(&kind.as_str()))
}

/// A directory the kernel's mount table reports as disk-backed, if one is
/// reachable.
///
/// `None` where every candidate is memory-backed, which is a real state — a
/// build sandbox whose whole tree is a tmpfs is one — and is reported rather
/// than treated as a pass.
///
/// `preferred` goes in front of the conventional candidates rather than behind
/// them, so a caller holding a directory of its own — [`Fixture::disk_staging_dir`]
/// is the one that does — gets that one back and not a directory the rest of the
/// machine also writes into. It is still checked against the mount table like
/// any other candidate, so the answer remains the kernel's rather than the
/// caller's assumption about where its own scratch landed.
#[must_use]
pub fn disk_backed_directory(preferred: Option<&Path>) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = preferred.map(Path::to_path_buf).into_iter().collect();
    // The checkout is last. It is the one candidate that is somebody's working
    // tree, and a scratch directory appearing in it mid-run is visible to every
    // `git` this suite and its operator run.
    candidates.extend([
        std::env::temp_dir(),
        PathBuf::from("/tmp"),
        PathBuf::from("/var/tmp"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    ]);
    candidates
        .into_iter()
        .find(|path| kernel_says_memory_backed(path) == Some(false))
}

/// A mode-700 disk-backed directory unique to one fixture, where this machine
/// has a disk-backed filesystem at all.
///
/// The base is chosen by the same independent oracle every other disk-backed
/// selection uses, so a machine that is tmpfs throughout answers `None` here and
/// `None` from [`disk_backed_directory`], and the drills report themselves
/// undrilled once rather than disagreeing.
fn disk_backed_scratch() -> Option<PathBuf> {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base = disk_backed_directory(None)?;
    let path = base.join(format!(
        "safix-disk-stage-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    // Unwrapped rather than swallowed: falling back to a shared directory here
    // would restore the residue-under-concurrency defect this exists to remove,
    // and would do it silently.
    std::fs::create_dir_all(&path).unwrap();
    permit_owner_only(&path);
    Some(path)
}

/// The stubbed clan's store holds an encoding rather than the value.
///
/// See `tests/support/clan-stub.rs`: a real clan writes ciphertext, so the
/// plaintext bytes never reach a regular file, and the stub reproduces that one
/// property so the syscall reading stays a statement about safix.
fn to_hex(value: &str) -> String {
    use std::fmt::Write as _;
    value.bytes().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

fn from_hex(text: &str) -> Option<String> {
    let digits: Vec<char> = text.trim().chars().collect();
    let bytes: Option<Vec<u8>> = digits
        .chunks(2)
        .map(|pair| {
            let text: String = pair.iter().collect();
            u8::from_str_radix(&text, 16).ok()
        })
        .collect();
    String::from_utf8(bytes?).ok()
}

/// Every staging root sitting directly under one directory, by the prefix
/// `staging.rs` gives them.
fn roots_under(directory: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("safix-stage-"))
        })
        .collect()
}

/// One placement, in the shape `flake.safix.lib.placements` has.
fn placement(file: &str, key: &str, origin: &str, owner: &str) -> Value {
    json!({
        "file": file, "key": key, "origin": origin,
        "owner": owner, "shared": false, "generator": null, "public": null,
    })
}

/// The creation rules, with the shared audience's granting the named anchors.
fn rules_block(shared_anchors: &[&str]) -> String {
    let mut rules = String::from("creation_rules:\n");
    rules.push_str("  - path_regex: ^secrets/safix/users/alice/[^/]*\\.yaml$\n");
    rules.push_str("    key_groups:\n      - age:\n          - *alice\n");
    rules.push_str("  - path_regex: ^secrets/safix/shared/alice,bob/[^/]*\\.yaml$\n");
    rules.push_str("    key_groups:\n      - age:\n");
    for anchor in shared_anchors {
        writeln!(rules, "          - *{anchor}").unwrap();
    }
    rules
}

/// The argument vector a run gets, carrying the acknowledgement where the
/// platform leaves nothing else.
///
/// `SAFIX_TEST_DISK_STAGING` says the caller accepts disk-backed staging — see
/// [`staging_root`], which reads it for the suite's own scratch directory. The
/// runtime needs telling the same fact separately, because it decides for itself
/// where a value may be staged and on a platform with no tmpfs nothing it would
/// stage into answers memory-backed: `safix_core::staging::memory_backed` reads
/// `statfs` for linux's tmpfs and ramfs and declines to guess at anything else.
/// Its answer there is a refusal naming this flag. So where the escape is in
/// force, a run of a verb that stages carries the acknowledgement, and the suite
/// exercises the platform's documented mode instead of two dozen copies of its
/// refusal.
///
/// The three verbs are the three places `Staging::establish` is reached from —
/// `edit`, `generate` and `enroll` — so the list is closed by the runtime rather
/// than chosen. Every other verb refuses an option it does not know, or worse
/// reads it as a positional, which is why this is not a flag to add blindly.
///
/// Inserted after the verb rather than before it, for two reasons that are both
/// load-bearing: `generate` stops reading flags at its first positional, and
/// [`refuse_a_real_card`] and [`refuse_a_real_database`] identify a run by
/// `arguments.first()`, which anything prepended would disarm.
///
/// A vector already carrying the flag is returned untouched, which leaves the
/// acknowledged leg of `the_acknowledgement_is_the_only_way_past_the_refusal`
/// saying what it says. Its refusing leg needs a run *without* the flag and keeps
/// one: that drill names a directory the kernel calls disk-backed, and where no
/// mount table can be read none can be named, so it reports itself skipped on
/// exactly the platforms this escape is in force on.
fn acknowledged(arguments: &[&str]) -> Vec<String> {
    let given: Vec<String> = arguments.iter().map(|word| (*word).to_owned()).collect();
    let stages = matches!(arguments.first(), Some(&"edit" | &"generate" | &"enroll"));
    if std::env::var_os("SAFIX_TEST_DISK_STAGING").is_none()
        || !stages
        || arguments.contains(&safix_core::staging::ACKNOWLEDGEMENT)
    {
        return given;
    }
    let mut acknowledged = given;
    acknowledged.insert(1, safix_core::staging::ACKNOWLEDGEMENT.to_owned());
    acknowledged
}

/// Refuse an enrollment run that has not been pointed at the card stub.
///
/// A structural guard rather than a convention, because of what forgetting costs.
/// The machines this suite is developed on have the real `ykman`, the real age
/// plugin and a real password store on their path, and a hardware key in a reader
/// holding master identities for everything the fleet owns. A run that reached
/// those would provision a live card — a new PIN, a new PUK, a management key
/// nobody recorded — and there is no undoing it.
///
/// Every override the runtime reads for that surface has to be present and has to
/// name the stub. A test that builds its environment any way other than
/// [`Fixture::card_env`] fails here, loudly, before a process is spawned.
fn refuse_a_real_card(arguments: &[&str], extra: &[(&str, &str)]) {
    if arguments.first() != Some(&"enroll") {
        return;
    }
    for override_variable in [
        "SAFIX_YKMAN",
        "SAFIX_AGE_PLUGIN_YUBIKEY",
        "SAFIX_SECRET_TOOL",
        "SAFIX_KEEPASSXC_CLI",
    ] {
        let named = extra
            .iter()
            .find(|(variable, _)| *variable == override_variable)
            .map(|(_, value)| *value);
        assert_eq!(
            named,
            Some(card_stub()),
            "an enrollment run was not pointed at the card stub through \
             {override_variable}; it would have reached the real tool, and the card in \
             the reader is not a fixture. Build the environment with Fixture::card_env."
        );
    }
}

/// Refuse a sync or audit run that has not been pointed at the store stub.
///
/// The counterpart of [`refuse_a_real_card`], and it exists for the same reason
/// with a different loss at the end of it. The machines this suite is developed on
/// have the real `keepassxc-cli` and the operator's own 292 MB database, and a run
/// that reached it would edit or create entries in the fleet's root of trust. A
/// run whose database is anywhere but the fixture's own scratch directory is
/// refused here, loudly, before a process is spawned.
///
/// `sync` and `audit` both reach the keepassxc target — bare, or named — so both
/// are guarded the same way; `sync clan` and `audit clan` never touch a database
/// at all, real or fixture, and are exempt.
///
/// Both halves are checked, because either one alone would let the accident
/// through: the override has to name the stub, and the database the fixture
/// declares has to be under the scratch directory. A test that builds its
/// environment any way other than [`Fixture::store_env`] fails on the first, and
/// one that declares a database of its own fails on the second.
fn refuse_a_real_database(fixture: &Fixture, arguments: &[&str], extra: &[(&str, &str)]) {
    let touches_keepassxc = match arguments.first() {
        Some(&("sync" | "audit")) => arguments.get(1) != Some(&"clan"),
        _ => false,
    } && !arguments.contains(&"-h")
        && !arguments.contains(&"--help");
    if !touches_keepassxc {
        return;
    }
    let named = extra
        .iter()
        .find(|(variable, _)| *variable == "SAFIX_KEEPASSXC_CLI")
        .map(|(_, value)| *value);
    assert_eq!(
        named,
        Some(card_stub()),
        "a sync run was not pointed at the store stub through SAFIX_KEEPASSXC_CLI; it \
         would have reached the real keepassxc-cli, and the database on this machine is \
         not a fixture. Build the environment with Fixture::store_env."
    );

    let declared = std::fs::read_to_string(fixture.work.join("keepassxc.json")).unwrap_or_default();
    let scratch = fixture.work.to_string_lossy().into_owned();
    for word in declared.split('"') {
        let names_a_database = word
            .rsplit_once('.')
            .is_some_and(|(_, tail)| tail == "kdbx");
        assert!(
            !names_a_database || word.starts_with(&scratch),
            "a sync run declared the database '{word}', which is outside the fixture's own \
             scratch directory {scratch}. Declare Fixture::kdbx and nothing else."
        );
    }
}

/// One list the fixture keeps for the modelled database, as lines.
fn read_lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

fn write_lines(path: &Path, lines: &[String]) {
    let mut sorted = lines.to_vec();
    sorted.sort();
    sorted.dedup();
    std::fs::write(path, sorted.join("\n") + "\n").unwrap();
}

/// One age identity, minted into a named file.
///
/// The proof's stand-in for a card: an ordinary identity, in a file of its own, so
/// that "the isolated source is what opened this" is a claim about one identity
/// rather than about whichever key happened to be reachable.
pub fn mint_identity(path: &Path) {
    run_to_success(
        Command::new("age-keygen").arg("-o").arg(path),
        "minting a stand-in identity",
    );
}

/// The recipient of an identity file.
pub fn recipient_of(path: &Path) -> String {
    capture(Command::new("age-keygen").arg("-y").arg(path))
}

/// A mode-700 scratch directory on tmpfs, unique to one fixture.
///
/// Verified rather than assumed: `/tmp` on the machines this runs on is
/// disk-backed, so a value staged there would outlive the test in a way removing
/// the directory does not undo.
fn scratch() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let root = staging_root();
    let unique = format!(
        "safix-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let path = root.join(unique);
    std::fs::create_dir_all(&path).unwrap();
    permit_owner_only(&path);
    path
}

/// Where staged plaintext is allowed to live.
fn staging_root() -> PathBuf {
    let shm = PathBuf::from("/dev/shm");
    if is_tmpfs(&shm) {
        let root = shm.join(format!("safix-tests-{}", user_id()));
        std::fs::create_dir_all(&root).unwrap();
        permit_owner_only(&root);
        return root;
    }
    assert!(
        std::env::var_os("SAFIX_TEST_DISK_STAGING").is_some(),
        "/dev/shm is not tmpfs here and this suite stages plaintext. Set \
         SAFIX_TEST_DISK_STAGING=1 to accept disk-backed staging."
    );
    std::env::temp_dir()
}

/// Whether a path is a tmpfs mount point, read from the kernel rather than
/// assumed from its name.
fn is_tmpfs(path: &Path) -> bool {
    let Ok(mounts) = std::fs::read_to_string("/proc/mounts") else {
        return false;
    };
    mounts.lines().any(|line| {
        let mut fields = line.split_whitespace();
        let (_, point, kind) = (fields.next(), fields.next(), fields.next());
        point == path.to_str() && kind == Some("tmpfs")
    })
}

/// Nobody but the owner reads a directory plaintext is staged in.
fn permit_owner_only(path: &Path) {
    use std::os::unix::fs::PermissionsExt as _;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
}

/// This process's user, for a staging directory nobody else can enter.
fn user_id() -> String {
    capture(Command::new("id").arg("-u"))
}

/// The sops a shim stands in front of, found the way the runtime finds it.
///
/// A shim that resolved `sops` by name would find itself, because the fixture
/// puts it on the runtime's `PATH` under that name.
pub fn real_sops() -> String {
    std::env::var_os("PATH")
        .and_then(|path| {
            std::env::split_paths(&path)
                .map(|directory| directory.join("sops"))
                .find(|candidate| candidate.is_file())
        })
        .map(|path| path.display().to_string())
        .expect("sops is not on PATH")
}

/// `setsid`, when this process has a controlling terminal and so would hand one
/// to the command.
///
/// A build sandbox has none and this is `None` there, which is the branch the
/// checks exercise. On a developer's terminal the command would read a prompt or a
/// confirmation from `/dev/tty` rather than from the standard input the test
/// wrote, so the run is detached into its own session first — including the
/// pseudoterminal runs, where a `/dev/tty` that opened would be the developer's
/// keyboard and not the terminal the test allocated.
fn detached() -> Option<PathBuf> {
    if std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .is_err()
    {
        return None;
    }
    let found = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|directory| directory.join("setsid"))
            .find(|candidate| candidate.is_file())
    });
    assert!(
        found.is_some(),
        "this terminal would answer the command's prompt instead of the test. \
         Run the suite where there is no controlling terminal, or install setsid."
    );
    found
}

/// Feed a run its standard input, then collect what it left on each stream.
fn finish(mut child: Child, stdin: Option<&str>) -> Run {
    if let (Some(text), Some(mut pipe)) = (stdin, child.stdin.take()) {
        let _ = pipe.write_all(text.as_bytes());
        let _ = pipe.flush();
        drop(pipe);
    }
    let output = child
        .wait_with_output()
        .expect("the command did not finish");
    Run {
        code: output.status.code(),
        stdout: output.stdout,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// One command's standard output, as trimmed text.
fn capture(command: &mut Command) -> String {
    String::from_utf8_lossy(&capture_bytes(command))
        .trim_end_matches('\n')
        .to_owned()
}

/// One command's standard output, verbatim.
fn capture_bytes(command: &mut Command) -> Vec<u8> {
    let output = command.output().expect("could not run the command");
    assert!(
        output.status.success(),
        "{command:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

/// Run a command that has nothing to say.
fn run_to_success(command: &mut Command, what: &str) {
    let output = command.output().expect("could not run the command");
    assert!(
        output.status.success(),
        "{what} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Write one fixture document.
fn write_json(path: &Path, value: &Value) {
    std::fs::write(path, serde_json::to_string_pretty(value).unwrap()).unwrap();
}

/// The first file under a root holding this text, if any.
fn search(root: &Path, needle: &str) -> Option<PathBuf> {
    let mut found = Vec::new();
    collect(root, &mut found, &|_| true);
    found.into_iter().find(|path| {
        std::fs::read(path).is_ok_and(|bytes| {
            bytes
                .windows(needle.len())
                .any(|window| window == needle.as_bytes())
        })
    })
}

/// Every file under a root whose name the predicate accepts.
fn collect(root: &Path, found: &mut Vec<PathBuf>, accept: &dyn Fn(&str) -> bool) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some(".git") {
                continue;
            }
            collect(&path, found, accept);
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(accept)
        {
            found.push(path);
        }
    }
}
