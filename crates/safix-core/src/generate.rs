//! Minting values from the generators the declarations attach to them.
//!
//! The order is the resolver's. `flake.safix.lib.generatorPlan` computes a
//! topological order over one user's generators and refuses a cycle, so an order
//! existing at all is that refusal's postcondition; this module walks it and
//! derives no order of its own.
//!
//! It does check that postcondition before the first generator runs — see
//! [`UserPlan::cycle`] — and the reason is the same one that put the question at
//! evaluation. A run commits as it walks, so a cycle met part-way through has
//! already committed values it cannot finish deriving from. The plan is a value
//! with public fields that a stand-in for nix can print and an embedder can
//! build, and for those two callers the refusal has not already been thrown.
//!
//! # One generator at a time
//!
//! The walk is sequential, and that is a property rather than an omission.
//! Three things depend on it. A prompt is read from one standard input, and two
//! generators prompting at once is not a faster question but an unanswerable
//! one. Each generator commits as it goes, so the order of the commits is the
//! order of the plan and not of the scheduler. And each generator's plaintext
//! lives in a staging root for the duration of its run — see [`crate::inputs`]
//! and [`crate::staging`] — so a fan-out would buy latency at the price of
//! several roots holding plaintext at once, over a longer window, with no
//! ordering between the shreds.
//!
//! # What a run records about the declaration it ran
//!
//! Each generator's outputs are accompanied by a [`definition`] record — a digest
//! of the declaration that minted them, written in the same commit as the values.
//! Nothing here reads it; [`crate::check`] does, and reports a value whose
//! declaration has changed since. It is written from this module because this is
//! the only place that knows a mint happened, and it rides the mint's own commit
//! because a record and a value that landed in different commits would be a pair
//! `git log` reads as two unrelated events.
//!
//! # What a run leaves when it stops
//!
//! Nothing, up to the generator it stopped in. Each generator's outputs are
//! staged into candidates beside their targets and renamed into place together,
//! so a run that refuses partway through one generator leaves that generator's
//! files as it found them. A generator's outputs resolve to one audience, so a
//! multi-output write is one staged document and one rename, and the window a
//! crash between two renames used to open for a keypair is closed. It is not
//! closed in general: a `--regenerate` cascade still commits per generator, so
//! generators that already committed stay committed, which is what the cascade
//! confirmation warns about before it starts rather than after.

use std::path::PathBuf;
use std::process::Stdio;

use crate::error::{Error, Result};
use crate::inputs::Tree;
use crate::model::{Generator, PromptKind, UserPlan};
use crate::progress::{Progress, log, note};
use crate::sandbox::Envelope;
use crate::secret::Secret;
use crate::sops::document;
use crate::workspace::Workspace;
use crate::{definition, git, public, scratch, set};

/// Where a generator's prompts are answered and its cascade confirmed.
///
/// The command's, because both need a terminal and this crate has none. Both
/// methods are also where the shell runtime writes its "no terminal" line, so an
/// implementation that falls back to standard input says so there.
pub trait Interaction {
    /// One prompt, answered.
    ///
    /// # Errors
    ///
    /// [`Error::NoValueForPrompt`] when the stream ended before an answer did,
    /// and whatever reading it failed with otherwise.
    fn prompt(&mut self, kind: PromptKind, name: &str, description: &str) -> Result<Secret>;

    /// One yes-or-no question, answered. Anything but yes is no.
    ///
    /// # Errors
    ///
    /// Whatever reading the answer failed with. A stream that ends without one
    /// is not an error — it is a no.
    fn confirm(&mut self, question: &str) -> Result<bool>;
}

/// How a run was asked for.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Re-run over outputs that already hold a value, which is the rotation.
    pub regenerate: bool,
    /// Answer the cascade confirmation in advance.
    pub assume_yes: bool,
    /// Accept that plaintext will be staged on a disk-backed filesystem.
    pub allow_disk_staging: bool,
}

/// What running one generator came to.
enum Outcome {
    /// It ran, wrote its outputs and committed them.
    Ran,
    /// Every output already held a value and no rotation was asked for.
    Skipped,
    /// The run stopped before anything was renamed, and this is the status it
    /// is to end with.
    ///
    /// Two things reach here. `sops` refusing, with its own status, which the
    /// command exits with because sops has already said why. And a signal,
    /// with 130 or 143, which is not a refusal of anything and must not be
    /// reported as one — see [`Step`].
    Refused(i32),
}

