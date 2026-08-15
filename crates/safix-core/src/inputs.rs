//! How a generator's inputs and outputs reach its script.
//!
//! This is clan's contract, adopted so that a generator written for either
//! system runs under the other. It was read off
//! `pkgs/clan-cli/clan_lib/vars/generator.py` rather than off clan's
//! documentation, and the places where clan's behaviour is surprising are
//! matched and recorded here rather than corrected.
//!
//! One run gets one staging root, and inside it:
//!
//! ```text
//! <root>/in/<producing generator>/<output name>   a dependency's plaintext
//! <root>/prompts/<prompt name>                    one answered prompt
//! <root>/out/<output name>                        what the script writes
//! ```
//!
//! `$in` and `$out` name the first and third; `$prompts` names the second and
//! is set only when the generator declares prompts. The script's working
//! directory is the root. All three are what clan sets, so `$out/publickey` and
//! `$in/openssh-ca/id_ed25519` mean here what they mean there.
//!
//! # What is not clan's, and why
//!
//! *Only declared dependencies are materialized.* clan's dependency edge names
//! a generator and materializes every file that generator writes, so a script
//! depending on a keypair's public half is handed the private half as well.
//! safix's edge names an entry, and materializing an entry's siblings would hand
//! a script plaintext it never declared — the property the whole isolation
//! discipline exists for. The directory is still keyed by the *producing
//! generator*, which is what makes the path spelling clan's; what differs is
//! which files appear under it.
//!
//! *`$prompts` is removed from the environment when no prompts are declared.*
//! clan copies the ambient environment and sets `prompts` only when there are
//! any, so an ambient `$prompts` survives into the script. Here it is cleared,
//! so a script cannot distinguish "none declared" from "directory missing" by
//! reading a variable somebody else set.
//!
//! *`$in` and `$out` are created mode `0700` rather than at the umask.* clan
//! makes the generator's directories with the process umask in force, so on a
//! host whose umask is the common `0022` they are world-readable — the files
//! inside are not, and clan's containment rests on that. Here both directories
//! are `0700` as well, and the files inside stay `0600`.
//!
//! This is defence in depth rather than a correction: the mode on a directory
//! does not protect a file whose own mode already refuses, and no attack on
//! clan's arrangement is claimed here. What it buys is that the containment
//! stops resting on one mode. A future output written by a path that got its
//! permissions wrong, a script that creates a file of its own beside the ones
//! safix placed, and anything that makes a file inside more permissive than
//! intended are all still behind a directory nobody else can enter. The whole
//! tree already sits inside a `0700` staging root — see [`crate::staging`] — so
//! this costs one flag per directory and removes a single point of failure
//! rather than adding a barrier.
//!
//! # What the earlier interface bought and this one does not
//!
//! Until 0.2 every input reached the script as `$in_<name>`, the path of a
//! read-once file descriptor, and no plaintext was ever a file. That absolute
//! cannot survive a contract whose inputs and outputs are addressed by path.
//! What stands in its place is [`crate::staging`]: a mode-`0700` directory on a
//! filesystem verified to be memory-backed, shredded on every exit path. It is
//! bounded containment rather than an absolute, and that module states what it
//! does not achieve.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Error, Result};
use crate::secret::Secret;
use crate::sops::Sops;
use crate::staging::{self, Staging};

/// The directory a dependency's outputs are placed under.
const INPUT: &str = "in";

/// The directory a generator writes its outputs into.
const OUTPUT: &str = "out";

/// The directory one answered prompt per file is placed under.
const PROMPTS: &str = "prompts";

/// One generator run's staging tree, and the guard that removes it.
///
/// Dropping this shreds every file under the root and removes it, whether the
/// drop comes from a return, an error, or a panic unwinding through it.
#[derive(Debug)]
pub struct Tree {
    staging: Staging,
    prompts: Option<PathBuf>,
}

