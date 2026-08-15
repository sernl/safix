//! Minting values from the generators the declarations attach to them.
//!
//! The order is the resolver's. `flake.safix.lib.generatorPlan` computes a
//! topological order over one user's generators and refuses a cycle, so an order
//! existing at all is that refusal's postcondition; this module walks it and
//! re-derives nothing.
//!
//! # One generator at a time
//!
//! The walk is sequential, and that is a property rather than an omission.
//! Three things depend on it. A prompt is read from one standard input, and two
//! generators prompting at once is not a faster question but an unanswerable
//! one. Each generator commits as it goes, so the order of the commits is the
//! order of the plan and not of the scheduler. And a generator's inputs reach it
//! on inherited descriptors, which means any process spawned between the
//! handover and the exec would inherit them too — see [`crate::inputs`]. A
//! fan-out over independent branches would buy latency and give up the third,
//! which is the isolation the whole descriptor discipline exists for.
//!
//! # What a run leaves when it stops
//!
//! Nothing, up to the generator it stopped in. Each generator's outputs are
//! staged into candidate documents beside their targets and renamed into place
//! together, so a run that refuses partway through one generator leaves that
//! generator's files as it found them. Generators that already committed stay
//! committed, which is what the cascade confirmation warns about before it
//! starts rather than after.

use std::process::Stdio;

use crate::error::{Error, Result};
use crate::inputs::Inputs;
use crate::model::{Generator, PromptKind};
use crate::progress::{Progress, log, note};
use crate::secret::Secret;
use crate::sops::document;
use crate::workspace::Workspace;
use crate::{git, scratch, set};

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
}

/// What running one generator came to.
enum Outcome {
    /// It ran, wrote its outputs and committed them.
    Ran,
    /// Every output already held a value and no rotation was asked for.
    Skipped,
    /// `sops` refused, with this status, before anything was renamed.
    Refused(i32),
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

