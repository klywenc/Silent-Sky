import { invoke } from "@tauri-apps/api/core";

interface TableEntry {
  table: string;
  tag: string;
  label: string;
}

interface ScoreRow {
  sha256: string;
  title: string;
  artist: string;
  level: number;
  difficulty: number;
  clear: number;
  exscore: number;
  minbp: number;
  date: number;
  tables: TableEntry[];
}

const CLEAR: Record<number, { lamp: string; label: string }> = {
  0: { lamp: "NO_PLAY", label: "NP" },
  1: { lamp: "FAILED", label: "FAIL" },
  2: { lamp: "ASSIST", label: "A-EZ" },
  3: { lamp: "ASSIST", label: "LA-EZ" },
  4: { lamp: "EASY", label: "EASY" },
  5: { lamp: "NORMAL", label: "CLEAR" },
  6: { lamp: "HARD", label: "HARD" },
  7: { lamp: "EX_HARD", label: "EX-H" },
  8: { lamp: "FULL_COMBO", label: "FC" },
  9: { lamp: "FULL_COMBO", label: "PERF" },
  10: { lamp: "FULL_COMBO", label: "MAX" },
};

const DIFF: Record<number, string> = { 0: "?", 1: "B", 2: "N", 3: "H", 4: "A", 5: "I" };

function timeAgo(epochSec: number): string {
  if (!epochSec) return "";
  const diff = Date.now() / 1000 - epochSec;
  if (diff < 60) return "now";
  if (diff < 3600) return `${Math.floor(diff / 60)} min`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} h`;
  return `${Math.floor(diff / 86400)} d`;
}

export class ScoresView {
  private root: HTMLElement;
  private limit: number;
  private showClear = true;

  constructor(root: HTMLElement, limit = 7) {
    this.root = root;
    this.limit = limit;
    this.renderEmpty("Set the database folder in Settings → Score database");
  }

  private renderEmpty(msg: string) {
    this.root.innerHTML = `<h2 class="scores__title">Recent scores</h2>
      <p class="scores__empty">${msg}</p>`;
  }

  async refresh() {
    try {
      const rows = await invoke<ScoreRow[]>("get_recent_scores", { limit: this.limit });
      if (!rows.length) {
        this.renderEmpty("No entries in scoredatalog.");
        return;
      }
      this.render(rows);
    } catch (e) {
      this.renderEmpty(`Database error: ${String(e)}`);
    }
  }

  private renderTables(tables: TableEntry[]): string {
    if (!tables || !tables.length) return "";
    const seen = new Set<string>();
    const uniq = tables
      .filter((t) => t.label && !seen.has(t.label) && seen.add(t.label))
      .sort((a, b) => a.label.length - b.label.length);

    const MAX = 6;
    const shown = uniq.slice(0, MAX);
    const rest = uniq.slice(MAX);
    const chips = shown
      .map((t) => `<span class="tbl" title="${t.table}">${t.label}</span>`)
      .join("");
    const more = rest.length
      ? `<span class="tbl tbl--more" title="${rest.map((t) => `${t.table}: ${t.label}`).join(" | ")}">+${rest.length}</span>`
      : "";
    return `<span class="score__tables">${chips}${more}</span>`;
  }

  private render(entries: ScoreRow[]) {
    this.root.innerHTML = `<h2 class="scores__title">Recent scores</h2>`;
    const list = document.createElement("ul");
    list.className = "scores__list";
    for (const e of entries) {
      const li = document.createElement("li");
      li.className = "score";
      li.title = `${e.title} — ${e.artist}`;
      if (this.showClear) {
        const c = CLEAR[e.clear] ?? { lamp: "NO_PLAY", label: String(e.clear) };
        li.dataset.lamp = c.lamp;
      }
      const lamp = this.showClear
        ? `<span class="score__lamp">${(CLEAR[e.clear] ?? { label: String(e.clear) }).label}</span>`
        : "";
      li.innerHTML = `
        <span class="score__diff" data-diff="${e.difficulty}">${DIFF[e.difficulty] ?? "?"}${e.level}</span>
        <span class="score__main">
          <span class="score__title">${e.title}</span>
          ${this.renderTables(e.tables)}
        </span>
        ${lamp}
        <span class="score__ex">${e.exscore}</span>
        <span class="score__ago">${timeAgo(e.date)}</span>`;
      list.appendChild(li);
    }
    this.root.appendChild(list);
  }
}
