pub mod mcp;
mod ai;
mod api;
mod chatgpt;
pub use api::request_body;
mod exercise;
mod habits;
mod offline;
mod project;
mod prompts;
mod storage;

use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc, Mutex,
    },
};

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, serde::Serialize)]
pub struct Error {
    pub status: u16,
    pub message: String,
}
impl Error {
    pub fn bad(s: impl Into<String>) -> Self {
        Self {
            status: 400,
            message: s.into(),
        }
    }
    pub fn missing() -> Self {
        Self {
            status: 404,
            message: "That roadmap or lesson no longer exists. Go back to your library and open it again.".into(),
        }
    }
    // The learner stopped the tutor reply.
    pub fn stopped() -> Self {
        Self {
            status: 499,
            message: "Stopped.".into(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        use std::io::ErrorKind::*;
        let why = match e.kind() {
            NotFound => return Self::missing(),
            StorageFull => "the disk is full. Free some space and try again.".to_string(),
            PermissionDenied | ReadOnlyFilesystem => "it is not allowed to write there. Check that your home folder is writable.".into(),
            _ => format!("{e}. Try again; if it keeps happening, restart the app."),
        };
        Self {
            status: 500,
            message: format!("The app could not read or save a file: {why}"),
        }
    }
}
impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        Self::bad("A saved file is damaged and could not be read.")
    }
}

pub struct App {
    pub home: PathBuf,
    pub data: PathBuf,
    pub conf: PathBuf,
    pub(crate) lock: Mutex<()>,
    pub(crate) off: Mutex<HashSet<String>>,
    pub(crate) meter: Mutex<()>,
    // Roadmaps whose video search is running, so reopening one does not start a second search.
    pub(crate) fetching: Mutex<HashSet<String>>,
    // Video ids waiting for a caption fetch, so reopening a roadmap does not queue another thread per open.
    pub(crate) transcribing: Mutex<HashSet<String>>,
    // Roadmaps whose unpicked videos were already searched again this run.
    pub(crate) repicked: Mutex<HashSet<String>>,
    pub(crate) day_offset: AtomicI64,
    // Set when the learner stops the tutor reply that is on its way; cleared when the next one starts.
    pub(crate) stop_chat: AtomicBool,
    // How many yt-dlp processes are running; see `yt`.
    pub(crate) yt_running: (Mutex<usize>, std::sync::Condvar),
    // This app's own ChatGPT sign-in; see `chatgpt.rs`.
    pub(crate) chatgpt: chatgpt::ChatGpt,
    // The latest thinking summary of each request that is still running, by the key the page sent; see `thinking`.
    pub(crate) thoughts: Mutex<HashMap<String, String>>,
    // The answer text streamed so far while a request builds a roadmap, by the same key; `api/thinking` shows it as an outline.
    pub(crate) drafts: Mutex<HashMap<String, String>>,
    // Offline video saving per roadmap: {"running", "stop", "failed", "error"}; see `offline.rs`.
    pub(crate) offline: Mutex<HashMap<String, Value>>,
    // Parsed roadmap files with the modified time and size they were read at; see `parsed` in storage.rs.
    #[allow(clippy::type_complexity)]
    pub(crate) parsed: Mutex<HashMap<PathBuf, ((Option<std::time::SystemTime>, u64), Option<Arc<Value>>)>>,
    pub(crate) pick: Box<dyn Fn(&str) -> String + Send + Sync>,
    pub(crate) yt_program: PathBuf,
}
impl App {
    pub fn new(pick: impl Fn(&str) -> String + Send + Sync + 'static) -> Result<Arc<Self>> {
        let home = dirs::home_dir().ok_or_else(|| Error::bad("Home directory is unavailable."))?;
        #[cfg(target_os = "linux")]
        let xdg = |k: &str, fallback: &str| {
            std::env::var_os(k)
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(fallback))
        };
        #[cfg(target_os = "linux")]
        let (data, config) = (xdg("XDG_DATA_HOME", ".local/share"), xdg("XDG_CONFIG_HOME", ".config"));
        #[cfg(not(target_os = "linux"))]
        let (data, config) = (
            dirs::data_dir().ok_or_else(|| Error::bad("Data directory is unavailable."))?,
            dirs::config_dir().ok_or_else(|| Error::bad("Config directory is unavailable."))?,
        );
        Ok(Arc::new(Self {
            data: data.join("roadmap-app"),
            conf: config.join("roadmap-app").join("config.json"),
            home,
            lock: Mutex::new(()),
            off: Mutex::new(HashSet::new()),
            meter: Mutex::new(()),
            fetching: Mutex::new(HashSet::new()),
            transcribing: Mutex::new(HashSet::new()),
            repicked: Mutex::new(HashSet::new()),
            day_offset: AtomicI64::new(0),
            stop_chat: AtomicBool::new(false),
            yt_running: Default::default(),
            chatgpt: Default::default(),
            thoughts: Mutex::new(HashMap::new()),
            drafts: Mutex::new(HashMap::new()),
            offline: Mutex::new(HashMap::new()),
            parsed: Mutex::new(HashMap::new()),
            pick: Box::new(pick),
            yt_program: if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" }.into(),
        }))
    }
    // A panic in one request must not poison saving for every later request.
    // The file lock keeps the MCP server's writes apart from the app's. Without it (a read-only disk) the save itself reports the problem.
    pub(crate) fn guard(&self) -> (std::sync::MutexGuard<'_, ()>, Option<fs::File>) {
        let guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let _ = fs::create_dir_all(&self.data);
        let file = fs::OpenOptions::new().create(true).truncate(false).write(true)
            .open(self.data.join(".roadmap.lock")).ok()
            .filter(|f| f.lock().is_ok());
        (guard, file)
    }
    // Days by the learner's own calendar, so reviews fall due at local midnight, not at UTC midnight.
    pub(crate) fn today(&self) -> i64 {
        chrono::Local::now().date_naive().signed_duration_since(chrono::DateTime::UNIX_EPOCH.date_naive()).num_days()
            + self.day_offset.load(Ordering::Relaxed)
    }
}
// Temp file in the same folder, flushed to disk, then renamed over the target: a crash leaves the old file or the new one, never half of one.
// The temp name is unique, so two writers never share it; on Unix it is created 0600.
pub(crate) fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let dir = path.parent().ok_or_else(Error::missing)?;
    fs::create_dir_all(dir)?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    // On Windows a virus scanner or indexer briefly holding the target makes the rename fail with "access denied"; it clears in moments.
    for _ in 0..10 {
        match tmp.persist(path) {
            Ok(_) => return Ok(()),
            Err(e) if e.error.kind() == std::io::ErrorKind::PermissionDenied => {
                tmp = e.file;
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => return Err(e.error.into()),
        }
    }
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}
pub(crate) fn random_hex(n: usize) -> Result<String> {
    let mut b = vec![0; n];
    getrandom::fill(&mut b).map_err(|e| Error { status: 500, message: format!("Random number generator failed: {e}") })?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}
