# plaintext-staging Specification

## Purpose

Where plaintext is permitted to exist while a run is in progress, how the runtime establishes that the location is what it claims to be, what happens when it is not, what the removal achieves, and what it does not.

## Requirements

### Requirement: Plaintext staged during a run lives in a private directory on a memory-backed filesystem

Any plaintext the runtime materializes during generation or editing SHALL be placed inside a directory created for that run with owner-only permissions, on a memory-backed filesystem, with owner-only permissions on every file in it.

#### Scenario: The directory is private

- **WHEN** the staging root is created
- **THEN** its mode permits the owner only
- **AND** every file created inside it permits the owner only

#### Scenario: The staging root is per-run

- **WHEN** two runs stage plaintext
- **THEN** each has its own root
- **AND** neither can observe the other's

#### Scenario: Nothing plaintext is placed outside it

- **WHEN** the paths at which plaintext exists during a run are enumerated
- **THEN** every one is inside that run's staging root

### Requirement: The memory-backed filesystem is verified at run time rather than assumed

On a platform where filesystem type is observable, the runtime SHALL confirm that the mount it is about to stage into is memory-backed, and SHALL NOT infer this from the path's name or from convention.

#### Scenario: The mount is interrogated

- **WHEN** the runtime selects a staging location
- **THEN** it asks the operating system what filesystem is mounted there
- **AND** it proceeds only if the answer is a memory-backed one

#### Scenario: A conventional path that is not what it appears is rejected

- **WHEN** the conventional shared-memory path has been remounted or replaced with a disk-backed filesystem
- **THEN** the runtime does not stage into it
- **AND** the reason this check exists is recorded: the common case is not what it is for

#### Scenario: There is no disk-backed fallback path

- **WHEN** the runtime cannot find a memory-backed mount
- **THEN** it does not fall back to any other temporary directory
- **AND** the reason is recorded: this fleet's conventional temporary directory is disk-backed, so a silent fallback would be the exact failure this rule prevents, under a code path that appears to have succeeded

### Requirement: Disk-backed staging happens only under an explicit acknowledgement

When no memory-backed filesystem is available the runtime SHALL refuse, and SHALL proceed only when the operator passes a flag whose name states what is being accepted.

#### Scenario: The default is refusal

- **WHEN** no memory-backed mount is available and no acknowledgement was given
- **THEN** the run is refused before any plaintext is produced
- **AND** the refusal states that staging would be disk-backed and names the flag

#### Scenario: The acknowledgement is named for what it accepts

- **WHEN** the flag is read in the command's usage
- **THEN** its name states that staging will be disk-backed
- **AND** it does not read as a convenience or a performance option

#### Scenario: Acknowledgement does not relax the rest

- **WHEN** a run proceeds under the acknowledgement
- **THEN** the directory is still owner-only, the files are still owner-only, and the removal still runs on every exit path

### Requirement: The staging root is removed on every exit path

The staging root and everything in it SHALL be overwritten and removed on normal return, on error, on panic, and on interruption or termination by signal.

#### Scenario: The root is registered before it is created

- **WHEN** the staging root is established
- **THEN** it is registered for removal before it exists
- **AND** the reason is recorded: registering after creation opens exactly the window a signal arrives in

#### Scenario: Every exit path removes it

- **WHEN** a run ends by return, by error, by panic, by interruption, or by termination
- **THEN** the staging root is overwritten and removed

#### Scenario: Whatever the editor or script left beside the file goes too

- **WHEN** a program the runtime invoked created additional files inside the staging root
- **THEN** those are removed with the root
- **AND** the removal is of the root rather than of the file the runtime itself created

### Requirement: What the removal achieves is stated, including what it does not

The documentation of this behaviour SHALL state the residual exposures rather than describing the removal as complete.

#### Scenario: Swap is named

- **WHEN** the removal's effect is described
- **THEN** it states that overwriting a page in a memory-backed filesystem does not reach a copy of that page written to swap before the overwrite
- **AND** it states that closing that exposure is a host-level decision outside the runtime

#### Scenario: Same-user reachability is named

- **WHEN** the containment is compared against the pipe it replaces
- **THEN** it states that a directory readable by the owner is reachable by any process running as that owner for the duration of the run
- **AND** it does not describe the two as equivalent

#### Scenario: What a script or an editor does with a value is named

- **WHEN** the boundary of the runtime's responsibility is described
- **THEN** it states that a program handed a value may place it where the runtime does not look
- **AND** it names the concrete cases: a script redirecting an input elsewhere, and an editor configured to write undo history or backups outside the directory it was given
