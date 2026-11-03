#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
//! Dedicated QA entry point. Never calls the production run/startup, plugins,
//! discovery, keychain, process management, installer, updater or model code.
use std::path::PathBuf;
use tauri::Manager;
#[path = "../../../src-tauri/src/lucille.rs"]
mod lucille;

#[tauri::command]
fn qa_file(
    window: tauri::WebviewWindow,
    operation: String,
    path: String,
    content: Option<String>,
) -> Result<String, String> {
    lucille_memory::isolation::validate_host(
        cfg!(debug_assertions),
        &window.app_handle().config().identifier,
        window.label(),
    )?;
    let root = PathBuf::from(std::env::var_os("LUCILLE_PHASE1_ROOT").ok_or("Missing QA root")?)
        .canonicalize()
        .map_err(|_| "Invalid QA root")?;
    let workspace = root
        .join("workspace")
        .canonicalize()
        .map_err(|_| "Missing synthetic workspace")?;
    // Deliberately small QA tool surface; no shell and no arbitrary directories.
    if path.is_empty()
        || std::path::Path::new(&path).is_absolute()
        || path.contains(['/', '\\', ':'])
        || path == ".."
    {
        return Err("Outside synthetic workspace".into());
    }
    let target = workspace.join(&path);
    if target.exists()
        && (target
            .symlink_metadata()
            .map_err(|_| "Invalid fixture")?
            .file_type()
            .is_symlink()
            || target
                .canonicalize()
                .map_err(|_| "Invalid fixture")?
                .parent()
                != Some(workspace.as_path()))
    {
        return Err("Outside synthetic workspace".into());
    }
    match operation.as_str() {
        "file_write" => {
            let content = content.ok_or("Content required")?;
            if content.len() > 16000 {
                return Err("Fixture too large".into());
            }
            std::fs::write(target, content).map_err(|_| "Synthetic write failed")?;
            Ok("Synthetic file created".into())
        }
        "file_read" => std::fs::read_to_string(target).map_err(|_| "Synthetic read failed".into()),
        "file_edit" => {
            let text = std::fs::read_to_string(&target).map_err(|_| "Synthetic read failed")?;
            std::fs::write(target, text.replace("blue", "green"))
                .map_err(|_| "Synthetic edit failed")?;
            Ok("Synthetic file edited".into())
        }
        _ => Err("Unsupported QA tool".into()),
    }
}

fn main() {
    assert!(cfg!(debug_assertions), "QA host cannot run as release");
    let root =
        PathBuf::from(std::env::var_os("LUCILLE_PHASE1_ROOT").expect("Explicit QA root required"));
    lucille_memory::isolation::isolated_database_path(&root, &[])
        .expect("Synthetic marker required");
    let root = root.canonicalize().expect("Canonical QA root");
    assert!(root
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("lucille-synthetic-"));
    for name in ["profiles", "workspace"] {
        std::fs::create_dir_all(root.join(name)).expect("QA directory");
    }
    let profile = root.join("profiles");
    tauri::Builder::default()
        .manage(lucille::LucilleState::default())
        .invoke_handler(tauri::generate_handler![
            lucille::lucille_context,
            lucille::lucille_manage,
            qa_file
        ])
        .setup(move |app| {
            let window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("qa/lucille.html".into()),
            )
            .title("Lucille Phase 1.5 - Synthetic QA ONLY")
            .data_directory(profile.clone())
            .inner_size(1120.0, 850.0)
            .build()?;
            // The launcher hides the console/startup window; explicitly show this QA WebView.
            window.show()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Isolated QA host");
}
