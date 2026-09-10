use crate::{
    db::Database,
    models::*,
    services::{normalize, score},
};
use parking_lot::RwLock;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::Mutex,
    time::sleep,
};
#[cfg(windows)]
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
#[cfg(not(windows))]
use tokio::net::UnixStream;

// 0x08000000 = CREATE_NO_WINDOW, only meaningful for console hosts on Windows.
#[cfg(windows)]
trait NoWindow {
    fn no_window(&mut self) -> &mut Self;
}
#[cfg(windows)]
impl NoWindow for Command {
    fn no_window(&mut self) -> &mut Self {
        self.creation_flags(0x08000000)
    }
}
#[cfg(not(windows))]
trait NoWindow {
    fn no_window(&mut self) -> &mut Self;
}
#[cfg(not(windows))]
impl NoWindow for Command {
    fn no_window(&mut self) -> &mut Self {
        self
    }
}

pub struct AppState {
    pub db: Database,
    pub paths: Paths,
    pub settings: Arc<RwLock<Settings>>,
    client: reqwest::Client,
    tracks: Arc<RwLock<HashMap<String, Track>>>,
    queue: Arc<RwLock<Vec<String>>>,
    current: Arc<RwLock<Option<String>>>,
    mpv: Arc<Mutex<Option<Mpv>>>,
}
struct Mpv {
    child: Child,
    pipe: String,
}
impl Drop for Mpv {
    fn drop(&mut self) {
        // Dropping tokio::process::Child does not terminate the process by
        // default. Ensure audio can never outlive the desktop application.
        let _ = self.child.start_kill();
    }
}
impl AppState {
    pub fn new(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let root = app.path().app_local_data_dir()?;
        let paths = Paths {
            cache_audio: root.join("cache").join("audio"),
            covers: root.join("cache").join("covers"),
            downloads: root.join("downloads"),
            runtime: root.join("runtime"),
            logs: root.join("logs"),
            root: root.clone(),
        };
        for p in [
            &paths.cache_audio,
            &paths.covers,
            &paths.downloads,
            &paths.runtime,
            &paths.logs,
        ] {
            fs::create_dir_all(p)?
        }
        ensure_embedded_runtime(&paths)?;
        let db = Database::open(&root.join("soulmusic.db"))?;
        let mut settings = db.settings();
        if settings.auto_account {
            if settings.username.is_empty() {
                settings.username = format!(
                    "SoulMusic_{}",
                    &uuid::Uuid::new_v4().simple().to_string()[..6].to_uppercase()
                );
            }
            if settings.password.is_empty() {
                settings.password = uuid::Uuid::new_v4().simple().to_string()[..20].to_string();
            }
            db.save_settings(&settings)?;
        }
        let queue_tracks = db.queue();
        let mut tracks = HashMap::new();
        for t in &queue_tracks {
            tracks.insert(t.id.clone(), t.clone());
        }
        Ok(Self {
            db,
            paths,
            settings: Arc::new(RwLock::new(settings)),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()?,
            tracks: Arc::new(RwLock::new(tracks)),
            queue: Arc::new(RwLock::new(
                queue_tracks.iter().map(|x| x.id.clone()).collect(),
            )),
            current: Arc::new(RwLock::new(None)),
            mpv: Arc::new(Mutex::new(None)),
        })
    }
    pub async fn bootstrap(&self) -> Result<Bootstrap, String> {
        let favorites=self.db.tracks("SELECT t.data FROM favorites f JOIN tracks t ON t.id=f.track_id ORDER BY f.created_at DESC LIMIT 100");
        let history=self.db.tracks("SELECT t.data FROM play_history h JOIN tracks t ON t.id=h.track_id GROUP BY t.id ORDER BY MAX(h.played_at) DESC LIMIT 100");
        for t in favorites.iter().chain(history.iter()) {
            self.tracks.write().insert(t.id.clone(), t.clone());
        }
        let settings = self.settings.read().clone();
        let connection = self.reconnect().await;
        Ok(Bootstrap {
            onboarded: self.db.get_setting("onboarded").as_deref() == Some("true"),
            tracks: vec![],
            favorites,
            history,
            downloads: self.db.downloads(),
            queue: self.queue_tracks(),
            settings,
            connection,
            cache_used: dir_size(&self.paths.cache_audio),
        })
    }
    fn auth(&self, r: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let s = self.settings.read();
        if s.api_key.is_empty() {
            r
        } else {
            r.header("X-API-Key", &s.api_key)
        }
    }
    async fn application_state(&self) -> Option<Value> {
        let url = format!(
            "{}/api/v0/application",
            self.settings.read().slskd_url.trim_end_matches('/')
        );
        self.auth(self.client.get(url))
            .send()
            .await
            .ok()?
            .json::<Value>()
            .await
            .ok()
    }
    async fn health(&self) -> bool {
        self.application_state()
            .await
            .map(|v| {
                v.pointer("/server/isLoggedIn")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }
    pub fn monitor_connection(&self, app: AppHandle) {
        let client = self.client.clone();
        let settings = self.settings.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                let (url, api_key) = {
                    let s = settings.read();
                    (
                        format!("{}/api/v0/application", s.slskd_url.trim_end_matches('/')),
                        s.api_key.clone(),
                    )
                };
                let mut request = client.get(url);
                if !api_key.is_empty() {
                    request = request.header("X-API-Key", api_key);
                }
                let state = match request.send().await {
                    Ok(response) => match response.json::<Value>().await {
                        Ok(value) => {
                            let logged_in = value
                                .pointer("/server/isLoggedIn")
                                .and_then(Value::as_bool)
                                .unwrap_or(false);
                            if logged_in {
                                "connected"
                            } else {
                                "connecting"
                            }
                        }
                        Err(_) => "connecting",
                    },
                    Err(_) => "offline",
                };
                // Emit periodically as bootstrap may still be running when the
                // first state transition happens and the UI is not ready yet.
                let _ = app.emit("connection://state", state);
                sleep(Duration::from_secs(2)).await;
            }
        });
    }
    pub async fn reconnect(&self) -> String {
        let exe = slskd_exe(&self.paths.runtime);
        if exe.exists() {
            let _ = self.write_slskd_config();
            if self.application_state().await.is_some() {
                if let Ok(mut child) = kill_slskd_command()
                    .no_window()
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    let _ = child.wait().await;
                }
                sleep(Duration::from_millis(500)).await;
            }
            let _ = Command::new(exe)
                .arg("--config")
                .arg(self.paths.runtime.join("slskd/slskd.yml"))
                .arg("--app-dir")
                .arg(self.paths.runtime.join("slskd-data"))
                .arg("--no-version-check")
                .no_window()
                .spawn();
            for _ in 0..60 {
                sleep(Duration::from_millis(500)).await;
                if self.health().await {
                    return "connected".into();
                }
            }
        }
        if self.health().await {
            "connected"
        } else {
            "offline"
        }
        .into()
    }
    pub async fn search(&self, q: &str, refresh: bool) -> Result<Vec<Track>, String> {
        let key = normalize::clean(q);
        if !refresh {
            if let Some(x) = self.db.cache_get(&key) {
                return Ok(x);
            }
        }
        if !self.health().await {
            return Err(
                "Soulseek 尚未登录。请在设置中填写有效账号密码并点击“重新连接”，连接成功后再搜索"
                    .into(),
            );
        }
        let base = self
            .settings
            .read()
            .slskd_url
            .trim_end_matches('/')
            .to_string();
        let mut search_ids = Vec::new();
        for network_query in expand_search_queries(q) {
            let body = json!({
                "searchText": network_query,
                "responseLimit": 1000,
                "fileLimit": 50000,
                "filterResponses": false,
                "minimumResponseFileCount": 0,
                "maximumPeerQueueLength": 1000000,
                "minimumPeerUploadSpeed": 0
            });
            if let Ok(response) = self
                .auth(
                    self.client
                        .post(format!("{base}/api/v0/searches"))
                        .json(&body),
                )
                .send()
                .await
            {
                if let Ok(v) = response.error_for_status().and_then(|r| Ok(r)) {
                    if let Ok(data) = v.json::<Value>().await {
                        if let Some(id) = data
                            .get("id")
                            .or_else(|| data.get("searchId"))
                            .and_then(Value::as_str)
                        {
                            search_ids.push(id.to_string());
                        }
                    }
                }
            }
            sleep(Duration::from_millis(80)).await;
        }
        if search_ids.is_empty() {
            return Err("未能启动 Soulseek 搜索，请稍后重试".into());
        }
        // Wait for all original/alias/broad searches. Responses can arrive late and
        // different distributed parents often cover different parts of the network.
        for _ in 0..30 {
            sleep(Duration::from_millis(800)).await;
            let mut complete = 0usize;
            for id in &search_ids {
                if let Ok(response) = self
                    .auth(self.client.get(format!("{base}/api/v0/searches/{id}")))
                    .send()
                    .await
                {
                    if let Ok(state) = response.json::<Value>().await {
                        if state
                            .get("isComplete")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                        {
                            complete += 1;
                        }
                    }
                }
            }
            if complete == search_ids.len() {
                break;
            }
        }
        let mut groups: HashMap<String, Track> = HashMap::new();
        let mut responses = Vec::new();
        for id in &search_ids {
            if let Ok(response) = self
                .auth(
                    self.client
                        .get(format!("{base}/api/v0/searches/{id}/responses")),
                )
                .send()
                .await
            {
                if let Ok(data) = response.json::<Value>().await {
                    responses.extend(data.as_array().cloned().unwrap_or_default());
                }
            }
        }
        for response in responses {
            let user = response
                .get("username")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let speed = response.get("uploadSpeed").and_then(Value::as_u64);
            let queue = response
                .get("queueLength")
                .and_then(Value::as_u64)
                .map(|x| x as u32);
            for f in response
                .get("files")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
            {
                let path = f
                    .get("filename")
                    .or_else(|| f.get("path"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if !normalize::is_audio(path) {
                    continue;
                }
                let size = f.get("size").and_then(Value::as_u64).unwrap_or(0);
                let ext = path.rsplit('.').next().unwrap_or("").to_ascii_uppercase();
                let bitrate = f
                    .get("bitRate")
                    .or_else(|| f.get("bitrate"))
                    .and_then(Value::as_u64)
                    .map(|x| x as u32);
                if ext == "MP3" && bitrate.unwrap_or(320) < self.settings.read().minimum_bitrate {
                    continue;
                }
                let (title, artist) = normalize::parse(path);
                let norm = normalize::clean(&format!("{} {}", artist, title));
                let track_id = normalize::id(&norm);
                let sid = normalize::id(&format!("{}{}{}", user, path, size));
                let mut src = Source {
                    id: sid,
                    username: user.into(),
                    remote_path: path.into(),
                    filename: path.rsplit(['\\', '/']).next().unwrap_or(path).into(),
                    size,
                    format: ext.clone(),
                    bitrate,
                    sample_rate: None,
                    bit_depth: None,
                    upload_speed: speed,
                    queue_length: queue,
                    score: 0.,
                };
                src.score = score::source(&src, &self.settings.read());
                let t = groups.entry(track_id.clone()).or_insert(Track {
                    id: track_id.clone(),
                    title,
                    artist,
                    album: path
                        .rsplit_once(['\\', '/'])
                        .map(|x| x.0.rsplit(['\\', '/']).next().unwrap_or("").into())
                        .unwrap_or_default(),
                    duration: f.get("length").and_then(Value::as_f64).unwrap_or(0.),
                    cover: None,
                    local_path: None,
                    formats: vec![],
                    source_count: 0,
                    favorite: self.db.is_favorite(&track_id),
                    sources: vec![],
                });
                if !t.formats.contains(&ext) {
                    t.formats.push(ext)
                }
                if !t.sources.iter().any(|existing| existing.id == src.id) {
                    t.sources.push(src);
                }
                t.source_count = t.sources.len();
            }
        }
        // slskd has already applied the network query. Keep every whitelisted audio
        // result here; an additional whole-query filename filter hid valid files
        // whose artist/title separators or translated names differed.
        let mut out: Vec<_> = groups.into_values().collect();
        for t in &mut out {
            t.sources.sort_by(|a, b| b.score.total_cmp(&a.score));
            let _ = self.db.save_track(t);
            self.tracks.write().insert(t.id.clone(), t.clone());
        }
        out.sort_by(|a, b| {
            b.source_count.cmp(&a.source_count).then_with(|| {
                b.sources
                    .first()
                    .map(|x| x.score)
                    .unwrap_or(0.)
                    .total_cmp(&a.sources.first().map(|x| x.score).unwrap_or(0.))
            })
        });
        if !out.is_empty() {
            self.db.cache_put(&key, &out);
        }
        Ok(out)
    }
    pub async fn play(
        &self,
        id: &str,
        source_id: Option<String>,
        app: AppHandle,
    ) -> Result<CommandStatus, String> {
        let track = self.get_track(id).ok_or("歌曲不存在")?;
        let _ = app.emit("player://track", &track);
        let local_file = track
            .local_path
            .as_ref()
            .map(PathBuf::from)
            .filter(|p| p.is_file() && normalize::is_audio(&p.to_string_lossy()))
            // Backward compatibility for library rows scanned by older builds,
            // where the local audio path was stored in the cover field.
            .or_else(|| {
                track
                    .cover
                    .as_ref()
                    .map(PathBuf::from)
                    .filter(|p| p.is_file() && normalize::is_audio(&p.to_string_lossy()))
            });
        let downloaded = track.sources.iter().find_map(|source| {
            find_ready_download(&self.paths.downloads, &source.filename, source.size)
        });
        let cached = find_cached(&self.paths.cache_audio, id)
            .or(local_file)
            .or(downloaded);
        if let Some(path) = cached {
            self.open_mpv(&path, app.clone()).await?;
        } else {
            let candidates: Vec<Source> = if let Some(chosen) = source_id {
                track
                    .sources
                    .iter()
                    .find(|s| s.id == chosen)
                    .cloned()
                    .into_iter()
                    .collect()
            } else {
                track.sources.iter().take(3).cloned().collect()
            };
            let mut selected = None;
            let mut last_error = String::new();
            for candidate in candidates {
                let _ = app.emit("player://state", "connecting");
                match self.request_download(&candidate).await {
                    Ok(()) => {
                        selected = Some(candidate);
                        break;
                    }
                    Err(error) => {
                        last_error = error;
                        let _ = app.emit("player://state", "searching_source");
                    }
                }
            }
            let source = selected.ok_or_else(|| {
                if last_error.is_empty() {
                    "当前没有可播放来源".into()
                } else {
                    format!("音源连接失败：{last_error}")
                }
            })?;
            let threshold = buffer_threshold(&source);
            let this = self.clone_parts();
            let app2 = app.clone();
            let incomplete_root = self.paths.cache_audio.clone();
            let completed_root = self.paths.downloads.clone();
            let remote_name = source.filename.clone();
            tokio::spawn(async move {
                let mut finished = false;
                for attempt in 0..300 {
                    if let Some(final_path) =
                        find_ready_download(&completed_root, &remote_name, source.size)
                    {
                        match this.open_path(&final_path, app2.clone()).await {
                            Ok(()) => {
                                finished = true;
                                break;
                            }
                            Err(_) => {
                                // slskd can keep the destination locked briefly
                                // after moving it out of the incomplete folder.
                                let _ = app2.emit("player://state", "buffering");
                            }
                        }
                    }
                    if let Some(temp) = find_incomplete(&incomplete_root, &remote_name) {
                        if let Ok(m) = fs::metadata(&temp) {
                            let len = m.len();
                            let _ = app2.emit("player://state", "downloading");
                            let _=app2.emit("player://position",json!({"position":0,"duration":track.duration,"buffered":if source.size>0{track.duration*len as f64/source.size as f64}else{0.}}));
                            if len >= threshold {
                                let _ = app2.emit("player://state", "buffering");
                            }
                        }
                    }
                    if attempt == 10 {
                        let _ = app2.emit("player://state", "queued");
                    }
                    sleep(Duration::from_millis(300)).await;
                }
                if !finished {
                    let _ = app2.emit("player://state", "error");
                }
            });
        }
        self.current.write().replace(id.into());
        self.db.history_add(id);
        Ok(CommandStatus {
            status: "buffering".into(),
        })
    }
    async fn request_download(&self, s: &Source) -> Result<(), String> {
        let base = self
            .settings
            .read()
            .slskd_url
            .trim_end_matches('/')
            .to_string();
        let body = json!([{"filename":s.remote_path,"size":s.size}]);
        let response = self
            .auth(
                self.client
                    .post(format!(
                        "{base}/api/v0/transfers/downloads/{}",
                        urlencoding::encode(&s.username)
                    ))
                    .json(&body),
            )
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        let result: Value = response.json().await.map_err(|e| e.to_string())?;
        let enqueued = result
            .get("enqueued")
            .or_else(|| result.get("Enqueued"))
            .and_then(Value::as_array)
            .map(|v| v.len())
            .unwrap_or(0);
        if enqueued == 0 {
            let failed = result
                .get("failed")
                .or_else(|| result.get("Failed"))
                .map(Value::to_string)
                .unwrap_or_else(|| "slskd 未接受该文件".into());
            return Err(failed);
        }
        Ok(())
    }
    async fn open_mpv(&self, path: &Path, app: AppHandle) -> Result<(), String> {
        self.clone_parts().open_path(path, app).await
    }
    fn clone_parts(&self) -> PlayerParts {
        PlayerParts {
            mpv: self.mpv.clone(),
            runtime: self.paths.runtime.clone(),
        }
    }
    pub async fn player_command(&self, property: &str, value: Value) -> Result<(), String> {
        let p = self
            .mpv
            .lock()
            .await
            .as_ref()
            .map(|m| m.pipe.clone())
            .ok_or("播放器尚未启动")?;
        mpv_send(&p, json!({"command":["set_property",property,value]})).await
    }
    pub fn stop_player(&self) {
        if let Ok(mut player) = self.mpv.try_lock() {
            if let Some(mut mpv) = player.take() {
                let _ = mpv.child.start_kill();
            }
        }
    }
    pub async fn next(&self, app: AppHandle) -> Result<(), String> {
        let ids = self.queue.read().clone();
        let cur = self.current.read().clone();
        let n = cur
            .and_then(|c| ids.iter().position(|x| x == &c))
            .map(|i| i + 1)
            .unwrap_or(0);
        if let Some(id) = ids.get(n) {
            self.play(id, None, app).await.map(|_| ())
        } else {
            Ok(())
        }
    }
    pub fn add_queue(&self, id: &str, app: AppHandle) -> Result<(), String> {
        if self.get_track(id).is_none() {
            return Err("歌曲不存在".into());
        }
        let ids = {
            let mut queue = self.queue.write();
            if !queue.iter().any(|item| item == id) {
                queue.push(id.into());
            }
            queue.clone()
        };
        // Do not call queue_tracks while holding the write lock: it acquires a
        // read lock on the same RwLock and would deadlock the command thread.
        self.db.queue_save(&ids);
        let _ = app.emit("queue://updated", self.queue_tracks());
        Ok(())
    }
    pub fn remove_queue(&self, i: usize, app: AppHandle) -> Result<(), String> {
        let ids = {
            let mut queue = self.queue.write();
            if i < queue.len() {
                queue.remove(i);
            }
            queue.clone()
        };
        self.db.queue_save(&ids);
        let _ = app.emit("queue://updated", self.queue_tracks());
        Ok(())
    }
    pub fn clear_queue(&self, app: AppHandle) -> Result<(), String> {
        self.queue.write().clear();
        self.db.queue_save(&[]);
        let _ = app.emit("queue://updated", Vec::<Track>::new());
        Ok(())
    }
    fn queue_tracks(&self) -> Vec<Track> {
        self.queue
            .read()
            .iter()
            .filter_map(|x| self.get_track(x))
            .collect()
    }
    fn get_track(&self, id: &str) -> Option<Track> {
        self.tracks
            .read()
            .get(id)
            .cloned()
            .or_else(|| self.db.track(id))
    }
    pub fn remember_tracks(&self, tracks: &[Track]) {
        let mut known = self.tracks.write();
        for track in tracks {
            known.insert(track.id.clone(), track.clone());
        }
    }
    pub fn favorite(&self, id: &str, on: bool) -> Result<(), String> {
        self.db.favorite(id, on)
    }
    pub async fn download(&self, id: &str, _pref: &str, app: AppHandle) -> Result<(), String> {
        let track = self.get_track(id).ok_or("歌曲不存在")?;
        let source = track.sources.first().cloned().ok_or("没有下载来源")?;
        let did = format!("download_{}", uuid::Uuid::new_v4());
        let ext = source.format.to_ascii_lowercase();
        let dir = if self.settings.read().download_directory.is_empty() {
            self.paths.downloads.clone()
        } else {
            PathBuf::from(&self.settings.read().download_directory)
        };
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!(
            "{} - {}.{}",
            safe_name(&track.artist),
            safe_name(&track.title),
            ext
        ));
        let item = DownloadItem {
            id: did.clone(),
            track,
            status: "queued".into(),
            downloaded_bytes: 0,
            total_bytes: source.size,
            speed: 0,
            local_path: Some(path.to_string_lossy().into()),
        };
        self.db.download_save(&item);
        if let Err(error) = self.request_download(&source).await {
            let mut failed = item.clone();
            failed.status = "下载失败".into();
            self.db.download_save(&failed);
            let _ = app.emit("transfer://progress", &failed);
            return Err(error);
        }
        let _ = app.emit("transfer://progress", &item);
        let client = self.client.clone();
        let settings = self.settings.clone();
        let db = self.db.clone();
        let completed_root = self.paths.downloads.clone();
        let mut tracked = item;
        tokio::spawn(async move {
            for _ in 0..3600 {
                // Cancellation removes the row; do not recreate it on the next poll.
                if !db
                    .downloads()
                    .iter()
                    .any(|download| download.id == tracked.id)
                {
                    break;
                }
                let (base, api_key) = {
                    let current = settings.read();
                    (
                        current.slskd_url.trim_end_matches('/').to_string(),
                        current.api_key.clone(),
                    )
                };
                let mut request = client.get(format!("{base}/api/v0/transfers/downloads"));
                if !api_key.is_empty() {
                    request = request.header("X-API-Key", api_key);
                }
                if let Ok(response) = request.send().await {
                    if let Ok(transfers) = response.json::<Value>().await {
                        if let Some(transfer) =
                            find_transfer(&transfers, &source.username, &source.remote_path)
                        {
                            tracked.downloaded_bytes = transfer
                                .get("bytesTransferred")
                                .and_then(Value::as_u64)
                                .unwrap_or(tracked.downloaded_bytes);
                            tracked.speed = transfer
                                .get("averageSpeed")
                                .and_then(Value::as_f64)
                                .unwrap_or(0.0) as u64;
                            let state = transfer.get("state").and_then(Value::as_str).unwrap_or("");
                            tracked.status =
                                if state.contains("Completed") && state.contains("Succeeded") {
                                    "已完成".into()
                                } else if state.contains("InProgress") {
                                    "正在下载".into()
                                } else if state.contains("Failed")
                                    || state.contains("Cancelled")
                                    || state.contains("Rejected")
                                    || state.contains("TimedOut")
                                {
                                    "下载失败".into()
                                } else {
                                    "等待远程用户".into()
                                };
                            if tracked.status == "已完成" {
                                if let Some(actual) = find_ready_download(
                                    &completed_root,
                                    &source.filename,
                                    source.size,
                                ) {
                                    let destination = tracked
                                        .local_path
                                        .as_ref()
                                        .map(PathBuf::from)
                                        .unwrap_or_else(|| actual.clone());
                                    if destination != actual {
                                        if let Some(parent) = destination.parent() {
                                            let _ = fs::create_dir_all(parent);
                                        }
                                        if fs::copy(&actual, &destination).is_ok() {
                                            tracked.local_path =
                                                Some(destination.to_string_lossy().into());
                                        } else {
                                            sleep(Duration::from_millis(500)).await;
                                            continue;
                                        }
                                    } else {
                                        tracked.local_path = Some(actual.to_string_lossy().into());
                                    }
                                    tracked.downloaded_bytes = source.size;
                                    db.download_save(&tracked);
                                    let _ = app.emit("transfer://progress", &tracked);
                                    break;
                                }
                            }
                            db.download_save(&tracked);
                            let _ = app.emit("transfer://progress", &tracked);
                            if tracked.status == "下载失败" {
                                break;
                            }
                        }
                    }
                }
                sleep(Duration::from_millis(500)).await;
            }
        });
        Ok(())
    }
    pub fn cancel_download(&self, id: &str) -> Result<(), String> {
        self.db.download_remove(id)
    }
    pub fn get_downloads(&self) -> Result<Vec<DownloadItem>, String> {
        Ok(self.db.downloads())
    }
    pub fn update_settings(&self, s: Settings) -> Result<(), String> {
        self.db.save_settings(&s)?;
        *self.settings.write() = s;
        self.write_slskd_config()?;
        Ok(())
    }

    fn write_slskd_config(&self) -> Result<(), String> {
        let s = self.settings.read();
        let token = if s.api_key.is_empty() {
            "soulmusic-local"
        } else {
            &s.api_key
        };
        let downloads = self.paths.downloads.to_string_lossy().into_owned();
        let incomplete = self.paths.cache_audio.to_string_lossy().into_owned();
        #[cfg(windows)]
        let (downloads, incomplete) = (
            downloads.replace('/', "\\"),
            incomplete.replace('/', "\\"),
        );
        let yaml = format!("soulseek:\n  username: '{}'\n  password: '{}'\nweb:\n  port: 5030\n  url_base: /\n  content_path: wwwroot\n  authentication:\n    disabled: true\n  ip_address: 127.0.0.1\n  api_keys:\n    soulmusic:\n      key: '{}'\n      role: administrator\ndirectories:\n  downloads: '{}'\n  incomplete: '{}'\nshares:\n  directories: []\n", s.username.replace('\'', "''"), s.password.replace('\'', "''"), token, downloads, incomplete);
        fs::write(self.paths.runtime.join("slskd/slskd.yml"), yaml).map_err(|e| e.to_string())
    }
    pub fn clear_cache(&self) -> Result<u64, String> {
        let before = dir_size(&self.paths.cache_audio);
        for e in fs::read_dir(&self.paths.cache_audio).map_err(|e| e.to_string())? {
            let p = e.map_err(|e| e.to_string())?.path();
            if p.is_file() {
                let _ = fs::remove_file(p);
            }
        }
        Ok(before)
    }
}
struct PlayerParts {
    mpv: Arc<Mutex<Option<Mpv>>>,
    runtime: PathBuf,
}
impl PlayerParts {
    async fn open_path(&self, path: &Path, app: AppHandle) -> Result<(), String> {
        let mut lock = self.mpv.lock().await;
        if lock.is_none() {
            let exe = if cfg!(windows) {
                [self.runtime.join("mpv/mpv.exe"), PathBuf::from("mpv.exe")]
                    .into_iter()
                    .find(|p| p.exists())
                    .unwrap_or(PathBuf::from("mpv.exe"))
            } else {
                [
                    self.runtime.join("mpv/mpv.app/Contents/MacOS/mpv"),
                    self.runtime.join("mpv/mpv"),
                    PathBuf::from("/opt/homebrew/bin/mpv"),
                    PathBuf::from("/usr/local/bin/mpv"),
                ]
                .into_iter()
                .find(|p| p.exists())
                .unwrap_or(PathBuf::from("mpv"))
            };
            let pipe = if cfg!(windows) {
                format!(r"\\.\pipe\soulmusic-mpv-{}", std::process::id())
            } else {
                std::env::temp_dir()
                    .join(format!("soulmusic-mpv-{}.sock", std::process::id()))
                    .to_string_lossy()
                    .into_owned()
            };
            #[cfg(not(windows))]
            let _ = fs::remove_file(&pipe);
            let ipc_arg = format!("--input-ipc-server={pipe}");
            let child = Command::new(exe)
                .args([
                    "--no-config",
                    "--no-video",
                    "--idle=yes",
                    "--keep-open=no",
                    "--audio-display=no",
                    "--force-window=no",
                    "--terminal=no",
                    &ipc_arg,
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .no_window()
                .spawn()
                .map_err(|_| {
                    if cfg!(windows) {
                        "未找到 mpv。请将 mpv.exe 放入应用 runtime/mpv 目录或系统 PATH".to_string()
                    } else {
                        "mpv 启动失败。请将 mpv 放入应用 runtime/mpv 目录，或执行 brew install mpv 后重试"
                            .to_string()
                    }
                })?;
            *lock = Some(Mpv {
                child,
                pipe: pipe.clone(),
            });
            for _ in 0..30 {
                if mpv_send(&pipe, json!({"command":["get_property","idle-active"]}))
                    .await
                    .is_ok()
                {
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
            let monitor_pipe = pipe.clone();
            let monitor_app = app.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let position = mpv_get(&monitor_pipe, "time-pos").await;
                    let duration = mpv_get(&monitor_pipe, "duration").await;
                    if position.is_err() && duration.is_err() {
                        sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                    let position = position.ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let duration = duration.ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let _ = monitor_app.emit(
                        "player://position",
                        json!({"position": position, "duration": duration, "buffered": duration}),
                    );
                    sleep(Duration::from_millis(500)).await;
                }
            });
        }
        let p = lock.as_ref().unwrap().pipe.clone();
        drop(lock);
        for _ in 0..5 {
            mpv_request(
                &p,
                json!({"command":["loadfile",path.to_string_lossy(),"replace"]}),
            )
            .await?;
            // mpv keeps the pause property across loadfile calls. A previously paused
            // track would therefore load correctly while remaining at time-pos 0.
            mpv_send(&p, json!({"command":["set_property","pause",false]})).await?;
            // A successful `loadfile` command only means it was accepted. Wait for
            // mpv to confirm that it actually opened the file before telling the UI
            // playback has started (locked/incomplete files fail asynchronously).
            for _ in 0..10 {
                if mpv_get(&p, "path")
                    .await
                    .ok()
                    .and_then(|value| value.as_str().map(PathBuf::from))
                    .is_some_and(|loaded| loaded == path)
                {
                    let _ = app.emit("player://state", "playing");
                    return Ok(());
                }
                sleep(Duration::from_millis(100)).await;
            }
            sleep(Duration::from_millis(200)).await;
        }
        Err("播放器无法打开已下载音频，文件可能仍被下载服务占用".into())
    }
}
#[cfg(windows)]
async fn mpv_connect(pipe: &str) -> Result<NamedPipeClient, String> {
    ClientOptions::new().open(pipe).map_err(|e| e.to_string())
}
#[cfg(not(windows))]
async fn mpv_connect(pipe: &str) -> Result<UnixStream, String> {
    UnixStream::connect(pipe).await.map_err(|e| e.to_string())
}
async fn mpv_send(pipe: &str, msg: Value) -> Result<(), String> {
    let mut c = mpv_connect(pipe).await?;
    let mut b = serde_json::to_vec(&msg).map_err(|e| e.to_string())?;
    b.push(b'\n');
    c.write_all(&b).await.map_err(|e| e.to_string())
}
async fn mpv_get(pipe: &str, property: &str) -> Result<Value, String> {
    mpv_request(
        pipe,
        json!({"command": ["get_property", property], "request_id": 1}),
    )
    .await
}
async fn mpv_request(pipe: &str, message: Value) -> Result<Value, String> {
    let mut client = mpv_connect(pipe).await?;
    let mut bytes = serde_json::to_vec(&message).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    client.write_all(&bytes).await.map_err(|e| e.to_string())?;
    let mut response = String::new();
    BufReader::new(client)
        .read_line(&mut response)
        .await
        .map_err(|e| e.to_string())?;
    let value: Value = serde_json::from_str(&response).map_err(|e| e.to_string())?;
    if value.get("error").and_then(Value::as_str) != Some("success") {
        return Err(value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("mpv command failed")
            .to_string());
    }
    Ok(value.get("data").cloned().unwrap_or(Value::Null))
}
fn find_cached(dir: &Path, id: &str) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .map(|x| {
                    x.to_string_lossy().starts_with(id)
                        && p.extension().map(|x| x != "part").unwrap_or(false)
                })
                .unwrap_or(false)
        })
}
fn find_incomplete(root: &Path, filename: &str) -> Option<PathBuf> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .map(|e| e.into_path())
        .find(|p| {
            p.is_file()
                && p.file_name()
                    .map(|n| n.to_string_lossy().eq_ignore_ascii_case(filename))
                    .unwrap_or(false)
        })
}
fn find_ready_download(root: &Path, filename: &str, expected_size: u64) -> Option<PathBuf> {
    let path = find_incomplete(root, filename)?;
    let size = fs::metadata(&path).ok()?.len();
    if expected_size > 0 && size < expected_size {
        return None;
    }
    // slskd opens incomplete files without read sharing on Windows. Opening the
    // destination here prevents handing mpv a path that is visible but locked.
    fs::File::open(&path).ok()?;
    Some(path)
}
fn find_transfer<'a>(value: &'a Value, username: &str, filename: &str) -> Option<&'a Value> {
    if value.get("username").and_then(Value::as_str) == Some(username)
        && value.get("filename").and_then(Value::as_str) == Some(filename)
    {
        return Some(value);
    }
    match value {
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_transfer(item, username, filename)),
        Value::Object(fields) => fields
            .values()
            .find_map(|item| find_transfer(item, username, filename)),
        _ => None,
    }
}
fn expand_search_queries(query: &str) -> Vec<String> {
    // Soulseek filenames are frequently tagged with romanized artist names even
    // when the songs themselves have Chinese titles. Use deterministic aliases;
    // this stays local and does not depend on an online metadata service.
    const ALIASES: &[(&str, &str)] = &[
        ("陈奕迅", "eason"),
        ("周杰伦", "jay chou"),
        ("林俊杰", "jj lin"),
        ("邓紫棋", "gem"),
        ("王菲", "faye wong"),
        ("张学友", "jacky cheung"),
        ("刘德华", "andy lau"),
        ("五月天", "mayday"),
        ("孙燕姿", "stefanie sun"),
        ("蔡依林", "jolin tsai"),
        ("张国荣", "leslie cheung"),
        ("Beyond", "beyond"),
    ];
    let original = query.trim().to_string();
    let mut expanded = original.clone();
    for (name, alias) in ALIASES {
        if expanded.contains(name) {
            expanded = expanded.replace(name, alias);
        }
    }
    let mut queries = vec![original.clone()];
    if expanded != original {
        queries.push(expanded.clone());
    }
    // A short artist-only form often reaches substantially more peers than an
    // exact artist+title phrase; the original phrase remains the primary query.
    if let Some(alias) = ALIASES
        .iter()
        .find_map(|(name, alias)| original.contains(name).then_some(*alias))
    {
        queries.push(alias.to_string());
        let remainder = ALIASES
            .iter()
            .fold(original.clone(), |s, (name, _)| s.replace(name, ""));
        if !remainder.trim().is_empty() {
            queries.push(remainder.trim().to_string());
        }
    }
    queries.sort();
    queries.dedup();
    queries.truncate(4);
    queries
}
fn dir_size(p: &Path) -> u64 {
    walkdir::WalkDir::new(p)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|x| x.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}