/// A step a signal can cut short.
///
/// The distinction this carries is the one an interrupted generator lost. A
/// script killed by the `SIGINT` the operator sent is waited on like any other
/// child, and what `wait` reports is a process that died on a signal: no exit
/// code, which the surrounding code reads as a failure and reports as "the
/// generator failed". It did not fail. It was interrupted, and the run has to
/// end with 130 or 143 rather than with 1 and a sentence blaming the script.
///
/// So every wait in this module answers with this, and the interrupt is read
/// before the status is interpreted.
enum Step<T> {
    /// The step finished, with this.
    Done(T),
    /// The run was asked to stop, and is to end with this status.
    Stopped(i32),
}

/// Run one user's generators, or the one that writes a named secret.
///
/// Returns zero, or the status `sops` exited with when `sops` refused.
///
/// # Errors
///
/// [`Error::UnknownUser`] and [`Error::UnknownName`] for a name no declaration
/// places, [`Error::NoGenerator`] for one nothing mints, [`Error::CascadeDeclined`]
/// when a rotation is declined, and every refusal one generator's run can raise.
pub fn run(
    workspace: &Workspace,
    progress: &dyn Progress,
    interaction: &mut dyn Interaction,
    user: &str,
    name: Option<&str>,
    options: Options,
) -> Result<i32> {
    scratch::set_floor(workspace.root());
    let _guard = scratch::Guard;

    let placements = workspace.placements()?;
    let plan = workspace.generator_plan()?;
    workspace.require_user(user)?;
    let held = placements.held_by(user).ok_or_else(|| Error::UnknownUser {
        user: user.to_owned(),
        declared: placements.users().map(str::to_owned).collect(),
    })?;
    let mine = plan.for_user(user);
    if let Some(cycle) = mine.and_then(UserPlan::cycle) {
        return Err(Error::GeneratorCycle {
            user: user.to_owned(),
            cycle,
        });
    }

    let order: Vec<String> = if let Some(want) = name {
        if !held.contains_key(want) {
            return Err(Error::UnknownName {
                user: user.to_owned(),
                name: want.to_owned(),
                held: held.keys().cloned().collect(),
            });
        }
        let producer = mine
            .and_then(|mine| mine.producer_of(want))
            .ok_or_else(|| Error::NoGenerator {
                user: user.to_owned(),
                name: want.to_owned(),
            })?
            .to_owned();

        // Only a rotation cascades. A first mint of one name leaves nothing
        // derived from a value that no longer exists, and the bulk form already
        // walks every generator in this same order.
        if options.regenerate {
            let cascade = mine.map(|mine| mine.cascade(&producer)).unwrap_or_default();
            confirm_cascade(
                progress,
                interaction,
                &producer,
                &cascade,
                options.assume_yes,
            )?;
            cascade
        } else {
            vec![producer]
        }
    } else {
        mine.map(|mine| mine.order.clone()).unwrap_or_default()
    };

    if order.is_empty() {
        log(
            progress,
            &format!("safix: flake.safix.users.{user} declares no generator."),
        );
        return Ok(0);
    }

    // Once, and before the first fragment. Availability does not change mid-run,
    // and a refusal after generator three has committed is worse than the same
    // refusal before generator one. A user who declares no generator never
    // reaches here, because there is no fragment to confine.
    let envelope = Envelope::probe(workspace.nix(), workspace.root())?;

    let mut ran = 0_usize;
    for generator in &order {
        match run_one(
            workspace,
            progress,
            interaction,
            user,
            generator,
            envelope,
            options,
        )? {
            Outcome::Ran => ran = ran.saturating_add(1),
            Outcome::Skipped => {}
            Outcome::Refused(status) => return Ok(status),
        }
    }
    note(progress, &format!("{ran} generator(s) ran."));
    Ok(0)
}

