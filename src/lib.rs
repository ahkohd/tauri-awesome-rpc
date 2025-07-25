use jsonrpc_ws_server::jsonrpc_core::*;
use jsonrpc_ws_server::*;
use serde_json::json;
use tauri::{
  http::HeaderMap,
  ipc::{CallbackFn, InvokeBody, InvokeResponse},
  webview::InvokeRequest,
  AppHandle, Manager, Runtime, Url,
};

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::timeout;

#[derive(Deserialize)]
struct InvokeRpcParams {
  window_label: String,
  payload: String,
}

#[derive(Debug, Deserialize)]
pub struct InvokeRpcPayload {
  pub cmd: String,
  pub callback: u32,
  pub error: u32,
  pub payload: Value,
  pub invoke_key: String,
}

#[derive(Serialize, Deserialize)]
enum RpcResponseStatus {
  Success,
  Error,
  Invalid,
}

#[derive(Serialize, Deserialize)]
struct RpcResult {
  status: RpcResponseStatus,
  data: Value,
}

/// Convenience macro for emitting events through AwesomeRpc WebSocket
///
/// # Examples
///
/// Emit to all windows:
/// ```rust,no_run
/// # use tauri::Manager;
/// # use serde_json::json;
/// emit!(app_handle, "event-name", json!({"data": "value"}));
/// ```
///
/// Emit to specific window:
/// ```rust,no_run
/// # use tauri::Manager;
/// # use serde_json::json;
/// emit!(app_handle, "main", "event-name", json!({"data": "value"}));
/// ```
#[macro_export]
macro_rules! emit {
  // emit!(handle, event, payload) - emit to all windows
  ($handle:expr, $event:expr, $payload:expr) => {
    $handle
      .state::<$crate::AwesomeEmitter>()
      .emit_all($event, $payload)
  };

  // emit!(handle, window, event, payload) - emit to specific window
  ($handle:expr, $window:expr, $event:expr, $payload:expr) => {
    $handle
      .state::<$crate::AwesomeEmitter>()
      .emit($window, $event, $payload)
  };
}

/// Convenience macro for listening to events through AwesomeRpc WebSocket
///
/// # Examples
///
/// Listen continuously:
/// ```rust,no_run
/// # use tauri::Manager;
/// let unlisten = listen!(app_handle, "event-name", |payload| {
///     println!("Received: {:?}", payload);
/// });
/// ```
#[macro_export]
macro_rules! listen {
  ($handle:expr, $event:expr, $handler:expr) => {
    $handle
      .state::<$crate::AwesomeListener>()
      .listen($event, $handler)
  };
}

/// Convenience macro for listening to events once through AwesomeRpc WebSocket
///
/// # Examples
///
/// ```rust,no_run
/// # use tauri::Manager;
/// let unlisten = once!(app_handle, "event-name", |payload| {
///     println!("Received once: {:?}", payload);
/// });
/// ```
#[macro_export]
macro_rules! once {
  ($handle:expr, $event:expr, $handler:expr) => {
    $handle
      .state::<$crate::AwesomeListener>()
      .once($event, $handler)
  };
}

pub struct AwesomeRpc {
  port: u16,
  allowed_origins: DomainsValidation<Origin>,
  invoke_timeout: Duration,
  max_connections: Option<usize>,
  max_payload: Option<usize>,
  max_in_buffer_capacity: Option<usize>,
  max_out_buffer_capacity: Option<usize>,
}

impl AwesomeRpc {
  pub fn new(allowed_origins: Vec<&str>) -> Self {
    Self::with_timeout(allowed_origins, Duration::from_secs(30)) // Default 30 second timeout
  }

  pub fn with_timeout(allowed_origins: Vec<&str>, invoke_timeout: Duration) -> Self {
    let port = portpicker::pick_unused_port().expect("failed to get unused port for invoke");
    let allowed_origins =
      DomainsValidation::AllowOnly(allowed_origins.iter().map(|i| i.into()).collect());

    Self {
      port,
      allowed_origins,
      invoke_timeout,
      max_connections: Some(1), // Default to 1 connection for single app
      max_payload: None,
      max_in_buffer_capacity: None,
      max_out_buffer_capacity: None,
    }
  }

  pub fn max_connections(mut self, max_connections: usize) -> Self {
    self.max_connections = Some(max_connections);
    self
  }

  pub fn max_payload(mut self, max_payload: usize) -> Self {
    self.max_payload = Some(max_payload);
    self
  }

  pub fn max_in_buffer_capacity(mut self, max_in_buffer_capacity: usize) -> Self {
    self.max_in_buffer_capacity = Some(max_in_buffer_capacity);
    self
  }

  pub fn max_out_buffer_capacity(mut self, max_out_buffer_capacity: usize) -> Self {
    self.max_out_buffer_capacity = Some(max_out_buffer_capacity);
    self
  }

