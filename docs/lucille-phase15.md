# Lucille phase 1.5 — unified identity, Code and verified environment

This is a stacked development change based on phase 1 at `c2032580aeb21a0826abe237f76de8c838c4afd6`, in `feature/lucille-unified-identity-code`. Its review target is `feature/lucille-memory-hybrid-phase1` in the same fork. PR #1 remains unmerged. No operational installation or personal-data migration is part of this change.

## Root cause of the Code identity problem

In the audited Code hook, `getActivePersona()` was read when creating a conversation and its system prompt was saved, but generation assembled `systemPrompt` from Code's own full/review/plan/lean constants. It did not compose the selected persona or its reminder into that request. The full prompt explicitly began `You are the Coding Agent, an autonomous coding agent inside LU.` Review and plan had equivalent self-labels. Changing only instruction order would therefore not fix the missing identity input. Native memory was independently retrieved into this operational system prompt, and the phase 1 binding setter deliberately rejected Code conversations.

The prompts now live in `src/lib/codex-prompts.ts`, imported by the real `useCodex.ts` and the dedicated QA frontend. Tests reproduce the legacy generic self-label, verify unchanged behavior while unbound, and exercise identity composition with full, review, plan and lean operational prompts. The persistent Lucille binding is explicitly selectable for ordinary Chat/Agent and Code conversations; group/remote contexts remain unsupported. This does not globally reinterpret existing private personas or silently import their content.

## Shared identity and context

`withLucilleContext` is the shared send-time boundary for Chat, Agent and Code. Each actual transport invocation/retry loads current identity and active SQLite memory with exact scope/project filters. Profile ID, identity scope and version are embedded as applied-profile metadata. Code's known self-introduction is converted into an operational-mode description, retaining the original operational rules. Coding Agent is a mode, not the configured identity's name.

All Code transport branches use the adapter: native Ollama, native provider streaming, thinking downgrades/retries and Hermes prompt transport. Code reserves the Lucille allocation in the existing history budget and bypasses native memory retrieval and automatic extraction when bound. The same model/provider selection and tool executors remain in place. Native memory/workflow/delegation tools are fenced at both catalog and executable lookup for Lucille; ordinary authorized coding tools retain their original approval and workspace boundaries. Legacy unbound prompt transport retains its original message normalization. Existing verbose Code diagnostics are omitted for bound turns instead of logging user instructions or operational paths.

Identity, environment, retrieved data, history and the current request remain separate. Duplicate injected messages and memory IDs are removed on retries. Retrieved memory is encoded as untrusted contextual data, never parsed as permissions or an administrative call. Unbound conversations return through the original path without a new identity or environment message.

## SQLite migration 3

`003_code.sql` adds `code` to the retrieval-audit mode constraint by replacing that table transactionally and copying its prior rows, IDs, references and timestamps. Identity and memories are not migrated or imported. A real SQLite test starts at phase 1 schema version 2 and verifies that its existing Chat audit survives while a new Code audit is recorded. Migration replay, integrity, FTS5 and restart tests continue to run. Logical backup exports identify schema 3; existing schema 2 logical backups remain accepted by the isolated restore API.

## Verified clock, route and tool evidence

`verified-environment.ts` samples the host clock and IANA timezone at each send, recording UTC observation time and formatted local date/time. It carries the actual routed backend/model and the tool names actually offered by that transport. It contains no hardware guesses, private paths, credentials, schemas, arguments or result bodies. The serialized environment allocation is reserved before memory retrieval, so it cannot silently expand the configured Lucille budget.

The shared executor now adds `dispatched` only at its actual `runtime.execute` boundary, after validation and approval. `executorEvidence` creates metadata-only receipts from executor settlements, not from model prose. Receipts distinguish requested calls, actual dispatch, successful confirmed results, failures, rejected requests and cache hits. A dispatched operation may still fail; a cached result is not a fresh execution. Failed tool-result text is not mislabeled as confirmed success. Code and Agent pass a bounded recent receipt list to their next model step. Chat has no tools on its plain transport; tool-enabled chat already routes through Agent.

The prompt explicitly prohibits claiming tool execution without executor evidence. This is provenance and grounding, not a proof that arbitrary model prose cannot hallucinate. No real inference was used to claim universal behavioral reliability. The existing tool trace/approval UI and executor enforcement remain authoritative.

## Dedicated native QA surface

`crates/lucille-qa` is a small Tauri executable with identifier `dev.ygnio.lucille.phase15`. It reuses the real phase 1 IPC module, Rust store, shared frontend adapter, Code operational prompt and shared tool executor. It never calls the ordinary application's startup, engine discovery, provider/keychain setup, updater, installer, tray, process-control or model services.

The QA runtime lives under `.lucille-qa-runtime/lucille-synthetic-gui` inside the checkout, guarded by an explicit synthetic marker. Its WebView profile, SQLite database, workspace, process environment and temporary directories are exclusive. Its backend registers only identity/context management and a tiny file read/write/edit surface confined to the synthetic workspace. No shell tool is registered. The mock model endpoint listens only on `127.0.0.1:15274`; the WebView CSP permits that endpoint and local IPC. Operational ports 8080/8081/8128/8188 are not used. No second model is loaded.

The GUI is an explicit integration test surface, not the entire standard LU application. It proves real Tauri/SQLite/shared-adapter interaction and real scoped file operations. It does not establish production GUI startup safety, sidecar packaging/inference behavior, or semantic model responses. The standard application GUI and installed application were not used as test targets.

### Reproduce without manual assistance

