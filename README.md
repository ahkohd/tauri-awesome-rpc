# Tauri AwesomeRpc

This is a crate provides a custom invoke system for Tauri using a localhost json RPC websocket.
Each message is delivered through a `Websocket` using Parity JSON RPC 2.0 specification.

With the advantage of websocket, this library also provides a way for the Rust backend to emit events to the frontend using `AwesomeEmit` & `AwesomeEvent`.

## Usage

First, add the dependency to your `src-tauri/Cargo.toml` file:

```
[dependencies]
tauri-awesome-rpc = { git = "https://github.com/ahkohd/tauri-awesome-rpc", branch = "dev" }
```

Then, setup the HTTP invoke system on the `main.rs` file:

```rust
use serde_json::json;
use tauri::{Manager, Window, Wry};
use tauri_awesome_rpc::{AwesomeEmit, AwesomeRpc};

#[tauri::command]
fn my_command(args: u64) -> Result<String, ()> {
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
  let awesome_rpc = AwesomeRpc::new(vec!["tauri://localhost"]);

  tauri::Builder::default()
    .invoke_system(awesome_rpc.initialization_script(), AwesomeRpc::responder())
    .setup(move |app| {
      awesome_rpc.start(app.handle());
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![my_command, report_time_elapsed])
    .run(tauri::generate_context!())
    .expect("error while running tauri application")
}
```
