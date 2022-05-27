use jsonrpc_ws_server::jsonrpc_core::{serde::Serialize, *};
use jsonrpc_ws_server::*;
use serde::Deserialize;
use serde_json::json;
use tauri::api::ipc::CallbackFn;
use tauri::{AppHandle, InvokePayload, InvokeResponder, InvokeResponse, Manager, Runtime, Window};

#[derive(Serialize, Deserialize)]
struct InvokeRpcParams {
  window_label: String,
  payload: String,
}

#[derive(Serialize, Deserialize)]
enum RpcResponseStatus {
  Processing,
  Success,
  Error,
  Invalid,
}

#[derive(Serialize, Deserialize)]
struct RpcResult {
  status: RpcResponseStatus,
  data: Value,
}

pub struct AwesomeRpc {
  port: u16,
  allowed_origins: DomainsValidation<Origin>,
}

impl AwesomeRpc {
  pub fn new(allowed_origins: Vec<&str>) -> Self {
    let port = portpicker::pick_unused_port().expect("failed to get unused port for invoke");
    let allowed_origins =
      DomainsValidation::AllowOnly(allowed_origins.iter().map(|i| i.into()).collect());

    Self {
      port,
      allowed_origins,
    }
  }

  pub fn start<R: Runtime>(&self, app_handle: AppHandle<R>) {
    let handle = app_handle.clone();

    let mut io = IoHandler::new();
    io.add_sync_method("invoke", move |params: Params| {
      let params = params.parse::<InvokeRpcParams>().unwrap();

      if let Some(window) = handle.get_window(params.window_label.as_str()) {
        let payload = serde_json::from_str::<InvokePayload>(params.payload.as_str()).unwrap();
        let _ = window.on_message(payload);

        return Ok(json!(RpcResult {
          status: RpcResponseStatus::Processing,
          data: Value::Null
        }));
      }

      Ok(json!(RpcResult {
        status: RpcResponseStatus::Invalid,
        data: Value::String("Malformed request".into())
      }))
    });

    let server = ServerBuilder::new(io)
      .allowed_origins(self.allowed_origins.clone())
      .max_connections((usize::MAX as f64 / 5.0) as usize)
      .start(&format!("0.0.0.0:{}", self.port).as_str().parse().unwrap())
      .expect("RPC server must start with no issues");

    app_handle.manage(AwesomeEmit::new(server.broadcaster().clone()));

    tauri::async_runtime::spawn(async { server.wait().unwrap() });
  }

  pub fn responder<R: Runtime>() -> Box<InvokeResponder<R>> {
    let responder = move |window: Window<R>,
                          response: InvokeResponse,
                          callback: CallbackFn,
                          error: CallbackFn| {
      let response = response.into_result();

      #[derive(Serialize, Deserialize)]
      struct JsonRpcResponse {
        jsonrpc: String,
        id: usize,
        result: RpcResult,
      }

      let result = match response {
        Ok(r) => RpcResult {
          status: RpcResponseStatus::Success,
          data: r,
        },
        Err(e) => RpcResult {
          status: RpcResponseStatus::Error,
          data: e,
        },
      };

      let r = JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id: callback.0 + error.0,
        result,
      };

      window.state::<AwesomeEmit>().send(r);
    };

    Box::new(responder)
  }

  pub fn initialization_script(&self) -> String {
    format!(
      "
      Object.defineProperty(window, '__TAURI_POST_MESSAGE__', {{
        value: (message) => {{
          const ws = new WebSocket('ws://localhost:{}', \"json\");
          const rpcMethodId = message.callback + message.error;

          ws.onmessage = function (event) {{
            let rpcMessage = JSON.parse(event.data);

            if (rpcMessage.id === rpcMethodId) {{
              if ([\"Invalid\", \"Error\"].includes(rpcMessage.result.status)) {{
                window[`_${{message.error}}`](rpcMessage.result.data);
                ws.close();
              }}

              if (rpcMessage.result.status === \"Success\") {{
                window[`_${{message.callback}}`](rpcMessage.result.data);
                ws.close();
              }}
            }}
          }};

        ws.onerror = (e) => {{
          ws.close();
          window[`_${{message.error}}`](e)
        }};


        ws.onopen = () => {{
          ws.send(
            JSON.stringify({{
              jsonrpc: \"2.0\",
              id: rpcMethodId,
              method: \"invoke\",
              params: {{window_label: window.__TAURI_METADATA__.__currentWindow.label, payload: JSON.stringify(message) }},
            }})
          );
        }};

        }}
      }});


      Object.defineProperty(window, 'AwesomeEvent', {{
        value: {{
          listen: (event_name, callback) => {{
            const ws = new WebSocket('ws://localhost:{}', \"json\");
            ws.onmessage = function (event) {{
              let message = JSON.parse(event.data);

              if (message.event_name && message.event_name === event_name && [null, window.__TAURI_METADATA__.__currentWindow.label].includes(message.window_label)) {{
                callback(message.payload);
              }}
            }};

            ws.onerror = (e) => {{
              ws.close();
            }};

            return () => ws.close();
          }}
        }}
      }})
    ",
      self.port, self.port
    )
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