1. Build the dedicated QA frontend with `npx vite build --config vite.lucille-qa.config.ts` and type-check with `npx tsc -p tsconfig.lucille-qa.json --noEmit`.
2. Build the native QA host with `cargo build --locked --manifest-path crates/lucille-qa/Cargo.toml` in a dedicated target directory. Do not run a release build.
3. Prepare the marked synthetic root and its profile/workspace/temp/platform directories. Start `node scripts/lucille-qa-server.mjs <absolute-synthetic-root>` from the checkout. Do not reuse a listener belonging to another task.
4. On Windows, launch the dedicated executable with `scripts/start-lucille-qa.ps1 -QaExe <absolute-qa-executable> -InspectBrowser`. The script restores its caller's environment after spawning the child and refuses an occupied inspection listener.
5. Run `node scripts/test-lucille-gui.mjs`. It checks the exact QA WebView URL before acting and writes synthetic results/screenshots under `.lucille-qa-runtime/reports`.

The automatic GUI flow creates a fictional identity/memory, explicitly binds the profile, validates the same identity and memory in all three modes, creates/reads/edits a real synthetic file, verifies escape refusal, updates and deletes the memory, rechecks all three modes and verifies the unbound Code default. Separate local restart validation confirms identity version, profile binding and deletion persist while transient tool receipts reset. All responses from `payload-inspector` are labeled simulated; they validate captured requests, not inference.

### Screenshot transport failure and scoped workaround

On the target Windows 10 Pro 19045 machine, native Computer Use again failed with `FrameArrived timed out: timed out waiting on channel`, followed by `window capture timed out: timed out waiting on channel`. Window discovery/accessibility and manual rendering worked. The active Windows session was local console, not RDP. The desktop updater reported installed Codex `26.1023.51421`, build `22530`, with `restart_required`; applying that pending app update has not been claimed to fix the native capture path.

For this QA application only, optional `-InspectBrowser` exposes WebView browser inspection on loopback port 15275. Playwright attaches to that dedicated WebView to click, inspect DOM and obtain renderer screenshots. This successfully completed the autonomous GUI flow. It is a workaround for this web-based QA surface, not a repair of Computer Use for arbitrary native applications. No Computer Use binary, graphics driver, registry, OS security setting, production browser profile or operational application was changed. Inspection is opt-in, debug-only in practice through the dedicated QA host, and never exposed on a non-loopback listener. Close the QA host when finished to close its inspection endpoint.

## Regression and CI findings carried from phase 1

The phase 1 memory-specific CI passed and CodeQL later passed. The general CI subsequently revealed two Rust failures: the new Lucille commands were absent from the mobile allow/block classification, and a sidecar-resource test treated a deliberately non-bundling QA overlay as a shipping bundle. This branch explicitly blocks Lucille IPC on phones and excludes only `bundle.active=false` overlays from shipping-sidecar assertions. The isolated overlay also explicitly retains HTML5 drag/drop behavior.

Five source-guard tests in the prior general CI expected the old direct send/prompt spelling. Their assertions now follow the shared adapter and asynchronous Hermes path while continuing to check history-copy use, bounded retries, thinking flags and system-prompt presence. Code prompt guards read the actual shared prompt definitions. These changes do not disable runtime tests or excuse behavioral failures.

The memory CI now includes Code/environment/executor tests and dedicated QA frontend checks, compiles the real QA host on Windows, and runs on this stacked branch/base. The general CI is enabled for PRs targeting phase 1 as well. The final delivery report records actual local counts, commits and observed live CI state, without treating pending checks as passed.

## Privacy, compatibility, rollback and remaining work

### Local validation on 2026-11-03

The final affected-source and memory regression run passed 21 files / 313 tests, including real Rust/SQLite bridge integration. Earlier additional relevant executor and source-guard runs also passed; their overlapping counts are not added to that total. The Rust memory crate passed 24 tests. Both previously failing native mobile-classification and shipping-resource guard tests passed independently against the real application library. Application and QA TypeScript checks, changed-file ESLint, dependency-cycle analysis, the production frontend build and the dedicated frontend/native QA builds passed. The final QA GUI automation passed again after rebuilding the native host, with screenshots and structured synthetic evidence saved outside Git. An earlier combined run exposed a source guard still reading the moved Code prompt; its reference was corrected and the affected suite passed in the final run.

These results cover the selected relevant suites, not a claim that every application test or remote CI check has passed. The final delivery report links the observed remote workflows. The dedicated GUI validates request composition and backend behavior with a simulated model response, not real model inference.

All committed identities, memories, file contents and GUI fixtures are synthetic. Real profiles, ORDIS memory, installed LU databases and conversations are neither read nor imported. Generated runtime data, screenshots, logs, profiles and build outputs are ignored by Git. Tests use a dedicated temporary root inside the checkout; cleanup must resolve and verify that root before deleting only its synthetic descendants. Closing/restarting a test process must first verify its executable/session manifest; never kill processes by a broad name.

Native tests and the dedicated GUI run on the actual Windows 10 Pro 19045 / i7-3770 machine. The target remains baseline `x86_64-pc-windows-msvc`; no AVX2 requirement, embedding runtime, second inference model or hardware-specific optimization was added. No operational compatibility beyond the tested development paths is claimed.

Rollback before deployment is to leave this stacked PR unmerged; unbinding a QA conversation restores the native path. Schema 3 must not be opened by the older phase 1 binary; use an isolated backup/restore or revert the test database as a deliberate test-only rollback. No operational rollback is necessary because the installation was not modified.

Phase 2 should review the stacked changes, validate the full standard application's isolated startup and real hook flows, then perform separately authorized real inference/rollout. There is no legacy data migration, production LLM extraction, embedding/semantic search, Voice/remote/subagent integration, merge, release or operational install in this phase.