/// Say what a rotation carries with it, and get an answer before anything runs.
///
/// A rotation that stopped at the value it was asked to rotate would leave every
/// value derived from it standing, and a derived value outlives its input
/// silently: nothing in the tree records which run it came from, so a hash of a
/// retired password reads exactly like a hash of the current one. The set is
/// therefore announced before the first commit rather than after the last —
/// declining afterwards takes nothing back out of history.
fn confirm_cascade(
    progress: &dyn Progress,
    interaction: &mut dyn Interaction,
    generator: &str,
    cascade: &[String],
    assume_yes: bool,
) -> Result<()> {
    let count = cascade.len();
    if count <= 1 {
        return Ok(());
    }

    let mut announcement = format!(
        "\nsafix: {generator} outputs are read by {} other generator(s), which this\n\
        rotation retires the input of. All of them re-run, in this order:\n\n",
        count.saturating_sub(1),
    );
    for name in cascade {
        announcement.push_str("    ");
        announcement.push_str(name);
        announcement.push('\n');
    }
    announcement.push_str(
        "\nEach commits as it goes. Leaving them alone would leave values derived\n\
        from the value being replaced, which nothing afterwards can tell apart\n\
        from values derived from the new one.\n\n",
    );
    progress.write(&announcement);

    if assume_yes {
        log(
            progress,
            &format!("safix: --yes given; re-running all {count}."),
        );
        return Ok(());
    }
    if interaction.confirm(&format!("  re-run all {count}? [y/N] "))? {
        return Ok(());
    }
    Err(Error::CascadeDeclined)
}

/// Where one declared output's value is written.
///
/// The fork is the generator's own `files.<name>.secret`, resolved by the nix
/// half into [`crate::model::Placement::public`] so that the layout of the
/// plaintext store has one implementation rather than two that can disagree
/// about where a value is.
#[derive(Debug, Clone)]
enum Target {
    /// Into an encrypted document, under a key inside it.
    Secret {
        /// The repository-relative path of the document.
        file: String,
        /// The key the value is written under.
        key: String,
    },
    /// Into the repository in the clear, at a path of its own.
    Public {
        /// The repository-relative path of the `value` file.
        file: String,
    },
}

impl Target {
    /// The repository-relative path a commit names for this output.
    fn file(&self) -> &str {
        match self {
            Self::Secret { file, .. } | Self::Public { file } => file,
        }
    }
}

/// The definition records one generator's run is to leave behind.
///
/// One line for the whole run, because one generator has one declaration however
/// many outputs it writes, and one path per output, because drift is reported
/// against the entry an operator holds rather than against the generator. They
/// travel together so that the write path takes one argument rather than a pair
/// it could be handed out of step.
struct Records {
    /// The repository-relative path of each output's record.
    paths: Vec<String>,
    /// The line every one of them holds.
    line: String,
}

