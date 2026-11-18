//! Signs in to ChatGPT and asks its Codex server directly, with no Codex program on the computer.
//! shortcut: this uses the sign-in and server the Codex CLI uses (its client id, private endpoints), not a public API;
//! when OpenAI changes or blocks them, sign-in or answers fail with their message and this module needs updating.
use crate::ai::agent;
use crate::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Read};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
// The sign-in page sends the browser back only to this address, which Codex registered.
const PORT: u16 = 1455;
// The server lists the models that work for this client version.
const CLIENT_VERSION: &str = "0.170.0";

pub(crate) struct ChatGpt {
    pub(crate) issuer: String,
    pub(crate) api: String,
    // One refresh at a time: ChatGPT replaces the refresh token on each use, so two at once would sign the learner out.
    refresh: Mutex<()>,
    // The sign-in in progress: {"waiting", "url", "error"}.
    login: Mutex<Value>,
    // The models ChatGPT offers, kept from the last fetch; "default" means the first.
    models: Mutex<Vec<Value>>,
}
impl Default for ChatGpt {
    fn default() -> Self {
        Self::at("https://auth.openai.com", "https://chatgpt.com/backend-api/codex")
    }
}
impl ChatGpt {
    pub(crate) fn at(issuer: &str, api: &str) -> Self {
        Self { issuer: issuer.into(), api: api.into(), refresh: Mutex::new(()), login: Mutex::new(json!({})), models: Mutex::new(vec![]) }
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}
// The middle part of a JWT, unchecked: only read for the expiry, account id and email that the server itself checks.
fn claims(jwt: &str) -> Value {
    jwt.split('.').nth(1).and_then(|p| B64.decode(p.trim_end_matches('=')).ok()).and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}
fn enc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}
fn dec(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        let hex = (b[i] == b'%').then(|| s.get(i + 1..i + 3)).flatten()
            .filter(|h| h.bytes().all(|c| c.is_ascii_hexdigit()))
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (hex, b[i]) {
            (Some(x), _) => {
                out.push(x);
                i += 3;
                continue;
            }
            (None, b'+') => out.push(b' '),
            (None, x) => out.push(x),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
fn query(url: &str, key: &str) -> String {
    url.split_once('?').map_or("", |(_, q)| q).split('&').filter_map(|p| p.split_once('=')).find(|(k, _)| *k == key).map(|(_, v)| dec(v)).unwrap_or_default()
}
fn page(title: &str, body: &str) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let html = format!("<!doctype html><meta charset=utf-8><title>Roadmap</title><body style=\"font:16px system-ui;max-width:32em;margin:4em auto\"><h1>{title}</h1><p>{body}</p>");
    tiny_http::Response::from_string(html).with_header(tiny_http::Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap())
}

impl App {
    fn chatgpt_file(&self) -> std::path::PathBuf {
        self.conf.with_file_name("chatgpt.json")
    }
    fn chatgpt_saved(&self) -> Value {
        fs::read(self.chatgpt_file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
    }
    /// "ready" when this app holds a ChatGPT sign-in, else "signed_out".
    pub(crate) fn chatgpt_status(&self) -> Value {
        let a = self.chatgpt_saved();
        let login = self.chatgpt.login.lock().unwrap_or_else(|e| e.into_inner()).clone();
        json!({"status":if truth(&a["refresh_token"]) {"ready"} else {"signed_out"},"email":string(&a["email"]),"waiting":login["waiting"]==true,"error":string(&login["error"])})
    }
    pub(crate) fn chatgpt_logout(&self) -> Result<Value> {
        let _one = self.chatgpt.refresh.lock().unwrap_or_else(|e| e.into_inner());
        match fs::remove_file(self.chatgpt_file()) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
            _ => {}
        }
        self.chatgpt.models.lock().unwrap_or_else(|e| e.into_inner()).clear();
        Ok(self.chatgpt_status())
    }
    /// Starts a sign-in and returns the address to open in the browser; `chatgpt_status` says when it is done.
    pub(crate) fn chatgpt_login(self: &Arc<Self>) -> Result<Value> {
        let mut login = self.chatgpt.login.lock().unwrap_or_else(|e| e.into_inner());
        // Asking again while the browser is still open gives the same, still valid address.
        if login["waiting"] == true {
            return Ok(json!({"url":login["url"]}));
        }
        let server = tiny_http::Server::http(("127.0.0.1", PORT)).map_err(|_| {
            Error::bad("Another sign-in is using port 1455 on this computer, often a Codex sign-in left open. Close it and try again.")
        })?;
        let verifier = random_hex(32)?;
        let challenge = B64.encode(Sha256::digest(verifier.as_bytes()));
        let state = random_hex(16)?;
        let redirect = format!("http://localhost:{PORT}/auth/callback");
        let url = format!(
            "{}/oauth/authorize?response_type=code&client_id={CLIENT_ID}&redirect_uri={}&scope={}&code_challenge={challenge}&code_challenge_method=S256&id_token_add_organizations=true&codex_cli_simplified_flow=true&state={state}&originator=codex_cli_rs",
            self.chatgpt.issuer, enc(&redirect), enc("openid profile email offline_access")
        );
        *login = json!({"waiting":true,"url":url,"error":""});
        drop(login);
        let app = self.clone();
        std::thread::spawn(move || {
            let end = Instant::now() + Duration::from_secs(600);
            let result = loop {
                if Instant::now() > end {
                    break Err(Error::bad("The sign-in took longer than 10 minutes. Try again."));
                }
                let Ok(Some(req)) = server.recv_timeout(Duration::from_millis(500)) else { continue };
                let at = req.url().to_string();
                if !at.starts_with("/auth/callback") {
                    let _ = req.respond(tiny_http::Response::empty(404));
                    continue;
                }
                // A visit without this sign-in's state is not its answer, so the wait goes on.
                if query(&at, "state") != state {
                    let _ = req.respond(page("Old sign-in link", "Go back to Roadmap and press Sign in with ChatGPT again.").with_status_code(400));
                    continue;
                }
                let r = if !query(&at, "error").is_empty() {
                    Err(Error::bad(format!("ChatGPT did not sign you in: {}", cap(&query(&at, "error_description"), 200))))
                } else {
                    app.chatgpt_exchange(&query(&at, "code"), &verifier, &redirect)
                };
                let _ = req.respond(match &r {
                    Ok(()) => page("Signed in", "You can close this tab and go back to Roadmap."),
                    Err(e) => page("Not signed in", &e.message.replace('<', "&lt;")),
                });
                break r;
            };
            *app.chatgpt.login.lock().unwrap_or_else(|e| e.into_inner()) = json!({"waiting":false,"error":result.err().map(|e| e.message).unwrap_or_default()});
        });
        Ok(json!({"url":url}))
    }
    fn chatgpt_exchange(&self, code: &str, verifier: &str, redirect: &str) -> Result<()> {
        let url = format!("{}/oauth/token", self.chatgpt.issuer);
        let r = agent(&url, 30)
            .and_then(|a| a.post(&url).send_form(&[("grant_type", "authorization_code"), ("code", code), ("redirect_uri", redirect), ("client_id", CLIENT_ID), ("code_verifier", verifier)]))
            .map_err(|_| Error::bad("ChatGPT did not accept the sign-in. Try again."))?;
        let d: Value = serde_json::from_reader(r.into_reader().take(1 << 20))?;
        self.chatgpt_keep(&d, &Value::Null)
    }
    // Saves new tokens; a refresh may leave out the id token or refresh token, which are then kept from before.
    fn chatgpt_keep(&self, d: &Value, before: &Value) -> Result<()> {
        let access = string(&d["access_token"]);
        if access.is_empty() {
            return Err(Error::bad("ChatGPT sent no sign-in token. Try signing in again."));
        }
        let id = claims(&string(&d["id_token"]));
        let pick = |new: String, old: &Value| if new.is_empty() { string(old) } else { new };
        let saved = json!({
            "access_token":access,
            "refresh_token":pick(string(&d["refresh_token"]), &before["refresh_token"]),
            "account_id":pick(string(&id["https://api.openai.com/auth"]["chatgpt_account_id"]), &before["account_id"]),
            "email":pick(string(&id["email"]), &before["email"]),
        });
        write_atomic(&self.chatgpt_file(), &serde_json::to_vec(&saved)?)
    }
    /// The access token and account id, refreshed first when it is about to expire or `force` says the server refused it.
    fn chatgpt_token(&self, force: bool) -> Result<(String, String)> {
        let _one = self.chatgpt.refresh.lock().unwrap_or_else(|e| e.into_inner());
        let a = self.chatgpt_saved();
        let access = string(&a["access_token"]);
        if access.is_empty() || !truth(&a["refresh_token"]) {
            return Err(Error::bad("Roadmap is not signed in to ChatGPT. Open Settings and press Sign in with ChatGPT."));
        }
        if !force && claims(&access)["exp"].as_u64().unwrap_or(0) > now() + 300 {
            return Ok((access, string(&a["account_id"])));
        }
        let url = format!("{}/oauth/token", self.chatgpt.issuer);
        let body = json!({"client_id":CLIENT_ID,"grant_type":"refresh_token","refresh_token":a["refresh_token"],"scope":"openid profile email"});
        match agent(&url, 30).and_then(|g| g.post(&url).set("Content-Type", "application/json").send_string(&body.to_string())) {
            Ok(r) => {
                let d: Value = serde_json::from_reader(r.into_reader().take(1 << 20))?;
                self.chatgpt_keep(&d, &a)?;
                let a = self.chatgpt_saved();
                Ok((string(&a["access_token"]), string(&a["account_id"])))
            }
            Err(ureq::Error::Status(400 | 401 | 403, _)) => {
                let _ = fs::remove_file(self.chatgpt_file());
                Err(Error::bad("Your ChatGPT sign-in has expired. Open Settings and press Sign in with ChatGPT again."))
            }
            Err(_) => Err(Error::bad("Can't reach ChatGPT. Check your internet connection and try again.")),
        }
    }
    fn chatgpt_call(&self, path: &str, body: Option<&Value>, secs: u64) -> Result<ureq::Response> {
        let url = format!("{}{path}", self.chatgpt.api);
        let mut force = false;
        loop {
            let (token, account) = self.chatgpt_token(force)?;
            let g = agent(&url, secs).map_err(|_| Error::bad("Can't reach ChatGPT. Check your internet connection and try again."))?;
            let req = if body.is_some() { g.post(&url) } else { g.get(&url) }
                .set("Authorization", &format!("Bearer {token}"))
                .set("chatgpt-account-id", &account)
                .set("OpenAI-Beta", "responses=experimental")
                .set("originator", "codex_cli_rs")
                .set("session_id", &random_hex(16)?)
                .set("Accept", if body.is_some() { "text/event-stream" } else { "application/json" });
            let r = match body { Some(b) => req.set("Content-Type", "application/json").send_string(&b.to_string()), None => req.call() };
            match r {
                Ok(r) => return Ok(r),
                // An access token can be withdrawn before it expires; one refresh, then the error stands.
                Err(ureq::Error::Status(401, _)) if !force => force = true,
                Err(ureq::Error::Status(code, r)) => {
                    let mut bytes = vec![];
                    let _ = r.into_reader().take(2000).read_to_end(&mut bytes);
                    let d: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                    let said = [&d["error"]["message"], &d["detail"], &d["message"]].into_iter().map(string).find(|s| !s.is_empty())
                        .unwrap_or_else(|| cap(String::from_utf8_lossy(&bytes).trim(), 200));
                    return Err(Error::bad(match code {
                        401 | 403 => "ChatGPT turned down this app's sign-in. Open Settings and sign in with ChatGPT again.".into(),
                        429 => format!("You have used up your ChatGPT allowance for now. Wait until it resets and try again. ChatGPT said: {said}"),
                        500.. => format!("ChatGPT had a problem on its side (error {code}). Try again in a minute."),
                        _ => format!("ChatGPT turned down the request (error {code}): {said}"),
                    }));
                }
                Err(_) => return Err(Error::bad("Can't reach ChatGPT. Check your internet connection and try again.")),
            }
        }
    }
    pub(crate) fn chatgpt_models(&self) -> Result<Vec<Value>> {
        let r = self.chatgpt_call(&format!("/models?client_version={CLIENT_VERSION}"), None, 15)?;
        let d: Value = serde_json::from_reader(r.into_reader().take(4 << 20)).map_err(|_| Error::bad("ChatGPT sent a model list this app can't read."))?;
        // Models marked "hide" are ChatGPT's internal ones; Codex's own picker leaves them out too.
        let models: Vec<Value> = list(&d["models"]).iter()
            .filter(|m| m["visibility"] != "hide")
            .filter_map(|m| m["slug"].as_str().map(|id| json!({"id":id,"name":m["display_name"].as_str().unwrap_or(id),"note":m["description"].as_str().unwrap_or("")})))
            .collect();
        *self.chatgpt.models.lock().unwrap_or_else(|e| e.into_inner()) = models.clone();
        Ok(models)
    }
    /// One answer from ChatGPT in the chat-completions shape the rest of the app uses: {"content", "tool_calls"}.
    pub(crate) fn ask_chatgpt(&self, model: &str, msgs: &[Value], on_text: &mut crate::ai::Sink<'_>, extra: &Value) -> Result<Value> {
        let model = if model.is_empty() || model == "default" {
            let known = self.chatgpt.models.lock().unwrap_or_else(|e| e.into_inner()).first().cloned();
            let first = match known { Some(m) => m, None => self.chatgpt_models()?.into_iter().next().unwrap_or_default() };
            string(&first["id"])
        } else {
            model.to_string()
        };
        let mut instructions = vec![];
        let mut input = vec![];
        for m in msgs {
            let content = text(&m["content"]);
            match m["role"].as_str().unwrap_or("") {
                "system" => instructions.push(content),
                "assistant" => {
                    if !content.is_empty() {
                        input.push(json!({"type":"message","role":"assistant","content":[{"type":"output_text","text":content}]}));
                    }
                    for k in list(&m["tool_calls"]) {
                        input.push(json!({"type":"function_call","call_id":k["id"],"name":k["function"]["name"],"arguments":text(&k["function"]["arguments"])}));
                    }
                }
                "tool" => input.push(json!({"type":"function_call_output","call_id":m["tool_call_id"],"output":content})),
                _ => input.push(json!({"type":"message","role":"user","content":[{"type":"input_text","text":content}]})),
            }
        }
        let tools: Vec<Value> = list(&extra["tools"]).iter().map(|t| {
            let f = &t["function"];
            json!({"type":"function","name":f["name"],"description":f["description"],"parameters":f["parameters"],"strict":false})
        }).collect();
        let mut body = json!({
            "model":model,"instructions":instructions.join("\n\n"),"input":input,"tools":tools,"tool_choice":"auto",
            "parallel_tool_calls":false,"reasoning":{"effort":extra["effort"].as_str().unwrap_or("low"),"summary":"auto"},"store":false,"stream":true,"include":[],
        });
        let format = &extra["response_format"]["json_schema"];
        if format.is_object() {
            body["text"] = json!({"format":{"type":"json_schema","name":format["name"],"schema":format["schema"],"strict":true}});
        }
        let r = self.chatgpt_call("/responses", Some(&body), 900)?;
        // Read on its own thread, so Stop ends the wait at once even while ChatGPT sends nothing; the reader ends when the answer does.
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(r.into_reader()).split(b'\n') {
                let end = line.is_err();
                if tx.send(line).is_err() || end {
                    break;
                }
            }
        });
        let unreadable = || Error::bad("ChatGPT sent an answer this app can't read.");
        let (mut content, mut calls, mut summary) = (String::new(), vec![], String::new());
        loop {
            if on_text.is_some() && self.stop_chat.load(Ordering::Relaxed) {
                return Err(Error::stopped());
            }
            let line = match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(line) => line,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            };
            let line = line.map_err(|_| Error::bad("The connection to ChatGPT broke during the answer. Try again."))?;
            let line = String::from_utf8_lossy(&line);
            let Some(data) = line.trim().strip_prefix("data:") else { continue };
            let Ok(d) = serde_json::from_str::<Value>(data.trim()) else { continue };
            match d["type"].as_str().unwrap_or("") {
                "response.output_text.delta" => {
                    let t = string(&d["delta"]);
                    content.push_str(&t);
                    if let Some(out) = on_text.as_mut() {
                        out(&t)?;
                    } else if t.contains('"') {
                        // Low effort often means no reasoning summary at all, so the wait shows the part being written instead.
                        if let Some(title) = last_title(&content) {
                            self.thought(&format!("Writing “{title}”"));
                        }
                    }
                }
                "response.reasoning_summary_text.delta" => {
                    summary.push_str(&string(&d["delta"]));
                    // thought() keeps the first line without its closing "**".
                    self.thought(&format!("Thinking: {}", summary.trim_start().trim_start_matches('*')));
                }
                "response.reasoning_summary_part.added" => summary.clear(),
                "response.output_item.done" if d["item"]["type"] == "function_call" => {
                    let i = &d["item"];
                    calls.push(json!({"id":i["call_id"],"type":"function","function":{"name":i["name"],"arguments":i["arguments"]}}));
                }
                "response.completed" => {
                    let u = &d["response"]["usage"];
                    self.count(u["input_tokens"].as_u64().unwrap_or(0), u["output_tokens"].as_u64().unwrap_or(0));
                }
                "response.failed" | "error" => {
                    let said = [&d["response"]["error"]["message"], &d["error"]["message"], &d["message"]].into_iter().map(string).find(|s| !s.is_empty()).unwrap_or_default();
                    return Err(Error::bad(format!("ChatGPT stopped without an answer. Try again. ChatGPT said: {}", cap(&said, 300))));
                }
                _ => {}
            }
        }
        if content.is_empty() && calls.is_empty() {
            return Err(unreadable());
        }
        Ok(json!({"content":content,"tool_calls":if calls.is_empty() {Value::Null} else {json!(calls)}}))
    }
}

