# BeatOverlay

Floatująca nakładka (overlay) na grę rytmiczną — w stylu overlaya Discorda / RAGa.
Pokazuje na żywo wciskane przyciski kontrolera **IIDX** (7 klawiszy + scratch)
oraz **ostatnio zagrane piosenki i wyniki** z bazy **beatoraji**.
W pełni customizowalny w stylu CSS.

Stack: **Tauri 2** (Rust backend + web frontend), TypeScript + Vite.

## Uruchomienie (dev)

```bash
npm install
npm run tauri dev
```

> Wymaga Rust (rustup) + VS C++ Build Tools. PATH cargo: `%USERPROFILE%\.cargo\bin`.

Build produkcyjny:

```bash
npm run tauri build
```

## Skróty w overlayu

| Klawisz | Akcja |
|---------|-------|
| `F8` | przełącz tryb klik-przez (overlay przepuszcza mysz do gry) |
| `F9` | panel ustawień (Faza 4) |
| przeciągnij pasek tytułu | przesuń overlay |

### Mock inputu (tylko Faza 1, do czasu realnego kontrolera)

Klawisze `S D F [Spacja] J K L` = lane'y 1–7, strzałki `↑/↓` = scratch.

## Architektura

```
src/                 frontend (web, CSS-owa customizacja)
  config.ts          layout kontrolera + mock keymap
  controller.ts      render + API podświetlania lane'ów / scratcha
  scores.ts          panel ostatnich wyników (model + render)
  main.ts            bootstrap + input + sterowanie overlayem
  styles/base.css    struktura (na zmiennych CSS)
  styles/theme.css   domyślna skórka (zmienne do nadpisania w Fazie 4)
src-tauri/           backend Rust (okno, input, odczyt SQLite — kolejne fazy)
```

## Roadmap

- [x] **Faza 1** — przezroczyste okno always-on-top, makieta kontrolera + panel wyników, mock inputu.
- [ ] **Faza 2** — realny input kontrolera IIDX (crate `gilrs`/DirectInput) + mapowanie z beatoraja `keyconfig`.
- [ ] **Faza 3** — odczyt SQLite beatoraji (songdata.db + player score/scorelog), panel ostatnich wyników na żywo.
- [ ] **Faza 4** — customizacja: ładowanie user CSS + layout JSON, panel ustawień.
- [ ] **Faza 5 (stretch)** — overlay dla exclusive fullscreen (hookowanie DX/GL).
