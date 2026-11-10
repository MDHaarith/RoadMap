use crate::*;
use crate::{
    ai::said,
    exercise::{install_lang, run_code},
};
impl App {
    pub(crate) fn library_list(&self) -> Result<Value> {
        Ok(json!(self.courses()?.iter().map(|c|{let ls=lessons(c).collect::<Vec<_>>();let next=ls.iter().find(|l|!truth(&c["state"]["done"][string(&l["id"])]));json!({"id":c["id"],"title":c["title"],"summary":c["summary"],"total":ls.len(),"done":c["state"]["done"].as_object().map(|o|o.values().filter(|v|truth(v)).count()).unwrap_or(0),"paused":truth(&c["paused"]),"project":truth(&c["folder"]),"next":next.map(|l|json!({"id":l["id"],"title":l["title"]}))})}).collect::<Vec<_>>()))
    }
    pub(crate) fn reviewed(&self, cid: &str, lid: &str, i: &str, right: bool) -> Result<Value> {
        let _guard = self.guard();
        let mut c = self.load(cid)?;
        if !c["state"]["review"].is_object() {
            c["state"]["review"] = json!({});
        }
        let key = format!("{lid}#{i}");
        let step = if right {
            (c["state"]["review"][&key]["step"].as_i64().unwrap_or(-1) + 1).clamp(0, 4)
        } else {
            0
        };
        let r = json!({"step":step,"due":self.today()+[1,3,7,16,35][step as usize]});
        c["state"]["review"][&key] = r.clone();
        if i.parse::<usize>().is_ok() {
            if !right {
                missed(&mut c, &key, None, self.today());
            } else if c["state"]["mistakes"][&key].is_object() {
                c["state"]["mistakes"][&key]["fixed"] = json!(true);
            }
        }
        self.save(&c)?;
        Ok(r)
    }
    // A wrong quiz or exam answer goes in the mistakes notebook and comes back in today's review.
    pub(crate) fn mistake(&self, b: &Value) -> Result<Value> {
        let _guard = self.guard();
        let mut c = self.load(&text(&b["course"]))?;
        let l = lesson(&c, &b["lesson"])?;
        let i = b["i"]
            .as_u64()
            .filter(|i| (*i as usize) < list(&l["practice"]["quiz"]).len())
            .ok_or_else(Error::missing)?;
        let key = format!("{}#{i}", string(&l["id"]));
        missed(&mut c, &key, b["picked"].as_i64(), self.today());
        if !c["state"]["review"].is_object() {
            c["state"]["review"] = json!({});
        }
        c["state"]["review"][&key] = json!({"step":-1,"due":self.today()});
        self.save(&c)?;
        Ok(json!({}))
    }
    pub(crate) fn mistakes(&self) -> Result<Value> {
        let mut out = vec![];
        for c in self.courses()? {
            let st = &c["state"];
            let base = |l: &Value| json!({"course":c["id"],"roadmap":c["title"],"lesson":l["id"],"lesson_title":l["title"]});
            for (key, m) in st["mistakes"].as_object().into_iter().flatten() {
                let Some((lid, i)) = key.split_once('#') else {
                    continue;
                };
                let (Ok(l), Ok(i)) = (lesson(&c, &json!(lid)), i.parse::<usize>()) else {
                    continue;
                };
                if let Some(q) = list(&l["practice"]["quiz"]).get(i) {
                    out.push(merge(
                        merge(base(l), json!({"kind":"quiz","i":i})),
                        merge(q.clone(), m.clone()),
                    ));
                }
            }
            for l in lessons(&c) {
                let t = &st["tests"][string(&l["id"])];
                if t["runs"].as_u64().unwrap_or(0) > 0 && !truth(&t["ok"]) {
                    out.push(merge(base(l), json!({"kind":"code","title":l["practice"]["exercise"]["title"],"n":t["runs"],"fixed":false,"day":0})));
                }
            }
        }
        out.sort_by_key(|m| {
            (
                truth(&m["fixed"]),
                std::cmp::Reverse(m["day"].as_i64().unwrap_or(0)),
            )
        });
        out.truncate(200);
        Ok(json!({"items":out}))
    }
    // Every word must appear in a hit: lesson text, notes, tutor chats and sticky ideas.
    pub(crate) fn search(&self, q: &str) -> Result<Value> {
        let terms = q
            .to_lowercase()
            .split_whitespace()
            .take(6)
            .map(String::from)
            .collect::<Vec<_>>();
        let mut out = vec![];
        if terms.is_empty() {
            return Ok(json!({"results":out}));
        }
        let mut hit = |kind: &str, at: Value, title: String, body: &str| {
            let low = body.to_lowercase();
            if out.len() >= 40 || !terms.iter().all(|t| low.contains(t)) {
                return;
            }
            let before = low[..low.find(&terms[0]).unwrap_or(0)].chars().count();
            let snippet = body
                .chars()
                .skip(before.saturating_sub(60))
                .take(200)
                .collect::<String>();
            out.push(merge(at, json!({"kind":kind,"title":title,"snippet":snippet.split_whitespace().collect::<Vec<_>>().join(" ")})));
        };
        for n in list(&self.read_side("ideas.json")["notes"]) {
            hit("idea", json!({}), "Idea".into(), &string(n));
        }
        for c in self.courses()? {
            let chats = self
                .chat_file(&string(&c["id"]))
                .map(|f| self.read_side(&f))
                .unwrap_or_default();
            for l in lessons(&c) {
                let lid = string(&l["id"]);
                let at = json!({"course":c["id"],"roadmap":c["title"],"lesson":l["id"]});
                let title = text(&l["title"]);
                hit(
                    "lesson",
                    at.clone(),
                    title.clone(),
                    &format!(
                        "{title}. {} {} {}",
                        string(&l["goal"]),
                        joined(&l["concepts"]),
                        joined(&l["tasks"])
                    ),
                );
                hit(
                    "note",
                    at.clone(),
                    title.clone(),
                    &string(&c["state"]["notes"][&lid]),
                );
                for m in list(&chats[&lid]) {
                    hit("chat", at.clone(), title.clone(), &string(&m["content"]));
                }
            }
        }
        Ok(json!({"results":out}))
    }
    pub(crate) fn review_due(&self) -> Result<Value> {
        let mut due = vec![];
        for c in self.courses()? {
            if truth(&c["paused"]) {
                continue;
            }
            let st = &c["state"];
            let r = &st["review"];
            for l in lessons(&c) {
                let lid = string(&l["id"]);
                if st["quiz"].get(&lid).is_none() {
                    continue;
                }
                let base = json!({"course":c["id"],"roadmap":c["title"],"lesson":l["id"],"lesson_title":l["title"]});
                let pr = &l["practice"];
                let mut items = list(&pr["quiz"])
                    .iter()
                    .enumerate()
                    .map(|(i, q)| (json!(i), merge(json!({"kind":"quiz"}), q.clone())))
                    .collect::<Vec<_>>();
                let cs = list(&l["concepts"]);
                if !cs.is_empty() && r.get(format!("{lid}#explain")).is_some() {
                    let step = r[format!("{lid}#explain")]["step"].as_i64().unwrap_or(-1);
                    items.push((json!("explain"),json!({"kind":"explain","q":cs[(step+1).rem_euclid(cs.len() as i64) as usize]})));
                }
                let ex = &pr["exercise"];
                if truth(ex) && r.get(format!("{lid}#code")).is_some() {
                    items.push((json!("code"),json!({"kind":"code","lang":ex["lang"],"title":ex["title"],"prompt":ex["prompt"],"starter":ex["starter"]})));
                }
                for (i, item) in items {
                    let when = r[format!("{lid}#{}", text(&i))]["due"]
                        .as_i64()
                        .unwrap_or(0);
                    if when <= self.today() {
                        due.push((when, merge(merge(base.clone(), json!({"i":i})), item)));
                    }
                }
            }
        }
        due.sort_by_key(|(d, _)| *d);
        Ok(
            json!({"count":due.len(),"questions":due.into_iter().take(10).map(|(_,q)|q).collect::<Vec<_>>()}),
        )
    }
    pub fn route(self: &Arc<Self>, method: &str, path: &str, b: &Value) -> Result<Value> {
        match (method, path) {
            ("GET", "api/settings") => {
                let c = self.config();
                return Ok(
                    json!({"base":c.get("base").unwrap_or(&json!("")),"model":c.get("model").unwrap_or(&json!("")),"small":string(&c["small"]),"usage":self.read_side("usage.json"),"key_set":truth(&c["key"]),"theme":self.theme()}),
                );
            }
            ("PUT", "api/theme") => {
                if !["light", "dark", "system"].iter().any(|t| b["theme"] == *t) {
                    return Err(Error::bad("Unknown theme."));
                }
                let _guard = self.guard();
                self.write_config(&merge(self.config(), json!({"theme":b["theme"]})))?;
                return Ok(json!({}));
            }
            ("PUT", "api/settings") => {
                self.save_config(b)?;
                return Ok(json!({}));
            }
            ("GET", "api/courses") => {
                return self.library_list();
            }
            ("PUT", "api/usage") => {
                let _meter = self.meter.lock().unwrap_or_else(|e| e.into_inner());
                self.write_side("usage.json", &json!({"since":self.today()}))?;
                return Ok(json!({}));
            }
            ("GET", "api/ideas") => {
                return Ok(json!({"notes":list(&self.read_side("ideas.json")["notes"])}));
            }
            ("PUT", "api/ideas") => {
                if !b["notes"].is_array() {
                    return Err(Error::bad("Bad notes."));
                }
                let notes = list(&b["notes"])
                    .iter()
                    .filter_map(Value::as_str)
                    .take(200)
                    .map(|s| cap(s, 4000))
                    .collect::<Vec<_>>();
                let _guard = self.guard();
                self.write_side("ideas.json", &json!({"notes":notes}))?;
                return Ok(json!({}));
            }
            ("POST", "api/search") => return self.search(&string(&b["q"])),
            ("POST", "api/mistake") => return self.mistake(b),
            ("GET", "api/mistakes") => return self.mistakes(),
            ("GET", "api/review") => return self.review_due(),
            ("POST", "api/review") => {
                let i = b
                    .get("i")
                    .unwrap_or(&json!(-1))
                    .as_i64()
                    .or_else(|| b["i"].as_str().and_then(|s| s.parse().ok()))
                    .or_else(|| b["i"].as_f64().map(|x| x as i64))
                    .or_else(|| b["i"].as_bool().map(i64::from))
                    .ok_or_else(|| Error::bad("Bad review index."))?;
                return self.reviewed(
                    &text(&b["course"]),
                    &cap(&text(&b["lesson"]), 20),
                    &i.to_string(),
                    truth(&b["right"]),
                );
            }
            ("POST", "api/review/answer") => return self.review_answer(b),
            ("POST", "api/courses") | ("POST", "api/placement") => {
                let prompt = cap(b.get("prompt").map(text).unwrap_or_default().trim(), 2000);
                let folder = string(&b["folder"]).trim().to_string();
                if prompt.is_empty() && folder.is_empty() {
                    return Err(Error::bad("Say what you want to learn first."));
                }
                if path == "api/placement" {
                    return self.placement(&prompt, &folder, &b["answers"]);
                }
                let mut level = vec![cap(&b.get("level").map(text).unwrap_or_default(), 100)];
                let answers = said(&b["answers"]);
                if !answers.is_empty() {
                    level.push("Placement check:".into());
                    level.extend(answers);
                }
                let c = self.build_course(
                    &prompt,
                    &level.join("\n"),
                    &folder,
                    &string(&b["size"]),
                )?;
                self.save(&c)?;
                let cid = string(&c["id"]);
                let first = lessons(&c).next().cloned().unwrap_or_default();
                // A long roadmap is saved as an outline: write its first phase now so the learner can start.
                // If that fails, the lesson page asks again when it is opened.
                if truth(&first["stub"]) {
                    return Ok(self.expand(&cid, &first["id"]).unwrap_or(c));
                }
                self.prefetch(cid);
                return Ok(c);
            }
            ("POST", "api/run") => {
                let c = self.load(&text(&b["course"]))?;
                let l = lesson(&c, &b["lesson"])?;
                let ex = &l["practice"]["exercise"];
                if !truth(ex) {
                    return Err(Error::missing());
                }
                let tests = truth(&b["test"]);
                let out = run_code(
                    &string(&ex["lang"]),
                    &cap(&b.get("code").map(text).unwrap_or_default(), 100000),
                    if tests { ex["tests"].as_str() } else { None },
                )?;
                if tests && !truth(&out["missing"]) {
                    let _guard = self.guard();
                    let mut c = self.load(&text(&b["course"]))?;
                    if !c["state"]["tests"].is_object() {
                        c["state"]["tests"] = json!({});
                    }
                    let lid = string(&l["id"]);
                    let t = &c["state"]["tests"][&lid];
                    let t = json!({"runs":t["runs"].as_u64().unwrap_or(0)+1,"ok":truth(&t["ok"])||truth(&out["ok"])});
                    c["state"]["tests"][lid] = t;
                    self.save(&c)?;
                }
                return Ok(out);
            }
            ("POST", "api/import") => {
                let f = (self.pick)("file");
                return if f.is_empty() {
                    Ok(json!({}))
                } else {
                    self.import_course(&f)
                };
            }
            ("POST", "api/pick-folder") => return Ok(json!({"folder":(self.pick)("folder")})),
            ("POST", "api/file") => {
                let c = self.load(&text(&b["course"]))?;
                return self
                    .project_file(
                        if truth(&c["folder"]) {
                            c["folder"].as_str().unwrap_or("/nonexistent")
                        } else {
                            "/nonexistent"
                        },
                        &b["path"],
                    )
                    .map(|s| json!({"text":s}))
                    .ok_or_else(Error::missing);
            }
            ("POST", "api/install") => return install_lang(&string(&b["lang"])),
            ("POST", "api/chat") => {
                let (c, l, ms) = self.chat_args(b)?;
                let answer = self.tutor(&c, &l, ms.clone(), &mut None)?;
                self.save_chat(&string(&c["id"]), &string(&l["id"]), &ms, &answer);
                return Ok(json!({"text":answer}));
            }
            _ => {}
        }
        let re = regex::Regex::new(
            r"^api/courses/([0-9a-f]{12})(/state|/videos|/practice|/paused|/adapt|/revise|/expand|/export|/summary|/setup|/work)?$",
        )
        .unwrap();
        let m = re.captures(path).ok_or_else(Error::missing)?;
        let cid = &m[1];
        let sub = m.get(2).map(|m| m.as_str()).unwrap_or("");
        match (method, sub) {
            ("GET", "") => {
                let mut c = self.mark_known(self.load(cid)?)?;
                c["chats"] = self.read_side(&self.chat_file(cid)?);
                Ok(c)
            }
            ("DELETE", "") => {
                let _guard = self.guard();
                fs::remove_file(self.path_of(cid)?)?;
                let _ = fs::remove_file(self.data.join(self.chat_file(cid)?));
                Ok(json!({}))
            }
            ("PUT", "/state") => {
                if !["checks", "done", "notes"]
                    .iter()
                    .all(|k| b[*k].is_object())
                {
                    return Err(Error::bad("Bad progress data."));
                }
                let _guard = self.guard();
                let mut c = self.load(cid)?;
                let old = c["state"].clone();
                let mut st = b.clone();
                st["review"] = if truth(&old["review"]) {
                    old["review"].clone()
                } else {
                    json!({})
                };
                st["tests"] = if truth(&old["tests"]) {
                    old["tests"].clone()
                } else {
                    json!({})
                };
                // The mistakes notebook is written by the server only, like review and tests.
                if truth(&old["mistakes"]) {
                    st["mistakes"] = old["mistakes"].clone();
                } else if let Some(o) = st.as_object_mut() {
                    o.remove("mistakes");
                }
                if let Some(q) = b["quiz"].as_object() {
                    for lid in q.keys() {
                        if old["quiz"].get(lid).is_none() {
                            let l = lesson(&c, &json!(lid))?;
                            let pr = &l["practice"];
                            let mut items = (0..list(&pr["quiz"]).len())
                                .map(|i| (i.to_string(), 1))
                                .collect::<Vec<_>>();
                            items.push(("explain".into(), 2));
                            if truth(&pr["exercise"]) {
                                items.push(("code".into(), 4));
                            }
                            for (i, d) in items {
                                let key = format!("{lid}#{i}");
                                if st["review"].get(&key).is_none() {
                                    st["review"][key] = json!({"step":-1,"due":self.today()+d});
                                }
                            }
                        }
                    }
                }
                c["state"] = st;
                self.save(&c)?;
                Ok(json!({}))
            }
            ("POST", "/adapt") => self.adapt(cid, ""),
            ("POST", "/revise") => {
                let ask = cap(b.get("ask").map(text).unwrap_or_default().trim(), 600);
                if ask.is_empty() {
                    return Err(Error::bad("Say what you want to change first."));
                }
                self.adapt(cid, &ask)
            }
            ("POST", "/expand") => self.expand(cid, &b["lesson"]),
            ("POST", "/export") => self.export_course(&self.load(cid)?, truth(&b["template"])),
            ("POST", "/summary") => self.summary(&self.load(cid)?),
            ("POST", "/setup") => {
                let c = self.load(cid)?;
                let mut found = json!({});
                for cmd in list(&c["needs"])
                    .iter()
                    .filter_map(|n| n["command"].as_str())
                    .filter(|s| !s.is_empty())
                {
                    found[cmd] = json!(on_path(cmd));
                }
                Ok(json!({"found":found}))
            }
            ("PUT", "/work") => {
                let clear = truth(&b["clear"]);
                let folder = if clear {
                    String::new()
                } else {
                    (self.pick)("folder")
                };
                if folder.is_empty() && !clear {
                    return Ok(json!({"work":string(&self.load(cid)?["work"])}));
                }
                let root = if folder.is_empty() {
                    String::new()
                } else {
                    self.read_project(&folder)?.0
                };
                let _guard = self.guard();
                let mut c = self.load(cid)?;
                c["work"] = json!(root);
                self.save(&c)?;
                Ok(json!({"work":root}))
            }
            ("PUT", "/paused") => {
                let _guard = self.guard();
                let mut c = self.load(cid)?;
                c["paused"] = json!(truth(&b["paused"]));
                self.save(&c)?;
                Ok(json!({}))
            }
            ("POST", "/videos") => {
                let c = self.load(cid)?;
                let l = lesson(&c, &b["lesson"])?;
                let old = list(&l["videos"]);
                if let Some(query) = b["query"].as_str() {
                    let query = cap(query.trim(), 200);
                    if query.is_empty() { return Err(Error::bad("Enter a video search query.")); }
                    let found = self.query_videos(&query, &old.iter().map(|v| string(&v["id"])).collect::<Vec<_>>())?;
                    if found.is_empty() { return Err(Error::bad("No videos found.")); }
                    let _guard = self.guard();
                    let mut c = self.load(cid)?;
                    let l = lesson_mut(&mut c, &string(&l["id"]))?;
                    let mut videos = list(&l["videos"]).to_vec();
                    videos.extend(found.into_iter().filter(|v| !list(&l["videos"]).iter().any(|x| x["id"] == v["id"])));
                    while videos.len() > 8 {
                        let Some(i) = videos.iter().position(|v| truth(&v["searched"])) else { break };
                        videos.remove(i);
                    }
                    l["videos"] = json!(videos);
                    self.save(&c)?;
                    return Ok(json!(videos));
                }
                let fresh = truth(&b["fresh"]);
                let vids = if fresh || old.is_empty() {
                    self.find_videos(
                        &c,
                        l,
                        &if fresh {
                            old.iter().map(|v| string(&v["id"])).collect::<Vec<_>>()
                        } else {
                            vec![]
                        },
                    )?
                } else {
                    old.to_vec()
                };
                if !vids.is_empty() && (fresh || old.is_empty()) {
                    let _guard = self.guard();
                    let mut c = self.load(cid)?;
                    lesson_mut(&mut c, &string(&l["id"]))?["videos"] = json!(vids);
                    self.save(&c)?;
                }
                Ok(json!(if vids.is_empty() { old.to_vec() } else { vids }))
            }
            ("POST", "/practice") => {
                let c = self.load(cid)?;
                let l = lesson(&c, &b["lesson"])?;
                if truth(&l["practice"]) && !truth(&b["fresh"]) {
                    return Ok(l["practice"].clone());
                }
                let pr = self.build_practice(&c, l)?;
                let _guard = self.guard();
                let mut c = self.load(cid)?;
                lesson_mut(&mut c, &string(&l["id"]))?["practice"] = pr.clone();
                let prefix = format!("{}#", text(&l["id"]));
                // New questions make the old review dates and recorded mistakes meaningless.
                for k in ["review", "mistakes"] {
                    if k == "review" || c["state"][k].is_object() {
                        let mut r = c["state"][k].as_object().cloned().unwrap_or_default();
                        r.retain(|k, _| !k.starts_with(&prefix));
                        c["state"][k] = json!(r);
                    }
                }
                self.save(&c)?;
                Ok(pr)
            }
            _ => Err(Error::missing()),
        }
    }
    pub fn chat_stream(&self, body: &Value, on_piece: &mut dyn FnMut(&str) -> Result<()>) -> Result<String> {
        let (c, l, msgs) = self.chat_args(body)?;
        let mut pieces = String::new();
        let mut started = false;
        let result = {
            let mut out = |piece: &str| {
                on_piece(piece)?;
                started = true;
                pieces.push_str(piece);
                Ok(())
            };
            self.tutor(&c, &l, msgs.clone(), &mut Some(&mut out))
        };
        match result {
            Ok(text) => {
                self.save_chat(&string(&c["id"]), &string(&l["id"]), &msgs, &text);
                let text = if text.is_empty() { "(No answer.)".into() } else { text };
                if !started { on_piece(&text)?; }
                Ok(text)
            }
            Err(e) if started => {
                let note = format!("\n\n(The answer was cut off: {e})");
                let _ = on_piece(&note);
                pieces.push_str(&note);
                Ok(pieces)
            }
            Err(e) => Err(e),
        }
    }
}

pub fn request_body(body: Value) -> Result<Value> {
    let body = if body.is_null() { json!({}) } else { body };
    if !body.is_object() { return Err(Error::bad("Bad request.")); }
    if serde_json::to_vec(&body)?.len() > 1 << 20 {
        return Err(Error { status: 413, message: "Too large".into() });
    }
    Ok(body)
}

fn missed(c: &mut Value, key: &str, picked: Option<i64>, day: i64) {
    if !c["state"]["mistakes"].is_object() {
        c["state"]["mistakes"] = json!({});
    }
    let old = c["state"]["mistakes"][key].clone();
    c["state"]["mistakes"][key] = json!({"n":old["n"].as_u64().unwrap_or(0)+1,"picked":picked.map_or(old["picked"].clone(),|p|json!(p)),"day":day,"fixed":false});
}
