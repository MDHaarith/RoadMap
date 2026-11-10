pub mod mcp;
mod ai;
mod api;
pub use api::request_body;
mod exercise;
mod project;
mod prompts;
mod storage;

use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicI64, Ordering},
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
            message: "Not found".into(),
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
        if e.kind() == std::io::ErrorKind::NotFound {
            Self::missing()
        } else {
            Self {
                status: 500,
                message: format!("IOError: {e}"),
            }
        }
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::bad(e.to_string())
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
    pub(crate) day_offset: AtomicI64,
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
            day_offset: AtomicI64::new(0),
            pick: Box::new(pick),
            yt_program: if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" }.into(),
        }))
    }
    // A panic in one request must not poison saving for every later request.
    pub(crate) fn guard(&self) -> (std::sync::MutexGuard<'_, ()>, fs::File) {
        let guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        fs::create_dir_all(&self.data).expect("create data directory for lock");
        let file = fs::OpenOptions::new().create(true).truncate(false).write(true)
            .open(self.data.join(".roadmap.lock")).expect("open data lock");
        file.lock().expect("lock data directory");
        (guard, file)
    }
    pub(crate) fn today(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
            / 86400
            + self.day_offset.load(Ordering::Relaxed)
    }
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
pub(crate) fn matches(re: &str, s: &str) -> bool {
    regex::Regex::new(re).unwrap().is_match(s)
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
        "concepts":strs(&l["concepts"],8),"tasks":strs(&l["tasks"],8),"done":strs(&l["done"],8),
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
#[cfg(all(test, target_os = "linux"))]
mod tests;