/// One generator: its inputs, its run, its outputs, and one commit.
fn run_one(
    workspace: &Workspace,
    progress: &dyn Progress,
    interaction: &mut dyn Interaction,
    user: &str,
    generator: &str,
    envelope: Envelope,
    options: Options,
) -> Result<Outcome> {
    let plan = workspace.generator_plan()?;
    let mine = plan.for_user(user).ok_or_else(|| Error::NoGenerator {
        user: user.to_owned(),
        name: generator.to_owned(),
    })?;
    let outputs = mine.outputs.get(generator).cloned().unwrap_or_default();

    let record = workspace
        .resolve(user, generator)?
        .generator
        .as_ref()
        .ok_or_else(|| Error::NoGenerator {
            user: user.to_owned(),
            name: generator.to_owned(),
        })?;

    let mut targets = Vec::with_capacity(outputs.len());
    let mut records = Records {
        paths: Vec::with_capacity(outputs.len()),
        line: definition::line(record),
    };
    let mut missing = 0_usize;
    for output in &outputs {
        let placement = workspace.resolve(user, output)?;
        let target = match &placement.public {
            Some(file) => Target::Public { file: file.clone() },
            None => Target::Secret {
                file: placement.file.clone(),
                key: placement.key.clone(),
            },
        };
        if !holds_a_value(workspace, &target)? {
            missing = missing.saturating_add(1);
        }
        records
            .paths
            .push(definition::record_path(output, placement));
        targets.push(target);
    }

    if missing == 0 && !options.regenerate {
        note(
            progress,
            &format!(
                "{generator} already holds a value for every output; --regenerate rotates it."
            ),
        );
        return Ok(Outcome::Skipped);
    }

    // Distinct paths first, because the preflight and the write both work per
    // file: two secret outputs of one generator share a document whenever they
    // share an audience, which — since a generator's outputs are constrained to
    // one audience — is always.
    let mut distinct: Vec<String> = Vec::new();
    for target in &targets {
        if !distinct.iter().any(|file| file == target.file()) {
            distinct.push(target.file().to_owned());
        }
    }
    for file in &distinct {
        set::refuse_bad_repository_state(workspace, file)?;
    }

    let names = outputs.join(", ");
    log(progress, &format!("safix: generating {names} for {user}"));

    let values = match mint(
        workspace,
        interaction,
        user,
        generator,
        record,
        &outputs,
        envelope,
        options,
    )? {
        Step::Done(values) => values,
        Step::Stopped(status) => return Ok(Outcome::Refused(status)),
    };

    // Every candidate is judged before any is written. A validation that ran
    // after the first write would leave one output committed and the rest
    // refused, which for a keypair is the state where the halves do not match.
    for (output, value) in outputs.iter().zip(values.iter()) {
        if value.is_empty() {
            return Err(Error::GeneratorProducedNothing {
                generator: generator.to_owned(),
                output: output.clone(),
            });
        }
        if let Some(validation) = record.validation.as_deref().filter(|text| !text.is_empty())
            && let Step::Stopped(status) = validate(
                workspace, record, validation, generator, output, value, envelope,
            )?
        {
            return Ok(Outcome::Refused(status));
        }
    }

    write(
        workspace,
        progress,
        &format!("chore(safix): generate {names} for {user}"),
        &distinct,
        &targets,
        &values,
        &records,
    )
}

/// Whether an output already holds a value.
///
/// For an encrypted output, answered off the ciphertext: `check` asks the same
/// question about people whose files it cannot decrypt, so it may not decrypt to
/// find out, and asking the same way keeps the two from disagreeing about one
/// file. For a public output, answered off the file, and an empty file counts as
/// holding nothing for the same reason an empty key does — it is the state a
/// truncated write leaves behind.
fn holds_a_value(workspace: &Workspace, target: &Target) -> Result<bool> {
    match target {
        Target::Secret { file, key } => {
            let Some(text) = workspace.read_relative(file)? else {
                return Ok(false);
            };
            Ok(document::keys_of(&text)?
                .get(key)
                .is_some_and(|state| !state.empty))
        }
        Target::Public { file } => Ok(public::holds_a_value(&workspace.absolute(file))),
    }
}

