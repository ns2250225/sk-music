use crate::models::*;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::{path::Path, sync::Arc};
#[derive(Clone)]
pub struct Database(pub Arc<Mutex<Connection>>);
impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        let c = Connection::open(path).map_err(|e| e.to_string())?;
        c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
 CREATE TABLE IF NOT EXISTS tracks(id TEXT PRIMARY KEY,data TEXT NOT NULL,updated_at INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS sources(id TEXT PRIMARY KEY,track_id TEXT NOT NULL,data TEXT NOT NULL,last_seen INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS favorites(track_id TEXT PRIMARY KEY,created_at INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS play_history(id INTEGER PRIMARY KEY AUTOINCREMENT,track_id TEXT NOT NULL,played_at INTEGER NOT NULL,duration_played INTEGER DEFAULT 0,completion_ratio REAL DEFAULT 0);
 CREATE TABLE IF NOT EXISTS queue(position INTEGER PRIMARY KEY,track_id TEXT NOT NULL);
 CREATE TABLE IF NOT EXISTS downloads(id TEXT PRIMARY KEY,track_id TEXT NOT NULL,data TEXT NOT NULL,created_at INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS search_history(id INTEGER PRIMARY KEY AUTOINCREMENT,query TEXT NOT NULL,searched_at INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS search_cache(query TEXT PRIMARY KEY,data TEXT NOT NULL,expires_at INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);").map_err(|e|e.to_string())?;
        Ok(Self(Arc::new(Mutex::new(c))))
    }
    pub fn set_setting(&self, k: &str, v: &str) -> Result<(), String> {
        self.0.lock().execute("INSERT INTO settings(key,value)VALUES(?1,?2)ON CONFLICT(key)DO UPDATE SET value=excluded.value",params![k,v]).map(|_|()).map_err(|e|e.to_string())
    }
    pub fn get_setting(&self, k: &str) -> Option<String> {
        self.0
            .lock()
            .query_row("SELECT value FROM settings WHERE key=?1", [k], |r| r.get(0))
            .optional()
            .ok()
            .flatten()
    }
    pub fn save_settings(&self, s: &Settings) -> Result<(), String> {
        self.set_setting(
            "settings",
            &serde_json::to_string(s).map_err(|e| e.to_string())?,
        )
    }
    pub fn settings(&self) -> Settings {
        self.get_setting("settings")
            .and_then(|x| serde_json::from_str(&x).ok())
            .unwrap_or_default()
    }
    pub fn save_track(&self, t: &Track) -> Result<(), String> {
        self.0.lock().execute("INSERT INTO tracks(id,data,updated_at)VALUES(?1,?2,unixepoch())ON CONFLICT(id)DO UPDATE SET data=excluded.data,updated_at=unixepoch()",params![t.id,serde_json::to_string(t).unwrap()]).map(|_|()).map_err(|e|e.to_string())
    }
    pub fn track(&self, id: &str) -> Option<Track> {
        self.0
            .lock()
            .query_row("SELECT data FROM tracks WHERE id=?1", [id], |r| {
                r.get::<_, String>(0)
            })
            .optional()
            .ok()
            .flatten()
            .and_then(|x| serde_json::from_str(&x).ok())
    }
    pub fn tracks(&self, sql: &str) -> Vec<Track> {
        let c = self.0.lock();
        let mut st = match c.prepare(sql) {
            Ok(x) => x,
            Err(_) => return vec![],
        };
        st.query_map([], |r| r.get::<_, String>(0))
            .map(|xs| {
                xs.filter_map(|x| x.ok().and_then(|v| serde_json::from_str(&v).ok()))
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn favorite(&self, id: &str, on: bool) -> Result<(), String> {
        let c = self.0.lock();
        if on {
            c.execute(
                "INSERT OR REPLACE INTO favorites VALUES(?1,unixepoch())",
                [id],
            )
        } else {
            c.execute("DELETE FROM favorites WHERE track_id=?1", [id])
        }
        .map(|_| ())
        .map_err(|e| e.to_string())
    }
    pub fn is_favorite(&self, id: &str) -> bool {
        self.0
            .lock()
            .query_row(
                "SELECT 1 FROM favorites WHERE track_id=?1",
                [id],
                |_| Ok(()),
            )
            .is_ok()
    }
    pub fn cache_get(&self, q: &str) -> Option<Vec<Track>> {
        self.0
            .lock()
            .query_row(
                "SELECT data FROM search_cache WHERE query=?1 AND expires_at>unixepoch()",
                [q],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten()
            .and_then(|x| serde_json::from_str(&x).ok())
    }
    pub fn cache_put(&self, q: &str, t: &[Track]) {
        let _ = self.0.lock().execute(
            "INSERT OR REPLACE INTO search_cache VALUES(?1,?2,unixepoch()+90)",
            params![q, serde_json::to_string(t).unwrap_or_default()],
        );
    }
    pub fn history_add(&self, id: &str) {
        let _ = self.0.lock().execute(
            "INSERT INTO play_history(track_id,played_at)VALUES(?1,unixepoch())",
            [id],
        );
    }
    pub fn download_save(&self, d: &DownloadItem) {
        let _ = self.0.lock().execute(
            "INSERT OR REPLACE INTO downloads VALUES(?1,?2,?3,unixepoch())",
            params![
                d.id,
                d.track.id,
                serde_json::to_string(d).unwrap_or_default()
            ],
        );
    }
    pub fn downloads(&self) -> Vec<DownloadItem> {
        let c = self.0.lock();
        let mut s = match c.prepare("SELECT data FROM downloads ORDER BY created_at DESC") {
            Ok(x) => x,
            Err(_) => return vec![],
        };
        s.query_map([], |r| r.get::<_, String>(0))
            .map(|xs| {
                xs.filter_map(|x| x.ok().and_then(|v| serde_json::from_str(&v).ok()))
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn download_remove(&self, id: &str) -> Result<(), String> {
        self.0
            .lock()
            .execute("DELETE FROM downloads WHERE id=?1", [id])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    pub fn queue_save(&self, ids: &[String]) {
        let mut c = self.0.lock();
        let tx = match c.transaction() {
            Ok(x) => x,
            Err(_) => return,
        };
        let _ = tx.execute("DELETE FROM queue", []);
        for (i, id) in ids.iter().enumerate() {
            let _ = tx.execute("INSERT INTO queue VALUES(?1,?2)", params![i, id]);
        }
        let _ = tx.commit();
    }
    pub fn queue(&self) -> Vec<Track> {
        self.tracks(
            "SELECT t.data FROM queue q JOIN tracks t ON t.id=q.track_id ORDER BY q.position",
        )
    }
}
