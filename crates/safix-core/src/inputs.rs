//! How a generator's inputs reach its script.
//!
//! Each prompt and each dependency reaches the script as `$in_<identifier>`,
//! holding the path of a read-only file descriptor this process opened and the
//! script inherits. The identifier is the resolver's — `-` mapped to `_`, so it
//! is a spellable shell name — and two inputs colliding under that mapping are
//! refused at evaluation, so the script's name space is injective.
//!
//! # Why a descriptor and not a directory of files
//!
//! The directory shape needs `TMPDIR` to be memory-backed to be equivalent, and
//! on a machine where it is not, plaintext written there is plaintext on a disk,
//! surviving in free blocks after any unlink. So the value goes down a pipe and
//! is never a file at all. The consequence to know when writing a generator: a
//! pipe is read once, so `cat "$in_x"` twice gives the value and then nothing.
//!
//! # How a descriptor comes to be inherited
//!
//! Everything this process opens is close-on-exec, which is the right default
//! and the reason a descriptor has to be handed over deliberately. The flag is
//! cleared on the read end alone, immediately before the generator is spawned,
//! and the parent's own copy is dropped immediately after — see
//! [`Inputs::release`] for what that ordering buys.
//!
//! The window between clearing the flag and the spawn is a window in which any
//! other process this program started would inherit the same descriptor. The
//! generator graph is walked one generator at a time for that reason among
//! others, and it is why nothing on this path fans out.

use std::collections::BTreeMap;
use std::fs::File;

use std::os::fd::{AsRawFd as _, OwnedFd};
use std::path::Path;
use std::process::{Child, Command};
use std::thread::JoinHandle;

use rustix::io::{FdFlags, fcntl_setfd};
use rustix::pipe::{PipeFlags, pipe_with};

use crate::error::{Error, Result};
use crate::secret::Secret;
use crate::sops::Sops;

/// The descriptors one generator run was given, and what is feeding them.
///
/// Dropping this closes every descriptor the parent still holds and lets each
/// feeding thread and subprocess end. That is the whole of the isolation
/// property: a descriptor surviving into a later generator's process would be
/// that generator holding plaintext it never declared.
#[derive(Default)]
pub struct Inputs {
    held: Vec<OwnedFd>,
    writers: Vec<JoinHandle<()>>,
    producers: Vec<Child>,
    environment: BTreeMap<String, String>,
}

