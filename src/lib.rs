mod ai;
mod api;
mod exercise;
mod project;
mod prompts;
mod storage;

use serde_json::{json, Value};
use std::{collections::HashSet, fs, io::Read, path::PathBuf, sync::{Arc, Mutex, atomic::{AtomicI64, Ordering}}};

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub struct Error { pub status: u16, pub message: String }
impl Error {
    pub fn bad(s: impl Into<String>) -> Self { Self { status: 400, message: s.into() } }
    pub fn missing() -> Self { Self { status: 404, message: "Not found".into() } }
}
impl std::fmt::Display for Error { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.message) } }
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self { if e.kind() == std::io::ErrorKind::NotFound { Self::missing() } else { Self { status:500, message:format!("IOError: {e}") } } }
}
impl From<serde_json::Error> for Error { fn from(e: serde_json::Error) -> Self { Self::bad(e.to_string()) } }

pub struct App {
    pub home: PathBuf,
    pub data: PathBuf,
    pub conf: PathBuf,
    pub prefix: String,
    pub(crate) lock: Mutex<()>,
    pub(crate) off: Mutex<HashSet<String>>,
    pub(crate) day_offset: AtomicI64,
    pub(crate) pick: Box<dyn Fn(&str) -> String + Send + Sync>,
    pub(crate) yt_program: PathBuf,
}
impl App {
    pub fn new(pick: impl Fn(&str) -> String + Send + Sync + 'static) -> Result<Arc<Self>> {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| "/".into()));
        let xdg = |k: &str, fallback: &str| std::env::var_os(k).filter(|s| !s.is_empty()).map(PathBuf::from).unwrap_or_else(|| home.join(fallback));
        Ok(Arc::new(Self {
            data: xdg("XDG_DATA_HOME", ".local/share").join("roadmap-app"),
            conf: xdg("XDG_CONFIG_HOME", ".config").join("roadmap-app/config.json"),
            home, prefix:format!("/{}/", random_hex(16)?), lock:Mutex::new(()), off:Mutex::new(HashSet::new()),
            day_offset:AtomicI64::new(0), pick:Box::new(pick), yt_program:"yt-dlp".into(),
        }))
    }
    pub(crate) fn today(&self) -> i64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64 / 86400 + self.day_offset.load(Ordering::Relaxed) }
}
pub(crate) fn random_hex(n: usize) -> Result<String> {
    let mut b=vec![0;n]; fs::File::open("/dev/urandom")?.read_exact(&mut b)?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}
pub(crate) fn text(v: &Value) -> String { match v { Value::String(s)=>s.clone(), Value::Null=>"None".into(), Value::Bool(true)=>"True".into(), Value::Bool(false)=>"False".into(), _=>v.to_string() } }
pub(crate) fn truth(v: &Value) -> bool { match v { Value::Null=>false, Value::Bool(b)=>*b, Value::String(s)=>!s.is_empty(), Value::Array(a)=>!a.is_empty(), Value::Object(o)=>!o.is_empty(), Value::Number(n)=>n.as_f64()!=Some(0.) } }
pub(crate) fn string(v: &Value) -> String { if truth(v) { text(v) } else { String::new() } }
pub(crate) fn cap(s: &str, n: usize) -> String { s.chars().take(n).collect() }
pub(crate) fn tail(s: &str, n: usize) -> String { s.chars().skip(s.chars().count().saturating_sub(n)).collect() }
pub(crate) fn list(v: &Value) -> &[Value] { v.as_array().map(Vec::as_slice).unwrap_or(&[]) }
pub(crate) fn strs(v: &Value, n: usize) -> Vec<String> { list(v).iter().take(n).map(|x| cap(&text(x),600)).collect() }
pub(crate) fn joined(v: &Value) -> String { list(v).iter().map(text).collect::<Vec<_>>().join("; ") }
pub(crate) fn matches(re: &str, s: &str) -> bool { regex::Regex::new(re).unwrap().is_match(s) }
pub(crate) fn lessons(c: &Value) -> impl Iterator<Item=&Value> { list(&c["phases"]).iter().flat_map(|p| list(&p["lessons"])) }
pub(crate) fn lesson<'a>(c: &'a Value, lid: &Value) -> Result<&'a Value> { lessons(c).find(|l| l["id"]==*lid).ok_or_else(Error::missing) }
pub(crate) fn lesson_mut<'a>(c: &'a mut Value, lid: &str) -> Result<&'a mut Value> {
    c["phases"].as_array_mut().into_iter().flatten().flat_map(|p| p["lessons"].as_array_mut().into_iter().flatten()).find(|l| l["id"]==lid).ok_or_else(Error::missing)
}
pub(crate) fn parse(s: &str, what: &str) -> Result<Value> {
    let v = s.find('{').zip(s.rfind('}')).filter(|(a,b)|a<=b).and_then(|(a,b)|serde_json::from_str::<Value>(&s[a..=b]).ok()).filter(Value::is_object);
    v.ok_or_else(||Error::bad(format!("The AI didn't return {what}. Try again, or use a stronger model.")))
}
pub(crate) fn merge(mut a: Value, b: Value) -> Value { if let (Some(a),Some(b))=(a.as_object_mut(),b.as_object()) { a.extend(b.clone()); } a }
pub(crate) fn clean_lesson(l: &Value, lid: &str, files: &[String]) -> Value {
    json!({"id":lid,"title":cap(&if truth(&l["title"]) { text(&l["title"]) } else {"Lesson".into()},200),"goal":cap(&string(&l["goal"]),600),
        "concepts":strs(&l["concepts"],8),"tasks":strs(&l["tasks"],8),"done":strs(&l["done"],8),
        "query":cap(&if truth(&l["query"]) {text(&l["query"])}else{string(&l["title"])},200),"lookups":strs(&l["lookups"],4),
        "files":strs(&l["files"],6).into_iter().filter(|f|files.contains(f)).collect::<Vec<_>>(),"warn":cap(&string(&l["warn"]),600)})
}
#[cfg(test)] mod tests;
