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
