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
//! # Why some runs go through `setsid`
//!
//! The command reads a value from `/dev/tty` when it opens and from standard
//! input when it does not. A build sandbox has no controlling terminal, so the
//! stdin branch is what the checks exercise; a developer's terminal has one, so
//! a run whose value arrives on standard input is detached into its own session
//! first. Runs that are meant to block — the interrupted ones — are not
//! detached, because the signal has to reach the process itself.

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
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::{Value, json};

/// ana's own file: the audience is one person, and the creation rule grants her
/// alone.
pub const ANA_FILE: &str = "secrets/safix/users/ana/secrets.yaml";

/// The file the pair shares, in the audience directory named for both in sorted
/// order.
pub const SHARED_FILE: &str = "secrets/safix/shared/ana,bo/secrets.yaml";

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
    /// ana's recipient, minted here.
    pub ana: String,
    /// bo's recipient, minted here. His private half is destroyed as soon as the
    /// recipient is derived, which is also how the fixture shows that writing to
    /// a shared file needs no key but the writer's own.
    pub bo: String,
    key_file: PathBuf,
    placements: Value,
    audiences: Value,
    genplan: Value,
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

        let key_file = work.join("age-key.txt");
        run_to_success(
            Command::new("age-keygen").arg("-o").arg(&key_file),
            "minting ana's identity",
        );
        let ana = capture(Command::new("age-keygen").arg("-y").arg(&key_file));

        // Two distinct recipients rather than one key under two anchors, so
        // "the created file took its recipients from the creation rule" is a
        // claim a file encrypted to the writer alone would fail.
        let bo_key = work.join("bo-key.txt");
        run_to_success(
            Command::new("age-keygen").arg("-o").arg(&bo_key),
            "minting bo's identity",
        );
        let bo = capture(Command::new("age-keygen").arg("-y").arg(&bo_key));
        std::fs::remove_file(&bo_key).unwrap();

        let fixture = Self {
            work,
            repo,
            ana: ana.clone(),
            bo: bo.clone(),
            key_file,
            placements: json!({
                "ana": {
                    "api-token":      placement(ANA_FILE, "api-token", "carries", "ana"),
                    "mail-password":  placement(ANA_FILE, "mail-password", "private", "ana"),
                    "aliased-secret": placement(ANA_FILE, "custom-key", "private", "ana"),
                    "wifi-psk":       placement(SHARED_FILE, "wifi-psk", "shared", "bo"),
                    "no-rule-secret": placement(
                        "secrets/safix/users/cy/secrets.yaml", "no-rule-secret", "private", "ana"),
                    "not-yaml":       placement(
                        "secrets/safix/users/ana/secret.age", "not-yaml", "private", "ana"),
                },
                "bo": {
                    "wifi-psk": placement(SHARED_FILE, "wifi-psk", "private", "bo"),
                },
            }),
            audiences: json!({
                ANA_FILE: {
                    "audience": ["ana"],
                    "dir": "secrets/safix/users/ana",
                    "recipients": [ana],
                },
                SHARED_FILE: {
                    "audience": ["ana", "bo"],
                    "dir": "secrets/safix/shared/ana,bo",
                    "recipients": [ana, bo],
                },
            }),
            genplan: json!({
                "ana": { "order": [], "outputs": {}, "inputs": {} },
                "bo":  { "order": [], "outputs": {}, "inputs": {} },
            }),
            extras: Vec::new(),
        };

        fixture.write_policy(&["ana", "bo"]);
        fixture.git(&["init", "-q"]);
        fixture.git(&["config", "user.email", "selftest@example.invalid"]);
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
    pub fn write_policy(&self, shared_anchors: &[&str]) {
        let mut policy = String::from("keys:\n");
        write!(policy, "  - &ana {}\n  - &bo {}\n", self.ana, self.bo).unwrap();
        policy.push_str(&rules_block(shared_anchors));
        std::fs::write(self.repo.join(".sops.yaml"), policy).unwrap();
        std::fs::write(self.work.join("rules.txt"), rules_block(&["ana", "bo"])).unwrap();
    }

    /// Declare a placement for a name with no generator.
    pub fn seed_output(&mut self, name: &str, file: &str) {
        self.placements["ana"][name] = placement(file, name, "private", "ana");
        self.write_fixtures();
    }

    /// Declare an entry both people carry and `shared = true` makes one value
    /// of: two placements, each with its carrier as owner, both naming one file
    /// and one key.
    pub fn seed_shared(&mut self, name: &str, file: &str) {
        for user in ["ana", "bo"] {
            self.placements[user][name] = json!({
                "file": file, "key": name, "origin": "carries",
                "owner": user, "shared": true, "generator": null,
            });
        }
        self.write_fixtures();
    }

    /// Drop bo from a shared entry, leaving the remaining carrier's own file as
    /// its placement. The ciphertext is left exactly where it was, which is the
    /// state a revocation is discovered in.
    pub fn unshare_from(&mut self, name: &str, remaining: &str, file: &str) {
        if let Some(bo) = self.placements["bo"].as_object_mut() {
            bo.remove(name);
        }
        self.placements[remaining][name]["file"] = json!(file);
        self.write_fixtures();
    }

    /// Declare a generator and derive its run-plan entry from the same record
    /// the command reads.
    ///
    /// `inputs` is computed the way `modules/flake/safix/resolve.nix` computes
    /// it — prompts and dependencies in one name space, hyphens mapped to
    /// underscores — so a change to that mapping on one side and not the other
    /// fails here.
    pub fn seed_generator(&mut self, name: &str, file: &str, outputs: &[&str], record: &Value) {
        self.placements["ana"][name] = json!({
            "file": file, "key": name, "origin": "private",
            "owner": "ana", "shared": false, "generator": record.clone(),
        });

        let mut inputs = serde_json::Map::new();
        if let Some(prompts) = self.placements["ana"][name]["generator"]["prompts"].as_object() {
            for prompt in prompts.keys() {
                inputs.insert(
                    prompt.replace('-', "_"),
                    json!({ "kind": "prompt", "name": prompt }),
                );
            }
        }
        if let Some(dependencies) =
            self.placements["ana"][name]["generator"]["dependencies"].as_array()
        {
            for dependency in dependencies.iter().filter_map(Value::as_str) {
                inputs.insert(
                    dependency.replace('-', "_"),
                    json!({ "kind": "dependency", "name": dependency }),
                );
            }
        }

        let mut declared = vec![name.to_owned()];
        declared.extend(outputs.iter().map(|output| (*output).to_owned()));
        self.genplan["ana"]["order"]
            .as_array_mut()
            .unwrap()
            .push(json!(name));
        self.genplan["ana"]["outputs"][name] = json!(declared);
        self.genplan["ana"]["inputs"][name] = Value::Object(inputs);
        self.write_fixtures();
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

        let recipients = json!({ "ana": [self.ana], "bo": [self.bo] });
        let governed = json!({
            "required": required,
            "extra": self.extras,
            "managed": managed,
        });

        write_json(&self.work.join("placements.json"), &self.placements);
        write_json(&self.work.join("audiences.json"), &self.audiences);
        write_json(&self.work.join("genplan.json"), &self.genplan);
        write_json(&self.work.join("recipients.json"), &recipients);
        write_json(&self.work.join("governed.json"), &governed);
        if !self.work.join("hook.json").exists() {
            self.set_hook(None);
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

    /// `set`, with the value typed twice as the prompt asks for it.
    pub fn set(&self, user: &str, name: &str, value: &str) -> Run {
        self.run_with(&["set", user, name], &format!("{value}\n{value}\n"))
    }

    /// `set`, with a confirmation that differs from the value.
    pub fn set_confirming(&self, user: &str, name: &str, value: &str, again: &str) -> Run {
        self.run_with(&["set", user, name], &format!("{value}\n{again}\n"))
    }

    /// The command's own environment, ready for a caller that needs to spawn it
    /// itself.
    pub fn command(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new(safix());
        command.args(arguments);
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
        command.args(arguments);
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
        let mut command = match (stdin, detached()) {
            (Some(_), Some(setsid)) => {
                let mut command = Command::new(setsid);
                command.arg("-w").arg(program);
                command
            }
            _ => Command::new(program),
        };
        command.args(arguments);
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
            .env("USER", "ana")
            .env("SOPS_AGE_KEY_FILE", &self.key_file)
            .env("SAFIX_REPO_ROOT", &self.repo)
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
            .env("SAFIX_FIXTURE_GENPLAN", self.work.join("genplan.json"))
            .env("SAFIX_FIXTURE_HOOK", self.work.join("hook.json"))
            .env("SAFIX_FIXTURE_RULES", self.work.join("rules.txt"));
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
        std::fs::create_dir_all(self.repo.join("safix/users")).unwrap();
        for (user, recipient) in [("ana", &self.ana), ("bo", &self.bo)] {
            let declaration = format!(
                "{{\n  flake.safix.users.{user} = {{\n    recipient = \"{recipient}\";\n    carries = {{ }};\n    private = {{ }};\n  }};\n}}\n"
            );
            std::fs::write(
                self.repo.join(format!("safix/users/{user}.nix")),
                declaration,
            )
            .unwrap();
        }
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", "fixture: two declared people"]);
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

/// One placement, in the shape `flake.safix.lib.placements` has.
fn placement(file: &str, key: &str, origin: &str, owner: &str) -> Value {
    json!({
        "file": file, "key": key, "origin": origin,
        "owner": owner, "shared": false, "generator": null,
    })
}

/// The creation rules, with the shared audience's granting the named anchors.
fn rules_block(shared_anchors: &[&str]) -> String {
    let mut rules = String::from("creation_rules:\n");
    rules.push_str("  - path_regex: ^secrets/safix/users/ana/[^/]*\\.yaml$\n");
    rules.push_str("    key_groups:\n      - age:\n          - *ana\n");
    rules.push_str("  - path_regex: ^secrets/safix/shared/ana,bo/[^/]*\\.yaml$\n");
    rules.push_str("    key_groups:\n      - age:\n");
    for anchor in shared_anchors {
        writeln!(rules, "          - *{anchor}").unwrap();
    }
    rules
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
/// checks exercise. On a developer's terminal the command would read its value
/// from `/dev/tty` rather than from the standard input the test wrote, so the
/// run is detached into its own session first.
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