/// Stage every input, run the script, and read every declared output back.
///
/// The staging tree is established before a prompt is asked, so an operator who
/// answers a prompt has already had the refusal a disk-backed host would raise.
/// It is dropped when this returns, however it returns, which is what shreds the
/// answers and the outputs once they are values in memory.
#[expect(
    clippy::too_many_arguments,
    reason = "one run's whole context, and every argument is read here rather \
              than passed on: bundling them into a record would name the same \
              fields one indirection away"
)]
fn mint(
    workspace: &Workspace,
    interaction: &mut dyn Interaction,
    user: &str,
    generator: &str,
    record: &Generator,
    outputs: &[String],
    envelope: Envelope,
    options: Options,
) -> Result<Step<Vec<Secret>>> {
    let plan = workspace.generator_plan()?;
    let declared = plan
        .for_user(user)
        .and_then(|mine| mine.inputs.get(generator));

    let tree = Tree::establish(options.allow_disk_staging, !record.prompts.is_empty())?;

    for input in declared.into_iter().flatten().map(|(_, input)| input) {
        match input.kind {
            crate::model::InputKind::Prompt => {
                let asked =
                    record
                        .prompts
                        .get(&input.name)
                        .ok_or_else(|| Error::NixSchemaMismatch {
                            attribute: "flake.safix.lib.generatorPlan",
                            cause: format!(
                                "the plan names a prompt '{}' the entry does not declare",
                                input.name
                            ),
                        })?;
                let answer = interaction.prompt(asked.kind, &input.name, &asked.description)?;
                if answer.is_empty() {
                    return Err(Error::PromptUnanswered {
                        name: input.name.clone(),
                    });
                }
                tree.add_prompt(&input.name, &answer)?;
            }
            crate::model::InputKind::Dependency => {
                // The directory is named for the entry the value's producer is
                // declared on, which is clan's keying: `$in/openssh/…` means
                // here what it means there. A dependency nothing generates —
                // a hand-set value, which safix has and clan does not — is
                // keyed by its own name, because there is no producer to name
                // it after and inventing one would be a directory naming
                // nothing.
                //
                // Only the declared dependency is placed under it. clan
                // materializes every file of the dependency generator, which
                // would hand a script depending on a keypair's public half the
                // private half as well.
                let producer = plan
                    .for_user(user)
                    .and_then(|mine| mine.producer_of(&input.name))
                    .unwrap_or(&input.name)
                    .to_owned();
                let placement = workspace.resolve(user, &input.name)?;
                let absolute = workspace.absolute(&placement.file);
                tree.add_dependency(
                    &producer,
                    &input.name,
                    workspace.sops(),
                    &placement.file,
                    &absolute,
                    &placement.key,
                )?;
            }
        }
    }

    // Standard input is `/dev/null`, and that is part of the interface rather
    // than tidiness. A generator's inputs are files under its staging root; this
    // command's own standard input is where an operator's prompt answers arrive,
    // and a script that read it would eat the answers to every prompt after it —
    // silently, since a prompt reading end-of-input looks exactly like one
    // nobody answered. That property held under the descriptor interface too,
    // but it held there for a different reason, so it is re-asserted rather than
    // assumed to have carried over.
    //
    // Standard output is inherited rather than captured, because it is no longer
    // where a value travels: a script's output is diagnostic now, and capturing
    // it would hide a `set -x` trace the operator asked for.
    // The staging root is bound read-write at the path it has outside the
    // envelope, so the three directories `tree.apply` names resolve inside it by
    // the names the fragment was written against. What else a fragment can reach
    // is [`crate::sandbox`]'s to state rather than this call site's.
    let confinement = envelope.confine(Some(tree.root()), record.network);
    let mut command = workspace.nix().generator_shell(
        workspace.root(),
        &record.runtime_inputs,
        &record.script,
        &confinement,
    );
    command
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    tree.apply(&mut command);

    // A nix that cannot be run at all is reported as the generator exiting 127,
    // which is what the shell runtime records for it: there the failure is the
    // subshell's status, and 127 is what a shell exits with for a command it
    // could not find.
    let failed = |status: i32| Error::GeneratorFailed {
        generator: generator.to_owned(),
        status,
    };

    // Held across the script for the reason `set` holds it across sops, and for
    // one more that is specific to here. The shared reason is ordering: bash
    // runs a trap between commands, so an interruption is acted on once the
    // foreground child has been waited on, and a sweep from the handler thread
    // borrows that discipline rather than having one of its own. The reason
    // specific to here is that the thing the sweep would remove is the staging
    // root the script is still writing into.
    let finished = {
        let _quiet = scratch::quiet();
        command.status().map_err(|_| failed(127))?
    };

    // Before the status is interpreted, not after. A script the operator
    // interrupted is a child that died on a signal, so it has no exit code and
    // reads as a failure; reporting it as one would blame the script for the
    // operator's Ctrl-C and would end the run with 1 where 130 is the answer.
    if let Some(status) = stopped(finished) {
        return Ok(Step::Stopped(status));
    }

    let status = finished.code().unwrap_or(1);
    if status != 0 {
        return Err(failed(status));
    }

    Ok(Step::Done(tree.collect(generator, outputs)?))
}

