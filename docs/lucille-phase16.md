# Lucille phase 1.6 — full application validation

This branch stacks on phase 1.5, starting at `220f09311c71581288b750b1fea495f28428a622`. PRs 1 and 2 remain unmerged. Upstream and the fork's master were both still `5c9c4441c9cfd7240ee05ae4bb597077e8d89ff8` at the initial inspection. No local unpublished changes existed.

## What is actually being validated

The frontend is the complete `src/main.tsx` → `App` → `AppShell` application, with its real settings, conversations, Chat/Agent/Code composers, hooks, provider, streaming parser, executor, staging/review queue and file tools. It is not `qa/lucille.html`. The backend is the application library with its real SQLite IPC, Rust HTTP proxy and filesystem handlers, entered through a separate, explicit debug-only binary. Production lifecycle startup/shutdown, updater, shell plugin, single-instance activation, model discovery, installers, service startup and keychain registration are not used.

The model protocol endpoint is a deterministic simulator. Its responses explicitly say `[SIMULATED: no inference]`. Captured requests prove what the real hooks and provider sent, including applied identity, memory IDs/versions and environment. They do not prove that Gemma will follow those instructions. No personal profile, conversation, identity or memory is imported.

## Isolation boundary

`lucille-full-validation` requires the opt-in `lucille-validation` Cargo feature and refuses to run as release. Its context has identifier `dev.ygnio.lucille.phase16`. Startup requires the exact marked checkout directory `.lucille-qa-runtime/lucille-synthetic-full`, checks its canonical location and never calls the production `run()` function.

Rust path helpers explicitly use that root for home/data/cache/config, rather than relying on environment variables to redirect Windows Known Folders. `AppState::for_lucille_validation()` constructs empty state without interpreter lookup, configuration reads or discovery. The real filesystem jail's per-chat sandboxes are under this synthetic home. The WebView2 profile is explicitly `root/webview`; child APPDATA, LOCALAPPDATA, USERPROFILE and temporary directories are synthetic too. Generated data and logs are Git-ignored.

The native command boundary denies by default. It registers only canonical identity/memory IPC, real scoped filesystem operations, workspace checks and the real HTTP proxy/cancellation functions. The proxy boundary admits only `http://127.0.0.1:15284`; operational addresses and lifecycle/keychain/whole-computer/shell commands are refused before dispatch. The actual Code executor retains Ask/review and workspace checks, with the host boundary adding a second limit.

The dedicated Vite configuration serves the real frontend on `127.0.0.1:15283` without the production dev-server plugins/routes. Its HTTP response CSP permits only its own resources, local IPC and the mock endpoint; attempted default ComfyUI probes are blocked. Its watcher excludes generated profiles and native/runtime artifacts, because watching WebView temporary files caused an observed `EBUSY` startup failure before this exclusion. The optional debug inspection endpoint is loopback `15287`. The launcher refuses occupied ports. Ports 8080, 8081, 8128 and 8188, operational processes, models and installation paths are untouched.

`qa/full-bootstrap.js` seeds fictional choices only in a fresh exclusive WebView. It uses the real settings migration to fill all defaults, avoiding an incomplete current-version fixture that lacked `builtinEngine.ctx`. It never resets an existing validation profile.

## Execution evidence correction found in the complete GUI

Ask mode legitimately returns successful staging/overlay results before writing anything to disk. Phase 1.5 marked these as `result_confirmed=true`, which did not sufficiently distinguish successful tool dispatch from confirmed disk effects. The full application test reproduced that distinction.

The real staged writer now exposes pending-result metadata derived from its authoritative queue, using the existing normalized path matcher. The identity environment records these results as `outcome: staged`, `result_confirmed: false`; overlay reads are also pending. It does not infer staging from model prose or a tool-result string. Approved disk reads still confirm success; rejection before dispatch remains `executed: false`; filesystem errors remain failed. No approval, staging or filesystem rule is relaxed. The prompt explains this additional state.

The full GUI test verified the file was absent before Apply, used the actual Apply-all control, then confirmed `green` through the real Rust filesystem and a subsequent Code tool read. An attempted parent-path read failed. A missing file failed. A destructive shell request displayed the actual approval UI, was rejected with No, was never dispatched, and left the synthetic file intact.

## Reproduce on Windows without the operational installation

