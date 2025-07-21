# tauri-awesome-rpc

A crate that provides a custom invoke system for Tauri v2 using a localhost JSON-RPC WebSocket. Each message is delivered through WebSocket using the JSON-RPC 2.0 [specification](https://www.jsonrpc.org/specification).

## Features

- **JSON-RPC 2.0 over WebSocket** - Full compatibility with the JSON-RPC 2.0 specification
- **Single Persistent Connection** - Efficient connection management with automatic reconnection
- **Request Multiplexing** - Multiple concurrent requests over a single WebSocket connection
- **Async Support** - Non-blocking request handling with configurable timeouts

**Note: This version is compatible with Tauri v2.**

## Example

Check out the [example](./examples/vanilla) for a complete working demo.

## Usage

First, add the dependency to your `src-tauri/Cargo.toml` file:

```
[dependencies]
tauri-awesome-rpc = { git = "https://github.com/ahkohd/tauri-awesome-rpc", branch = "v2" }
```

Then, setup the Websocket JSON RPC invoke system on the `main.rs` file:

```rust
use tauri::{Manager, WebviewWindow};
use tauri_awesome_rpc::{AwesomeEmit, AwesomeRpc, emit};
use serde_json::json;

fn main() {
  let allowed_origins = if cfg!(dev) {
    vec!["http://localhost:1420", "http://localhost:5173"]
  } else {
    vec!["tauri://localhost"]
  };

  // Create with default 30 second timeout
  let awesome_rpc = AwesomeRpc::new(allowed_origins);
  
  // Or create with custom timeout
  // use std::time::Duration;
  // let awesome_rpc = AwesomeRpc::with_timeout(allowed_origins, Duration::from_secs(60));

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

      // IMPORTANT: Use AwesomeEmit to send events through WebSocket
      // Do NOT use window.emit() - that uses Tauri's built-in event system
      
      // Using the macro (recommended)
      emit!(window, "main", "time_elapsed", json!(start_time.elapsed()));
      
      // Or using the verbose method
      // window
      //   .state::<AwesomeEmit>()
      //   .emit("main", "time_elapsed", json!(start_time.elapsed()));
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

// Listen to events continuously
const unsubscribe = window.AwesomeEvent.listen("time_elapsed", (data) => {
  timeElapsed.innerText = JSON.stringify(data);
});

// Listen to an event only once
window.AwesomeEvent.once("app_ready", (data) => {
  console.log("App is ready:", data);
});

// Later: unsubscribe from continuous listener
// unsubscribe();
```

Add the following type definition to your project's `global.d.ts` file:

```typescript
interface Window {
  AwesomeEvent: {
    listen(eventName: string, callback: (data: any) => void): () => void;
    once(eventName: string, callback: (data: any) => void): () => void;
  };
}
```

### Using the TypeScript API

Alternatively, you can use the provided TypeScript API from the [`guest-js`](./guest-js/) folder:

```typescript
import { listen, once } from '@tauri-awesome-rpc/api';

// Listen continuously
const unlisten = listen('time_elapsed', (data) => {
  console.log('Time elapsed:', data);
});

// Listen only once
once('app_ready', (data) => {
  console.log('App ready event received once:', data);
});
```

## How It Works

1. **WebSocket Server**: A JSON-RPC WebSocket server runs on a dynamically allocated port
2. **Invoke Interception**: The initialization script intercepts Tauri's `postMessage` calls
3. **Request/Response Flow**: 
   - Frontend sends invoke requests via WebSocket as JSON-RPC messages
   - Rust backend processes the request through Tauri's standard invoke system
   - Responses are sent back through the same WebSocket connection
4. **Single Persistent Connection**: All requests and events share one WebSocket connection with automatic reconnection

### Integration with Tauri APIs

When tauri-awesome-rpc is configured, the integration works as follows:

#### Frontend → Backend (Automatic WebSocket)
All frontend-to-backend communication automatically uses WebSocket:

- **Commands**: `invoke()` calls use WebSocket instead of default IPC
- **Frontend Events**: `emit()`, `emitTo()` from JavaScript use WebSocket
- **Plugins**: Any Tauri plugin using the standard invoke system benefits from WebSocket transport

#### Backend → Frontend (Manual Choice Required)
For backend-to-frontend events, you must explicitly choose:

```rust
use tauri_awesome_rpc::emit;
use serde_json::json;

// ❌ This uses Tauri's built-in event system (NOT WebSocket)
app_handle.emit("my-event", payload)?;
window.emit("my-event", payload)?;

// ✅ This uses AwesomeRpc's WebSocket system (verbose)
app_handle.state::<AwesomeEmit>()
    .emit("window-label", "my-event", payload);

// ✅ This uses AwesomeRpc's WebSocket system (with macro)
emit!(app_handle, "my-event", payload);                     // Emit to all windows
emit!(app_handle, "main", "my-event", json!({"foo": 42}));  // Emit to specific window
```

**Important**: If you want all events to go through WebSocket, you must use `AwesomeEmit` in your Rust code instead of Tauri's built-in `emit()` methods.

#### Frontend Event Listening
Frontend event listeners work with both systems:

```typescript
// Frontend → Backend: automatically uses WebSocket
await invoke('my_command', { name: 'World' });
await emit('frontend-event', { data: 'Hello' });

// Listening to backend events (both systems work)
// If backend uses app_handle.emit() - receives via Tauri's system
// If backend uses AwesomeEmit - receives via WebSocket
await listen('backend-event', (event) => {
  console.log('Received:', event.payload);
});

// AwesomeEvent only receives events sent via AwesomeEmit
window.AwesomeEvent.listen('websocket-event', (data) => {
  console.log('Direct WebSocket event:', data);
});
```

### AwesomeEvent API

In addition to Tauri's built-in event system, tauri-awesome-rpc provides the `AwesomeEvent` API as an alternative event system:

```typescript
// AwesomeEvent - alternative event API with one-time listener support
window.AwesomeEvent.listen('realtime-data', (data) => {
  console.log('Event received:', data);
});

// One-time listeners with automatic cleanup
window.AwesomeEvent.once('app-ready', (data) => {
  console.log('App initialized:', data);
});
```

Use `AwesomeEvent` when you need:
- A simpler event API without Tauri's event wrapping
- One-time event listeners with automatic cleanup (`once()`)
- Direct access to the payload without the Tauri Event wrapper

## Configuration

### Timeout Configuration

You can configure the timeout for invoke requests:

```rust
use std::time::Duration;

// Default 30 second timeout
let awesome_rpc = AwesomeRpc::new(allowed_origins);

// Custom timeout with milliseconds precision
let awesome_rpc = AwesomeRpc::with_timeout(allowed_origins, Duration::from_millis(5500));
```

### Allowed Origins

Configure allowed origins based on your environment:

```rust
let allowed_origins = if cfg!(dev) {
  vec!["http://localhost:1420", "http://localhost:5173"]  // Development origins
} else {
  vec!["tauri://localhost"] // Production origins
};
```
