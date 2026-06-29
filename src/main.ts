import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { IIDX_SP, MOCK_KEYMAP, MOCK_SCRATCH } from "./config";
import { ControllerView } from "./controller";
import { ScoresView } from "./scores";
import { Settings } from "./settings";

const controllerRoot = document.getElementById("controller")!;
const scoresRoot = document.getElementById("scores")!;
const settingsRoot = document.getElementById("settings")!;

const controller = new ControllerView(controllerRoot, IIDX_SP);
const scores = new ScoresView(scoresRoot);
new Settings(settingsRoot);

void scores.refresh();
setInterval(() => void scores.refresh(), 5000);
window.addEventListener("db-updated", () => void scores.refresh());

interface LanePayload {
  lane: string;
  active: boolean;
}
interface ScratchPayload {
  dir: "up" | "down" | "none";
  active: boolean;
  value: number;
  source: "axis" | "button";
}

void listen<LanePayload>("input://lane", (e) => {
  controller.setLane(e.payload.lane, e.payload.active);
});

void listen<ScratchPayload>("input://scratch", (e) => {
  const { dir, active, value, source } = e.payload;
  if (source === "axis") {
    controller.addScratchDelta(value);
  } else if (active && (dir === "up" || dir === "down")) {
    controller.spinScratch(dir);
  } else {
    controller.setScratch(false);
  }
});

const pressed = new Set<string>();

window.addEventListener("keydown", (e) => {
  if (e.code === "F8") {
    e.preventDefault();
    void toggleClickThrough();
    return;
  }
  if (e.code === "F9") {
    e.preventDefault();
    document.body.classList.toggle("show-settings");
    return;
  }

  if (e.repeat) return;

  const lane = MOCK_KEYMAP[e.code];
  if (lane) {
    pressed.add(e.code);
    controller.setLane(lane, true);
    e.preventDefault();
    return;
  }

  const dir = MOCK_SCRATCH[e.code];
  if (dir) {
    controller.spinScratch(dir);
    e.preventDefault();
  }
});

window.addEventListener("keyup", (e) => {
  const lane = MOCK_KEYMAP[e.code];
  if (lane) {
    pressed.delete(e.code);
    controller.setLane(lane, false);
  }
});

window.addEventListener("blur", () => {
  pressed.clear();
  controller.releaseAll();
});

const appWindow = getCurrentWindow();
let clickThrough = false;

async function toggleClickThrough() {
  clickThrough = !clickThrough;
  await appWindow.setIgnoreCursorEvents(clickThrough);
  document.body.classList.toggle("is-click-through", clickThrough);
  btnClick.classList.toggle("is-on", clickThrough);
}

function applyTheme(light: boolean) {
  document.body.classList.toggle("theme-light", light);
  btnTheme.textContent = light ? "🌙" : "☀";
  localStorage.setItem("theme", light ? "light" : "dark");
}

const btnMove = document.getElementById("btn-move")!;
const btnTheme = document.getElementById("btn-theme")!;
const btnClick = document.getElementById("btn-click")!;
const btnSettings = document.getElementById("btn-settings")!;
const btnBack = document.getElementById("btn-back")!;

btnMove.addEventListener("pointerdown", () => void appWindow.startDragging());
btnTheme.addEventListener("click", () =>
  applyTheme(!document.body.classList.contains("theme-light")),
);
btnClick.addEventListener("click", () => void toggleClickThrough());
btnSettings.addEventListener("click", () => document.body.classList.toggle("show-settings"));
btnBack.addEventListener("click", () => document.body.classList.remove("show-settings"));

applyTheme(localStorage.getItem("theme") === "light");
