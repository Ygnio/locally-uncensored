//! Opt-in, debug-only host of the COMPLETE application frontend.
//! Does not enter production run/setup/shutdown or register lifecycle plugins.
use crate::{
    commands::{agent, filesystem, proxy},
    lucille,
    state::AppState,
};
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

static ROOT: OnceLock<PathBuf> = OnceLock::new();
pub(crate) fn path(name: &str) -> Option<PathBuf> {
    ROOT.get().map(|root| root.join(name))
}

fn allowed(command: &str, body: &serde_json::Value) -> bool {
    match command {
        "lucille_context"
        | "lucille_manage"
        | "lucille_learn"
        | "lucille_learning_wake"
        | "fs_read"
        | "fs_write"
        | "fs_list"
        | "fs_search"
        | "fs_info"
        | "validate_workspace_folder"
        | "set_chat_workspace_override"
        | "get_chat_workspace_override"
        | "list_agent_workspaces"
        | "cancel_proxy_call"
        | "cancel_proxy_stream" => true,
        "proxy_localhost" | "proxy_localhost_stream" | "proxy_localhost_stream_chunked" => body
            .get("url")
            .and_then(|v| v.as_str())
            .and_then(|u| url::Url::parse(u).ok())
            .is_some_and(|url| {
                url.scheme() == "http"
                    && url.host_str() == Some("127.0.0.1")
                    && url.port() == Some(15284)
                    && url.username().is_empty()
                    && url.password().is_none()
            }),
        "register_openai_host" => body
            .get("host")
            .and_then(|v| v.as_str())
            .is_some_and(|h| h == "127.0.0.1" || h == "http://127.0.0.1:15284/v1"),
        _ => false,
    }
}

pub fn run() {
    assert!(cfg!(debug_assertions), "Validation cannot run as release");
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".lucille-qa-runtime")
        .canonicalize()
        .expect("Checkout QA parent");
    let root = PathBuf::from(
        std::env::var_os("LUCILLE_PHASE1_ROOT").expect("Explicit synthetic root required"),
    )
    .canonicalize()
    .expect("Existing synthetic root");
    assert_eq!(
        root.parent(),
        Some(base.as_path()),
        "Root must be a direct checkout QA child"
    );
    assert!(
        [
            "lucille-synthetic-full",
            "lucille-synthetic-full22",
            "lucille-synthetic-full22b",
            "lucille-synthetic-full22c"
        ]
        .iter()
        .any(|n| root.file_name().is_some_and(|v| v == *n)),
        "Explicit full-profile allowlist"
    );
    lucille_memory::isolation::isolated_database_path(&root, &[]).expect("Synthetic marker");
    ROOT.set(root.clone()).expect("One validation host");
    for name in ["home", "data", "cache", "config", "webview", "tmp"] {
        std::fs::create_dir_all(root.join(name)).expect("Validation directory");
    }
    let context = tauri::generate_context!("tauri.lucille-validation.conf.json");
    assert_eq!(context.config().identifier, "dev.ygnio.lucille.phase16");
    let handler: Box<dyn Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync> =
        Box::new(tauri::generate_handler![
            lucille::lucille_context,
            lucille::lucille_manage,
            lucille::lucille_learn,
            lucille::lucille_learning_wake,
            filesystem::fs_read,
            filesystem::fs_write,
            filesystem::fs_list,
            filesystem::fs_search,
            filesystem::fs_info,
            filesystem::validate_workspace_folder,
            agent::set_chat_workspace_override,
            agent::get_chat_workspace_override,
            agent::list_agent_workspaces,
            proxy::proxy_localhost,
            proxy::proxy_localhost_stream,
            proxy::proxy_localhost_stream_chunked,
            proxy::cancel_proxy_call,
            proxy::cancel_proxy_stream,
            proxy::register_openai_host
        ]);
    tauri::Builder::default()
        .plugin(crate::host_kind::plugin())
        .manage(AppState::for_lucille_validation())
        .manage(lucille::LucilleState::default())
        .invoke_handler(move |invoke| {
            let command = invoke.message.command();
            let empty = serde_json::Value::Null;
            let body = match invoke.message.payload() {
                tauri::ipc::InvokeBody::Json(body) => body,
                _ => &empty,
            };
            // Narrow harmless UI answers; no persisted configuration or discovery.
            let answer = match command {
                "is_onboarding_done" => Some(serde_json::json!(true)),
                "onboarding_window_open" => Some(serde_json::json!(false)),
                "show_window" => Some(serde_json::Value::Null),
                _ => None,
            };
            if let Some(value) = answer {
                invoke.resolver.resolve(value);
                return true;
            }
            if !allowed(command, body) {
                invoke
                    .resolver
                    .reject("Unavailable in isolated full-application validation");
                return true;
            }
            handler(invoke)
        })
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External("http://127.0.0.1:15283".parse().unwrap()),
            )
            .title(
                if root.file_name().is_some_and(|v|v=="lucille-synthetic-full22c") {
                    "Lucille Phase 2.2C - FULL application - synthetic ONLY"
                } else if root.file_name().is_some_and(|v|v=="lucille-synthetic-full22b") {
                    "Lucille Phase 2.2B - FULL application - synthetic ONLY"
                } else if root.file_name().is_some_and(|v|v=="lucille-synthetic-full22")
                {
                    "Lucille Phase 2.2 - FULL application - synthetic ONLY"
                } else {
                    "Lucille Phase 1.6 - FULL application - synthetic ONLY"
                },
            )
            .data_directory(root.join("webview"))
            // Direct WebView2 options keep inspection scoped to this QA host.
            // Recent runtimes can ignore environment-only debug overrides.
            .additional_browser_args("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --remote-debugging-address=127.0.0.1 --remote-debugging-port=15287")
            .initialization_script(include_str!("../../qa/full-bootstrap.js"))
            .on_navigation(|url| {
                url.scheme() == "http"
                    && url.host_str() == Some("127.0.0.1")
                    && url.port() == Some(15283)
            })
            .focused(false)
            .inner_size(1320.0, 900.0)
            .build()?
            .show()?;
            Ok(())
        })
        .run(context)
        .expect("Full isolated validation host");
}

#[cfg(test)]
mod tests {
    use super::allowed;
    use serde_json::json;
    #[test]
    fn boundary_refuses_lifecycle_shell_and_operational_network() {
        for command in [
            "start_ollama",
            "stop_bundled_engine",
            "execute_shell",
            "execute_code",
            "keychain_get",
            "fs_delete",
            "fs_whole_computer",
        ] {
            assert!(!allowed(command, &json!({})));
        }
        for url in [
            "http://127.0.0.1:8080/v1/chat/completions",
            "http://localhost:15284/v1/models",
            "https://example.com",
            "http://127.0.0.1:8188",
        ] {
            assert!(!allowed("proxy_localhost", &json!({"url":url})));
        }
        assert!(allowed(
            "proxy_localhost_stream_chunked",
            &json!({"url":"http://127.0.0.1:15284/v1/chat/completions"})
        ));
    }
}
