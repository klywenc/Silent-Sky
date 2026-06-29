use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use flate2::read::GzDecoder;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbConfig {
    pub root: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableEntry {
    pub table: String,
    pub tag: String,
    pub label: String,
}

pub type TableIndex = HashMap<String, Vec<TableEntry>>;

pub struct DbState {
    pub cfg: Arc<Mutex<DbConfig>>,
    pub tables: Arc<Mutex<Option<Arc<TableIndex>>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoreRow {
    pub sha256: String,
    pub title: String,
    pub artist: String,
    pub level: i64,
    pub difficulty: i64,
    pub clear: i64,
    pub exscore: i64,
    pub minbp: i64,
    pub date: i64,
    pub tables: Vec<TableEntry>,
}

fn read_only(path: &Path) -> rusqlite::Result<Connection> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
}

fn resolve_paths(root: &Path) -> (Option<PathBuf>, Option<PathBuf>) {
    let songdata = {
        let p = root.join("songdata.db");
        p.exists().then_some(p)
    };
    let scorelog = newest_player_db(root, "scorelog.db").or_else(|| {
        let p = root.join("scorelog.db");
        p.exists().then_some(p)
    });
    (songdata, scorelog)
}

fn newest_player_db(root: &Path, file: &str) -> Option<PathBuf> {
    let player_dir = root.join("player");
    let entries = fs::read_dir(&player_dir).ok()?;
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for e in entries.flatten() {
        let p = e.path().join(file);
        if p.exists() {
            let mt = fs::metadata(&p)
                .and_then(|m| m.modified())
                .unwrap_or(UNIX_EPOCH);
            if best.as_ref().map_or(true, |(b, _)| mt > *b) {
                best = Some((mt, p));
            }
        }
    }
    best.map(|(_, p)| p)
}

pub fn build_table_index(root: &Path) -> TableIndex {
    let mut idx: TableIndex = HashMap::new();
    let dir = root.join("table");
    let rd = match fs::read_dir(&dir) {
        Ok(r) => r,
        Err(_) => return idx,
    };

    for e in rd.flatten() {
        let p = e.path();
        if p.extension().map_or(true, |x| x != "bmt") {
            continue;
        }
        let bytes = match fs::read(&p) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let mut gz = GzDecoder::new(&bytes[..]);
        let mut s = String::new();
        if gz.read_to_string(&mut s).is_err() {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(&s) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let table_name = v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let tag = v.get("tag").and_then(|x| x.as_str()).unwrap_or("").to_string();
        if tag.is_empty() {
            continue;
        }
        let Some(folders) = v.get("folder").and_then(|x| x.as_array()) else {
            continue;
        };

        for fo in folders {
            let label = fo.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let Some(songs) = fo.get("songs").and_then(|x| x.as_array()) else {
                continue;
            };
            for so in songs {
                if let Some(md5) = so.get("md5").and_then(|x| x.as_str()) {
                    idx.entry(md5.to_lowercase()).or_default().push(TableEntry {
                        table: table_name.clone(),
                        tag: tag.clone(),
                        label: label.clone(),
                    });
                }
            }
        }
    }
    idx
}

pub fn get_or_build_tables(state: &DbState, root: &str) -> Arc<TableIndex> {
    let mut guard = state.tables.lock().unwrap();
    if guard.is_none() {
        *guard = Some(Arc::new(build_table_index(Path::new(root))));
    }
    guard.clone().unwrap()
}

pub fn invalidate_tables(state: &DbState) {
    *state.tables.lock().unwrap() = None;
}

pub fn recent_scores(root: &str, limit: i64, tables: &TableIndex) -> Result<Vec<ScoreRow>, String> {
    let root = PathBuf::from(root);

    let songdata = root.join("songdata.db");
    if !songdata.exists() {
        return Err(format!("songdata.db not found in: {}", root.display()));
    }
    let datalog = newest_player_db(&root, "scoredatalog.db")
        .ok_or_else(|| format!("scoredatalog.db not found in {}/player/*/", root.display()))?;

    let song = read_only(&songdata).map_err(|e| format!("songdata.db: {e}"))?;
    let log = read_only(&datalog).map_err(|e| format!("scoredatalog.db: {e}"))?;

    let mut stmt = log
        .prepare(
            "SELECT sha256, clear, epg, lpg, egr, lgr, minbp, date \
             FROM scoredatalog ORDER BY date DESC LIMIT ?1",
        )
        .map_err(|e| format!("scoredatalog query: {e}"))?;

    let raw = stmt
        .query_map([limit], |r| {
            let sha256: String = r.get(0)?;
            let clear: i64 = r.get(1)?;
            let epg: i64 = r.get(2)?;
            let lpg: i64 = r.get(3)?;
            let egr: i64 = r.get(4)?;
            let lgr: i64 = r.get(5)?;
            let minbp: i64 = r.get(6)?;
            let date: i64 = r.get(7)?;
            let exscore = 2 * (epg + lpg) + (egr + lgr);
            Ok((sha256, clear, exscore, minbp, date))
        })
        .map_err(|e| format!("scoredatalog read: {e}"))?;

    let mut find = song
        .prepare("SELECT title, artist, level, difficulty, md5 FROM song WHERE sha256 = ?1 LIMIT 1")
        .map_err(|e| format!("song query: {e}"))?;

    let mut out = Vec::new();
    for row in raw {
        let (sha256, clear, exscore, minbp, date) = row.map_err(|e| e.to_string())?;
        let meta = find
            .query_row([&sha256], |r| {
                Ok((
                    r.get::<_, String>(0).unwrap_or_default(),
                    r.get::<_, String>(1).unwrap_or_default(),
                    r.get::<_, i64>(2).unwrap_or(0),
                    r.get::<_, i64>(3).unwrap_or(0),
                    r.get::<_, String>(4).unwrap_or_default(),
                ))
            })
            .ok();

        let (title, artist, level, difficulty, md5) = meta.unwrap_or_else(|| {
            ("(unknown — not in songdata)".into(), String::new(), 0, 0, String::new())
        });

        let song_tables = tables.get(&md5.to_lowercase()).cloned().unwrap_or_default();

        out.push(ScoreRow {
            sha256,
            title,
            artist,
            level,
            difficulty,
            clear,
            exscore,
            minbp,
            date,
            tables: song_tables,
        });
    }

    Ok(out)
}

pub fn list_tables(path: &str) -> Result<Vec<String>, String> {
    let conn = read_only(Path::new(path)).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .map_err(|e| e.to_string())?;
    let names = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(names)
}

fn table_names(conn: &Connection) -> Vec<String> {
    let mut stmt = match conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
    {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], |r| r.get::<_, String>(0))
        .map(|it| it.filter_map(|x| x.ok()).collect())
        .unwrap_or_default()
}

fn table_columns(conn: &Connection, table: &str) -> Vec<String> {
    let mut stmt = match conn.prepare(&format!("PRAGMA table_info(\"{}\")", table)) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], |r| r.get::<_, String>(1))
        .map(|it| it.filter_map(|x| x.ok()).collect())
        .unwrap_or_default()
}

