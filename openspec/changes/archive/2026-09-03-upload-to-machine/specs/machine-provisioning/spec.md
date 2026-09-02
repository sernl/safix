## Purpose

The command that gets a machine's operator-supplied host identity onto its disk before its first activation: the two ways it can be written, the honest report it gives when nothing is needed, the transport it borrows from clan, and the refusals that keep it from seeding the wrong key or overwriting a live machine's own identity.

## ADDED Requirements

### Requirement: `safix upload` targets one declared machine and refuses anything else

The command SHALL accept exactly one positional argument naming a machine declared under `flake.safix.machines`, and SHALL refuse when that name is not declared there, when it is declared but names no recipient, or when it names a person rather than a machine.

#### Scenario: An undeclared name is refused before anything else runs

- **WHEN** `safix upload` is given a name no machine declaration carries
- **THEN** it refuses, naming the machine and stating that no such machine is declared
- **AND** no network connection is attempted and no file is written

#### Scenario: A declared machine with no recipient is refused

- **WHEN** the named machine's `recipient` is null
- **THEN** the command refuses, naming the machine and stating that there is no declared recipient to check supplied identity material against
- **AND** the refusal is distinct from the undeclared-machine refusal, so an operator reading it knows the machine exists and knows what to add

#### Scenario: A person's name is refused the same way an undeclared machine is

- **WHEN** `safix upload` is given a name declared as a person and not as a machine
- **THEN** it is refused with the undeclared-machine message
- **AND** the command carries no separate code path for a person's name, so provisioning a person's own first identity is not reachable through this verb by any spelling

### Requirement: `--directory DIR` writes a pre-seed tree from identity material the operator supplies

The command SHALL accept `--identity PATH` naming a local file holding a private ed25519 host key, and with `--directory DIR` SHALL write that key and its derived public half into a tree rooted at `DIR`, at the paths and modes a fresh NixOS install's own host-key configuration reads, without contacting any network.
It SHALL mint no key of its own, and SHALL refuse before writing anything when the supplied key's derived age recipient does not equal the machine's declared recipient.

#### Scenario: A matching identity is written into the tree

- **WHEN** `safix upload <machine> --directory DIR --identity PATH` is run with a key whose derived recipient equals the machine's declared recipient
- **THEN** `DIR/etc/ssh/ssh_host_ed25519_key` is written at mode `0600` holding the supplied private key
- **AND** `DIR/etc/ssh/ssh_host_ed25519_key.pub` is written at mode `0644` holding the derived public key
- **AND** no other path under `DIR` is created

#### Scenario: `--directory` without `--identity` is refused

- **WHEN** `--directory DIR` is given with no `--identity`
- **THEN** the command refuses, naming `--identity` as required for a write and writing nothing under `DIR`

#### Scenario: Supplied identity that does not match the declared recipient is refused

- **WHEN** the key at `--identity PATH` derives to an age recipient other than the machine's declared one
- **THEN** the command refuses before creating `DIR` or any file inside it, naming both recipients
- **AND** the message states that seeding this key would not match what the machine's declared audience is already wrapped to

#### Scenario: This mode never asks the network anything

- **WHEN** `--directory` is given
- **THEN** the run makes no ssh connection and performs no host-key probe
- **AND** the honest no-op report of the remote path does not apply, because there is no target to be honest about

### Requirement: Remote mode probes the target's presented identity before writing anything

Without `--directory`, the command SHALL connect to an operator-named address and read the ed25519 host key, if any, the target currently presents, without authenticating, before performing any write.
It SHALL then take exactly one of three actions: report that nothing is needed, write the supplied identity, or refuse naming a mismatch — and SHALL NOT write in the first or third case without an explicit override.

#### Scenario: The presented key already matches — nothing is written

- **WHEN** the target presents an ed25519 host key whose age form equals the machine's declared recipient
- **THEN** the command reports that the machine already holds its declared identity and exits zero
- **AND** no ssh session that could write is opened, and no `--identity` value, even if one was given, is used

#### Scenario: No key is presented — the write proceeds

- **WHEN** the target presents no ed25519 host key
- **THEN** the command requires `--identity` and, given it, transfers the supplied private key and its derived public half onto the target
- **AND** without `--identity` it refuses, naming the flag, before opening the writing session

#### Scenario: A different key is presented — refused pending an explicit override

- **WHEN** the target presents an ed25519 host key whose age form is neither absent nor equal to the machine's declared recipient
- **THEN** the command refuses by default, naming both the presented and the declared recipient
- **AND** it proceeds only when `--force` is given together with a matching `--identity`, and states in its output that a changed host key was overridden rather than discovered absent

#### Scenario: `--force` has no effect on a match

- **WHEN** the presented key already matches the declared recipient and `--force` is also given
- **THEN** the outcome is the same as the unforced match: nothing is written and the honest report is printed
- **AND** the command states that `--force` applies only to a mismatch, because a match has nothing for it to override

### Requirement: The remote transport mirrors clan's own shape

Remote-mode writes SHALL use a tar-over-ssh transport as `root`, packing files at mode `0400` and directories at mode `0700`, wiping the destination before extracting, and refusing a destination shallower than three path components unless it starts with `/tmp/`, `/root/`, or `/etc/` at two.

#### Scenario: The wipe-then-extract sequence is used

- **WHEN** a write proceeds over ssh
- **THEN** the destination is created if absent, its existing contents are removed, and the tarball is extracted into it in one sequence
- **AND** the files inside carry mode `0400` and the directories mode `0700`, owned by root

#### Scenario: The fixed destination clears the depth safety by construction

- **WHEN** the destination used for a fresh install's pre-seed tree is inspected
- **THEN** it is at least three path components deep
- **AND** a check holds this against the same threshold the transport itself enforces, so a future change to the destination cannot silently drop below it

### Requirement: The command performs no deploy, switch, or rebuild

`safix upload` SHALL only ever write identity material to a directory or a remote target's filesystem, and SHALL NOT invoke, trigger, or wait on any activation, switch, or rebuild of the target.

#### Scenario: A successful write leaves activation to the machine's own next rebuild

- **WHEN** a write completes, locally or remotely
- **THEN** the command's own process exits without having run any activation or rebuild command
- **AND** its output states that the machine's own next rebuild is what will consume what was written

### Requirement: What this command does not cover is stated in its own help

The command's help SHALL state that it targets machines and not people, that a systemd-credentials delivery path for the same material does not exist yet, and that it triggers no deploy of its own.

#### Scenario: The three absences are readable without reading the source

- **WHEN** `safix upload -h` is read
- **THEN** it states that this command provisions machines and not people, refusing a person's name the way it refuses an undeclared machine
- **AND** it states that the material lands as plain files today and names no systemd-credentials mode
- **AND** it states that the machine's own next rebuild activates what was written, because this command does not
