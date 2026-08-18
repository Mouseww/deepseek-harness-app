import { DshBackendClient, isTauriEnvironment } from "./tauri-api";

const statusEl = () => document.querySelector<HTMLElement>("#status-msg");
const detailEl = () => document.querySelector<HTMLElement>("#greet-msg");

async function refreshStatus() {
  const status = statusEl();
  const detail = detailEl();
  if (!status) return;

  if (!isTauriEnvironment()) {
    status.textContent = "Open this app with Tauri to manage the DSH backend.";
    return;
  }

  const running = await DshBackendClient.getStatus();
  const port = await DshBackendClient.getPort();
  status.textContent = running
    ? `DSH is running on port ${port ?? "?"}`
    : "DSH is stopped";

  if (detail && running && port) {
    detail.textContent = `http://127.0.0.1:${port}`;
  } else if (detail) {
    detail.textContent = "";
  }
}

window.addEventListener("DOMContentLoaded", () => {
  document.querySelector("#start-btn")?.addEventListener("click", async () => {
    const config = await DshBackendClient.getConfig();
    await DshBackendClient.start(config);
    await refreshStatus();
  });

  document.querySelector("#stop-btn")?.addEventListener("click", async () => {
    await DshBackendClient.stop();
    await refreshStatus();
  });

  void (async () => {
    if (isTauriEnvironment()) {
      const config = await DshBackendClient.getConfig();
      if (config.auto_start) {
        await DshBackendClient.start(config);
      }
    }
    await refreshStatus();
  })();
});
