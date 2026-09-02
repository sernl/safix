---
title: Zero-knowledge direction for the separate secrets vault
---

# Zero-knowledge direction for the separate secrets vault

Research note, 2026-09-03.
It feeds the decision on the open change `openspec/changes/separate-secrets-vault/` and builds on the 2026-09-02 finding that ciphertext in the nix store is never plaintext and that the vault change is neutral on store exposure.
Sources were read at the revisions safix pins (sops 3.13.3, sops-nix a8627b21, clan-core 56e35624, nix 2.34.8, age 1.3.1, age-plugin-yubikey 0.5.1, age Rust crate 0.12.1) or at the dates given.

## Question

Would a zero-knowledge design be useful for the separate vault, is the pattern widely used, and how could it work here.

## Summary

Zero-knowledge in the sense every vendor uses it means the party storing the data holds only ciphertext and no key.
safix already has that property against every host it touches, because sops writes AES-256-GCM payloads with data keys wrapped to age recipients and nothing else, and moving the ciphertext into a separate vault changes nothing about it.
The property that is not held is metadata opacity: the vault as designed would give its host the complete guest list (`.sops.yaml` with every person, age public key, YubiKey serial, organization anchor and free-text note), audience-named directories with kind markers, every secret's name, every document's recipient list, the plaintext public outputs and the definition digests.
The pattern is mainstream by the vendors' own numbers, but no mainstream product hides that class of metadata either: Bitwarden, 1Password, Keeper, Proton Drive, Tresorit and Filen all expose counts, tree shape, share membership and (mostly) sizes to their operator, and in the infrastructure segment the leaders are not zero-knowledge at all.
An opaque vault is feasible today with sops and sops-nix for three of four ingredients (keyed opaque file names computed at evaluation, opaque key names, creation rules supplied out of band), and impossible for the fourth (hiding recipient public keys) without leaving the sops document format.
Its guarantee is bounded: it hides names from the vault host and from anyone holding only the vault; it hides nothing from anyone who can evaluate the declaring flake, which includes every local user of a machine that has it in the store, because nix has no keyed hash and the naming key must be an evaluation-time value.
Recommendation: keep the vault change, amend it before applying so that the policy file never enters the vault and the vault layout is opaque by construction, and do not pursue recipient hiding.

## What zero-knowledge means and who uses it

