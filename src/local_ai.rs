//! AI programs on this computer as the AI service: OpenCode (base "opencode") and Claude Code (base "claude").
//! OpenCode Zen's free models answer only inside OpenCode, and a Claude plan only inside Claude Code, so each
//! request runs the program once and reads its JSON events.
use crate::ai::{provider_name, Sink};
use crate::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

/// Used when no model is picked: free on OpenCode Zen.
pub(crate) const OPENCODE_MODEL: &str = "opencode/big-pickle";
const PREAMBLE: &str = "You are answering for Roadmap, a learning app. Do not use any tools, files or commands: everything you need is in this message. Reply with the answer only.";
// Each run is a separate process; a roadmap's parallel jobs take turns three at a time.
static RUNNING: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());
const MOST: usize = 3;

#[cfg_attr(not(windows), allow(unused_mut))]
fn command(bin: &Path) -> Command {
    let mut cmd = Command::new(bin);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd
}
fn program(found: Option<std::path::PathBuf>, name: &str) -> Result<std::path::PathBuf> {
    found.ok_or_else(|| Error::bad(format!("{name} is not installed on this computer. Install it, or pick another AI service in Settings.")))
}
/// OpenCode 2 can keep these runs out of the learner's own session list; OpenCode 1 has no such option.
fn archives(bin: &Path) -> bool {
    static V2: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *V2.get_or_init(|| {
        let out = command(bin).arg("--version").stdin(Stdio::null()).output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
        out.trim().trim_start_matches(|c: char| !c.is_ascii_digit()).split('.').next().and_then(|m| m.parse::<u32>().ok()).is_some_and(|m| m >= 2)
    })
}

/// The whole request as one message: these programs take a single prompt, and the app's own tools are not offered.
/// The app's instructions come first and the conversation after, so the program's prompt cache keeps the start.
fn prompt(msgs: &[Value], extra: &Value) -> String {
    let mut out = String::new();
    let mut talk = vec![];
    for m in msgs {
        let content = text(&m["content"]);
        match m["role"].as_str().unwrap_or("") {
            "system" => out.push_str(&format!("{content}\n\n")),
            "assistant" if !content.is_empty() => talk.push(format!("You:\n{content}")),
            "tool" => talk.push(format!("Tool result:\n{content}")),
            "user" => talk.push(format!("User:\n{content}")),
            _ => {}
        }
    }
    if talk.len() == 1 {
        out.push_str(talk[0].trim_start_matches("User:\n"));
    } else {
        out.push_str(&format!("The conversation so far:\n\n{}\n\nWrite your next reply to the user.", talk.join("\n\n")));
    }
    let schema = &extra["response_format"]["json_schema"]["schema"];
    if schema.is_object() {
        out.push_str(&format!("\n\nReply with one JSON object and nothing else. It must match this JSON Schema:\n{schema}"));
    }
    out
}

/// What one line of a program's output means.
enum Step {
    Text(String),
    /// Tokens in (cache included), cached, out; the model that answered, when the program says;
    /// and the whole answer, kept only when no text came before it.
    Usage([u64; 3], Option<String>, String),
    Fail(String),
    Skip,
}