/// The status a run is to end with, when this child's end means it was
/// interrupted.
///
/// Two readings, because there are two ways the interruption arrives and only
/// one of them is a race this process can win.
///
/// The first is [`scratch::interrupted`], which the signal handler sets. It is
/// the only reading available when the child *finished normally* after the
/// runtime was signalled — the shape the sops abort drill has, where the child
/// is signalled by nothing and exits zero.
///
/// The second is the child's own termination signal, and it is what makes the
/// answer deterministic. A keyboard interrupt reaches the whole process group,
/// so the child dies at the same instant the runtime is signalled; the handler
/// runs on another thread, and `wait` can return before that thread has been
/// scheduled at all. Reading the flag alone therefore answers "not interrupted"
/// on a run that plainly was, and reports the operator's Ctrl-C as the generator
/// failing — which is a race this lost often enough to be found.
///
/// Only `SIGINT` and `SIGTERM` are read this way. A child killed by `SIGSEGV`
/// or `SIGKILL` failed, and calling that an interruption would hide a crashing
/// generator behind an exit code that says the operator asked to stop.
///
/// The flag is set from here as well, so the rest of the run and the sweep agree
/// with what this concluded however the conclusion was reached.
fn stopped(finished: std::process::ExitStatus) -> Option<i32> {
    /// `SIGINT`, as the kernel numbers it.
    const INTERRUPT: i32 = 2;
    /// `SIGTERM`.
    const TERMINATE: i32 = 15;

    use std::os::unix::process::ExitStatusExt as _;

    if let Some(status) = scratch::interrupted() {
        return Some(status);
    }
    let status = match finished.signal()? {
        INTERRUPT => 130,
        TERMINATE => 143,
        _ => return None,
    };
    scratch::interrupt(status);
    Some(status)
}

/// The entry's validation fragment, judging one candidate value.
///
/// `$out_name` names which output is being judged, so one fragment can cover a
/// generator that writes several. The same shell and the same `runtimeInputs` as
/// the script, because a validation that could not run the tool that produced the
/// value could check almost nothing about it. The same envelope too, under the
/// same generator's grant — a validation verifying a minted token against the
/// API that issued it has the same need its script had — with one difference it
/// does not choose: the staging root has already been shredded by the time a
/// candidate is judged, so this fragment has no writable path on the host at
/// all. The candidate itself arrives on standard input, which is a pipe and
/// crosses the envelope as one.
fn validate(
    workspace: &Workspace,
    record: &Generator,
    validation: &str,
    generator: &str,
    output: &str,
    value: &Secret,
    envelope: Envelope,
) -> Result<Step<()>> {
    let confinement = envelope.confine(None, record.network);
    let mut command = workspace.nix().generator_shell(
        workspace.root(),
        &record.runtime_inputs,
        validation,
        &confinement,
    );
    command
        .env("out_name", output)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let rejected = || Error::ValidationRejected {
        generator: generator.to_owned(),
        output: output.to_owned(),
    };

    let mut child = command.spawn().map_err(|_| rejected())?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = value.write_to(&mut stdin);
    }

    let finished = {
        let _quiet = scratch::quiet();
        child.wait().map_err(|_| rejected())?
    };

    // The same reading as in `mint`: an interrupted validation is a child that
    // died on a signal, and calling that a rejected candidate would tell the
    // operator their value was judged and found wanting when nothing judged it.
    if let Some(status) = stopped(finished) {
        return Ok(Step::Stopped(status));
    }

    if finished.success() {
        Ok(Step::Done(()))
    } else {
        Err(rejected())
    }
}

