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
        $handle.state::<$crate::AwesomeEmit>()
            .emit_all($event, $payload)
    };
    
    // emit!(handle, window, event, payload) - emit to specific window
    ($handle:expr, $window:expr, $event:expr, $payload:expr) => {
        $handle.state::<$crate::AwesomeEmit>()
            .emit($window, $event, $payload)
    };
}

pub struct AwesomeRpc {
  port: u16,
  allowed_origins: DomainsValidation<Origin>,
  invoke_timeout: Duration,
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
    }
  }

  pub fn start<R: Runtime>(&self, app_handle: AppHandle<R>) {
    let handle = app_handle.clone();
    let timeout_duration = self.invoke_timeout;

    // Get the first allowed origin
    let origin_url = match &self.allowed_origins {
      DomainsValidation::AllowOnly(origins) => {
        origins.first()
          .map(|origin| origin.to_string())
          .expect("No allowed origins configured")
      },
      _ => panic!("Invalid allowed origins configuration")
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
                        serde_json::from_str(&json_str).unwrap_or_else(|_| Value::String(json_str))
                      },
                      tauri::ipc::InvokeResponseBody::Raw(bytes) => json!(bytes),
                    };
                    RpcResult {
                      status: RpcResponseStatus::Success,
                      data,
                    }
                  }
                  InvokeResponse::Err(tauri::ipc::InvokeError(e)) => {
                    RpcResult {
                      status: RpcResponseStatus::Error,
                      data: json!(e),
                    }
                  }
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
              }
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

    let server = ServerBuilder::new(io)
      .allowed_origins(self.allowed_origins.clone())
      .start(&format!("0.0.0.0:{}", self.port).as_str().parse().unwrap())
      .expect("RPC server must start with no issues");

    app_handle.manage(AwesomeEmit::new(server.broadcaster()));

    tauri::async_runtime::spawn(async { server.wait().unwrap() });
  }

  pub fn initialization_script(&self) -> String {
    include_str!("invoke_system.js")
      .replace("${AWESOME_RPC_PORT}", &self.port.to_string())
  }
}

#[derive(Serialize)]
struct AwesomeEvent<P> {
  event_name: String,
  window_label: Option<String>,
  payload: P,
}

#[derive(Clone)]
pub struct AwesomeEmit {
  broadcaster: Broadcaster,
}

impl AwesomeEmit {
  pub fn new(broadcaster: Broadcaster) -> Self {
    Self { broadcaster }
  }

  pub fn send<P: Serialize>(&self, payload: P) {
    self
      .broadcaster
      .send(serde_json::to_string(&payload).unwrap())
      .unwrap();
  }

  #[allow(dead_code)]
  pub fn emit_all<P: Serialize>(&self, name: &str, payload: P) {
    self
      .broadcaster
      .send(
        serde_json::to_string(&AwesomeEvent {
          event_name: name.into(),
          window_label: None,
          payload,
        })
        .unwrap(),
      )
      .unwrap();
  }

  #[allow(dead_code)]
  pub fn emit<P: Serialize>(&self, window_label: &str, name: &str, payload: P) {
    self
      .broadcaster
      .send(
        serde_json::to_string(&AwesomeEvent {
          event_name: name.into(),
          window_label: Some(window_label.into()),
          payload,
        })
        .unwrap(),
      )
      .unwrap();
  }
}