fn buffer_threshold(s: &Source) -> u64 {
    match s.format.to_ascii_lowercase().as_str() {
        "flac" | "alac" => 4 * 1024 * 1024,
        "ape" => 5 * 1024 * 1024,
        "wav" => 8 * 1024 * 1024,
        _ => 2 * 1024 * 1024,
    }
}
fn safe_name(s: &str) -> String {
    s.chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) { '_' } else { c })
        .collect()
}

// slskd ships as slskd.exe on Windows; elsewhere the app looks for a native
// binary next to the app data first, then common Homebrew locations.
fn slskd_exe(runtime: &Path) -> PathBuf {
    if cfg!(windows) {
        return runtime.join("slskd/slskd.exe");
    }
    [
        runtime.join("slskd/slskd"),
        PathBuf::from("/opt/homebrew/bin/slskd"),
        PathBuf::from("/usr/local/bin/slskd"),
    ]
    .into_iter()
    .find(|p| p.exists())
    .unwrap_or_else(|| runtime.join("slskd/slskd"))
}
fn kill_slskd_command() -> Command {
    #[cfg(windows)]
    {
        let mut c = Command::new("taskkill.exe");
        c.args(["/F", "/IM", "slskd.exe"]);
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = Command::new("pkill");
        c.args(["-x", "slskd"]);
        c
    }
}

