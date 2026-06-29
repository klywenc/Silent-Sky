# SilentSky

A floating overlay for LR2oraja / beatoraja. It sits on top of the game (or
anywhere) and shows your controller input live and your recent plays, in a
RAG-style layout. Built with Tauri, so it stays light and you can restyle it
with plain CSS.

> Status: early, but usable. Made for 7K SP on an IIDX-style controller.

## What it shows

- **Controller** — the 7 keys + turntable, lit in real time, with input stats
  (total notes, notes/s, average key hold time).
- **Recent scores** — pulled straight from your LR2oraja player database. Each
  row shows the difficulty, title, the *exact* clear of that play (not just your
  best), EX score, and which difficulty tables the chart belongs to (`sl11`,
  `★15`, `st8`, …).

The overlay can auto-attach to the game window, hide when the game isn't
focused, and toggle click-through so it never gets in the way.

## Requirements

- Windows 10/11 (WebView2 is already present on current Windows).
- A controller seen as a gamepad. The scratch can be an analog axis or two
  buttons — both work.
- For the scores panel: an LR2oraja install with its `songdata.db`,
  `player/<name>/scoredatalog.db`, and (optionally) difficulty tables under
  `table/`.

## Install

Grab the latest Windows installer from the
[Releases](https://github.com/klywenc/Silent-Sky/releases) page and run it.

## Usage

Open settings (the gear, or `F9`) and:

1. Pick your gamepad, then bind each key and the scratch (click *Bind*, then
   press the input on the controller).
2. Set the game window title to attach to (default `LR2oraja`).
3. Point *Score database* at your LR2oraja folder and hit *Load*.

Shortcuts:

| Key | Action |
| --- | --- |
| `F8` | toggle click-through |
| `F9` | open/close settings |
| ✥ in the toolbar | hold and drag to move the overlay |

There's a light and a dark theme; the choice is remembered.

## How the scores work

The score log only stores chart hashes, so titles and levels are joined in by
hash:

- Recent plays come from `scoredatalog` (one row per play), so the clear shown
  is the one you actually got that run. EX is computed from the judgement
  counts.
- Titles, levels and difficulty come from the `song` table in `songdata.db`,
  matched by `sha256`.
- Difficulty-table tags come from the gzip-compressed JSON in `table/*.bmt`,
  matched by `md5`.

## Build from source

```bash
npm install
npm run tauri dev      # run
npm run tauri build    # Windows installer in src-tauri/target/release/bundle
```

Needs Node 20+ and a Rust toolchain (MSVC) with the VS C++ Build Tools.

## Stack

Tauri 2, TypeScript + Vite on the front, Rust on the back (`gilrs` for input,
`rusqlite` for the database, the Win32 API for window attaching).