impl App {
    fn run_local(&self, base: &str, model: &str, mut cmd: Command, body: String, on_text: &mut Sink<'_>, mut read: impl FnMut(&Value) -> Step) -> Result<Value> {
        let name = provider_name(base);
        {
            let mut n = RUNNING.0.lock().unwrap_or_else(|e| e.into_inner());
            while *n >= MOST {
                n = RUNNING.1.wait(n).unwrap_or_else(|e| e.into_inner());
            }
            *n += 1;
        }
        struct Slot;
        impl Drop for Slot {
            fn drop(&mut self) {
                *RUNNING.0.lock().unwrap_or_else(|e| e.into_inner()) -= 1;
                RUNNING.1.notify_one();
            }
        }
        let _slot = Slot;
        // An empty folder, so the program has nothing of the learner's to look through.
        let room = tempfile::tempdir()?;
        let mut child = cmd.current_dir(room.path()).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
            .map_err(|e| Error::bad(format!("Could not start {name}: {e}")))?;
        // The prompt goes in on stdin: Windows caps a command line at 32K characters.
        let mut stdin = child.stdin.take().ok_or_else(Error::missing)?;
        std::thread::spawn(move || { let _ = stdin.write_all(body.as_bytes()); });
        let stderr = child.stderr.take().ok_or_else(Error::missing)?;
        let errors = std::thread::spawn(move || { let mut s = String::new(); let _ = stderr.take(64 << 10).read_to_string(&mut s); s });
        let (tx, rx) = std::sync::mpsc::channel();
        let stdout = child.stdout.take().ok_or_else(Error::missing)?;
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if tx.send(line).is_err() { break; }
            }
        });
        let (mut content, mut whole, mut failed, mut used, mut answered) = (String::new(), String::new(), None, [0u64; 3], None);
        let started = std::time::Instant::now();
        let outcome = loop {
            let late = started.elapsed() > Duration::from_secs(900);
            if (on_text.is_some() && self.stop_chat.load(Ordering::Relaxed)) || late {
                let _ = child.kill();
                break Err(if late { Error::bad(format!("{name} took too long to answer. Try again.")) } else { Error::stopped() });
            }
            let line = match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(Ok(line)) => line,
                Ok(Err(_)) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break Ok(()),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            };
            let Ok(d) = serde_json::from_str::<Value>(&line) else { continue };
            match read(&d) {
                Step::Text(t) if !t.is_empty() => {
                    content.push_str(&t);
                    self.draft(&t);
                    if let Some(out) = on_text.as_mut() {
                        if let Err(e) = out(&t) {
                            let _ = child.kill();
                            break Err(e);
                        }
                    }
                }
                Step::Usage(u, m, all) => {
                    (0..3).for_each(|i| used[i] += u[i]);
                    answered = m.or(answered);
                    whole = all;
                }
                Step::Fail(why) => failed = Some(why).filter(|m| !m.trim().is_empty()).or(Some(format!("{name} stopped with an error."))),
                _ => {}
            }
        };
        let status = child.wait();
        if used != [0; 3] {
            self.count(&name, answered.as_deref().unwrap_or(model), used[0], used[1], used[2]);
        }
        outcome?;
        if content.is_empty() && !whole.is_empty() {
            self.draft(&whole);
            if let Some(out) = on_text.as_mut() {
                out(&whole)?;
            }
            content = whole;
        }
        if let Some(why) = failed.filter(|_| content.is_empty()) {
            return Err(Error::bad(format!("{name} said: {}", why.trim())));
        }
        if content.is_empty() && !status.is_ok_and(|s| s.success()) {
            let said = errors.join().unwrap_or_default();
            return Err(Error::bad(format!("{name} did not answer: {}", cap(said.trim().lines().last().unwrap_or("it stopped without saying why"), 300))));
        }
        Ok(json!({"content":content,"tool_calls":Value::Null}))
    }

    pub(crate) fn ask_opencode(&self, model: &str, msgs: &[Value], on_text: &mut Sink<'_>, extra: &Value) -> Result<Value> {
        let bin = program(crate::harness::find_opencode(), "OpenCode")?;
        let model = if model.is_empty() || model == "default" { OPENCODE_MODEL } else { model };
        let mut cmd = command(&bin);
        // The plan agent cannot edit or run commands; Zen refuses its free models to an agent stripped of tools.
        cmd.args(["run", "--format", "json", "--agent", "plan", "-m", model]);
        if archives(&bin) {
            cmd.arg("--archived");
        }
        let mut first = true;
        self.run_local("opencode", model, cmd, format!("{PREAMBLE}\n\n{}", prompt(msgs, extra)), on_text, |d| match d["type"].as_str().unwrap_or("") {
            // Each text event is a whole part of the answer.
            "text" => {
                let t = string(&d["part"]["text"]);
                let t = if first || t.is_empty() { t } else { format!("\n\n{t}") };
                first &= t.is_empty();
                Step::Text(t)
            }
            "step_finish" => {
                let t = &d["part"]["tokens"];
                let n = |v: &Value| v.as_u64().unwrap_or(0);
                Step::Usage([n(&t["input"]) + n(&t["cache"]["read"]) + n(&t["cache"]["write"]), n(&t["cache"]["read"]), n(&t["output"]) + n(&t["reasoning"])], None, String::new())
            }
            "error" => Step::Fail(string(&d["error"]["message"])),
            _ => Step::Skip,
        })
    }
    /// `opencode models` lists every model OpenCode can reach: its free Zen ones and any service signed in there.
    pub(crate) fn opencode_models(&self) -> Result<Vec<Value>> {
        let bin = program(crate::harness::find_opencode(), "OpenCode")?;
        let out = command(&bin).arg("models").stdin(Stdio::null()).output().map_err(|e| Error::bad(format!("Could not start OpenCode: {e}")))?;
        let mut models: Vec<Value> = String::from_utf8_lossy(&out.stdout).lines().map(str::trim)
            .filter(|l| matches(r"^[\w.-]+/\S+$", l))
            .map(|id| {
                let free = id.starts_with("opencode/") && (id.ends_with("-free") || id == OPENCODE_MODEL);
                json!({"id":id,"name":id,"note":if free { "Free on OpenCode Zen" } else { "" }})
            })
            .collect();
        if models.is_empty() {
            return Err(Error::bad("OpenCode listed no models. Open OpenCode once to finish setting it up."));
        }
        // Free models first, then the rest of OpenCode Zen, then other services.
        models.sort_by_key(|m| (m["note"] == "", !string(&m["id"]).starts_with("opencode/"), string(&m["id"])));
        Ok(models)
    }

    pub(crate) fn ask_claude_code(&self, model: &str, msgs: &[Value], on_text: &mut Sink<'_>, extra: &Value) -> Result<Value> {
        let bin = program(crate::harness::find_claude(), "Claude Code")?;
        let mut cmd = command(&bin);
        // No tools, no MCP servers, and none of the learner's settings, hooks, plugins or CLAUDE.md files.
        cmd.args(["-p", "--output-format", "stream-json", "--verbose", "--include-partial-messages", "--tools", "", "--no-session-persistence",
            "--strict-mcp-config", "--disable-slash-commands", "--setting-sources", "", "--system-prompt", PREAMBLE]);
        if !model.is_empty() && model != "default" {
            cmd.args(["--model", model]);
        }
        self.run_local("claude", model, cmd, prompt(msgs, extra), on_text, |d| match d["type"].as_str().unwrap_or("") {
            "stream_event" if d["event"]["delta"]["type"] == "text_delta" => Step::Text(string(&d["event"]["delta"]["text"])),
            "result" if truth(&d["is_error"]) && string(&d["result"]).contains("Not logged in") => Step::Fail(NOT_SIGNED_IN.into()),
            "result" if truth(&d["is_error"]) => Step::Fail(string(&d["result"])),
            "result" => {
                let u = &d["usage"];
                let n = |v: &Value| v.as_u64().unwrap_or(0);
                let fresh = n(&u["input_tokens"]) + n(&u["cache_creation_input_tokens"]);
                let model = d["modelUsage"].as_object().and_then(|m| m.keys().next().cloned());
                Step::Usage([fresh + n(&u["cache_read_input_tokens"]), n(&u["cache_read_input_tokens"]), n(&u["output_tokens"])], model, string(&d["result"]))
            }
            _ => Step::Skip,
        })
    }
    /// Claude Code takes a name for the newest model of each kind; default is what the learner's plan picks.
    /// It has no command that lists models, so the names its --model help gives are added to the known ones.
    pub(crate) fn claude_code_models(&self) -> Result<Vec<Value>> {
        let bin = program(crate::harness::find_claude(), "Claude Code")?;
        if claude_auth().is_some_and(|(signed_in, _)| !signed_in) {
            return Err(Error::bad(NOT_SIGNED_IN));
        }
        let help = command(&bin).arg("--help").stdin(Stdio::null()).output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
        Ok(claude_aliases(&help).into_iter().map(|id| {
            let kind = format!("{}{}", id[..1].to_uppercase(), &id[1..]);
            match id.as_str() {
                "default" => json!({"id":id,"name":"Claude Code's default","note":"What your Claude plan picks"}),
                "haiku" => json!({"id":id,"name":"Claude Haiku","note":"Newest Haiku, fastest"}),
                _ => json!({"id":id,"name":format!("Claude {kind}"),"note":format!("Newest {kind}")}),
            }
        }).collect())
    }
}