// The last complete "title" string in a JSON answer that is still arriving.
fn last_title(json: &str) -> Option<String> {
    let rest = &json[json.rfind("\"title\"")? + 7..];
    let rest = rest.trim_start().strip_prefix(':')?.trim_start().strip_prefix('"')?;
    let mut end = None;
    let mut escaped = false;
    for (i, c) in rest.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '"' => { end = Some(i); break; }
            _ => {}
        }
    }
    let title: String = serde_json::from_str(&format!("\"{}\"", &rest[..end?])).ok()?;
    (!title.trim().is_empty()).then(|| title.trim().to_string())
}

#[test]
fn last_title_reads_the_newest_finished_title() {
    assert_eq!(last_title(r#"{"title":"C basics","phases":[{"title": "Loops \"and\" sums","less"#).as_deref(), Some("Loops \"and\" sums"));
    assert_eq!(last_title(r#"{"title":"C basics","phases":[{"title":"Loo"#).as_deref(), None);
    assert_eq!(last_title(r#"{"summary":"x"}"#), None);
}

#[test]
fn url_helpers_round_trip() {
    assert_eq!(enc("http://localhost:1455/auth callback"), "http%3A%2F%2Flocalhost%3A1455%2Fauth%20callback");
    assert_eq!(dec("a%20b+c%2Fd%zz"), "a b c/d%zz");
    assert_eq!(query("/auth/callback?code=x%2By&state=abc", "code"), "x+y");
    assert_eq!(query("/auth/callback?code=x", "state"), "");
    assert_eq!(claims("a.eyJleHAiOjV9.b")["exp"], 5);
}