/// Stage every output into a candidate beside its target, judge the recipients,
/// then move them all into place and commit once.
///
/// Public outputs take the same staging discipline as encrypted ones and are
/// renamed in the same pass, so a generator writing a private half and a public
/// half lands both or neither. What they do not take is sops: no creation rule
/// covers the public store, so an encrypted public output would be a document
/// nothing has a rule for — which is exactly what `safix-public-no-rule` checks
/// stays impossible.
///
/// The definition records ride the same staging and the same commit, for the
/// reason they exist: a record left behind by a run that did not commit its value
/// would assert a mint this repository never made, and a value committed without
/// its record would be one `check` has nothing to say about until the next
/// rotation.
fn write(
    workspace: &Workspace,
    progress: &dyn Progress,
    message: &str,
    distinct: &[String],
    targets: &[Target],
    values: &[Secret],
    records: &Records,
) -> Result<Outcome> {
    let mut candidates = Vec::with_capacity(distinct.len());
    for relative in distinct {
        let absolute = workspace.absolute(relative);
        let candidate = set::candidate_path(&absolute);
        scratch::register_file(&candidate);

        let written = targets.iter().zip(values.iter()).filter(|(target, _)| {
            let Target::Secret { file, .. } = target else {
                return false;
            };
            file == relative
        });

        if let Some((_, value)) = targets
            .iter()
            .zip(values.iter())
            .find(|(target, _)| matches!(target, Target::Public { file } if file == relative))
        {
            public::stage(&candidate, value)?;
            candidates.push(candidate);
            continue;
        }

        if absolute.exists() {
            std::fs::copy(&absolute, &candidate).map_err(|cause| Error::FileUnwritable {
                path: candidate.display().to_string(),
                cause,
            })?;
        } else {
            if let Some(directory) = absolute.parent()
                && !directory.is_dir()
            {
                scratch::register_dir(directory);
                std::fs::create_dir_all(directory).map_err(|cause| Error::FileUnwritable {
                    path: directory.display().to_string(),
                    cause,
                })?;
            }
            let first = targets
                .iter()
                .find_map(|target| match target {
                    Target::Secret { file, key } if file == relative => Some(key.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            note(
                progress,
                &format!(
                    "{relative} does not exist yet; creating it through sops so the creation rules apply."
                ),
            );
            let _quiet = scratch::quiet();
            workspace.sops().create_empty_document(
                workspace.root(),
                relative,
                &first,
                &candidate,
            )?;
        }

        for (target, value) in written {
            let Target::Secret { key, .. } = target else {
                continue;
            };
            let status = {
                let _quiet = scratch::quiet();
                workspace.sops().set_key(&candidate, key, value)?
            };
            if status != 0 {
                return Ok(Outcome::Refused(status));
            }
            if let Some(status) = scratch::interrupted() {
                return Ok(Outcome::Refused(status));
            }
        }

        // Once per file rather than once per key: recipients are a property of
        // the file, and the document judged is the one holding every key this run
        // writes, so the assertion covers the bytes that are about to land.
        set::refuse_recipient_drift(workspace, relative, &candidate)?;
        candidates.push(candidate);
    }

    let staged_records = stage_records(workspace, records)?;

    for (candidate, relative) in candidates
        .iter()
        .zip(distinct.iter())
        .chain(staged_records.iter().zip(records.paths.iter()))
    {
        let absolute = workspace.absolute(relative);
        std::fs::rename(candidate, &absolute).map_err(|cause| Error::FileUnwritable {
            path: absolute.display().to_string(),
            cause,
        })?;
    }
    scratch::keep_dirs();

    let mut committed: Vec<String> = distinct.to_vec();
    committed.extend(records.paths.iter().cloned());

    git::commit_written_files(
        workspace.git(),
        workspace.root(),
        progress,
        message,
        &committed,
    )?;
    Ok(Outcome::Ran)
}

/// Write each definition record to a candidate beside its target.
///
/// A candidate and a rename rather than a write in place, which is the discipline
/// every other write here takes and is what makes an interrupted run leave no
/// record: the candidate is registered before it exists, so the sweep removes it
/// however the run ends, and the file at the real path is only ever a complete
/// one.
///
/// The candidate carries no `.yaml` suffix, unlike [`set::candidate_path`]'s.
/// That suffix is there because sops reads a document's format off the extension,
/// and nothing here goes through sops: a record is one line of plaintext this
/// module writes and [`crate::check`] reads.
fn stage_records(workspace: &Workspace, records: &Records) -> Result<Vec<PathBuf>> {
    let mut staged = Vec::with_capacity(records.paths.len());
    for relative in &records.paths {
        let absolute = workspace.absolute(relative);
        let mut name = absolute.as_os_str().to_owned();
        name.push(format!(".safix-tmp.{}", std::process::id()));
        let candidate = PathBuf::from(name);
        scratch::register_file(&candidate);

        if let Some(directory) = absolute.parent()
            && !directory.is_dir()
        {
            scratch::register_dir(directory);
            std::fs::create_dir_all(directory).map_err(|cause| Error::FileUnwritable {
                path: directory.display().to_string(),
                cause,
            })?;
        }

        std::fs::write(&candidate, &records.line).map_err(|cause| Error::FileUnwritable {
            path: candidate.display().to_string(),
            cause,
        })?;
        staged.push(candidate);
    }
    Ok(staged)
}
