# tauri-awesome-rpc

A custom invoke system for Tauri v2 that replaces the default IPC with a WebSocket-based RPC system following the JSON-RPC 2.0 [specification](https://www.jsonrpc.org/specification).

## Features

- **🚀 JSON-RPC 2.0 over WebSocket** - Full spec compliance with request/response correlation
- **🔌 Single Persistent Connection** - Efficient connection management with automatic reconnection
- **📡 Request Multiplexing** - Multiple concurrent requests over one WebSocket connection  
- **⚡ Event System** - Alternative event API with both continuous and one-time listeners
- **🎯 Backend Event Listening** - Rust-side event subscriptions with convenient macros
- **⏱️ Configurable Timeouts** - Per-instance timeout configuration

## Installation

Add to your `src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri-awesome-rpc = { git = "https://github.com/ahkohd/tauri-awesome-rpc", branch = "v2" }
```

## Quick Start

### 1. Backend Setup

```rust
use tauri::Manager;
use tauri_awesome_rpc::{AwesomeRpc, EmitterExt, ListenerExt};
use serde_json::json;

fn main() {
  let allowed_origins = if cfg!(dev) {
    vec!["http://localhost:1420", "http://localhost:5173"]
  } else {
    vec!["tauri://localhost"]
  };

  let awesome_rpc = AwesomeRpc::new(allowed_origins);

  tauri::Builder::default()
    .invoke_system(awesome_rpc.initialization_script())
    .setup(move |app| {
      awesome_rpc.start(app.handle().clone());
      
      // Optional: Setup backend event listeners
      let handle = app.handle();
      
      let _unlisten = handle.listen("user-action", |payload| {
        println!("User action: {:?}", payload);
      });
      
      // Emit an event
      handle.emit("app-ready", json!({"version": "1.0.0"}));
      
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![my_command])
    .run(tauri::generate_context!())
    .expect("error while running tauri application")
}

#[tauri::command]
fn my_command(name: String) -> String {
  format!("Hello, {}!", name)
}
```

### 2. Frontend Setup

Add TypeScript types to your `global.d.ts`:

```typescript
interface Window {
  AwesomeListener: {
    listen(eventName: string, callback: (data: any) => void): () => void;
    once(eventName: string, callback: (data: any) => void): () => void;
  };
}
```

Use in your frontend code:

```typescript
import { invoke } from "@tauri-apps/api/tauri";

// Regular Tauri invoke - automatically uses WebSocket
const result = await invoke('my_command', { name: 'World' });

// Listen to backend events
const unlisten = window.AwesomeListener.listen('backend-event', (data) => {
  console.log('Received:', data);
});

// One-time event listener
window.AwesomeListener.once('app-ready', (data) => {
  console.log('App initialized:', data);
});
```

## API Reference

### Backend APIs

#### Using Extension Traits (Recommended)

```rust
use tauri_awesome_rpc::{EmitterExt, ListenerExt};
use serde_json::json;

// Emit events - looks just like Tauri's native API!
app_handle.emit("event-name", json!({"data": "value"}));
app_handle.emit_to("main", "event-name", json!({"data": "value"}));

// Listen to events
let unlisten = app_handle.listen("event-name", |payload| {
  println!("Event received: {:?}", payload);
});

let unlisten = app_handle.once("event-name", |payload| {
  println!("One-time event: {:?}", payload);
});

// Stop listening
unlisten();
```

**Note**: If you import both `tauri::Emitter` and `tauri_awesome_rpc::EmitterExt`, you'll get a conflict. Solutions:
- Use only `EmitterExt` for WebSocket transport
- Import with aliases: `use tauri::Emitter as TauriEmitter;`
- Use fully qualified syntax: `EmitterExt::emit(&handle, "event", data)?;`

#### Using Macros (Alternative)

```rust
use tauri_awesome_rpc::{emit, listen, once};
use serde_json::json;

// Emit events
emit!(app_handle, "event-name", json!({"data": "value"}));
emit!(app_handle, "main", "event-name", json!({"data": "value"}));

// Listen to events
let unlisten = listen!(app_handle, "event-name", |payload| {
  println!("Event received: {:?}", payload);
});

let unlisten = once!(app_handle, "event-name", |payload| {
  println!("One-time event: {:?}", payload);
});
```

### Frontend APIs

#### Using TypeScript Module

```typescript
import { listen, once } from '@tauri-awesome-rpc/api';

const unlisten = listen('event-name', (data) => {
  console.log('Event:', data);
});

const unlisten = once('startup-event', (data) => {
  console.log('Startup:', data);
});
```

#### Using Window API

```typescript
// Continuous listener
const unlisten = window.AwesomeListener.listen('event-name', (data) => {
  console.log('Event:', data);
});

// One-time listener  
const unlisten = window.AwesomeListener.once('event-name', (data) => {
  console.log('Once:', data);
});
```

## Advanced Configuration

### Custom Configuration

```rust
use std::time::Duration;

// Default configuration
let awesome_rpc = AwesomeRpc::new(allowed_origins);

// Custom configuration with builder pattern
let awesome_rpc = AwesomeRpc::new(allowed_origins)
  .invoke_timeout(Duration::from_secs(60));  // Default: 30 seconds
```

### WebSocket Buffer Configuration

Configure WebSocket buffer capacities to handle large payloads:

```rust
let awesome_rpc = AwesomeRpc::new(allowed_origins)
  .max_connections(2)                          // Default: 1 (single app connection)
  .max_payload(50 * 1024 * 1024)             // Default: 10MB
  .max_in_buffer_capacity(100 * 1024 * 1024)  // Default: 10MB
  .max_out_buffer_capacity(100 * 1024 * 1024); // Default: 10MB
```

This is particularly useful when:
- Reading large files through the RPC system
- Handling high-volume data transfers
- Supporting many concurrent connections

### Environment-based Origins

```rust
let allowed_origins = if cfg!(dev) {
  vec![
    "http://localhost:1420",     // Vite
    "http://localhost:5173",     // Vite alternative
    "http://localhost:3000"      // Next.js
  ]
} else {
  vec!["tauri://localhost"]      // Production
};
```

## How It Works

### Architecture

1. **WebSocket Server**: Runs on a dynamically allocated port
2. **Invoke Interception**: JavaScript `postMessage` calls are redirected to WebSocket
3. **JSON-RPC Protocol**: All messages follow JSON-RPC 2.0 specification
4. **Event Bus**: Internal broadcast channel for backend event distribution

### Integration Details

#### Frontend → Backend

All standard Tauri APIs automatically use WebSocket:
- `invoke()` commands
- `emit()` / `emitTo()` from JavaScript  
- Plugin invocations

#### Backend → Frontend

When using awesome-rpc with extension traits:

```rust
use tauri_awesome_rpc::{EmitterExt, ListenerExt};

// When extension traits are imported, these use WebSocket
app_handle.emit("event", data);
app_handle.emit_to("main", "event", data);

let unlisten = app_handle.listen("event", |payload| { 
  println!("Received: {:?}", payload);
});

let unlisten_once = app_handle.once("startup", |payload| { 
  println!("Startup event: {:?}", payload);
});

// Without importing the extension traits, emit would use Tauri's built-in IPC
```

## Important Notes

### Event System Behavior

When you import `tauri_awesome_rpc::EmitterExt`:
- `app_handle.emit()`, `app_handle.emit_to()` uses WebSocket transport
- `window.emit()`, `window.emit_to()` uses WebSocket transport

When you import `tauri_awesome_rpc::ListenerExt`:
- `app_handle.listen()`, `app_handle.once()` listens to WebSocket events
- `window.listen()`, `window.once()` listens to WebSocket events
- These shadow Tauri's built-in event listeners

Without importing the extension traits:
- `app_handle.emit()` uses Tauri's standard IPC
- `app_handle.listen()`, `window.listen()` use Tauri's standard event system
- You can still use macros: `emit!`, `listen!`, `once!` for WebSocket transport

### Command Execution Context

All commands invoked through awesome-rpc run in an async context, not on the main thread. This differs from Tauri's default behavior where sync commands run on the main thread.

**This is actually a benefit!** Running commands async by default:
- Prevents accidental blocking of the main thread with long-running sync operations
- Keeps the UI responsive during command execution
- Makes main thread usage explicit and intentional
- Makes debugging easier - explicit `run_on_main_thread` calls are easy to find and audit

If you need main thread execution (e.g., for UI operations):

```rust
#[tauri::command]
fn my_sync_command(app: tauri::AppHandle) {
  // This runs async with awesome-rpc
  
  // To run on main thread when needed:
  app.run_on_main_thread(|| {
    // UI operations or other main-thread-only code
    println!("Running on main thread!");
  });
}
```

This is particularly important for:
- Platform-specific UI operations
- Libraries that require main thread access
- Code that depends on sync command behavior

The explicit `run_on_main_thread` pattern makes it clear which operations need main thread access, improving code maintainability.

## Examples

Check out the [complete example](./examples/vanilla) for a working demo.