pub(crate) const NOT_SIGNED_IN: &str = "Claude Code is not signed in. Run claude in a terminal and sign in, then check again.";
/// Whether Claude Code is signed in (a Claude account or an API key), and its email; None when it is missing or cannot say.
pub(crate) fn claude_auth() -> Option<(bool, String)> {
    let bin = crate::harness::find_claude()?;
    let out = command(&bin).args(["auth", "status", "--json"]).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    let v: Value = serde_json::from_slice(&out.stdout).ok()?;
    v["loggedIn"].as_bool().map(|b| (b, string(&v["email"])))
}
/// The known model names, then any other the --model help quotes (it reads "e.g. 'fable', 'opus', or 'sonnet'").
fn claude_aliases(help: &str) -> Vec<String> {
    let mut ids: Vec<String> = ["default", "fable", "opus", "sonnet", "haiku"].map(String::from).to_vec();
    let model = help.split("--model <model>").nth(1).and_then(|s| s.split("\n  -").next()).unwrap_or("");
    for (i, word) in model.split('\'').enumerate() {
        if i % 2 == 1 && !word.is_empty() && word.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') && !ids.iter().any(|x| x == word) {
            ids.push(word.into());
        }
    }
    ids
}
/// Whether something answers on Ollama's port; a short wait, since a closed port on Windows takes seconds to refuse.
pub(crate) fn ollama_running() -> bool {
    std::net::TcpStream::connect_timeout(&([127, 0, 0, 1], 11434).into(), Duration::from_millis(250)).is_ok()
}

