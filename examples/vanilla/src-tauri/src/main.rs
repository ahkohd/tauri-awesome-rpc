#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use serde_json::json;
use tauri::{Manager, Window, Wry};
use tauri_awesome_rpc::{AwesomeEmit, AwesomeRpc};

#[tauri::command]
fn test_command(args: u64) -> Result<String, ()> {
  println!("executed command with args {:?}", args);
  Ok("executed".into())
}

#[tauri::command]
fn report_time_elapsed(window: Window<Wry>) {
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
  #[cfg(dev)]
  let allowed_domain = {
    let config: tauri_utils::config::Config = serde_json::from_value(
      tauri_utils::config::parse::read_from(std::env::current_dir().unwrap()).unwrap(),
    )
    .unwrap();
    config.build.dev_path.to_string()
  };

  #[cfg(not(dev))]
  let allowed_domain = "tauri://localhost".to_string();

  let awesome_rpc = AwesomeRpc::new(vec![&allowed_domain]);

  tauri::Builder::default()
    .invoke_system(awesome_rpc.initialization_script(), AwesomeRpc::responder())
    .setup(move |app| {
      awesome_rpc.start(app.handle());
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![test_command, report_time_elapsed])
    .run(tauri::generate_context!())
    .expect("error while running tauri application")
}