impl Tree {
    /// Establish the tree for one generator run.
    ///
    /// `in` and `out` always exist; `prompts` exists only when the generator
    /// declares any. That is clan's behaviour and is copied deliberately: a
    /// script cannot then distinguish "no prompts declared" from "the prompts
    /// directory is missing", so no script can come to rely on the difference.
    ///
    /// # Errors
    ///
    /// [`Error::StagingNotMemoryBacked`] when no memory-backed mount is
    /// available and the acknowledgement was not given, and
    /// [`Error::StagingUnusable`] when the tree cannot be created.
    pub fn establish(allow_disk_staging: bool, declares_prompts: bool) -> Result<Self> {
        let staging = Staging::establish(allow_disk_staging)?;
        staging.directory(Path::new(INPUT))?;
        staging.directory(Path::new(OUTPUT))?;
        let prompts = if declares_prompts {
            Some(staging.directory(Path::new(PROMPTS))?)
        } else {
            None
        };
        Ok(Self { staging, prompts })
    }

    /// The directory a generator writes its outputs into.
    #[must_use]
    pub fn output(&self) -> PathBuf {
        self.staging.root().join(OUTPUT)
    }

    /// One answered prompt, at `$prompts/<key>`.
    ///
    /// Written with nothing added and nothing removed, which is clan's
    /// `write_text(value)`. A newline convention on this boundary would silently
    /// corrupt a value whose last byte matters, and there is no way for the
    /// script to tell a convention from an answer.
    ///
    /// # Errors
    ///
    /// [`Error::StagingUnusable`] when the file cannot be written, and
    /// [`Error::NixSchemaMismatch`] for a prompt name that is not one path
    /// component — which the resolver's own name rule already forbids.
    pub fn add_prompt(&self, key: &str, value: &Secret) -> Result<()> {
        refuse_traversal(key)?;
        self.staging.write(&Path::new(PROMPTS).join(key), value)?;
        Ok(())
    }

    /// One dependency's plaintext, at `$in/<producer>/<name>`.
    ///
    /// The producing `sops` writes to a pipe this reads into a value that zeroes
    /// itself, which is then written to the staged file. The intermediate copy
    /// exists so that the bytes are zeroed if anything between here and the
    /// write fails; the staged file itself is what [`Tree`]'s drop shreds.
    ///
    /// # Errors
    ///
    /// [`Error::DependencyHasNoValue`] when the file the dependency is placed in
    /// does not exist, [`Error::SopsUnavailable`] when sops cannot be run, and
    /// [`Error::StagingUnusable`] when the staged file cannot be written.
    pub fn add_dependency(
        &self,
        producer: &str,
        name: &str,
        sops: &Sops,
        relative: &str,
        absolute: &Path,
        key: &str,
    ) -> Result<()> {
        refuse_traversal(producer)?;
        refuse_traversal(name)?;

        if !absolute.exists() {
            return Err(Error::DependencyHasNoValue {
                name: name.to_owned(),
                producer: producer.to_owned(),
                file: relative.to_owned(),
            });
        }

        let mut child = sops.decrypt_key_streaming(absolute, key)?;
        let value = {
            let mut stdout = child.stdout.take().ok_or(Error::SopsPipeMissing)?;
            Secret::read_from(&mut stdout)?
        };
        // A sops that failed has already said why on its own standard error, and
        // the empty value it leaves is what the script will fail on — naming the
        // script, which is the failure worth reporting.
        let _ = child.wait();

        self.staging
            .write(&Path::new(INPUT).join(producer).join(name), &value)?;
        Ok(())
    }

    /// Name the three directories in the command's environment and put its
    /// working directory at the root.
    ///
    /// `prompts` is *removed* rather than left alone when no prompts are
    /// declared. clan leaves whatever the ambient environment held, which means
    /// a script can read a `$prompts` somebody else set; removing it is the one
    /// place this executor is deliberately stricter than the one it copies.
    pub fn apply(&self, command: &mut Command) {
        let root = self.staging.root();
        command.current_dir(root);
        command.env(INPUT, root.join(INPUT));
        command.env(OUTPUT, root.join(OUTPUT));
        match &self.prompts {
            Some(path) => command.env(PROMPTS, path),
            None => command.env_remove(PROMPTS),
        };
    }