#[cfg(test)]
#[test]
fn claude_model_names_come_from_its_help() {
    let help = "  --model <model>    Model for the current session. Use an\n                     alias for the latest model (e.g.\n                     'fable', 'opus', or 'mythos') or a\n                     model's full name.\n  -n, --name <name>  Set a 'display' name";
    assert_eq!(claude_aliases(help), ["default", "fable", "opus", "sonnet", "haiku", "mythos"]);
    assert_eq!(claude_aliases(""), ["default", "fable", "opus", "sonnet", "haiku"]);
}

#[cfg(test)]
#[test]
fn local_programs_get_one_prompt() {
    let msgs = [json!({"role":"system","content":"Be a tutor."}), json!({"role":"user","content":"hi"}), json!({"role":"assistant","content":"hello"}), json!({"role":"user","content":"why?"})];
    let p = prompt(&msgs, &json!({}));
    assert_eq!(p, "Be a tutor.\n\nThe conversation so far:\n\nUser:\nhi\n\nYou:\nhello\n\nUser:\nwhy?\n\nWrite your next reply to the user.");
    let p = prompt(&msgs[..2], &json!({"response_format":{"json_schema":{"schema":{"type":"object"}}}}));
    assert_eq!(p, "Be a tutor.\n\nhi\n\nReply with one JSON object and nothing else. It must match this JSON Schema:\n{\"type\":\"object\"}");
}
