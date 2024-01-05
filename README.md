# 😎 tauri-awesome-rpc

This is a crate provides a custom invoke system for Tauri using a localhost JSON RPC WebSocket.
Each message is delivered through Websocket using JSON RPC 2.0 [specification](https://www.jsonrpc.org/specification).

With the advantage of using websocket, `tauri-awesome-rpc` also provides an event API. With `AwesomeEmit` you can emit event from the Rust backend and `AwesomeEvent` to listen to the event on the frontend.

## Usage 🔧

First, add the dependency to your `src-tauri/Cargo.toml` file:

```
[dependencies]
tauri-awesome-rpc = { git = "https://github.com/ahkohd/tauri-awesome-rpc", branch = "dev" }
```

Then, setup the Websocket JSON RPC invoke system on the `main.rs` file:

```rust
use tauri::{Manager, Window, Wry};
use tauri_awesome_rpc::{AwesomeEmit, AwesomeRpc};
use serde_json::json;

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
    .invoke_handler(tauri::generate_handler![my_command, report_time_elapsed])
    .run(tauri::generate_context!())
    .expect("error while running tauri application")
}

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

      // emit an awesome event to the main window
      window
        .state::<AwesomeEmit>()
        .emit("main", "time_elapsed", json!(start_time.elapsed()));
    }
  });
}
```

Then, on the frontend:

```html
<html>
  <body>
    <div>
      <h1>tauri-awesome-rpc</h1>

      <h5>invoke test</h5>
      <div id="response"></div>

      <h5>AwesomeEvent.listen test</h5>
      <div id="time_elapsed"></div>
    </div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- Use your Tauri `invoke` method as usual.
- Use `window.AwesomeEvent` to listen to the events emitted using `AwesomeEmit` from the Rust backend.

```ts
import { invoke } from "@tauri-apps/api/tauri";

const response = document.getElementById("response") as HTMLDivElement;
const timeElapsed = document.getElementById("time_elapsed") as HTMLDivElement;

invoke("test_command", { args: 5 })
  .then((data) => {
    response.innerText = data as string;
  })
  .catch(console.error);

invoke("report_time_elapsed");

const _unsubscribe = window.AwesomeEvent.listen("time_elapsed", (data) => {
  timeElapsed.innerText = JSON.stringify(data);
});
```

Add the following type definition to your project's `global.d.ts` file:

```typescript
interface Window {
  AwesomeEvent: {
    listen(eventName: string, callback: (data) => void): () => void;
  };
}
```