  pub fn start<R: Runtime>(&self, app_handle: AppHandle<R>) {
    let handle = app_handle.clone();
    let timeout_duration = self.invoke_timeout;

    // Get the first allowed origin
    let origin_url = match &self.allowed_origins {
      DomainsValidation::AllowOnly(origins) => origins
        .first()
        .map(|origin| origin.to_string())
        .expect("No allowed origins configured"),
      _ => panic!("Invalid allowed origins configuration"),
    };

    let mut io = IoHandler::new();
    let origin_url_clone = origin_url.to_string();
    io.add_method("invoke", move |params: Params| {
      let origin_url = origin_url_clone.clone();
      let handle = handle.clone();

      async move {
        let params = params.parse::<InvokeRpcParams>().unwrap();

        if let Some(window) = handle.get_webview_window(&params.window_label) {
          if let Ok(payload) = serde_json::from_str::<InvokeRpcPayload>(&params.payload) {
            let request = InvokeRequest {
              cmd: payload.cmd,
              callback: CallbackFn(payload.callback),
              error: CallbackFn(payload.error),
              url: Url::parse(&origin_url).expect("Invalid origin URL"),
              body: InvokeBody::Json(payload.payload),
              headers: HeaderMap::new(),
              invoke_key: payload.invoke_key,
            };

            let (tx, rx) = oneshot::channel();

            window.on_message(
              request,
              Box::new(move |_webview, _cmd, response, _callback, _error| {
                let result = match response {
                  InvokeResponse::Ok(body) => {
                    let data = match body {
                      tauri::ipc::InvokeResponseBody::Json(json_str) => {
                        serde_json::from_str(&json_str).unwrap_or(Value::String(json_str))
                      }
                      tauri::ipc::InvokeResponseBody::Raw(bytes) => json!(bytes),
                    };
                    RpcResult {
                      status: RpcResponseStatus::Success,
                      data,
                    }
                  }
                  InvokeResponse::Err(tauri::ipc::InvokeError(e)) => RpcResult {
                    status: RpcResponseStatus::Error,
                    data: json!(e),
                  },
                };

                let _ = tx.send(result);
              }),
            );

            // Wait for the response with timeout
            let result = match timeout(timeout_duration, rx).await {
              Ok(Ok(result)) => result,
              Ok(Err(_)) => RpcResult {
                status: RpcResponseStatus::Error,
                data: Value::String("Failed to receive response".into()),
              },
              Err(_) => RpcResult {
                status: RpcResponseStatus::Error,
                data: Value::String(format!("Request timed out after {:?}", timeout_duration)),
              },
            };

            Ok(json!(result))
          } else {
            Ok(json!(RpcResult {
              status: RpcResponseStatus::Invalid,
              data: Value::String("Invalid payload format".into()),
            }))
          }
        } else {
          Ok(json!(RpcResult {
            status: RpcResponseStatus::Invalid,
            data: Value::String("Window not found".into()),
          }))
        }
      }
    });

    let mut server_builder = ServerBuilder::new(io)
      .allowed_origins(self.allowed_origins.clone());

    if let Some(max_connections) = self.max_connections {
      server_builder = server_builder.max_connections(max_connections);
    }

    if let Some(max_payload) = self.max_payload {
      server_builder = server_builder.max_payload(max_payload);
    }

    if let Some(max_in_buffer_capacity) = self.max_in_buffer_capacity {
      server_builder = server_builder.max_in_buffer_capacity(max_in_buffer_capacity);
    }

    if let Some(max_out_buffer_capacity) = self.max_out_buffer_capacity {
      server_builder = server_builder.max_out_buffer_capacity(max_out_buffer_capacity);
    }

    let server = server_builder
      .start(&format!("0.0.0.0:{}", self.port).as_str().parse().unwrap())
      .expect("RPC server must start with no issues");

    // Create event bus for backend communication
    let (event_bus, _) = broadcast::channel(1000);

    // Manage both emitter and listener
    app_handle.manage(AwesomeEmitter::new(server.broadcaster(), event_bus.clone()));
    app_handle.manage(AwesomeListener::new(event_bus));

    tauri::async_runtime::spawn(async { server.wait().unwrap() });
  }

  pub fn initialization_script(&self) -> String {
    include_str!("invoke_system.js").replace("${AWESOME_RPC_PORT}", &self.port.to_string())
  }
}

#[derive(Serialize)]
struct AwesomeEvent<P> {
  event_name: String,
  window_label: Option<String>,
  payload: P,
}

use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AwesomeEmitter {
  broadcaster: Broadcaster,
  event_bus: broadcast::Sender<(String, Value)>,
}

impl AwesomeEmitter {
  pub fn new(broadcaster: Broadcaster, event_bus: broadcast::Sender<(String, Value)>) -> Self {
    Self {
      broadcaster,
      event_bus,
    }
  }

  pub fn emit<P: Serialize>(&self, window_label: &str, name: &str, payload: P) {
    let event = AwesomeEvent {
      event_name: name.into(),
      window_label: Some(window_label.into()),
      payload,
    };

    // Send to WebSocket clients
    self
      .broadcaster
      .send(serde_json::to_string(&event).unwrap())
      .unwrap();

    // Send to internal event bus
    let value = serde_json::to_value(&event.payload).unwrap();
    let _ = self.event_bus.send((name.to_string(), value));
  }