The architecture sense, not zero-knowledge proofs, is the one in use.
Bitwarden: "you are the only party with access to the keys required to decrypt the vault data" (https://bitwarden.com/help/what-encryption-is-used/).
Keeper: "Encryption and decryption of data always occurs locally on the user's device" (https://docs.keeper.io/en/enterprise-guide/keeper-encryption-model).
1Password says "end-to-end encryption" and never "zero-knowledge" (https://support.1password.com/1password-security/).
Proton distinguishes zero-access (encrypted after arrival) from end-to-end (encrypted before departure) and is the only vendor claiming to hide guest lists and file names (https://proton.me/learn/encryption/types-of-encryption/zero-access).
Akeyless calls itself zero-knowledge only when the customer holds a key fragment (https://docs.akeyless.io/docs/zero-knowledge).

Adoption, from the vendors' own pages read on 2026-09-03: 1Password over 200,000 businesses and 1.3 billion credentials (https://1password.com/company); Bitwarden over 80,000 businesses and 15 million users (press boilerplate at https://bitwarden.com/about/); Keeper 93,000 business customers and 4 million people (https://www.keepersecurity.com/); Proton 100 million accounts (https://proton.me/blog/proton-100-million-accounts, 2023); Tresorit 11,000 organizations (https://tresorit.com/).
Client-side-encrypted storage formats, GitHub stars on 2026-09-02: rclone 59,511, restic 35,835, Cryptomator 16,075, Borg 13,683, git-crypt 9,888, gocryptfs 4,593, git-remote-gcrypt 991.

In infrastructure secrets the picture inverts.
Infisical made end-to-end encryption opt-out in June 2023 because "companies self-hosting Infisical cared little about E2EE" (https://infisical.com/blog/infisical-update-june-2023) and its current internals decrypt on the server (https://infisical.com/docs/internals/security).
Doppler tokenizes plaintext server-side (https://docs.doppler.com/docs/security-fact-sheet).
HashiCorp Vault holds plaintext in memory after unseal (https://developer.hashicorp.com/vault/docs/internals/security).
AWS Secrets Manager decrypts server-side and leaves secret names, descriptions and tags in clear (https://docs.aws.amazon.com/secretsmanager/latest/userguide/security-encryption.html).
Bitwarden Secrets Manager and 1Password Secrets Automation are the client-side exceptions, and both need a live server or a warmed cache at boot plus a machine-held bootstrap token that embeds decryption capability (https://bitwarden.com/help/access-tokens/, https://www.1password.dev/connect).
That is the constraint the prior session used to reject runtime fetch for a fleet that must cold-boot without a network or a human, and nothing found here changes it.
Official Rust SDKs exist only for AWS (Apache-2.0), Bitwarden (a proprietary SDK licence forbidding redistribution) and Infisical (MIT, a year stale); 1Password, Akeyless, Doppler, Vault, OpenBao and EnvKey have none.

## What safix already has and what it exposes

Ciphertext: every leaf is `ENC[AES256_GCM,...]` with the data key wrapped per age recipient (`crates/safix-core/src/sops/document.rs:36-70`); no host, store, cache or fleet member can read a value without a recipient identity.
That is the zero-knowledge property in the vendors' sense, held without a vendor and without escrow.

Metadata, by observer, today and after the vault change as written:

| Observer | Today (`root = self`, `modules/flake/safix/default.nix:47`) | After the vault as designed |
|---|---|---|
| Declaring-flake host | Declarations, `.sops.yaml`, every path and key name, per-file recipients, `public/safix/`, `state/safix/definitions/`, sizes, one commit per operation | Loses the bytes of ciphertext, policy, public and state trees; keeps the full people, keys and audience graph because the declarations are the input the policy renders from, plus the vault short id per commit |
| Vault host | not applicable | `.sops.yaml` in full, audience-named directories with kind markers, every key name, every recipient list, `public/safix/` plaintext, definition digests keyed by secret name, sizes, counts, one commit per write |
| Local user reading `/nix/store` | The whole declaring flake, store-copied | Two store-copied trees instead of one; nothing removed |
| Binary cache | Manifest derivation with every `sopsFile` path, key name and `sopsFileHash`; source trees per push policy | Same, plus the vault tree where a closure references it |
| Fleet member with one machine identity | Decrypts its own files; reads everyone's names, recipients and sizes | Unchanged in kind |

The path templates are `secrets/safix/users/<user>/secrets.yaml` and `secrets/safix/shared/<a>,<b>/secrets.yaml` (`modules/flake/safix/resolve.nix:486-491`), where audience elements carry kind markers (`@group`, `@~machine`, `%service`, `=organization`, `resolve.nix:270-311`).
`.sops.yaml` names each anchor `<user>-safix`, `<machine>-safix` and `yubikey-<serial>` with its age public key and optional free-text note (`modules/flake/safix/policy.nix:129-214`; `crates/safix-core/src/enroll/mod.rs:403-407`).
Inside a document the top-level keys are the secret names, each recipient's public key is listed under `sops.age[].recipient`, and each leaf's ciphertext length equals its plaintext length (`document.rs:49-63`, `document.rs:110-114`).

Ranked by sensitivity under the project's own rules: YubiKey serials as policy anchors; the person-to-key-to-audience graph in `.sops.yaml` with organization custody and notes; audience directory names; secret names in documents, public paths and definition records; per-document recipient lists; sizes, counts and commit cadence; machine host recipients, which anyone who can connect to the host can derive anyway.
Two of these are documented features rather than accidents: "the path states who can open the file without opening it" (`resolve.nix:63-66`; `openspec/specs/secret-custody/spec.md:33-41`) and naming cards by serial in the policy (`enroll/mod.rs:403-405`).

## What the industry hides and does not hide

| Format | Names hidden | Tree hidden | Sizes hidden | Recipients hidden | Name primitive | Mapping recovered by |
|---|---|---|---|---|---|---|
| restic | yes | yes | no, pack granularity | key-file metadata plaintext | SHA-256 of ciphertext | encrypted index |
| Borg 2 | yes | yes | no, object header | not applicable | HMAC under secret `id_key` | encrypted manifest |
| rclone crypt | yes | no | no, within 16 bytes | not applicable | AES-EME, password-derived key | deterministic keyed |
| Cryptomator 8 | yes | yes, flattened | no | not applicable | AES-SIV under masterkey | keyed names plus `dir.c9r` pointers |
| gocryptfs | yes | no | no | not applicable | AES-EME with per-directory IV | deterministic keyed |
| git-remote-gcrypt | yes | yes | no, pack granularity | yes by default (`gpg -R`) | SHA-256 of ciphertext | encrypted manifest |
| git-crypt | no | no | no | no, fingerprint is a path | none | none |

Sources: restic `doc/design.rst`, Borg `docs/internals/security.rst` and `data-structures.rst`, https://rclone.org/crypt/, https://docs.cryptomator.org/security/vault/, https://nuetzlich.net/gocryptfs/forward_mode_crypto/, git-remote-gcrypt `README.rst`, git-crypt `README.md` and `commands.cpp:692`.
Every format leaks sizes and counts; the two that flatten the tree do it with an encrypted manifest, and the three that use deterministic keyed names keep the tree shape visible.
git-crypt is the structural analogue of safix today: plaintext paths and a per-recipient key file named by the recipient, with metadata hiding disclaimed.
Deterministic keyed naming is always keyed (EME, SIV, HMAC), never an unkeyed hash, because a public salt makes small inventories dictionary-attackable.
The mainstream password managers hide item fields and names but not counts, structure or share membership: Bitwarden's whitepaper lists item counts, folder ids and organization membership as administrative data processed in clear; 1Password's security design lists group names, user names and emails, public keys and vault membership as cleartext; Tresorit's privacy policy states folder name, size and members are unencrypted; Proton Drive keeps node type, size, timestamps and membership server-visible.

## How an opaque vault could work here

Four ingredients, with what the pinned sources allow.

1. Opaque file names computed at evaluation.
`builtins.hashString "sha256"` is pure, hashes the string's raw bytes and returns lowercase hex; string context on the hashed value is discarded (nix 2.34.8 `src/libexpr/primops.cc:4593-4605`).
A `flake = false` input's `outPath` is a store-path string, `vault + "/" + name` is a string that `builtins.pathExists` and `builtins.readFile` accept in pure mode, and sops-nix's validation accepts a store-prefixed string as `sopsFile` (`sops-nix modules/sops/manifest-for.nix:11-28`).
Verified end to end on a throwaway flake.
Physical names are produced at exactly two nix functions, `audienceFileOf` and `publicFileOf` (`resolve.nix:486-491`, `557-562`), and one Rust constant (`crates/safix-core/src/definition.rs:13-14`); every other consumer takes their output.
Nix has no HMAC, so the construction is `hashString "sha256" (key + "/" + logicalPath)` with a per-vault naming key declared in the declaring flake; the key is therefore visible to anyone who can evaluate the declarations, and the opacity holds only against the vault host and vault-only readers.
2. Opaque key names inside documents.
sops accepts any top-level key except `sops` (sops `stores/stores.go:29-30`) and binds the key path into the AES-GCM associated data (`sops.go:604-611`), so an opaque key name is fixed at encrypt time.
sops-nix's `key` is a free string with `/` nesting and `name`/`path` decide `/run/secrets/<logical>` independently (`modules/sops/default.nix:56-82`); `-check-mode=sopsfile` verifies the opaque key exists in the ciphertext (`pkgs/sops-install-secrets/main.go:556-563`).
`materializeFor` already emits `key` separately from the attribute name (`resolve.nix:2244-2252`).
3. Creation rules supplied out of band.
The global `--config` flag disables discovery and may point anywhere (sops `cmd/sops/main.go:1845-1849`); `updatekeys` matches `path_regex` against the absolute document path, or the config-relative path when the document sits under the config's directory (`config/config.go:576-602`); `encrypt --age r1,r2` needs no rule at all (`main.go:2482-2508`); `decrypt` never consults rules (`main.go:1944-1949`).
clan-core writes a temporary JSON config per invocation and passes `--config` (`pkgs/clan-cli/clan_cli/secrets/sops.py:248-268`), which is production evidence that a vault needs no committed `.sops.yaml`.
The safix runtime today sets the working directory and lets sops walk upward (`crates/safix-core/src/sops/mod.rs:195-204`, `295-303`); the change is one flag on two commands.
The committed policy file stays at the declaring root, where the drift check and `check` already read it (`policy.nix:276-295`; `crates/safix-core/src/check.rs:188`).
4. Hiding recipient public keys.
Not available in the sops format: every document lists `recipient` and `enc` per age key with no `omitempty` (sops `age/keysource.go:285-290`; `stores/stores.go:106-109`) and `updatekeys` reads those groups.
Age-native files hide X25519 recipients (the stanza carries only an ephemeral share, `age.md` "X25519 recipient stanza"), but age-plugin-yubikey 0.5.1 emits a static 4-byte tag equal to `SHA-256(recipient)[:4]` (`src/p256.rs:71-74`), and ssh-key stanzas carry the same kind of fingerprint (age `agessh/agessh.go:35-38`).
sops-nix cannot consume age-native files (`decrypt.File` requires sops metadata, `pkgs/sops-install-secrets/main.go:340-346`), so this leg means an agenix-style activation module, one secret per file, and losing per-key extraction, the build-time key check and templates.
Machine recipients are derived from ssh host keys that any connecting client can scan, so hiding them from the vault host buys little.

What an opaque vault would still show its host: the number of documents (one per audience), the number of keys per document (one per secret), each leaf's plaintext length (sops does not pad), each document's recipient public keys, and one commit per write.
That residual is the same class every format in the table above accepts.

## What it would cost

Code: the two naming functions and the `refOfElement` inverse (`resolve.nix:316-327`), the definition-record prefix, the policy's `Audience:` comments and layout header (`policy.nix:44-49`, `188-196`), `--config` on `encrypt` and `updatekeys` with the policy rendered to a scratch config or read from the declaring root, `fix::write_policy` writing to the declaring root rather than the vault (`crates/safix-core/src/fix.rs:101-119`), the structural checks and their drills, and fixture updates.
Specs: the vault delta's scenario "A vault declared" asserts "the same audience still derives the same relative file name" and must be reversed; `secret-custody` "A multi-member audience", `public-outputs` "The reason is recorded" and "The layout distinguishes shared from per-user", and `recipient-policy` "The policy file is generated and never hand-edited" need vault-mode amendments.
Operations: the vault stops being browsable by hand; `sops <file>` on a vault document needs `safix` to have rendered the config first; renaming an entry re-encrypts its leaf because the key name is associated data; a lost naming key makes the vault unaddressable, though never undecryptable, since the declarations regenerate every name.

## What it would not fix

Local users, binary caches and anyone with the declaring flake see the declarations and therefore every name, because flake sources are store-copied by nix itself; no vault layout changes that.
Sizes, counts and cadence remain visible everywhere.
Recipient public keys remain in every document.
World-readability of ciphertext in the store remains; the only mechanism found that changes it is systemd credentials, which the operator deferred on fleet-safety grounds on 2026-09-02.

## Options

O1. Apply `separate-secrets-vault` as written: the vault host sees the same metadata the declaring repository holds today; appropriate only when both repositories sit on hosts you trust equally.
O2. Amend the change before applying so the policy file never enters the vault (ingredient 3 alone): removes the single richest document, the serials and the notes from the vault host at the cost of one flag on two sops commands and two spec amendments; the audience directories and key names still name everyone.
O3. Amend the change before applying so the vault layout is opaque by construction (ingredients 1 to 3), leaving the in-repository layout readable when no vault is declared: the vault can then sit on a host that is not trusted with the guest list; costs the code and spec work above.
O4. Add age-native documents for recipient hiding: rejected, because sops-nix cannot consume them, YubiKey recipients stay tagged, and machine keys are scannable anyway.
O5. Drop the vault: not recommended; it is the only mechanism for separate access control and retention, and it is the seam an opaque layout needs.

Recommendation: O3 if the vault will ever live on a host or with collaborators not trusted with the declarations, otherwise O2; in both cases amend before applying, because no vault exists yet and a second layout migration after adoption would cost more than a larger first change.
