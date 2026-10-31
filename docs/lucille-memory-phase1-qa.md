# Phase 1 validation record — 2026-10-31

Development occurred in a fresh, independent clone of the fork, on `feature/lucille-memory-hybrid-phase1`. Both source baselines matched their audited revisions exactly and were initially clean. No application instance or installer was started.

## Revisions

| Repository | Initial HEAD |
| --- | --- |
| Ygnio/locally-uncensored, master | `5c9c4441c9cfd7240ee05ae4bb597077e8d89ff8` |
| Ygnio/ORDIS, reference only | `066f3d3f711d336150ac90f5476185696374356b` |

Implementation commits:

- `1c0e572bd4220a3205b34ecf9232e7551cd51686`: standalone canonical SQLite/FTS5 memory, identity, isolation policy and real Rust tests.
- `0cfec6199582c6a1fd8b13471408358cf317831d`: isolated Tauri commands, common Chat/Agent adapter, opt-in settings, native writer fence and frontend/integration tests.
- This documentation/CI commit records the architecture, validation and reproducible Windows/Linux workflow. The PR's head is the authoritative final revision; the private delivery report records its full hash.

## Environment

Windows 10 Pro `10.0.19045`, Intel Core i7-3770 at 3.40 GHz. Node `24.22.1`, isolated npm `11.6.2`, Rust/rustc `1.99.1 (58cf8f9c3 2026-10-13)`, target `x86_64-pc-windows-msvc`. The fork's frontend dependency lock was retained unchanged. The standalone Rust lock and application Rust lock include rusqlite `0.38.0` with bundled SQLite/FTS5.

Toolchain homes, npm cache, build outputs, synthetic SQLite directories, state, profiles, logs and QA records were under the task's development workspace. Final synthetic test runs explicitly redirected TEMP and TMP to the dedicated test-data directory. No private application profile or real memory database was read.

## Results

| Validation | Result |
| --- | --- |
| Real Rust database/identity/isolation tests | **23 passed**, 0 failed, 0 ignored on Windows. Linux CI additionally exercises a symlink-escape test. |
| Frontend adapter + native-memory regressions | **128 passed** in 11 files. Includes two tests that invoke the real compiled Rust bridge, without mocking SQLite. |
| Final affected tests after test cleanup/type corrections | **4 passed** in the real-SQLite and persisted-binding files. |
| TypeScript `tsc -b --force` | Passed on the final source tree. |
| ESLint on all changed TypeScript/TSX files | Passed, 0 errors/warnings. |
| Madge cycles | No circular dependency in 3,723 processed files; 41 resolver warnings. |
| Vite frontend build | Passed; final build 22.33 s. |
| Rust/Tauri isolated cargo check | Passed with isolated configuration. |
| Rust/Tauri full debug executable build | Passed with isolated configuration; initial build 10m 54s, final locked rebuild 1m 38s. No application launch. |
| Staged/source artifact review | Source, lockfiles, configuration, tests and documentation only; no database, binary, model, credential or personal conversation included. |
| Operational GUI / installed application validation | **Not performed**, intentionally outside phase 1. |

Final isolated executable SHA-256: `7f3cb22f73db3510af0f3f17241f2516bca0ced0e8088e5eaf0b0d5e36025a25` (69,018,624 bytes). The executable remains a local development artifact and is not committed or installed. Running actual Rust test executables and the integration bridge on the i7-3770 validates the added memory path on that CPU; this is not a compatibility certification of every upstream binary or model server.

The database tests cover migration/repeated migration, actual WAL/version/integrity/FTS5 behavior, all eleven categories, persistence after reopen, typed validation, explicit identity authorization and version checks, candidate review/rejection, negation/correction conflicts, preference changes, optimistic concurrency, update/delete/obsoletion, ID replay prevention, project/scope separation, malicious-memory data, backup/restore with metadata audits/deletion tombstones and write-lock failure.

Frontend tests cover both modes through the same adapter, persistent identity without history, relevant facts and durable preferences, cross-project exclusion, deletion freshness, duplicate exclusion, tool/image metadata, cancellation before/during retrieval, fail-closed behavior, memory-tool/delegation exclusion, preserved native extraction behavior, revocation of in-flight native writes, and persisted opt-in binding restoration.

## Resolved setup and verification issues

The initial environment had no configured Rust toolchain or npm executable. Tools were prepared in development-local homes/cache without changing global configuration or the operational application. Git author identity was configured only in this clone, using the account's GitHub noreply address.

A normal cargo check initially failed because the repository does not ship its llama-server sidecar. The isolated overlay disables sidecar/resource bundling, allowing a real check and real debug build without copying, replacing or starting an operational inference server. This does not validate server packaging. An unrelated generated Windows permission schema was restored after builds rather than committed.

During development, regression tests exposed an unnecessary extraction-time import of the chat/remote store graph. Replacing it with a small ID-only routing fence restored native test behavior. Type, lint and type-only import-cycle findings were corrected, and final checks passed.

Non-failing output retained for review: the existing native plugin uses an atomic method deprecated by the new toolchain; Vite reports existing chunk-size/dynamic-import/plugin-timing warnings. npm's audit reported 25 dependency findings (4 low, 19 moderate, 2 high) in the unchanged frontend dependency tree; no broad dependency upgrade was mixed into this feature.

## Isolation evidence and pending work

All writes made for this task target the independent development checkout or its dedicated tooling/data/build/report directories. No commands modified the installed application, operational launchers, model files, provider settings, ORDIS memory, or running Lucille processes. No test contacted or controlled ports 8080, 8081, 8128 or 8188. No personal memory, profile or conversation was accessed, copied or published. No merge, release, installer or operational deployment occurred.

The new CI workflow runs real Rust SQLite tests and the frontend integration/regression selection on Windows and Linux. GitHub run status belongs to the live PR checks; it is not implied by this local validation record. The final delivery report records the observed status and PR/branch links.

Pending by design: complete isolated-GUI startup/process isolation review; operational GUI validation; sidecar packaging/inference compatibility; authorized migration of existing memory; production extraction; embeddings/semantic ranking; Code, Voice, remote and subagent context integration. See [architecture, API, limitations and rollback](lucille-memory-phase1.md) before considering deployment.
