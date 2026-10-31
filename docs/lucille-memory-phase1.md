# Lucille memory and identity — phase 1

This branch adds an explicitly selected Lucille profile with persistent identity and canonical SQLite/FTS5 memory to single-model Chat and Agent. It is a development implementation: IPC accepts requests only in an isolated debug application. There is no operational installation, migration, release, or automatic activation.

## Baselines and attribution

The development fork is [Ygnio/locally-uncensored](https://github.com/Ygnio/locally-uncensored), based on `master` at `5c9c4441c9cfd7240ee05ae4bb597077e8d89ff8`. The independent clone was clean and had no differences from that audited revision. The source project is [PurpleDoubleD/locally-uncensored](https://github.com/PurpleDoubleD/locally-uncensored).

The reference clone of [Ygnio/ORDIS](https://github.com/Ygnio/ORDIS) was clean at `066f3d3f711d336150ac90f5476185696374356b`, also exactly the audited revision. Its `app/src/identity.rs`, `app/src/memory.rs`, and migration organization informed the separation of configured identity, reviewed memories, and retrieval audits. This implementation does not import ORDIS databases, personal profiles, or conversations. All committed profiles and tests are fictional.

## Architecture and ownership

```text
Chat / Agent transport sends and retries
  -> withLucilleContext (one shared adapter)
  -> lucille_context (isolated Tauri IPC)
  -> lucille-memory::Store (identity and memory service contracts)
  -> canonical SQLite + FTS5 lexical search
  -> stable system identity + untrusted contextual data + original history/user turn
```

`crates/lucille-memory` has no Tauri, inference, networking, process-management, embedding, or GUI dependency. `Store` owns the transactional connection and exposes separate identity and memory methods. Identity occupies `identity_profile`, independently of `memories`; candidate creation, approval, retrieval, and editing never write identity.

The conversation's optional `lucille` binding names the explicit `profile: 'lucille'`, exact scope, exact project, and context allocation. Model names, provider aliases, and persona text cannot activate it. IndexedDB persists this routing choice and the ordinary chat transcript; it is not a writer of Lucille memory records or identity. A tiny in-process ID registry fences native automatic extraction, including writes already in flight when a conversation is bound. Native memory remains available unchanged on unbound conversations.

For a bound turn, the existing persona prompt/reminder and native memory retrieval are bypassed. Permissions, tool approval, executors, inference sampling, streaming, and cancellation remain host-owned. Native memory tools and unsupported workflow/delegation entry points are excluded at both tool selection and executable lookup. Group turns are rejected for a Lucille-bound conversation. Code, Voice, remote-phone context and subagent integration require later work.

## Database and migrations

`PRAGMA user_version` is the migration version. `001_memory.sql` creates identity, scopes, memories, metadata-only change history and retrieval audits. `002_fts.sql` creates the Unicode FTS5 external-content index and insert/update/delete triggers. Migration bodies and version updates run transactionally. Newer unknown schemas are rejected. The store enables foreign keys, WAL, FULL synchronization, a bounded busy timeout and SQLite secure-delete. Integrity checks exercise SQLite, foreign keys and FTS5's external-content consistency check.

All eleven categories are retained: `fact`, `preference`, `hardware`, `project`, `decision`, `solution`, `goal`, `task`, `constraint`, `summary`, `hypothesis`. States are `candidate`, `active`, `conflict`, `obsolete`. Origins include a typed `user`, `manual`, or `reviewed_import` source and an explicit immutable `source_id`. Assistant-origin candidates are not accepted.

Creation always produces a candidate. IDs are UUIDs; exact repeated content/key/category/provenance-type in the same project/scope is deduplicated, retaining the first origin. Reusing an ID with another payload fails. Deleted IDs remain metadata tombstones and cannot be replayed. Every update requires the expected version and returns the record to candidate status. Approval, rejection, obsoletion, and deletion are explicit transitions with optimistic version checks and transactions. Deleted content is removed from current records/FTS; history retains IDs, operations, versions and timestamps, not old personal content.

Approval checks existing active factual keys across categories in the same project/scope. A collision becomes `conflict`; the existing active record is retained. Resolving it requires explicitly obsoleting the prior fact and approving the replacement. This is an exact-key safeguard, not natural-language contradiction detection. Different hardware keys can still describe contradictory equipment; there is no claim that ORDIS's factual grouping limitation is solved. Negation and correction text is never automatically promoted or interpreted by an LLM.

Backup is a versioned, typed JSON snapshot of identity, records, metadata history and retrieval audits, read in one transaction. Restoration validates the complete structure, keys, references and conflicting active facts, rebuilds FTS through triggers, and replaces the isolated database transactionally. Backup input is data; it cannot execute SQL or supply a filesystem path. Restoring an earlier backup is an explicit administrative rollback, so it can intentionally restore earlier records. This is not forensic erasure of WAL files or older backups.

## Internal API contracts

`lucille_context({request})` accepts a typed `TurnRequest` with conversation/message IDs, mode (`chat` or `agent`), exact scope/project, query and a 512–16,384 token ceiling. It returns `TurnContext` with the persistent identity/version, selected records, record ID/version references, scope/project, budget/usage and retrieval state. No query text or memory content is written to retrieval audits or operational logs.

`lucille_manage({request, confirmation})` exposes these tagged operations:

| Operation | Contract |
| --- | --- |
| `identity` | Read configured profile, version and creation/update timestamps; absent profiles fail closed. |
| `configure_identity` | Profile plus expected version; explicit settings authorization required. |
| `reset_fixture` | Explicitly initialize or reset the fictional profile, with expected version. |
| `list` | Records for an exact scope/project, capped at 1,000 rows per call. |
| `create` | Typed candidate with UUID, category, text, factual key and origin. |
| `change` | ID, scope, project, expected version and `approve`, `reject`, `obsolete`, `delete` or `update`. |
| `history` | Metadata changes for a UUID, including deletion tombstones. |
| `export` | Logical synthetic backup, with configuration authorization. |
| `restore` | Replace isolated data from a validated logical backup, with configuration authorization. |
| `integrity` | SQLite, foreign-key and FTS5 checks. |

Mutating/export/restore calls require `CONFIRM LUCILLE CONFIGURATION` from explicit settings actions. This is an intent marker at the trusted WebView boundary, not a password or a defense against a compromised frontend. These commands are not model tools. Retrieved text is never parsed as an API call or a permission grant. The model cannot obtain administrative rights by placing that marker in a memory. Existing tool permissions still enforce every executable tool call; prompt wording alone is not treated as a complete prompt-injection defense.

The Rust API validates IDs, bounded text, scopes, factual keys, enum values, expected versions and unknown JSON fields. Frontend SQL, arbitrary database paths and extraction-side identity updates are not supported.

## Retrieval and context policy

Each actual transport invocation, including Agent steps and retries, invokes the same adapter and reads a fresh SQLite transaction immediately before sending. The scope/project binding is fixed for the current turn; settings changes affect subsequent turns. Records changed, obsoleted or deleted before this read cannot appear as their old versions. Already-sent requests cannot be retroactively recalled.

Queries are converted into at most 24 quoted alphanumeric lexical terms, combined with OR. Search uses FTS5/BM25, an exact scope/project filter, active-state filter and a 100-row candidate cap. Durable preferences have a reserved share capped at one quarter of the available memory allocation. Relevant non-preference facts use the remainder. No implicit cross-project or cross-scope global fallback is performed. Use an explicit shared project ID for intentionally shared information.

Identity is a separate stable system message. Retrieved records are JSON-encoded data in a user-role context message before the latest user turn; they are never appended as system instructions. The adapter deduplicates IDs, checks the returned references/versions and scope, replaces its prior injected messages on retries, preserves image/tool metadata and preserves tool call/result adjacency. It never persists these injected messages in the conversation store.

The allocation uses conservative serialized UTF-8 byte counts plus framing allowance rather than assuming four characters per token. It reserves space for the profile and caps preferences. History budgets reserve the configured Lucille allocation; no inference-server context setting is changed. Oversized identity, invalid context, cancellation and backend failure fail closed. This deliberately favors a predictable upper bound over recall; it is not tokenizer-specific capacity maximization.

## Isolation and reproducible validation

All development and compilation happen in an independent checkout. Synthetic data, state, logs, temporary profiles, build outputs and QA reports have separate development directories. Unit and integration runs do not start a second application, model server or installer. No operational databases or personal conversations are required.

IPC is lazy: app startup opens no Lucille database. Requests require all of:

- A debug build with application identifier `dev.ygnio.lucille.phase1` and the local `main` window.
- An absolute `LUCILLE_PHASE1_ROOT`, with a `.lucille-isolated` file containing exactly `synthetic-data-only` followed by LF.
- A canonical root outside APPDATA, LOCALAPPDATA and Program Files, with database and SQLite sidecars confined to that root.

The database filename is fixed as `lucille-phase1.sqlite3`; IPC accepts no path. `tauri.lucille-isolated.conf.json` selects a separate identifier and dev URL `15273`, disables bundling and omits server/resources packaging. It prepares compilation isolation; it does not certify all GUI startup/process behavior. No GUI was launched during phase 1. Existing application processes, launchers, provider configuration, models and servers on ports 8080/8081/8128/8188 are not accessed or controlled by this implementation's tests.

From an isolated checkout, with Node 22+ and a Rust/MSVC development toolchain:

```powershell
# Set these to dedicated development directories before running tests.
$env:CARGO_TARGET_DIR = '<isolated-build-directory>'
$env:TEMP = '<synthetic-test-directory>'
$env:TMP = $env:TEMP
cargo test --locked --manifest-path crates/lucille-memory/Cargo.toml
cargo build --locked --manifest-path crates/lucille-memory/Cargo.toml --example synthetic_bridge
$env:LUCILLE_QA_BRIDGE = "$env:CARGO_TARGET_DIR\debug\examples\synthetic_bridge.exe"
npx vitest run src/lib/__tests__/lucille-context.test.ts src/lib/__tests__/lucille-real-sqlite.test.ts src/hooks/__tests__/memory-silent-call-gate.test.ts --maxWorkers=1
npm run typecheck
npm run build
$env:TAURI_CONFIG = Get-Content src-tauri/tauri.lucille-isolated.conf.json -Raw
cargo check --locked --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc
cargo build --locked --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc --bin locally-uncensored
```

Compilation commands do not launch the resulting application. Do not run the binary as an operational deployment. The separate workflow `lucille-memory.yml` tests real SQLite/FTS5 and the frontend adapter on Windows and Linux; a missing bridge is reported as skipped in ordinary unit-only runs, and CI explicitly builds/provides it. Existing general CI continues to cover the broader repository.

## Validation scope and known limits

The QA report records exact commands/results and final commit IDs. Local binary tests target Windows 10 Pro 19045 and Intel i7-3770, using baseline `x86_64-pc-windows-msvc`. The added memory library/SQLite requires no embedding runtime or AVX2 option. Running these binaries on the actual i7 is stronger evidence for the added memory path than mocks; it does not certify all existing model/server/GPU executables. GUI behavior and operational installation compatibility remain pending.

No production LLM extraction, semantic search, embeddings, Nomic authentication change, model load, Code/Voice/subagent integration, migration of existing memories, operational backup restoration or deployment is included. `SemanticIndexSpec` defines provider, effective model, dimension, pooling, normalization, document/query prefixes and index version for a future implementation. `CandidateExtractor` is a contract only, with no production implementation.

Further limits: synchronous local IPC work is serialized behind a mutex; cancellation prevents the send after a completed lookup but does not interrupt SQLite halfway through a transaction. Metadata history has no previous content snapshots. Search/listing are bounded; preferences are recent-first within their reserved share. Factual keys remain manually assigned. The default frontend production build hides the QA settings, and a normal/release application cannot use phase 1 IPC even if it imports a conversation binding.

## Rollback and next phase

Before deployment, rollback is simply to leave the review branch unused. In an isolated QA profile, unbind the conversation to resume native memory; this does not migrate or dual-write records. Reverting the feature commits removes the hooks and API. Keep any synthetic database outside Git; remove only verified test directories after closing the test process. No operational rollback is necessary because phase 1 never modifies the installed application.

Next: review this PR, validate an isolated GUI's complete startup/process/data-directory behavior, then design an explicitly authorized profile/memory import and operational rollout. Add tokenizer-aware allocation and pagination if needed, refine factual keys/conflict review, and only then consider local semantic indexing or reviewed LLM extraction. Deployment, merge and release require separate authorization.
