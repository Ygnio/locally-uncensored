//! Phase 1 IPC is available only in an explicitly identified, isolated debug host.
//! No database is opened at app startup. No command is registered as a model tool.
use lucille_memory::{Action, Candidate, ConfigurationPermit, Profile, Store, TurnRequest};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::Manager;

#[derive(Default)]
pub struct LucilleState(pub Mutex<Option<Store>>, pub AtomicBool);

fn start_learning_worker(app: tauri::AppHandle) {
    if app
        .state::<LucilleState>()
        .inner()
        .1
        .swap(true, Ordering::AcqRel)
    {
        return;
    }
    std::thread::spawn(move || {
        while app.get_webview_window("main").is_some() {
            let lease = {
                let state = app.state::<LucilleState>();
                let Ok(mut guard) = state.inner().0.lock() else {
                    break;
                };
                guard
                    .as_mut()
                    .and_then(|store| store.claim_learning(epoch()).ok())
                    .flatten()
            };
            if let Some(lease) = lease {
                let state = app.state::<LucilleState>();
                if let Ok(mut guard) = state.inner().0.lock() {
                    if let Some(store) = guard.as_mut() {
                        if store
                            .complete_learning(&lease, &lease.claims, epoch())
                            .is_err()
                        {
                            let _ = store.fail_learning(&lease, epoch());
                        }
                    }
                }
            } else {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
        app.state::<LucilleState>()
            .inner()
            .1
            .store(false, Ordering::Release);
    });
}
fn epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
#[tauri::command]
pub fn lucille_learn(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, LucilleState>,
    interaction: lucille_memory::intelligent::Interaction,
) -> Result<Value, String> {
    let path = isolated_root(&window)?;
    let result = {
        let mut guard = state
            .inner()
            .0
            .lock()
            .map_err(|_| "Learning state unavailable")?;
        if guard.is_none() {
            *guard = Some(Store::open(&path)?)
        }
        guard.as_mut().unwrap().enqueue_learning(interaction)?
    };
    if result["state"] != "disabled" && result["state"] != "ignored" {
        start_learning_worker(window.app_handle().clone());
    }
    Ok(result)
}
#[tauri::command]
pub fn lucille_learning_wake(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, LucilleState>,
) -> Result<(), String> {
    let path = isolated_root(&window)?;
    let enabled = {
        let mut guard = state
            .inner()
            .0
            .lock()
            .map_err(|_| "Learning state unavailable")?;
        if guard.is_none() {
            *guard = Some(Store::open(&path)?)
        }
        guard.as_ref().unwrap().learning_policy()?.enabled
    };
    if enabled {
        start_learning_worker(window.app_handle().clone())
    }
    Ok(())
}

fn isolated_root(window: &tauri::WebviewWindow) -> Result<PathBuf, String> {
    lucille_memory::isolation::validate_host(
        cfg!(debug_assertions),
        &window.app_handle().config().identifier,
        window.label(),
    )?;
    let root = PathBuf::from(
        std::env::var_os("LUCILLE_PHASE1_ROOT").ok_or("Isolated root not configured")?,
    );
    let protected = [
        "APPDATA",
        "LOCALAPPDATA",
        "ProgramFiles",
        "ProgramFiles(x86)",
    ]
    .into_iter()
    .filter_map(std::env::var_os)
    .map(PathBuf::from)
    .collect::<Vec<_>>();
    lucille_memory::isolation::isolated_database_path(&root, &protected)
}

#[tauri::command]
pub fn lucille_context(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, LucilleState>,
    request: TurnRequest,
) -> Result<Value, String> {
    let path = isolated_root(&window)?;
    let mut guard = state
        .inner()
        .0
        .lock()
        .map_err(|_| "Lucille state unavailable")?;
    if guard.is_none() {
        *guard = Some(Store::open(&path)?);
    }
    serde_json::to_value(guard.as_mut().unwrap().context(request)?)
        .map_err(|_| "Context serialization failed".into())
}

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    LearningPolicy,
    SaveLearningPolicy {
        policy: lucille_memory::intelligent::LearningPolicy,
    },
    LearningJobs {
        scope: String,
        project: String,
    },
    CancelLearning {
        id: String,
    },
    Details {
        scope: String,
        project: String,
    },
    Deleted {
        scope: String,
        project: String,
    },
    UndoConsolidation {
        id: String,
        scope: String,
        project: String,
        expected_version: i64,
        revision: i64,
    },
    ResolveConflict {
        id: String,
        scope: String,
        project: String,
        expected_version: i64,
        prior_version: i64,
    },
    Identity,
    ConfigureIdentity {
        profile: Profile,
        expected_version: i64,
    },
    ResetFixture {
        expected_version: i64,
    },
    List {
        scope: String,
        project: String,
    },
    Create {
        candidate: Candidate,
    },
    Change {
        id: String,
        scope: String,
        project: String,
        expected_version: i64,
        change: Action,
    },
    History {
        id: String,
    },
    Export,
    Restore {
        backup: String,
    },
    Integrity,
}

#[tauri::command]
pub fn lucille_manage(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, LucilleState>,
    request: Request,
    confirmation: Option<String>,
) -> Result<Value, String> {
    let readonly = matches!(
        &request,
        Request::Identity
            | Request::List { .. }
            | Request::History { .. }
            | Request::Integrity
            | Request::LearningPolicy
            | Request::LearningJobs { .. }
            | Request::Details { .. }
            | Request::Deleted { .. }
    );
    let permit = ConfigurationPermit::explicit_settings_action(
        readonly || confirmation.as_deref() == Some("CONFIRM LUCILLE CONFIGURATION"),
    )?;
    let path = isolated_root(&window)?;
    let mut guard = state
        .inner()
        .0
        .lock()
        .map_err(|_| "Lucille state unavailable")?;
    if guard.is_none() {
        *guard = Some(Store::open(&path)?);
    }
    let store = guard.as_mut().unwrap();
    let value = match request {
        Request::LearningPolicy => json!(store.learning_policy()?),
        Request::SaveLearningPolicy { policy } => {
            store.set_learning_policy(policy, &permit)?;
            json!({"saved":true})
        }
        Request::LearningJobs { scope, project } => json!(store.learning_jobs(&scope, &project)?),
        Request::CancelLearning { id } => json!({"cancelled":store.cancel_learning(&id,&permit)?}),
        Request::Details { scope, project } => json!(store.memory_details(&scope, &project)?),
        Request::Deleted { scope, project } => json!(store.deleted_memories(&scope, &project)?),
        Request::UndoConsolidation {
            id,
            scope,
            project,
            expected_version,
            revision,
        } => json!(store.undo_consolidation(
            &id,
            &scope,
            &project,
            expected_version,
            revision,
            &permit
        )?),
        Request::ResolveConflict {
            id,
            scope,
            project,
            expected_version,
            prior_version,
        } => json!(store.resolve_conflict(
            &id,
            &scope,
            &project,
            expected_version,
            prior_version,
            &permit
        )?),
        Request::Identity => json!(store.identity()?),
        Request::ConfigureIdentity {
            profile,
            expected_version,
        } => json!(store.configure_identity(profile, expected_version, &permit)?),
        Request::ResetFixture { expected_version } => {
            json!(store.configure_identity(Profile::fixture(), expected_version, &permit)?)
        }
        Request::List { scope, project } => json!(store.list(&scope, &project)?),
        Request::Create { candidate } => json!(store.create_candidate(candidate)?),
        Request::Change {
            id,
            scope,
            project,
            expected_version,
            change,
        } => json!(store.change(&id, &scope, &project, expected_version, change)?),
        Request::History { id } => json!(store.history(&id)?),
        Request::Export => json!(store.export_backup(&permit)?),
        Request::Restore { backup } => {
            store.restore_isolated(&backup, &permit)?;
            json!({"restored":true})
        }
        Request::Integrity => {
            store.check_integrity()?;
            json!({"ok":true})
        }
    };
    Ok(value)
}