    /// Read every declared output back, refusing the run if any is absent.
    ///
    /// The presence of *all* of them is established before any is read, which is
    /// what makes a partial generator refuse having written nothing: the values
    /// reach neither sops nor the public store until the whole set is known to
    /// exist.
    ///
    /// Bytes exactly as the script wrote them. Nothing is stripped, and that is a
    /// change from 0.1, where one trailing newline came off a single-output
    /// value so that an `echo`-shaped one-liner stored what it looked like it
    /// stored. Under this contract the file *is* the value — it is what clan
    /// reads with `read_bytes()` — and a convention that removed a byte would
    /// corrupt every key whose last byte is a newline while looking like it had
    /// tidied one up. A generator that wants no trailing newline writes with
    /// `printf` rather than `echo`.
    ///
    /// # Errors
    ///
    /// [`Error::GeneratorOutputMissing`] naming the first absent output and
    /// listing what the directory did hold, and whatever reading a present one
    /// failed with.
    pub fn collect(&self, generator: &str, outputs: &[String]) -> Result<Vec<Secret>> {
        let directory = self.output();

        let mut paths = Vec::with_capacity(outputs.len());
        for output in outputs {
            refuse_traversal(output)?;
            let path = directory.join(output);
            if !staging::is_output(&path) {
                return Err(Error::GeneratorOutputMissing {
                    generator: generator.to_owned(),
                    output: output.clone(),
                    produced: staging::names_in(&directory),
                });
            }
            paths.push(path);
        }

        paths
            .iter()
            .map(|path| self.staging.read(path))
            .collect::<Result<Vec<Secret>>>()
    }
}