pub(crate) fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "None".into(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::Array(a) => format!("[{}]", a.iter().map(repr).collect::<Vec<_>>().join(", ")),
        Value::Object(o) => format!(
            "{{{}}}",
            o.iter()
                .map(|(k, v)| format!("{}: {}", repr(&json!(k)), repr(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => v.to_string(),
    }
}
// Preserve the original string coercion for non-string fields in untrusted JSON.
fn repr(v: &Value) -> String {
    let Value::String(s) = v else {
        return text(v);
    };
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::from(quote);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_control() && (c as u32) < 256 => {
                out.push_str(&format!("\\x{:02x}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}
pub(crate) fn truth(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
        Value::Number(n) => n.as_f64() != Some(0.),
    }
}
pub(crate) fn string(v: &Value) -> String {
    if truth(v) {
        text(v)
    } else {
        String::new()
    }
}
pub(crate) fn cap(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}
pub(crate) fn tail(s: &str, n: usize) -> String {
    s.chars()
        .skip(s.chars().count().saturating_sub(n))
        .collect()
}
pub(crate) fn list(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or(&[])
}
pub(crate) fn strs(v: &Value, n: usize) -> Vec<String> {
    list(v).iter().take(n).map(|x| cap(&text(x), 600)).collect()
}
pub(crate) fn joined(v: &Value) -> String {
    list(v).iter().map(text).collect::<Vec<_>>().join("; ")
}
pub(crate) fn matches(re: &'static str, s: &str) -> bool {
    regex(re).is_match(s)
}
// Patterns are compiled once and shared: compiling one, or warming a fresh copy's match cache, costs far more than matching.
// Only `'static` patterns (literals in this crate) can be cached, so a pattern built from data can never grow the leaked set.
pub(crate) fn regex(re: &'static str) -> &'static regex::Regex {
    static CACHE: std::sync::LazyLock<Mutex<std::collections::HashMap<&'static str, &'static regex::Regex>>> = std::sync::LazyLock::new(Default::default);
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    cache.entry(re).or_insert_with(|| Box::leak(Box::new(regex::Regex::new(re).unwrap())))
}
/// No video with fewer views is searched for, picked or kept.
pub(crate) const MIN_VIEWS: i64 = 1000;
/// Removes saved videos known to be under MIN_VIEWS and says whether any went; a video with no view count (an imported one) stays.
pub(crate) fn drop_obscure(c: &mut Value) -> bool {
    let mut dropped = false;
    for p in c["phases"].as_array_mut().into_iter().flatten() {
        for l in p["lessons"].as_array_mut().into_iter().flatten() {
            if let Some(vs) = l["videos"].as_array_mut() {
                let n = vs.len();
                vs.retain(|v| truth(&v["gem"]) || !v["views"].as_i64().is_some_and(|n| n < MIN_VIEWS));
                dropped |= vs.len() != n;
            }
        }
    }
    dropped
}
/// The words that end most of a course's video queries (two or more, shared by at least half and at least three lessons),
/// such as "small business beginner worked example tutorial". YouTube matches such filler instead of the topic.
pub(crate) fn common_tail(c: &Value) -> Vec<String> {
    let qs = lessons(c)
        .map(|l| string(&l["query"]).to_lowercase().split_whitespace().map(String::from).collect::<Vec<_>>())
        .filter(|q| !q.is_empty())
        .collect::<Vec<_>>();
    for k in (2..=8).rev() {
        let mut seen: std::collections::HashMap<&[String], usize> = Default::default();
        for q in qs.iter().filter(|q| q.len() > k) {
            *seen.entry(&q[q.len() - k..]).or_default() += 1;
        }
        if let Some((tail, _)) = seen.into_iter().find(|(_, n)| *n >= 3 && *n * 2 >= qs.len()) {
            return tail.to_vec();
        }
    }
    vec![]
}
/// The query without that tail.
pub(crate) fn strip_tail(query: &str, tail: &[String]) -> String {
    let words = query.split_whitespace().collect::<Vec<_>>();
    let n = words.len().saturating_sub(tail.len());
    if tail.is_empty() || n == 0 || !words[n..].iter().map(|w| w.to_lowercase()).eq(tail.iter().cloned()) {
        return query.to_string();
    }
    words[..n].join(" ")
}
/// Video preferences with unknown values blanked; a blank field means "the default" (or, per roadmap, "use Settings").
pub(crate) fn clean_prefs(v: &Value) -> Value {
    let one = |k: &str, ok: &[&str]| {
        let s = string(&v[k]);
        if ok.contains(&s.as_str()) { s } else { String::new() }
    };
    let lang = cap(string(&v["lang"]).trim(), 40);
    json!({"style":one("style",&["popular","depth","gems"]),"length":one("length",&["short","medium","long"]),
        "lang":if matches(r"^[\p{L} ()-]*$",&lang){lang}else{String::new()},"recent":one("recent",&["yes","no"])})
}
/// Lower view floor for the "hidden gems" style; such picks are marked `gem` so `drop_obscure` keeps them.
pub(crate) const GEM_VIEWS: i64 = 100;
/// Raw search results saved when the AI could not pick: no reason, not the learner's own search, and a view count (imports have none).
pub(crate) fn unpicked(vs: &[Value]) -> bool {
    !vs.is_empty()
        && vs.iter().all(|v| !truth(&v["why"]) && !truth(&v["searched"]) && v["views"].is_number())
}
/// Whether the prefetch should search for this lesson: never searched, or (when `retry`) left with raw results.
pub(crate) fn needs_videos(l: &Value, retry: bool) -> bool {
    let vs = list(&l["videos"]);
    !truth(&l["stub"]) && (vs.is_empty() && !truth(&l["novideos"]) || retry && unpicked(vs))
}
/// Saves a search outcome; an empty one records that nothing fits, so the lesson is not searched on every open.
pub(crate) fn set_videos(l: &mut Value, vids: Vec<Value>) {
    if vids.is_empty() {
        l["novideos"] = json!(true);
    } else if let Some(o) = l.as_object_mut() {
        o.remove("novideos");
    }
    l["videos"] = json!(vids);
}
pub(crate) fn lessons(c: &Value) -> impl Iterator<Item = &Value> {
    list(&c["phases"]).iter().flat_map(|p| list(&p["lessons"]))
}
pub(crate) fn lesson<'a>(c: &'a Value, lid: &Value) -> Result<&'a Value> {
    lessons(c)
        .find(|l| l["id"] == *lid)
        .ok_or_else(Error::missing)
}
pub(crate) fn lesson_mut<'a>(c: &'a mut Value, lid: &str) -> Result<&'a mut Value> {
    c["phases"]
        .as_array_mut()
        .into_iter()
        .flatten()
        .flat_map(|p| p["lessons"].as_array_mut().into_iter().flatten())
        .find(|l| l["id"] == lid)
        .ok_or_else(Error::missing)
}
pub(crate) fn parse(s: &str, what: &str) -> Result<Value> {
    let v = s
        .find('{')
        .zip(s.rfind('}'))
        .filter(|(a, b)| a <= b)
        .and_then(|(a, b)| serde_json::from_str::<Value>(&s[a..=b]).ok())
        .filter(Value::is_object);
    v.ok_or_else(|| {
        Error::bad(format!(
            "The AI didn't return {what}. Try again, or use a stronger model."
        ))
    })
}
pub(crate) fn merge(mut a: Value, b: Value) -> Value {
    if let (Some(a), Some(b)) = (a.as_object_mut(), b.as_object()) {
        a.extend(b.clone());
    }
    a
}
pub(crate) fn clean_lesson(l: &Value, lid: &str, files: &[String]) -> Value {
    json!({"id":lid,"title":cap(&if truth(&l["title"]) { text(&l["title"]) } else {"Lesson".into()},200),"goal":cap(&string(&l["goal"]),600),
        "concepts":strs(&l["concepts"],8),"tasks":strs(&l["tasks"],15),"done":strs(&l["done"],8),
        "query":cap(&if truth(&l["query"]) {text(&l["query"])}else{string(&l["title"])},200),"lookups":strs(&l["lookups"],4),
        "files":strs(&l["files"],6).into_iter().filter(|f|files.contains(f)).collect::<Vec<_>>(),"warn":cap(&string(&l["warn"]),600),
        "charts":clean_charts(&l["charts"])})
}
// Graphs drawn on the lesson page: at most 3, each with 2 to 24 points and 1 to 4 series of that length.
pub(crate) fn clean_charts(v: &Value) -> Vec<Value> {
    list(v).iter().filter(|c| c.is_object()).take(3).filter_map(|c| {
        let labels: Vec<String> = strs(&c["labels"], 24).iter().map(|s| cap(s, 40)).collect();
        let series: Vec<Value> = list(&c["series"]).iter().filter(|s| s.is_object()).take(4)
            .filter_map(|s| {
                let values: Vec<f64> = list(&s["values"]).iter().filter_map(|x| x.as_f64().filter(|f| f.is_finite())).take(24).collect();
                (values.len() == labels.len()).then(|| json!({"name":cap(&string(&s["name"]),80),"values":values}))
            })
            .collect();
        (labels.len() >= 2 && !series.is_empty()).then(|| json!({"title":cap(&string(&c["title"]),200),"kind":if c["kind"]=="bar"{"bar"}else{"line"},
            "x":cap(&string(&c["x"]),80),"y":cap(&string(&c["y"]),80),"labels":labels,"series":series,"note":cap(&string(&c["note"]),600)}))
    }).collect()
}
// The stages of a long roadmap: about a month each, shown above their phases.
pub(crate) fn clean_stages(v: &Value) -> Vec<Value> {
    list(v)
        .iter()
        .filter(|s| s.is_object())
        .take(12)
        .enumerate()
        .map(|(i, s)| json!({"title":cap(&if truth(&s["title"]){text(&s["title"])}else{format!("Stage {}",i+1)},200),"blurb":cap(&string(&s["blurb"]),600)}))
        .collect()
}
// What a course needs before lesson one. `command` is only ever looked up on PATH, never run.
pub(crate) fn clean_needs(v: &Value) -> Vec<Value> {
    list(v)
        .iter()
        .filter(|n| n.is_object() && truth(&n["name"]))
        .take(20)
        .map(|n| {
            let cmd = string(&n["command"]);
            json!({"name":cap(&text(&n["name"]),120),"kind":if n["kind"]=="tool"{"tool"}else{"part"},
                "command":if matches(r"^[A-Za-z0-9_+-][A-Za-z0-9_.+-]{0,39}$",&cmd){cmd}else{String::new()},"why":cap(&string(&n["why"]),300)})
        })
        .collect()
}
pub(crate) fn on_path(cmd: &str) -> bool {
    matches(r"^[A-Za-z0-9_+-][A-Za-z0-9_.+-]{0,39}$", cmd)
        && std::env::var_os("PATH")
            .is_some_and(|p| std::env::split_paths(&p).any(|d| {
                if d.join(cmd).is_file() { return true; }
                #[cfg(windows)]
                {
                    let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
                    extensions.split(';').filter(|ext| !ext.is_empty())
                        .any(|ext| d.join(format!("{cmd}{ext}")).is_file())
                }
                #[cfg(not(windows))]
                false
            }))
}
#[cfg(test)]
mod tests;