    let mut ran = 0_usize;
    for generator in &order {
        match run_one(workspace, progress, interaction, user, generator, options)? {
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

/// One generator: its inputs, its run, its outputs, and one commit.
fn run_one(
    workspace: &Workspace,
    progress: &dyn Progress,
    interaction: &mut dyn Interaction,
    user: &str,
    generator: &str,
    options: Options,
) -> Result<Outcome> {
    let plan = workspace.generator_plan()?;
    let mine = plan.for_user(user).ok_or_else(|| Error::NoGenerator {
        user: user.to_owned(),
        name: generator.to_owned(),
    })?;
    let outputs = mine.outputs.get(generator).cloned().unwrap_or_default();

    let mut files = Vec::with_capacity(outputs.len());
    let mut keys = Vec::with_capacity(outputs.len());
    let mut missing = 0_usize;
    for output in &outputs {
        let placement = workspace.resolve(user, output)?;
        if !holds_a_value(workspace, &placement.file, &placement.key)? {
            missing = missing.saturating_add(1);
        }
        files.push(placement.file.clone());
        keys.push(placement.key.clone());
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

    // Distinct files first, because the preflight and the write both work per
    // file and two outputs of one generator share a file whenever they share an
    // audience.
    let mut distinct: Vec<String> = Vec::new();
    for file in &files {
        if !distinct.contains(file) {
            distinct.push(file.clone());
        }
    }
    for file in &distinct {
        set::refuse_bad_repository_state(workspace, file)?;
    }

    let record = workspace
        .resolve(user, generator)?
        .generator
        .as_ref()
        .ok_or_else(|| Error::NoGenerator {
            user: user.to_owned(),
            name: generator.to_owned(),
        })?;

    let names = outputs.join(", ");
    log(progress, &format!("safix: generating {names} for {user}"));

    let produced = mint(workspace, interaction, user, generator, record)?;
    let values = split(generator, &outputs, produced)?;

    for (output, value) in outputs.iter().zip(values.iter()) {
        if value.is_empty() {
            return Err(Error::GeneratorProducedNothing {
                generator: generator.to_owned(),
                output: output.clone(),
            });
        }
        if let Some(validation) = record.validation.as_deref().filter(|text| !text.is_empty()) {
            validate(workspace, record, validation, generator, output, value)?;
        }
    }

    write(
        workspace,
        progress,
        &format!("chore(safix): generate {names} for {user}"),
        &distinct,
        &files,
        &keys,
        &values,
    )
}

/// Whether a name already holds a value, answered off the ciphertext.
///
/// `check` asks this about people whose files it cannot decrypt, so it may not
/// decrypt to find out; `generate` asks it to decide whether a run is a mint or a
/// rotation, and asking the same way keeps the two from disagreeing about one
/// file.
fn holds_a_value(workspace: &Workspace, relative: &str, key: &str) -> Result<bool> {
    let Some(text) = workspace.read_relative(relative)? else {
        return Ok(false);
    };
    Ok(document::keys_of(&text)?
        .get(key)
        .is_some_and(|state| !state.empty))
}

/// Open every input, run the script, and take what it printed.
fn mint(
    workspace: &Workspace,
    interaction: &mut dyn Interaction,
    user: &str,
    generator: &str,
    record: &Generator,
) -> Result<Secret> {
    let plan = workspace.generator_plan()?;
    let declared = plan
        .for_user(user)
        .and_then(|mine| mine.inputs.get(generator));

    let mut inputs = Inputs::new();
    for (identifier, input) in declared.into_iter().flatten() {
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
                inputs.add_prompt(identifier, answer)?;
            }
            crate::model::InputKind::Dependency => {
                let placement = workspace.resolve(user, &input.name)?;
                let absolute = workspace.absolute(&placement.file);
                inputs.add_dependency(
                    identifier,
                    workspace.sops(),
                    &placement.file,
                    &absolute,
                    &placement.key,
                )?;
            }
        }
    }

    // Standard input is `/dev/null`, and that is part of the interface rather
    // than tidiness. A generator's inputs are its descriptors; this command's
    // own standard input is where an operator's prompt answers arrive, and a
    // script that read it would eat the answers to every prompt after it —
    // silently, since a prompt reading end-of-input looks exactly like one
    // nobody answered.
    let mut command =
        workspace
            .nix()
            .generator_shell(workspace.root(), &record.runtime_inputs, &record.script);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    inputs.apply(&mut command);

    // A nix that cannot be run at all is reported as the generator exiting 127,
    // which is what the shell runtime records for it: there the failure is the
    // subshell's status, and 127 is what a shell exits with for a command it
    // could not find.
    let mut child = command.spawn().map_err(|_| Error::GeneratorFailed {
        generator: generator.to_owned(),
        status: 127,
    })?;
    inputs.release();

    let produced = {
        let mut stdout = child.stdout.take().ok_or(Error::SopsPipeMissing)?;
        Secret::read_from(&mut stdout)?
    };
    let status = child
        .wait()
        .map_err(|_| Error::GeneratorFailed {
            generator: generator.to_owned(),
            status: 127,
        })?
        .code()
        .unwrap_or(1);
    inputs.finish();

    if status != 0 {
        return Err(Error::GeneratorFailed {
            generator: generator.to_owned(),
            status,
        });
    }
    Ok(produced)
}

/// One output takes the script's standard output; several take a JSON object.
///
/// No newline comes off a value read out of the JSON form. JSON states a string
/// exactly, so there is no echo-shaped artifact to remove and removing one would
/// corrupt a value that meant it.
fn split(generator: &str, outputs: &[String], produced: Secret) -> Result<Vec<Secret>> {
    if outputs.len() == 1 {
        return Ok(vec![produced.without_echoed_newline()]);
    }

    let not_an_object = || Error::GeneratorNotAnObject {
        generator: generator.to_owned(),
        outputs: outputs.len(),
    };
    let mut members = produced
        .json_members()
        .map_err(|_| not_an_object())?
        .ok_or_else(not_an_object)?;

    let mut sorted = outputs.to_vec();
    sorted.sort();
    let declared = render(&sorted);
    let actual = render(&members.keys().cloned().collect::<Vec<_>>());
    if declared != actual {
        return Err(Error::GeneratorKeysDiffer {
            generator: generator.to_owned(),
            actual,
            declared,
        });
    }

    outputs
        .iter()
        .map(|output| {
            members
                .remove(output)
                .ok_or_else(|| Error::GeneratorKeysDiffer {
                    generator: generator.to_owned(),
                    actual: actual.clone(),
                    declared: declared.clone(),
                })
        })
        .collect()
}

/// A list of names as `jq -c` renders it, which is how the refusal quotes both
/// sides.
fn render(names: &[String]) -> String {
    serde_json::to_string(names).unwrap_or_else(|_| String::from("[]"))
}

/// The entry's validation fragment, judging one candidate value.
///
/// `$out_name` names which output is being judged, so one fragment can cover a
/// generator that writes several. The same shell and the same `runtimeInputs` as
/// the script, because a validation that could not run the tool that produced the
/// value could check almost nothing about it.
fn validate(
    workspace: &Workspace,
    record: &Generator,
    validation: &str,
    generator: &str,
    output: &str,
    value: &Secret,
) -> Result<()> {
    let mut command =
        workspace
            .nix()
            .generator_shell(workspace.root(), &record.runtime_inputs, validation);
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
    if child.wait().map_err(|_| rejected())?.success() {
        Ok(())
    } else {
        Err(rejected())
    }
}

/// Stage every output into a candidate beside its target, judge the recipients,
/// then move them all into place and commit once.
fn write(
    workspace: &Workspace,
    progress: &dyn Progress,
    message: &str,
    distinct: &[String],
    files: &[String],
    keys: &[String],
    values: &[Secret],
) -> Result<Outcome> {
    let mut candidates = Vec::with_capacity(distinct.len());
    for relative in distinct {
        let absolute = workspace.absolute(relative);
        let candidate = set::candidate_path(&absolute);
        scratch::register_file(&candidate);

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
            let first = files
                .iter()
                .zip(keys.iter())
                .find(|(file, _)| *file == relative)
                .map(|(_, key)| key.clone())
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

        for ((file, key), value) in files.iter().zip(keys.iter()).zip(values.iter()) {
            if file != relative {
                continue;
            }
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

    for (candidate, relative) in candidates.iter().zip(distinct.iter()) {
        let absolute = workspace.absolute(relative);
        std::fs::rename(candidate, &absolute).map_err(|cause| Error::FileUnwritable {
            path: absolute.display().to_string(),
            cause,
        })?;
    }
    scratch::keep_dirs();

    git::commit_written_files(
        workspace.git(),
        workspace.root(),
        progress,
        message,
        distinct,
    )?;
    Ok(Outcome::Ran)
}
