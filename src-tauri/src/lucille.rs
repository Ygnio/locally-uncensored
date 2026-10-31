//! Phase 1 IPC is available only in an explicitly identified, isolated debug host.
//! No database is opened at app startup. No command is registered as a model tool.
use lucille_memory::{Action, Candidate, ConfigurationPermit, Profile, Store, TurnRequest};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Mutex};
use tauri::Manager;

#[derive(Default)]
pub struct LucilleState(pub Mutex<Option<Store>>);

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
        Request::Identity | Request::List { .. } | Request::History { .. } | Request::Integrity
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
