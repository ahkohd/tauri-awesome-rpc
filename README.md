# Tauri Invoke HTTP

This is a crate t at provides a custom invoke system for Tauri using a localhost server.
Each message is delivered through a `XMLHttpRequest` and the server is responsible for replying to it.

## Usage

First, add the dependency to your `src-tauri/Cargo.toml` file:

```
[dependencies]
tauri-awesome-rpc = { git = "https://github.com/tauri-apps/tauri-awesome-rpc", branch = "dev" }
```

Then, setup the HTTP invoke system on the `main.rs` file:

```rust
fn main() {
  // initialize the custom invoke system as a HTTP server, allowing the given origins to access it.
  let http = tauri_awesome_rpc::Invoke::new(if cfg!(feature = "custom-protocol") {
    ["tauri://localhost"]
  } else {
    ["http://localhost:8080"]
  });
  tauri::Builder::default()
    .invoke_system(http.initialization_script(), http.responder())
    .setup(move |app| {
      http.start(app.handle());
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application")
}
```
