import { invoke } from "@tauri-apps/api/core";

const response = document.getElementById("response") as HTMLDivElement;
const timeElapsed = document.getElementById("time_elapsed") as HTMLDivElement;

// Debug: Check if our custom system is initialized
console.log("Checking tauri-awesome-rpc initialization...");
console.log("window.__TAURI_INTERNALS__:", window.__TAURI_INTERNALS__);
console.log("window.__TAURI_INTERNALS__.postMessage:", window.__TAURI_INTERNALS__?.postMessage);
console.log("window.AwesomeListener:", window.AwesomeListener);

// Test basic invoke command
invoke("test_command", { args: 5 })
  .then((data) => {
    response.innerText = data as string;
  })
  .catch((error) => {
    console.error("Error invoking test_command:", error);
    response.innerText = `Error: ${error}`;
  });

// Start time elapsed reporting
invoke("report_time_elapsed")
  .catch((error) => {
    console.error("Error invoking report_time_elapsed:", error);
  });

// Listen to time elapsed events using AwesomeListener
if (window.AwesomeListener) {
  const _unsubscribe = window.AwesomeListener.listen("time_elapsed", (data) => {
    timeElapsed.innerText = JSON.stringify(data);
  });
} else {
  console.error("AwesomeListener is not available!");
  timeElapsed.innerText = "AwesomeListener not initialized";
}