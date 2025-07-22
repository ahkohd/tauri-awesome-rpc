#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use serde_json::json;
use tauri::{Manager, WebviewWindow};
use tauri_awesome_rpc::{AwesomeRpc, emit, listen, once};

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

    // Emit test event to demonstrate backend listening
    emit!(window, "time_reporter_started", json!(start_time.elapsed()));

    loop {
      interval.tick().await;

      // Use the emit! macro for cleaner syntax
      emit!(window, "main", "time_elapsed", json!(start_time.elapsed()));
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

      // Example of backend event listening using macros
      let handle = app.handle();
      
      // Listen for time reporter start event (only once)
      let _unlisten_once = once!(handle, "time_reporter_started", |payload| {
          println!("Time reporter started! Payload: {:?}", payload);
      });

      // Listen to time elapsed events continuously
      let _unlisten = listen!(handle, "time_elapsed", |payload| {
          println!("Time elapsed: {:?}", payload);
      });

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![test_command, report_time_elapsed])
    .run(tauri::generate_context!())
    .expect("error while running tauri application")
}
