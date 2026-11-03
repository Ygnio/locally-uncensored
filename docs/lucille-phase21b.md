# Lucille Phase 2.1B — synthetic quality closure, live inference pending

Base: `f721b5a4e7d83908d9caafb5b997b50e24fc4247`, PR #5. Branch `feature/lucille-phase21b-quality` is stacked on `feature/lucille-phase21-semantic-learning`. Initial local/remote heads matched, checkout was clean, and PRs #1–5 were open/draft/unmerged. No merge, release, install or general activation is performed.

## Audit of the three omissions

The prior second sample had 7 TP/0 FP/3 misses, precision 1.0 and recall .70. Its bytes and labels are preserved; it is now historical regression data, never unseen evaluation.

| Historical omission | Cause | Supported change |
|---|---|---|
| Personal coffee choice with “zero açúcar” | Normalization and word-order coverage | Narrow direct-choice grammar, preserving the source sentence and canonical sugar value |
| English turn-based squad-tactics choice | Paraphrase and missing preference vocabulary | Separate squad-tactics topic; does not assert an RPG/classes preference |
| Campaigns whose decisions change the story | Missing preference topic | Branching-story preference, distinct from fiction about the user's identity |

All three contain explicit personal preference evidence. They are not safety abstentions. In contrast, unnamed/ambiguous equipment, unspecific capacity units, quoted/third-party narratives and negated uncertain statements may correctly abstain or require review. More records is not the objective.

## Reused system and bounded changes

The canonical Store/SQLite/FTS5, five migrations, queue/leases, policy, source purge, versioned vectors, identity binding, history and tombstones remain authoritative. No new schema or database is needed. Backups 2–5 retain compatibility. The LLM still proposes, then schema/evidence/deterministic validation precede policy and native transactional ingest; model confidence cannot grant approval or permissions.

`coverage.rs` adds narrow evidence-preserving alternatives before the existing classifier. Direct preference choice and game-story forms are supported, along with unambiguous work-laptop aliases. Explicit withdrawal of liking coffee uses a negative value on the legacy coffee topic so it conflicts with and suspends a prior positive sugar preference; approval obsoletes the old value atomically. Negated sugar/language preferences are not blindly inverted. Goal completion references a compatible canonical goal topic, preserves history and is excluded from ordinary planning retrieval after review.

Capacity updates require care: an explicit equipment update without a named capacity property remains tentative. An unnamed old→new capacity correction is associated only when one active scoped prior RAM record matches. Multiple matching devices remain unspecified, suspend compatible stale capacity context and cannot be approved until clarified. An isolated “8 GB” statement is not invented as RAM. Named equipment, hypotheses and other devices remain separate. Hardware updates require review. The legacy implicit-device grammar is retained for backward compatibility; this is not unrestricted entity resolution.

Complete source attribution remains required. Forwarded, third-party, permission-changing and supported fiction forms stay excluded. Mixed-device or adversative clauses abstain instead of combining inconsistent values. All original safety regressions remain present. Unknown language and implicit references outside these bounded grammars are still limitations.

## Development, regression and final evaluation

Acceptance was fixed before changes in `lucille-phase21b-acceptance.md`: extraction precision >=95%, recall >=80%, zero critical unsafe/scope writes, duplicate current claims and stale/deleted retrieval. Report categories/languages and modes separately. Failure blocks approval/default activation; a scripted response is never real-model evidence.

Development examples are in `tests/phase21b.rs`; historical fixtures from Phases 2/2.1 remain byte-for-byte intact, including the initial sample's independence limitation and all previous misses. The former three omissions are explicitly regression cases. A fresh final dataset was authored after production algorithm freeze, checked for exact overlap with previous corpus texts and pre-freeze development literals, then sealed before execution. No production extraction/ranking algorithm was changed after its results were opened.

Final dataset: `phase21b-final-v1`, 40 interactions, 21 expected claims, SHA-256 `5db97e439c1d8b4e7ee5f9e666983f034e9194e9940169c2565fdcc154d85dc0`. The manifest records version, count, scoring and raw working-source freeze hashes. Dataset hash is enforced by the test. Scoring is one-to-one typed claims after real SQLite commit and scope/project filtering: category/entity plus specified property/value/temporal/correction constraints; unmatched actual=FP and unmatched expected=miss. Multiple claims are scored separately. Per-kind and language counts are retained. Not every optional field is specified for unconstrained goal/solution text.

Rules: 21 TP/0 FP/0 misses, precision/recall 1.0; critical rejected-case writes 0. Hybrid with an explicitly empty simulated LLM and grounded-rule fallback has the same measured result. This is a fallback-control score, not proof of LLM reasoning. Simulated LLM protocol tests are separate; real LLM/embedding metrics remain null. The historical 20-case regression now retrieves its 10 expected claims, without relabelling. Improvements there are regression coverage, not an independent generalization estimate.

