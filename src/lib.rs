// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
  collections::HashMap,
  str::FromStr,
  sync::{Arc, Mutex},
};

use tauri::{AppHandle, InvokePayload, InvokeResponder, InvokeResponse, Manager, Runtime};
use tiny_http::{Header, Method, Request, Response};

fn cors<R: std::io::Read>(request: &Request, r: &mut Response<R>, allowed_origins: &[String]) {
  if let Some(origin) = request.headers().iter().find(|h| h.field.equiv("Origin")) {
    if allowed_origins.iter().any(|o| o == &origin.value) {
      r.add_header(
        Header::from_str(&format!("Access-Control-Allow-Origin: {}", origin.value)).unwrap(),
      );
    }
  }
  r.add_header(Header::from_str("Access-Control-Allow-Headers: *").unwrap());
  r.add_header(Header::from_str("Access-Control-Allow-Methods: POST, OPTIONS").unwrap());
}

pub struct Invoke {
  allowed_origins: Vec<String>,
  port: u16,
  requests: Arc<Mutex<HashMap<String, Request>>>,
}

impl Invoke {
  pub fn new<I: Into<String>, O: IntoIterator<Item = I>>(allowed_origins: O) -> Self {
    let port = portpicker::pick_unused_port().expect("failed to get unused port for invoke");
    let requests = Arc::new(Mutex::new(HashMap::new()));
    Self {
      allowed_origins: allowed_origins.into_iter().map(|o| o.into()).collect(),
      port,
      requests,
    }
  }

  pub fn start<R: Runtime>(&self, app: AppHandle<R>) {
    let server = tiny_http::Server::http(format!("localhost:{}", self.port)).unwrap();
    let requests = self.requests.clone();
    let allowed_origins = self.allowed_origins.clone();
    std::thread::spawn(move || {
      for mut request in server.incoming_requests() {
        if request.method() == &Method::Options {
          let mut r = Response::empty(200);
          cors(&request, &mut r, &allowed_origins);
          request.respond(r).unwrap();
          continue;
        }
        let url = request.url().to_string();
        let pieces = url.split("/").collect::<Vec<_>>();
        let window_label = pieces[1];

        if let Some(window) = app.get_window(window_label) {
          let command = pieces[2].to_string();
          let content_type = request
            .headers()
            .iter()
            .find(|h| h.field.equiv("Content-Type"))
            .map(|h| h.value.to_string())
            .unwrap_or_else(|| "application/json".into());

          let payload: InvokePayload = if content_type == "application/json" {
            let mut content = String::new();
            request.as_reader().read_to_string(&mut content).unwrap();
            serde_json::from_str(&content).unwrap()
          } else {
            unimplemented!()
          };
          let req_key = payload.callback.clone();
          requests.lock().unwrap().insert(req_key, request);
          let _ = window.on_message(command, payload);
        } else {
          let mut r = Response::empty(404);
          cors(&request, &mut r, &allowed_origins);
          request.respond(r).unwrap();
        }
      }
    });
  }

  pub fn responder<R: Runtime>(&self) -> Box<InvokeResponder<R>> {
    let requests = self.requests.clone();
    let allowed_origins = self.allowed_origins.clone();
    let responder = move |_window, response: InvokeResponse, callback, _error| {
      let request = requests.lock().unwrap().remove(&callback).unwrap();
      let response = response.into_result();
      let status = if response.is_ok() { 200 } else { 400 };

      let mut r = Response::from_string(
        serde_json::to_string(&match response {
          Ok(r) => r,
          Err(e) => e,
        })
        .unwrap(),
      )
      .with_status_code(status);
      cors(&request, &mut r, &allowed_origins);

      request.respond(r).unwrap();
    };
    Box::new(responder)
  }

  pub fn initialization_script(&self) -> String {
    format!(
      "
        Object.defineProperty(window, '__TAURI_POST_MESSAGE__', {{
          value: (command, args) => {{
            const request = new XMLHttpRequest();
            request.addEventListener('load', function () {{
              let arg
              let success = this.status === 200
              try {{
                arg = JSON.parse(this.response)
              }} catch (e) {{ 
                arg = e
                success = false
              }}
              window[success ? args.callback : args.error](arg)
            }})
            request.open('POST', 'http://localhost:{}/' + window.__TAURI__.__currentWindow.label + '/' + command, true)
            request.setRequestHeader('Content-Type', 'application/json')
            request.send(JSON.stringify(args))
          }}
        }})
    ",
      self.port
    )
  }
}
