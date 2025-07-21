#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use serde_json::json;
use tauri::{Manager, WebviewWindow};
use tauri_awesome_rpc::{AwesomeEmit, AwesomeRpc};

#[tauri::command]
fn test_command(args: u64) -> Result<String, ()> {
  println!("executed command with args {:?}", args);
  Ok("executed".into())
}

#[tauri::command]
fn report_time_elapsed(window: WebviewWindow) {
  tauri::async_runtime::spawn(async move {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(250));
    let start_time = std::time::Instant::now();

    loop {
      interval.tick().await;

      window
        .state::<AwesomeEmit>()
        .emit("main", "time_elapsed", json!(start_time.elapsed()));
    }
  });
}

fn main() {
  let allowed_origins = if cfg!(dev) {
    vec!["http://localhost:1420", "http://localhost:5173"]
  } else {
    vec!["tauri://localhost", "https://tauri.localhost"]
  };

  let awesome_rpc = AwesomeRpc::new(allowed_origins);

  tauri::Builder::default()
    .invoke_system(awesome_rpc.initialization_script())
    .setup(move |app| {
      awesome_rpc.start(app.handle().clone());
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![test_command, report_time_elapsed])
    .run(tauri::generate_context!())
    .expect("error while running tauri application")
}