/// Refuse a name that would reach outside the staging root.
///
/// Every name arriving here comes from the resolver, which admits
/// `[a-z0-9][a-z0-9_-]*` and refuses everything else, so this cannot fire on a
/// declared name. It is here because "cannot fire" is a claim about a rule two
/// files away, and this is the one place where a name carrying `/` or `..`
/// would write plaintext outside the directory that gets shredded.
fn refuse_traversal(name: &str) -> Result<()> {
    if staging::is_one_component(name) {
        return Ok(());
    }
    Err(Error::NixSchemaMismatch {
        attribute: "flake.safix.lib.generatorPlan",
        cause: format!("'{name}' is not a single path component, so it cannot name a staged file"),
    })
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::process::Stdio;

    use super::*;

    fn value(bytes: &[u8]) -> Secret {
        Secret::read_from(&mut Cursor::new(bytes.to_vec())).unwrap_or_else(|_| {
            unreachable!("a cursor over a literal reads to its end");
        })
    }

    /// A shell fragment, run the way the executor runs one, without nix in the
    /// way. What is under test is the tree and the environment, not the shell.
    fn run(tree: &Tree, script: &str) -> std::process::Output {
        let mut command = Command::new("bash");
        command
            .arg("-euo")
            .arg("pipefail")
            .arg("-c")
            .arg(script)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        tree.apply(&mut command);
        command.output().unwrap_or_else(|_| {
            unreachable!("bash is on PATH wherever these tests run");
        })
    }

    #[test]
    fn a_prompt_is_a_file_the_script_may_read_twice() {
        let _exclusive = crate::scratch::exclusive();
        let Ok(tree) = Tree::establish(false, true) else {
            return;
        };
        let Ok(()) = tree.add_prompt("seed", &value(b"a fixture value")) else {
            return;
        };

        let produced = run(
            &tree,
            r#"cat "$prompts/seed"; printf ' | '; cat "$prompts/seed""#,
        );
        assert_eq!(
            String::from_utf8_lossy(&produced.stdout),
            "a fixture value | a fixture value",
            "a prompt was not re-readable, which the descriptor interface it replaced was not"
        );
    }

    #[test]
    fn a_prompts_directory_is_absent_and_unnamed_when_none_are_declared() {
        let _exclusive = crate::scratch::exclusive();
        let Ok(tree) = Tree::establish(false, false) else {
            return;
        };
        let produced = run(&tree, r#"printf '%s' "${prompts-unset}""#);
        assert_eq!(String::from_utf8_lossy(&produced.stdout), "unset");
        assert!(!tree.staging.root().join(PROMPTS).exists());
    }

    #[test]
    fn an_ambient_prompts_variable_does_not_reach_a_generator_that_declares_none() {
        let _exclusive = crate::scratch::exclusive();
        let Ok(tree) = Tree::establish(false, false) else {
            return;
        };
        let mut command = Command::new("bash");
        command
            .arg("-euo")
            .arg("pipefail")
            .arg("-c")
            .arg(r#"printf '%s' "${prompts-unset}""#)
            .env("prompts", "/somewhere/else")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        tree.apply(&mut command);
        let produced = command.output().unwrap_or_else(|_| {
            unreachable!("bash is on PATH wherever these tests run");
        });
        assert_eq!(
            String::from_utf8_lossy(&produced.stdout),
            "unset",
            "an ambient $prompts reached a generator that declares none"
        );
    }

    #[test]
    fn the_working_directory_is_the_root_holding_the_three_directories() {
        let _exclusive = crate::scratch::exclusive();
        let Ok(tree) = Tree::establish(false, true) else {
            return;
        };
        let produced = run(&tree, r#"printf '%s' "$(ls | sort | tr '\n' ' ')""#);
        assert_eq!(String::from_utf8_lossy(&produced.stdout), "in out prompts ");
    }

    #[test]
    fn an_output_is_read_back_byte_for_byte_including_a_trailing_newline() {
        let _exclusive = crate::scratch::exclusive();
        let Ok(tree) = Tree::establish(false, false) else {
            return;
        };
        let produced = run(
            &tree,
            r#"echo -n unstripped > "$out/a"; echo padded > "$out/b""#,
        );
        assert!(produced.status.success());

        let Ok(values) = tree.collect("paired", &["a".to_owned(), "b".to_owned()]) else {
            unreachable!("both outputs were written");
        };
        let [first, second] = values.as_slice() else {
            unreachable!("two outputs were asked for");
        };
        assert_eq!(first.len(), "unstripped".len());
        assert_eq!(
            second.len(),
            "padded\n".len(),
            "a trailing newline was stripped, which would corrupt a key that meant it"
        );
    }

    #[test]
    fn a_missing_output_refuses_and_names_what_was_produced() {
        let _exclusive = crate::scratch::exclusive();
        let Ok(tree) = Tree::establish(false, false) else {
            return;
        };
        let produced = run(
            &tree,
            r#"printf x > "$out/written"; printf y > "$out/also""#,
        );
        assert!(produced.status.success());

        let refused = tree.collect("paired", &["written".to_owned(), "absent".to_owned()]);
        let Err(Error::GeneratorOutputMissing {
            output, produced, ..
        }) = refused
        else {
            unreachable!("a declared output was absent and the run did not refuse");
        };
        assert_eq!(output, "absent");
        assert_eq!(produced, ["also", "written"]);
    }

    #[test]
    fn a_name_reaching_outside_the_root_is_refused_before_anything_is_written() {
        let _exclusive = crate::scratch::exclusive();
        let Ok(tree) = Tree::establish(false, true) else {
            return;
        };
        assert!(tree.add_prompt("../escape", &value(b"fixture")).is_err());
        assert!(tree.collect("g", &["../escape".to_owned()]).is_err());
    }
}