  pub fn emit_all<P: Serialize>(&self, name: &str, payload: P) {
    let event = AwesomeEvent {
      event_name: name.into(),
      window_label: None,
      payload,
    };

    // Send to WebSocket clients
    self
      .broadcaster
      .send(serde_json::to_string(&event).unwrap())
      .unwrap();

    // Send to internal event bus
    let value = serde_json::to_value(&event.payload).unwrap();
    let _ = self.event_bus.send((name.to_string(), value));
  }
}

#[derive(Clone)]
pub struct AwesomeListener {
  event_bus: broadcast::Sender<(String, Value)>,
}

impl AwesomeListener {
  pub fn new(event_bus: broadcast::Sender<(String, Value)>) -> Self {
    Self { event_bus }
  }

  /// Listen to events on the backend
  /// Returns an unlistener function
  pub fn listen<F>(&self, event_name: &str, handler: F) -> impl FnOnce() + Send + Sync + 'static
  where
    F: Fn(Value) + Send + Sync + 'static,
  {
    let event_name = event_name.to_string();
    let mut rx = self.event_bus.subscribe();

    // Spawn a task to handle events using tauri's async runtime
    let handle = tauri::async_runtime::spawn(async move {
      loop {
        match rx.recv().await {
          Ok((name, payload)) => {
            if name == event_name {
              handler(payload);
            }
          }
          Err(_) => break, // Channel closed
        }
      }
    });

    // Return unlistener that aborts the task
    move || {
      handle.abort();
    }
  }

  /// Listen to events only once on the backend
  /// Returns an unlistener function
  pub fn once<F>(&self, event_name: &str, handler: F) -> impl FnOnce() + Send + Sync + 'static
  where
    F: FnOnce(Value) + Send + Sync + 'static,
  {
    let event_name = event_name.to_string();
    let mut rx = self.event_bus.subscribe();

    // Spawn a task to handle the event once using tauri's async runtime
    let handle = tauri::async_runtime::spawn(async move {
      loop {
        match rx.recv().await {
          Ok((name, payload)) => {
            if name == event_name {
              handler(payload);
              break; // Stop after first event
            }
          }
          Err(_) => break, // Channel closed
        }
      }
    });

    // Return unlistener that aborts the task
    move || {
      handle.abort();
    }
  }
}

// Extension trait that shadows Tauri's emit methods
pub trait EmitterExt<R: Runtime> {
  fn emit<S: Serialize + Clone>(&self, event: &str, payload: S);
  fn emit_to<S: Serialize + Clone>(&self, window: &str, event: &str, payload: S);
}

impl<R: Runtime> EmitterExt<R> for tauri::AppHandle<R> {
  fn emit<S: Serialize + Clone>(&self, event: &str, payload: S) {
    self.state::<AwesomeEmitter>().emit_all(event, payload);
  }

  fn emit_to<S: Serialize + Clone>(&self, window: &str, event: &str, payload: S) {
    self.state::<AwesomeEmitter>().emit(window, event, payload);
  }
}

impl<R: Runtime> EmitterExt<R> for tauri::Window<R> {
  fn emit<S: Serialize + Clone>(&self, event: &str, payload: S) {
    self.state::<AwesomeEmitter>().emit_all(event, payload);
  }

  fn emit_to<S: Serialize + Clone>(&self, window: &str, event: &str, payload: S) {
    self.state::<AwesomeEmitter>().emit(window, event, payload);
  }
}

// Extension trait for listening to events
pub trait ListenerExt<R: Runtime> {
  fn listen<F>(&self, event: &str, handler: F) -> impl FnOnce() + Send + Sync + 'static
  where
    F: Fn(Value) + Send + Sync + 'static;

  fn once<F>(&self, event: &str, handler: F) -> impl FnOnce() + Send + Sync + 'static
  where
    F: FnOnce(Value) + Send + Sync + 'static;
}

impl<R: Runtime> ListenerExt<R> for tauri::AppHandle<R> {
  fn listen<F>(&self, event: &str, handler: F) -> impl FnOnce() + Send + Sync + 'static
  where
    F: Fn(Value) + Send + Sync + 'static,
  {
    self.state::<AwesomeListener>().listen(event, handler)
  }

  fn once<F>(&self, event: &str, handler: F) -> impl FnOnce() + Send + Sync + 'static
  where
    F: FnOnce(Value) + Send + Sync + 'static,
  {
    self.state::<AwesomeListener>().once(event, handler)
  }
}

impl<R: Runtime> ListenerExt<R> for tauri::Window<R> {
  fn listen<F>(&self, event: &str, handler: F) -> impl FnOnce() + Send + Sync + 'static
  where
    F: Fn(Value) + Send + Sync + 'static,
  {
    self.state::<AwesomeListener>().listen(event, handler)
  }

  fn once<F>(&self, event: &str, handler: F) -> impl FnOnce() + Send + Sync + 'static
  where
    F: FnOnce(Value) + Send + Sync + 'static,
  {
    self.state::<AwesomeListener>().once(event, handler)
  }
}