fn ensure_embedded_runtime(paths: &Paths) -> Result<(), Box<dyn std::error::Error>> {
    // Each platform ships its own slskd/mpv builds inside the binary and
    // unpacks them on first launch. Files written by the app carry no
    // quarantine attribute, so the runtime binaries pass Gatekeeper.
    #[cfg(windows)]
    {
        let slskd_dir = paths.runtime.join("slskd");
        if !slskd_dir.join("slskd.exe").exists() {
            fs::create_dir_all(&slskd_dir)?;
            let bytes = include_bytes!("../runtime-assets/slskd.zip");
            let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
            archive.extract(&slskd_dir)?;
        }
        let mpv_dir = paths.runtime.join("mpv");
        if !mpv_dir.join("mpv.exe").exists() {
            fs::create_dir_all(&mpv_dir)?;
            let archive_path = paths.runtime.join("mpv-runtime.7z");
            fs::write(&archive_path, include_bytes!("../runtime-assets/mpv.7z"))?;
            sevenz_rust::decompress_file(&archive_path, &mpv_dir)?;
            let _ = fs::remove_file(archive_path);
        }
    }
    #[cfg(not(windows))]
    {
        let slskd_dir = paths.runtime.join("slskd");
        if !slskd_dir.join("slskd").exists() {
            fs::create_dir_all(&slskd_dir)?;
            let bytes = include_bytes!("../runtime-assets/slskd-macos.zip");
            let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
            archive.extract(&slskd_dir)?;
            make_executable(&slskd_dir.join("slskd"));
        }
        let mpv_dir = paths.runtime.join("mpv");
        if !mpv_dir.join("mpv.app/Contents/MacOS/mpv").exists() {
            fs::create_dir_all(&mpv_dir)?;
            let bytes = include_bytes!("../runtime-assets/mpv-macos.zip");
            let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
            archive.extract(&mpv_dir)?;
            make_executable(&mpv_dir.join("mpv.app/Contents/MacOS/mpv"));
        }
    }
    let _ = paths;
    Ok(())
}

#[cfg(not(windows))]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(perms.mode() | 0o111);
        let _ = fs::set_permissions(path, perms);
    }
}