fn schema_report(label: &str, path: &Path, out: &mut String) {
    out.push_str(&format!("\n== {} ({}) ==\n", label, path.display()));
    if !path.exists() {
        out.push_str("  (missing)\n");
        return;
    }
    let conn = match read_only(path) {
        Ok(c) => c,
        Err(e) => {
            out.push_str(&format!("  open error: {e}\n"));
            return;
        }
    };
    for t in table_names(&conn) {
        out.push_str(&format!("  [{}] {}\n", t, table_columns(&conn, &t).join(", ")));
    }
}

fn dump_head(label: &str, path: &Path, max: usize, out: &mut String) {
    out.push_str(&format!("\n--- {} ({}) ---\n", label, path.display()));
    match fs::read(path) {
        Ok(bytes) => {
            let n = bytes.len().min(max);
            out.push_str(&String::from_utf8_lossy(&bytes[..n]));
            if bytes.len() > max {
                out.push_str("\n...[truncated]...\n");
            }
        }
        Err(e) => out.push_str(&format!("(read error: {e})\n")),
    }
}

fn list_dir(label: &str, dir: &Path, out: &mut String) {
    out.push_str(&format!("\n{} ({}):\n", label, dir.display()));
    match fs::read_dir(dir) {
        Ok(rd) => {
            for e in rd.flatten() {
                let slash = if e.path().is_dir() { "/" } else { "" };
                out.push_str(&format!("  {}{}\n", e.file_name().to_string_lossy(), slash));
            }
        }
        Err(_) => out.push_str("  (missing / unavailable)\n"),
    }
}

pub fn diagnostics(root: &str) -> Result<String, String> {
    let root = PathBuf::from(root);
    let mut out = String::new();
    out.push_str(&format!("ROOT: {}\n", root.display()));

    list_dir("Files in root", &root, &mut out);
    let table_dir = root.join("table");
    list_dir("table folder", &table_dir, &mut out);

    dump_head("table/default.json", &table_dir.join("default.json"), 1500, &mut out);
    if let Ok(rd) = fs::read_dir(&table_dir) {
        if let Some(bmt) = rd
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().map_or(false, |x| x == "bmt"))
        {
            dump_head("table/<first>.bmt", &bmt, 1500, &mut out);
        }
    }

    let player_dir = root.join("player");
    out.push_str(&format!("\nPlayers ({}):\n", player_dir.display()));
    if let Ok(rd) = fs::read_dir(&player_dir) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                out.push_str(&format!("  {}/\n", e.file_name().to_string_lossy()));
                if let Ok(inner) = fs::read_dir(e.path()) {
                    for f in inner.flatten() {
                        out.push_str(&format!("    {}\n", f.file_name().to_string_lossy()));
                    }
                }
            }
        }
    } else {
        out.push_str("  (no player folder)\n");
    }

    let (songdata, scorelog) = resolve_paths(&root);
    if let Some(p) = songdata {
        schema_report("songdata.db", &p, &mut out);
    }
    if let Some(p) = scorelog {
        schema_report("scorelog.db", &p, &mut out);
    }
    if let Some(p) = newest_player_db(&root, "score.db") {
        schema_report("score.db", &p, &mut out);
    }
    if let Some(p) = newest_player_db(&root, "scoredatalog.db") {
        schema_report("scoredatalog.db", &p, &mut out);
    }
    schema_report("songinfo.db", &root.join("songinfo.db"), &mut out);

    Ok(out)
}
