import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { IIDX_SP } from "./config";

export interface Mapping {
  gamepad: string | null;
  buttons: Record<string, string>;
  scratch_axis: string | null;
  scratch_threshold: number;
  scratch_up_btn: string | null;
  scratch_down_btn: string | null;
}

interface RawInput {
  gamepad: string;
  kind: "button" | "axis";
  code: string;
  value: number;
}

interface AttachConfig {
  enabled: boolean;
  target_title: string;
  anchor: string;
  margin: number;
  hide_when_inactive: boolean;
}

type BindTarget =
  | { kind: "lane"; id: string }
  | { kind: "scratchAxis" }
  | { kind: "scratchUp" }
  | { kind: "scratchDown" };

function emptyMapping(): Mapping {
  return {
    gamepad: null,
    buttons: {},
    scratch_axis: null,
    scratch_threshold: 0,
    scratch_up_btn: null,
    scratch_down_btn: null,
  };
}

export class Settings {
  private gpSelect: HTMLSelectElement;
  private bindList: HTMLElement;
  private rawReadout: HTMLElement;
  private scrSens: HTMLInputElement;
  private scrSensVal: HTMLElement;

  private mapping: Mapping = emptyMapping();
  private gamepads: string[] = [];
  private bindTarget: BindTarget | null = null;

  private atEnabled: HTMLInputElement;
  private atTitle: HTMLInputElement;
  private atAnchor: HTMLSelectElement;
  private atHide: HTMLInputElement;
  private atStatus: HTMLElement;
  private attach: AttachConfig = {
    enabled: true,
    target_title: "LR2oraja",
    anchor: "top-right",
    margin: 16,
    hide_when_inactive: true,
  };

  private dbRoot: HTMLInputElement;
  private dbSave: HTMLButtonElement;
  private dbStatus: HTMLElement;
  private dbDiagBtn: HTMLButtonElement;
  private dbDiag: HTMLElement;

  constructor(root: HTMLElement) {
    this.gpSelect = root.querySelector("#gp-select")!;
    this.bindList = root.querySelector("#bind-list")!;
    this.rawReadout = root.querySelector("#raw-readout")!;
    this.scrSens = root.querySelector("#scr-sens")!;
    this.scrSensVal = root.querySelector("#scr-sens-val")!;
    this.atEnabled = root.querySelector("#at-enabled")!;
    this.atTitle = root.querySelector("#at-title")!;
    this.atAnchor = root.querySelector("#at-anchor")!;
    this.atHide = root.querySelector("#at-hide")!;
    this.atStatus = root.querySelector("#at-status")!;
    this.dbRoot = root.querySelector("#db-root")!;
    this.dbSave = root.querySelector("#db-save")!;
    this.dbStatus = root.querySelector("#db-status")!;
    this.dbDiagBtn = root.querySelector("#db-diag-btn")!;
    this.dbDiag = root.querySelector("#db-diag")!;
    void this.init();
  }

  private async init() {
    try {
      this.mapping = await invoke<Mapping>("get_mapping");
      this.gamepads = await invoke<string[]>("get_gamepads");
      this.attach = await invoke<AttachConfig>("get_attach_config");
      const dbCfg = await invoke<{ root: string | null }>("get_db_config");
      this.dbRoot.value = dbCfg.root ?? "";
      this.dbStatus.textContent = dbCfg.root ? dbCfg.root : "not set";
    } catch (e) {
      console.error("Failed to load configuration:", e);
    }

    await listen<string[]>("input://gamepads", (e) => {
      this.gamepads = e.payload;
      this.renderGamepads();
    });

    await listen<RawInput>("input://raw", (e) => this.onRaw(e.payload));

    await listen<{ active: boolean; title: string }>("attach://status", (e) => {
      this.atStatus.textContent = e.payload.active ? `● ${e.payload.title}` : "○ game inactive";
      this.atStatus.classList.toggle("is-active", e.payload.active);
    });

    this.gpSelect.addEventListener("change", () => {
      this.mapping.gamepad = this.gpSelect.value || null;
      void this.save();
    });

    const sens = this.mapping.scratch_threshold > 0 ? this.mapping.scratch_threshold : 0.012;
    this.scrSens.value = String(sens);
    this.scrSensVal.textContent = sens.toFixed(3);
    this.scrSens.addEventListener("input", () => {
      this.mapping.scratch_threshold = parseFloat(this.scrSens.value);
      this.scrSensVal.textContent = this.mapping.scratch_threshold.toFixed(3);
      void this.save();
    });

    window.addEventListener("keydown", (e) => {
      if (e.code === "Escape" && this.bindTarget) {
        this.bindTarget = null;
        this.render();
      }
    });

    this.initAttachControls();
    this.initDbControls();
    this.render();
  }