impl std::fmt::Debug for Inputs {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Inputs")
            .field("identifiers", &self.environment.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl Inputs {
    /// No inputs yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An answered prompt, on a descriptor of its own.
    ///
    /// The value is written by a thread rather than before the spawn, because a
    /// value longer than the pipe's buffer would otherwise block this process
    /// against a reader that has not started. The thread owns the value and
    /// drops it when it is done, so it is zeroed whether the generator read it
    /// or not.
    ///
    /// # Errors
    ///
    /// [`Error::GeneratorPipe`] when a pipe cannot be made or handed over.
    pub fn add_prompt(&mut self, identifier: &str, value: Secret) -> Result<()> {
        let (reader, writer) =
            pipe_with(PipeFlags::CLOEXEC).map_err(|cause| Error::GeneratorPipe {
                identifier: identifier.to_owned(),
                cause: cause.to_string(),
            })?;

        self.writers.push(std::thread::spawn(move || {
            let mut sink = File::from(writer);
            // A generator that never reads its input leaves this writing into a
            // pipe nobody will drain; the failure that ends it is the reader
            // going away, which is not a failure of the run.
            let _ = value.write_to(&mut sink);
        }));

        self.hand_over(identifier, reader)
    }

    /// Another secret's plaintext, on a descriptor of its own.
    ///
    /// The producing `sops` writes straight into the pipe the generator reads,
    /// so the value is never a file and is never this process's to hold.
    ///
    /// # Errors
    ///
    /// [`Error::DependencyHasNoValue`] when the file the dependency is placed in
    /// does not exist, [`Error::SopsUnavailable`] when sops cannot be run, and
    /// [`Error::GeneratorPipe`] when the descriptor cannot be handed over.
    pub fn add_dependency(
        &mut self,
        identifier: &str,
        sops: &Sops,
        relative: &str,
        absolute: &Path,
        key: &str,
    ) -> Result<()> {
        if !absolute.exists() {
            return Err(Error::DependencyHasNoValue {
                identifier: identifier.to_owned(),
                file: relative.to_owned(),
            });
        }

        let mut child = sops.decrypt_key_streaming(absolute, key)?;
        let stdout = child.stdout.take().ok_or(Error::SopsPipeMissing)?;
        self.producers.push(child);
        self.hand_over(identifier, OwnedFd::from(stdout))
    }

    /// Name a descriptor in the environment the script will read, and clear the
    /// flag that would otherwise close it across the exec.
    fn hand_over(&mut self, identifier: &str, reader: OwnedFd) -> Result<()> {
        fcntl_setfd(&reader, FdFlags::empty()).map_err(|cause| Error::GeneratorPipe {
            identifier: identifier.to_owned(),
            cause: cause.to_string(),
        })?;
        self.environment.insert(
            format!("in_{identifier}"),
            format!("/dev/fd/{}", reader.as_raw_fd()),
        );
        self.held.push(reader);
        Ok(())
    }

    /// Name every descriptor in the command's environment.
    pub fn apply(&self, command: &mut Command) {
        for (name, path) in &self.environment {
            command.env(name, path);
        }
    }

    /// Drop the parent's own copy of every descriptor, once the generator holds
    /// its own.
    ///
    /// Called immediately after the spawn and not later, and that ordering is
    /// what prevents a deadlock rather than tidiness. A generator that never
    /// reads a dependency leaves the producing `sops` blocked on a full pipe;
    /// with the parent still holding a read end, no reader would ever close and
    /// sops would block for as long as the run does. Dropping here means the
    /// generator's exit closes the last read end, sops fails on the write, and
    /// the run ends.
    pub fn release(&mut self) {
        self.held.clear();
    }

    /// Let every feeder end, and reap it.
    ///
    /// A producing `sops` that failed is not reported here: its own standard
    /// error is inherited and has already said why, and the generator reading an
    /// empty input is what makes the run fail with the generator's own status —
    /// which is the failure worth reporting, because it names the script.
    pub fn finish(&mut self) {
        self.held.clear();
        for mut producer in std::mem::take(&mut self.producers) {
            let _ = producer.wait();
        }
        for writer in std::mem::take(&mut self.writers) {
            let _ = writer.join();
        }
    }
}

impl Drop for Inputs {
    fn drop(&mut self) {
        self.finish();
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::process::Stdio;

    use super::Inputs;
    use crate::secret::Secret;

    fn value(bytes: &[u8]) -> Secret {
        Secret::read_from(&mut Cursor::new(bytes.to_vec())).expect("a cursor can be read")
    }

    #[test]
    fn a_prompt_reaches_a_child_as_a_descriptor_it_can_read_once() {
        let mut inputs = Inputs::new();
        inputs
            .add_prompt("seed", value(b"a fixture value"))
            .expect("a pipe can be made");

        let mut command = std::process::Command::new("sh");
        command
            .arg("-c")
            .arg(r#"cat "$in_seed"; printf ' | '; cat "$in_seed""#)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        inputs.apply(&mut command);

        let child = command.spawn().expect("sh is on PATH");
        inputs.release();
        let produced = child
            .wait_with_output()
            .expect("the child can be waited on");
        inputs.finish();

        assert_eq!(
            String::from_utf8_lossy(&produced.stdout),
            "a fixture value | "
        );
    }

    #[test]
    fn a_descriptor_is_not_inherited_by_a_child_spawned_after_the_release() {
        let mut inputs = Inputs::new();
        inputs
            .add_prompt("seed", value(b"a fixture value"))
            .expect("a pipe can be made");
        let path = inputs
            .environment
            .get("in_seed")
            .expect("the identifier is named")
            .clone();

        let mut first = std::process::Command::new("sh");
        first
            .arg("-c")
            .arg("exit 0")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        inputs.apply(&mut first);
        let mut running = first.spawn().expect("sh is on PATH");
        inputs.release();
        let _ = running.wait();
        inputs.finish();

        let later = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("cat {path}"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .expect("sh is on PATH");
        assert!(
            !later.status.success(),
            "a later child could still open the released descriptor"
        );
    }
}
