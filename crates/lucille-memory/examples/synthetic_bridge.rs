//! Test transport only: stdin JSON -> real Rust/SQLite service -> stdout JSON.
//! Does not start Tauri, a server or an inference engine.
use lucille_memory::*;
use serde_json::{json, Value};
use std::{io::Read, path::PathBuf};
fn run() -> std::result::Result<Value, String> {
    let root =
        PathBuf::from(std::env::var_os("LUCILLE_QA_ROOT").ok_or("Synthetic QA root required")?);
    if !root.is_absolute()
        || root
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| !s.starts_with("lucille-synthetic-"))
            .unwrap_or(true)
    {
        return Err("Synthetic QA directory required".into());
    }
    let mut input = String::new();
    std::io::stdin()
        .take(1024 * 1024)
        .read_to_string(&mut input)
        .map_err(|_| "Input failed")?;
    let command: Value = serde_json::from_str(&input).map_err(|_| "Invalid command")?;
    let mut s = Store::open(&root.join("synthetic.sqlite3"))?;
    let permit = ConfigurationPermit::explicit_settings_action(true)?;
    match command["operation"].as_str() {
        Some("seed") => {
            s.configure_identity(Profile::fixture(), 0, &permit)?;
            for c in command["candidates"]
                .as_array()
                .ok_or("Missing candidates")?
            {
                let c: Candidate =
                    serde_json::from_value(c.clone()).map_err(|_| "Invalid fixture")?;
                let m = s.create_candidate(c)?;
                s.change(
                    &m.input.id,
                    &m.input.scope,
                    &m.input.project,
                    m.version,
                    Action::Approve,
                )?;
            }
            Ok(json!({"seeded":true}))
        }
        Some("context") => Ok(json!(s.context(
            serde_json::from_value(command["request"].clone()).map_err(|_| "Invalid request")?
        )?)),
        Some("delete") => {
            s.change(
                command["id"].as_str().ok_or("Missing ID")?,
                "lucille",
                "synthetic-a",
                2,
                Action::Delete,
            )?;
            Ok(json!({"deleted":true}))
        }
        _ => Err("Unsupported synthetic operation".into()),
    }
}
fn main() {
    match run() {
        Ok(v) => println!("{v}"),
        Err(_) => {
            eprintln!("Synthetic bridge failed");
            std::process::exit(1);
        }
    }
}