1. Use the existing checkout dependencies and an explicitly selected local Rust toolchain. Do not install or update global tooling.
2. Set `TAURI_CONFIG` to the text of `src-tauri/tauri.lucille-validation.conf.json`, and build with `cargo build --locked --manifest-path src-tauri/Cargo.toml --features lucille-validation --bin lucille-full-validation`. Use a dedicated target directory. The overlay removes bundled sidecars; do not create an operational server stub and launch it.
3. Run `scripts/start-lucille-full-validation.ps1 -QaExe <absolute-debug-executable>`. This starts only the dedicated frontend/mock/host and writes their process IDs and executable to the synthetic session manifest. The script restores its caller's environment.
4. Run `node scripts/test-lucille-full.mjs`. It requires the synthetic marker and exact full-application WebView origin. Fixture provisioning calls the same stores/IPC as Settings; all sends go through the actual composer. Reports and screenshots are written under the marked root's `reports` directory.
5. Use `scripts/stop-lucille-full-validation.ps1` to close only the executable and script processes identified in that manifest. No synthetic data is deleted. For persistence checks, restart using the same root and inspect identity version, binding and canonical records before provisioning a new fixture run.

The autonomous script covers identity/memory across modes, project isolation, actual staged/reviewed/applied filesystem operations, negative path/error/approval cases, deletion and unbound defaults. Additional local GUI checks covered settings-based initialization/configuration, conflict/candidate exclusion, update/reapproval, actual model-picker changes, reactivation and restart persistence. The delivery report records observed counts and evidence separately.

## Remote CI investigation

Repository permissions reported Actions enabled, all actions allowed, default token permission read and no automatic review approval. All three registered workflows were active. Nevertheless no check suite or workflow run existed for phase 1.5. Explicit dispatch of the memory workflow, general CI and CodeQL on this branch returned HTTP 422: `Actions has been disabled for this repository.` This is a concrete GitHub execution block despite the contradictory permissions readout. Its account/administrative cause has not been established. No security setting was weakened or policy changed to bypass it.

Branch filters independently showed CodeQL only covered PRs targeting master. The stacked bases are now included, phase 1.6 push checks are enabled, and memory/general workflows expose explicit dispatch. Security actions remain pinned and their permissions/checks are retained. General Windows/Linux build lanes additionally compile the isolated full-application binary. These changes prepare the correct checks; an absent or rejected run is never reported as green. The prior phase 1 run results remain historical evidence, not results for this branch.

## Capture, compatibility and limits

The native Computer Use capture was retried on this dedicated full-application window and again returned `window capture timed out: timed out waiting on channel`. Accessibility worked. The updater still reported installed Codex 26.1023.51421, build 22530, `restart_required`. Codex was not restarted during the active task, so no post-restart repair is claimed. WebView inspection provided working autonomous actions and screenshots. No driver, Windows clock, graphics/security/privacy setting or Computer Use binary was modified.

Native development validation runs on Windows 10 Pro 19045 / i7-3770 with baseline x86_64. The known GPU is RTX 3060, but no AVX2, hardware tuning, embedding engine or second model is introduced. The operational 8080 health endpoint did not respond to a read-only probe; no real inference was performed and no server/model was started, restarted or unloaded. Linux builds, CodeQL and remote security checks remain blocked by GitHub Actions execution. Standard production startup and release packaging are not validated by running this restricted debug host.

The final delivery report must therefore retain the remote CI and real-inference/capture limits. Automatic LLM extraction, semantic retrieval, personal migration, deployment, merge, release and operational installation are outside this phase. Keep the stacked PRs unmerged until remote checks can run and the changes are reviewed.

## Observed local results

The complete-GUI autonomous script passed. Separate restart validation after rebuilding the host preserved identity version 4, the saved Code binding and deletion; a new real Code send after restart carried that same version and no deleted memory. The earlier additional GUI sequence verified identity version 3 and approved memory version 4 consistently across all three hooks, conflict exclusion and actual model-picker switching. Native memory collections remained empty; automatic native extraction was disabled for deterministic QA, while regression tests cover the Lucille extraction fence.

The memory crate passed 24 real Rust/SQLite tests. The native validation command/network guard and both mobile/resource regression guards passed. Application and dedicated-QA TypeScript, changed-file ESLint, production frontend build and final native Windows debug build passed. Selected TypeScript/executor/staging regression suites passed; exact final counts are recorded in the delivery report. The existing native plugin emits an unrelated `fetch_update` deprecation warning; no global toolchain change was made to suppress it. Remote checks remain blocked, not passed.

The five dependency-security regression tests passed. A current production-dependency npm audit found four low-severity reports in the KaTeX dependency chain (GHSA-238p-pmpm-9mq7), with no high/critical report; the existing high-severity audit gate was not weakened. Its proposed force upgrade is breaking and was not applied as part of this validation branch. Rust advisory scanning and remote CodeQL remain unexecuted for the new HEAD while Actions rejects dispatch.