The final corpus covers positive/negative preferences, hardware, temporary facts, decisions/projects, goals/completion, tasks, solutions and hypotheses, informal/formal Portuguese, English, compact units/abbreviations, multiple topics and critical rejected sources. This small bounded corpus does not validate arbitrary natural language, general factuality, every category or every dialect. Project facts/constraints and manual summaries also retain prior tests; absent categories are not assigned invented scores.

## Expanded retrieval and naturalness limits

A separate 24-query, eight-concept comparison uses the same data for FTS5, semantic-only and weighted hybrid modes, with exact words, lexical-free paraphrases and unrelated questions. Precision is 1.0 in all three; FTS5 recall .50, semantic/hybrid recall 1.0. Zero irrelevant results in this fixture. Empirical Wilson 95% intervals are reported: FTS5 recall approximately [.280,.720], semantic/hybrid recall [.806,1.0]; precision intervals reflect only 8 or 16 retrieved results. These are descriptive intervals for a handcrafted synthetic fixture, not population guarantees. Hybrid ties semantic here; no Nomic superiority or general hybrid advantage is established.

Scope, project, current state, expiry, conflicts, deletion, version/fingerprint, diversity and budget filters remain mandatory. Existing and new tests exercise those restrictions and empty memory context on irrelevant questions. Completed goals do not appear as pending goals, prior capacities are suppressed while updates are reviewed, and deletion/replay remains fenced. Instructions still permit provenance phrases when appropriate; there is no blind phrase ban. Naturalness of long real generation remains unmeasured.

## Safe live-model preparation

Read-only preflight inspected target process names/listeners, free RAM/VRAM and GGUF filename/size metadata without executing launchers, reading personal chats/configuration, loading models, hashing operational weights or changing inference/context. Available filename/size metadata is not proof of model identity or suitability. No safe authorized isolated live endpoint was established; Nomic weights were not identified in that limited inventory. This is not a claim that no model exists anywhere else.

`scripts/lucille-real-preflight.ps1` is reproducible read-only inspection of the marked QA root. It never starts a model, sends inference or changes context/ports. Optional contracts must be inside that root and use exclusive loopback contract ports. The example contract deliberately contains invalid null revisions/fingerprints, keeps Gemma context 65536 and records Nomic prefixes/pooling with unknown effective dimension/normalization/auth. Even a syntactically valid contract is explicitly **not runtime/weight attestation**. The existing transport remains simulated-only until independently verified model/endpoint/auth/transforms/resources and authorization are supplied.

File size does not establish RAM/VRAM fit, especially for a 65,536-token KV cache. Architecture/cache parameters, quantization, runtime identity, cancellation behavior and actual CPU/RAM/VRAM costs still need safe real verification. Windows 10/i7-3770 debug execution adds no AVX2 requirement, but real Gemma/Nomic inference compatibility is not proven by client tests. No additional model or operational instance was started.

## Full GUI, persistence and publication

The complete isolated app, actual composers/hooks and native SQLite were used. Prior Chat/Agent/Code and LLM/hybrid protocol regressions remain. New GUI coverage checks coffee withdrawal/replacement, PC32/work-laptop8 separation, candidate upgrade hypothesis, completed-goal history, preference deletion/replay, HTTP embedding failure with lexical fallback, and a question receiving no irrelevant memory. All conversations are fictional and generated responses/vectors explicitly simulated. Native reopening resumes a scheduled persisted LLM fixture, retains identity/binding and purges source with one simulated request, not a model restart.

Tests cover existing migrations/backup, races, mode changes, cancellation, lease fencing and tombstones. The installed application, operational profiles/identity/SQLite/chats, GGUFs, C:\AI launchers, llama.cpp settings, ports 8080/8081/8128/8188 and Gemma context are untouched. Only manifest-identified QA processes are closed, and synthetic data is retained.

GitHub Actions remains independently audited. One dispatch refusal is sufficient evidence of the known global HTTP 422 block; do not repeatedly dispatch the same blocked workflows. No administrative changes, weaker gates or merge are permitted. Report the exact final HEAD and absence of checks honestly.

Result levels: synthetic implementation/quality can be approved only with the recorded passing gates and GUI/tests; real Gemma/Nomic extraction, embeddings, quality, naturalness and resource costs remain pending. Keep the stacked PR draft, without release/install/general activation.

## Local verification recorded for this change

57 Rust tests passed (24 baseline, 15 Phase 2, 10 Phase 2.1, 8 Phase 2.1B); all historical fixtures/tests remain. 159 TypeScript tests in 16 files passed with the real isolated native bridge. App and QA TypeScript checks, changed-file ESLint, app/QA frontend builds, native full-host/QA builds, three native boundary/mobile/resource guards and five dependency/security tests passed. The dependency graph check reported zero cycles; pre-existing chunk/dynamic-import warnings remain. Full GUI Phase 2, Phase 2.1, Phase 2.1B and approved file-tool regressions plus native queue reopening passed. These local checks do not replace missing remote CI or real-model validation.
