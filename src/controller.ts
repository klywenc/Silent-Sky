import { ControllerLayout, SCRATCH_ID } from "./config";

export class ControllerView {
  private root: HTMLElement;
  private lanes = new Map<string, HTMLElement>();
  private laneActive = new Map<string, boolean>();
  private pressTime = new Map<string, number>();

  private needle!: HTMLElement;
  private scratchEl!: HTMLElement;
  private scratchAngle = 0;
  private scratchTimer: number | undefined;

  private countEl!: HTMLElement;
  private npsEl!: HTMLElement;
  private relEl!: HTMLElement;
  private total = 0;
  private pressStamps: number[] = [];
  private holdTimes: number[] = [];

  constructor(root: HTMLElement, layout: ControllerLayout) {
    this.root = root;
    this.render(layout);
    setInterval(() => this.refreshStats(), 100);
  }

  private render(layout: ControllerLayout) {
    this.root.innerHTML = "";
    this.root.dataset.layout = layout.name;

    const keys = document.createElement("div");
    keys.className = "keys";
    const topRow = document.createElement("div");
    topRow.className = "keys__row keys__row--top";
    const bottomRow = document.createElement("div");
    bottomRow.className = "keys__row keys__row--bottom";
    for (const lane of layout.lanes) {
      const el = document.createElement("div");
      el.className = `key key--${lane.kind}`;
      el.dataset.lane = lane.id;
      this.lanes.set(lane.id, el);
      this.laneActive.set(lane.id, false);
      (lane.row === "top" ? topRow : bottomRow).appendChild(el);
    }
    keys.append(topRow, bottomRow);

    const stats = document.createElement("div");
    stats.className = "stats";
    stats.innerHTML = `
      <div class="stat stat--count"><span class="js-count">0</span></div>
      <div class="stat stat--nps"><span class="js-nps">0</span> /s</div>
      <div class="stat stat--rel">Release Avg : <span class="js-rel">0</span> ms</div>`;
    this.countEl = stats.querySelector(".js-count")!;
    this.npsEl = stats.querySelector(".js-nps")!;
    this.relEl = stats.querySelector(".js-rel")!;

    const keysCol = document.createElement("div");
    keysCol.className = "keys-col";
    keysCol.append(keys, stats);

    const scratch = document.createElement("div");
    scratch.className = "scratch";
    scratch.dataset.lane = SCRATCH_ID;
    scratch.innerHTML = `
      <div class="scratch__ring"></div>
      <div class="scratch__needle"></div>`;
    this.scratchEl = scratch;
    this.needle = scratch.querySelector(".scratch__needle")!;

    if (layout.scratchSide === "left") {
      this.root.append(scratch, keysCol);
    } else {
      this.root.append(keysCol, scratch);
    }
  }

  setLane(id: string, active: boolean) {
    if (this.laneActive.get(id) === active) return;
    this.laneActive.set(id, active);
    this.lanes.get(id)?.classList.toggle("is-active", active);

    const now = performance.now();
    if (active) {
      this.pressTime.set(id, now);
      this.registerPress(now);
    } else {
      const t = this.pressTime.get(id);
      if (t !== undefined) {
        this.holdTimes.push(now - t);
        if (this.holdTimes.length > 24) this.holdTimes.shift();
      }
    }
  }

  releaseAll() {
    this.lanes.forEach((el, id) => {
      el.classList.remove("is-active");
      this.laneActive.set(id, false);
    });
    this.setScratch(false);
  }

  private registerPress(now: number) {
    this.total += 1;
    this.pressStamps.push(now);
  }

  private refreshStats() {
    const now = performance.now();
    while (this.pressStamps.length && now - this.pressStamps[0] > 1000) {
      this.pressStamps.shift();
    }
    const rel = this.holdTimes.length
      ? Math.round(this.holdTimes.reduce((a, b) => a + b, 0) / this.holdTimes.length)
      : 0;
    this.countEl.textContent = String(this.total);
    this.npsEl.textContent = String(this.pressStamps.length);
    this.relEl.textContent = String(rel);
  }

  setScratch(active: boolean) {
    this.scratchEl.classList.toggle("is-active", active);
  }

  addScratchDelta(delta: number) {
    this.scratchAngle += delta * 180;
    this.needle.style.transform = `translate(-50%, -50%) rotate(${this.scratchAngle}deg)`;
    this.pulseScratch();
  }

  spinScratch(dir: "up" | "down") {
    this.scratchAngle += dir === "up" ? 12 : -12;
    this.needle.style.transform = `translate(-50%, -50%) rotate(${this.scratchAngle}deg)`;
    this.pulseScratch();
  }

  private pulseScratch() {
    this.setScratch(true);
    window.clearTimeout(this.scratchTimer);
    this.scratchTimer = window.setTimeout(() => this.setScratch(false), 160);
  }
}