  private onRaw(raw: RawInput) {
    this.rawReadout.textContent = `${raw.gamepad || "?"} · ${raw.kind} · ${raw.code} = ${raw.value.toFixed(2)}`;
    if (!this.bindTarget) return;

    const t = this.bindTarget;
    if (t.kind === "scratchAxis") {
      if (raw.kind !== "axis") return;
      this.mapping.scratch_axis = raw.code;
    } else {
      if (raw.kind !== "button" || raw.value < 0.5) return;
      if (t.kind === "lane") this.mapping.buttons = { ...this.mapping.buttons, [raw.code]: t.id };
      if (t.kind === "scratchUp") this.mapping.scratch_up_btn = raw.code;
      if (t.kind === "scratchDown") this.mapping.scratch_down_btn = raw.code;
    }

    if (!this.mapping.gamepad && raw.gamepad) this.mapping.gamepad = raw.gamepad;
    this.bindTarget = null;
    void this.save();
  }

  private async save() {
    try {
      await invoke("set_mapping", { mapping: this.mapping });
    } catch (e) {
      console.error("Failed to save mapping:", e);
    }
    this.render();
  }

  private initAttachControls() {
    this.atEnabled.checked = this.attach.enabled;
    this.atTitle.value = this.attach.target_title;
    this.atAnchor.value = this.attach.anchor;
    this.atHide.checked = this.attach.hide_when_inactive;

    const push = () => {
      this.attach = {
        ...this.attach,
        enabled: this.atEnabled.checked,
        target_title: this.atTitle.value.trim(),
        anchor: this.atAnchor.value,
        hide_when_inactive: this.atHide.checked,
      };
      void this.saveAttach();
    };

    this.atEnabled.addEventListener("change", push);
    this.atHide.addEventListener("change", push);
    this.atAnchor.addEventListener("change", push);
    this.atTitle.addEventListener("change", push);
  }

  private async saveAttach() {
    try {
      await invoke("set_attach_config", { config: this.attach });
    } catch (e) {
      console.error("Failed to save attach config:", e);
    }
  }

  private initDbControls() {
    const save = async () => {
      const root = this.dbRoot.value.trim();
      try {
        await invoke("set_db_root", { root });
        this.dbStatus.textContent = root || "not set";
        window.dispatchEvent(new Event("db-updated"));
      } catch (e) {
        this.dbStatus.textContent = `error: ${String(e)}`;
      }
    };
    this.dbSave.addEventListener("click", () => void save());
    this.dbRoot.addEventListener("change", () => void save());

    this.dbDiagBtn.addEventListener("click", async () => {
      this.dbDiag.textContent = "…";
      try {
        await invoke("set_db_root", { root: this.dbRoot.value.trim() });
        const report = await invoke<string>("db_diagnostics");
        this.dbDiag.textContent = report;
      } catch (e) {
        this.dbDiag.textContent = `error: ${String(e)}`;
      }
    });
  }

  private codeForLane(id: string): string | null {
    const entry = Object.entries(this.mapping.buttons).find(([, lane]) => lane === id);
    return entry ? entry[0] : null;
  }

  private renderGamepads() {
    const cur = this.mapping.gamepad ?? "";
    this.gpSelect.innerHTML =
      `<option value="">— any —</option>` +
      this.gamepads
        .map((g) => `<option value="${g}"${g === cur ? " selected" : ""}>${g}</option>`)
        .join("");
    if (this.gamepads.length === 0) {
      this.gpSelect.innerHTML = `<option value="">none (connect a controller)</option>`;
    }
  }

  private bindRow(label: string, current: string | null, target: BindTarget): string {
    const active = this.isBinding(target);
    const val = current ? `<code>${current}</code>` : `<span class="muted">—</span>`;
    const key = JSON.stringify(target);
    return `
      <div class="bind-row${active ? " is-binding" : ""}">
        <span class="bind-row__label">${label}</span>
        <span class="bind-row__val">${active ? "press…" : val}</span>
        <button class="btn bind-row__btn" data-target='${key}'>${active ? "Cancel" : "Bind"}</button>
      </div>`;
  }

  private isBinding(t: BindTarget): boolean {
    return JSON.stringify(this.bindTarget) === JSON.stringify(t);
  }

  private render() {
    this.renderGamepads();

    const rows = [
      ...IIDX_SP.lanes.map((l) =>
        this.bindRow(`Key ${l.label}`, this.codeForLane(l.id), { kind: "lane", id: l.id }),
      ),
      this.bindRow("Scratch (axis)", this.mapping.scratch_axis, { kind: "scratchAxis" }),
      this.bindRow("Scratch ↑ (button)", this.mapping.scratch_up_btn, { kind: "scratchUp" }),
      this.bindRow("Scratch ↓ (button)", this.mapping.scratch_down_btn, { kind: "scratchDown" }),
    ];
    this.bindList.innerHTML = rows.join("");

    this.bindList.querySelectorAll<HTMLButtonElement>(".bind-row__btn").forEach((btn) => {
      btn.addEventListener("click", () => {
        const target = JSON.parse(btn.dataset.target!) as BindTarget;
        this.bindTarget = this.isBinding(target) ? null : target;
        this.render();
      });
    });
  }
}
