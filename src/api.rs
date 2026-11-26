use crate::*;
use crate::{
    ai::{question_changed, recall_ask, recall_k, said, shown_quiz},
    exercise::{install_lang, install_tool, run_code},
};
impl App {
    pub(crate) fn library_list(&self) -> Result<Value> {
        Ok(json!(self.courses()?.iter().map(|c|{let ls=lessons(c).collect::<Vec<_>>();let next=ls.iter().find(|l|!truth(&c["state"]["done"][string(&l["id"])]));json!({"id":c["id"],"title":c["title"],"summary":c["summary"],"total":ls.len(),"done":ls.iter().filter(|l|truth(&c["state"]["done"][string(&l["id"])])).count(),"paused":truth(&c["paused"]),"project":truth(&c["folder"]),"next":next.map(|l|json!({"id":l["id"],"title":l["title"]}))})}).collect::<Vec<_>>()))
    }
    // `at` finds, in the lesson as saved now, the review key of the question the learner answered; None means it is gone.
    pub(crate) fn reviewed(&self, cid: &str, lid: &str, right: bool, at: impl Fn(&Value) -> Option<String>) -> Result<Value> {
        let _guard = self.guard();
        let mut c = self.load(cid)?;
        let Some(i) = lesson(&c, &json!(lid)).ok().and_then(at) else {
            return Ok(question_changed());
        };
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
        self.log_activity(1, &[], right as u64, 1);
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
                let base = json!({"course":c["id"],"roadmap":c["title"],"lesson":l["id"],"lesson_title":l["title"]});
                // Cards made from the learner's notes are due on their own schedule, quiz or no quiz.
                for (k, card) in list(&l["cards"]).iter().enumerate() {
                    let i = format!("c{k}");
                    if let Some(when) = r[format!("{lid}#{i}")]["due"].as_i64().filter(|d| *d <= self.today()) {
                        due.push((when, merge(base.clone(), json!({"i":i,"kind":"card","q":card["front"],"back":card["back"]}))));
                    }
                }
                if st["quiz"].get(&lid).is_none() {
                    continue;
                }
                let pr = &l["practice"];
                let mut items = list(&pr["quiz"])
                    .iter()
                    .enumerate()
                    .map(|(i, q)| (json!(i), merge(json!({"kind":"quiz"}), q.clone())))
                    .collect::<Vec<_>>();
                let cs = list(&l["concepts"]);
                if !cs.is_empty() && r.get(format!("{lid}#explain")).is_some() {
                    items.push((json!("explain"),json!({"kind":"explain","q":recall_ask(l, recall_k(&r[format!("{lid}#explain")], cs.len()))})));
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
    // A page that sends "thinking" with a request can read what the model is thinking from GET api/thinking/<key> while it waits.
    pub fn route(self: &Arc<Self>, method: &str, path: &str, b: &Value) -> Result<Value> {
        if let Some(key) = path.strip_prefix("api/thinking/") {
            let text = self.thoughts.lock().unwrap_or_else(|e| e.into_inner()).get(key).cloned().unwrap_or_default();
            let draft = self.drafts.lock().unwrap_or_else(|e| e.into_inner()).get(key).cloned();
            let draft = draft.as_deref().and_then(crate::ai::partial_json).map(|d| crate::ai::outline(&d));
            return Ok(json!({"text":text,"draft":draft}));
        }
        if let Some(id) = path.strip_prefix("api/captions/") {
            return self.captions(id);
        }
        // Whose tokens these are: an open roadmap, or a draft that creating one starts (the page sends its key).
        let meter = match path.strip_prefix("api/courses/") {
            Some(rest) => (cap(rest.split('/').next().unwrap_or(""), 12), "use"),
            None if method == "POST" && ["api/courses", "api/placement", "api/blueprint"].contains(&path) => {
                let d = string(&b["draft"]);
                (format!("new:{}", if matches(r"^[A-Za-z0-9-]{1,40}$", &d) { d } else { random_hex(6).unwrap_or_default() }), "create")
            }
            None if path == "api/chat" => (cap(&string(&b["course"]), 12), "tutor"),
            None => (String::new(), "use"),
        };
        crate::ai::metering(meter, || self.with_effort(&b["effort"], || self.thinking(&cap(&string(&b["thinking"]), 64), || self.route_on(method, path, b))))
    }
    fn route_on(self: &Arc<Self>, method: &str, path: &str, b: &Value) -> Result<Value> {
        match (method, path) {
            ("GET", "api/models") => return self.models(),
            ("POST", "api/chatgpt/login") => return self.chatgpt_login(),
            ("GET", "api/chatgpt/status") => return Ok(self.chatgpt_status()),
            ("POST", "api/chatgpt/logout") => return self.chatgpt_logout(),
            ("POST", "api/models/fetch") => return self.fetch_models(b),
            ("PUT", "api/models/hidden") => return self.hide_model(b),
            ("PUT", "api/model") => {
                self.choose_model(b)?;
                return Ok(json!({}));
            }
            ("GET", "api/settings") => {
                let c = self.config();
                let mut out = json!({"base":c.get("base").unwrap_or(&json!("")),"model":c.get("model").unwrap_or(&json!("")),"small":string(&c["small"]),"usage":self.usage(),"prices":c["prices"].as_object().cloned().unwrap_or_default(),"key_set":truth(&c["key"]),"theme":self.theme(),"videos":clean_prefs(&c["videos"]),
                    "effort":self.default_effort(),"damaged":self.damaged()?,"goal":self.goal(),"os":std::env::consts::OS,"unsandboxed":truth(&c["unsandboxed"])});
                // Settings shows which AI programs this computer has.
                out["opencode"] = json!(crate::harness::find_opencode().is_some());
                out["claude"] = json!(crate::harness::find_claude().is_some());
                // Services used before, so picking one again fills in its model; keys never leave the backend.
                out["providers"] = c["providers"].as_object().map(|p| p.iter().map(|(b, s)| (b.clone(), json!({"model":string(&s["model"]),"small":string(&s["small"]),"key_set":truth(&s["key"])}))).collect()).unwrap_or_default();
                if c["base"] == "codex" {
                    let s = self.chatgpt_status();
                    out["codex"] = s["status"].clone();
                    out["chatgpt_email"] = s["email"].clone();
                }
                if c["base"] == "claude" {
                    local_auth(&mut out);
                }
                return Ok(out);
            }
            // The provider picker's checks, slower than the rest of Settings: Claude Code's sign-in and whether Ollama runs.
            ("GET", "api/local") => {
                let ollama = std::thread::spawn(crate::local_ai::ollama_running);
                let mut out = json!({"opencode":crate::harness::find_opencode().is_some(),"claude":crate::harness::find_claude().is_some()});
                local_auth(&mut out);
                out["ollama"] = json!(ollama.join().unwrap_or(false));
                return Ok(out);
            }
            ("PUT", "api/video-prefs") => {
                let _guard = self.guard();
                let mut c = self.config();
                c["videos"] = clean_prefs(b);
                self.write_config(&c)?;
                return Ok(json!({}));
            }
            ("PUT", "api/theme") => {
                if !["light", "dark", "system"].iter().any(|t| b["theme"] == *t) {
                    return Err(Error::bad("Unknown theme."));
                }
                let _guard = self.guard();
                self.write_config(&merge(self.config(), json!({"theme":b["theme"]})))?;
                return Ok(json!({}));
            }
            // Price per million tokens of one model, in US dollars: [input, cached input, output]; null forgets it.
            ("GET", "api/usage") => return Ok(json!({"usage":self.usage(),"prices":self.config()["prices"].as_object().cloned().unwrap_or_default()})),
            ("PUT", "api/prices") => {
                let model = cap(&string(&b["model"]), 200);
                let price = list(&b["price"]).iter().map(|v| v.as_f64().filter(|x| x.is_finite() && (0.0..=10_000.0).contains(x))).collect::<Option<Vec<_>>>();
                if model.is_empty() || (!b["price"].is_null() && price.as_ref().is_none_or(|p| p.len() != 3)) {
                    return Err(Error::bad("Give a model and three prices."));
                }
                let _guard = self.guard();
                let c = self.config();
                let mut prices = c["prices"].as_object().cloned().unwrap_or_default();
                match price {
                    Some(p) if !b["price"].is_null() => { prices.insert(model, json!(p)); }
                    _ => { prices.remove(&model); }
                }
                self.write_config(&merge(c, json!({"prices":prices})))?;
                return Ok(json!({}));
            }
            ("PUT", "api/effort") => {
                if !crate::ai::EFFORTS.iter().any(|e| b["effort"] == *e) {
                    return Err(Error::bad("Unknown effort."));
                }
                let _guard = self.guard();
                self.write_config(&merge(self.config(), json!({"effort":b["effort"]})))?;
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
            ("GET", "api/activity") => return Ok(self.activity()),
            ("GET", "api/progress") => return self.stats(),
            ("PUT", "api/goal") => return self.set_goal(b),
            ("POST", "api/time") => {
                self.log_time(b["secs"].as_u64().unwrap_or(0), &string(&b["course"]));
                return Ok(json!({}));
            }
            ("GET", "api/stamp") => return Ok(self.library_stamp()),
            ("PUT", "api/unsandboxed") => {
                let _guard = self.guard();
                self.write_config(&merge(self.config(), json!({"unsandboxed":truth(&b["on"])})))?;
                return Ok(json!({}));
            }
            ("POST", "api/review") => {
                // The question is found by the text the page showed, not by its place in the list, which changes with new questions.
                let q = string(&b["q"]);
                let card = b["kind"] == "card";
                return self.reviewed(&text(&b["course"]), &cap(&text(&b["lesson"]), 20), truth(&b["right"]), |l| {
                    if card {
                        list(&l["cards"]).iter().position(|x| string(&x["front"]) == q).map(|k| format!("c{k}"))
                    } else {
                        shown_quiz(l, &q).map(|j| j.to_string())
                    }
                });
            }
            ("POST", "api/review/answer") => return self.review_answer(b),
            ("POST", "api/courses") | ("POST", "api/placement") | ("POST", "api/blueprint") => {
                let prompt = cap(b.get("prompt").map(text).unwrap_or_default().trim(), 2000);
                let folder = string(&b["folder"]).trim().to_string();
                if prompt.is_empty() && folder.is_empty() {
                    return Err(Error::bad("Say what you want to learn first."));
                }
                let mut level = vec![cap(&b.get("level").map(text).unwrap_or_default(), 100)];
                if path == "api/placement" {
                    return self.placement(&prompt, &level[0], &folder);
                }
                let answers = said(&b["answers"]);
                if !answers.is_empty() {
                    level.push("Placement check:".into());
                    level.extend(answers);
                }
                if path == "api/blueprint" { return self.blueprint(&prompt,&level.join("\n"),&folder,&string(&b["size"])); }
                let blueprint = b.get("blueprint").map(crate::ai::clean_blueprint).transpose()?;
                let mut brief = prompt.clone();
                if let Some(plan) = &blueprint {
                    brief.push_str(&format!("\nApproved blueprint: {plan}"));
                }
                let baseline = cap(&string(&b["baseline"]),2000);
                if !baseline.trim().is_empty() { level.push(format!("Optional starting task submission (self-reported): {baseline}")); }
                let mut c = self.build_course(&brief,&level.join("\n"),&folder,&string(&b["size"]))?;
                c["prompt"] = json!(prompt);
                if let Some(plan) = blueprint {
                    c["blueprint"] = plan;
                    // The course took minutes to build: a failed audit must not throw it away, only leave it without checkpoints.
                    let mut audited = c.clone();
                    match self.audit_milestones(&mut audited) {
                        Ok(()) => c = audited,
                        Err(e) => eprintln!("milestone audit failed: {e}"),
                    }
                }
                self.save(&c)?;
                let cid = string(&c["id"]);
                // Writing the first phase, videos and the walkthrough below still count as creating it.
                self.claim_usage(&crate::ai::meter().0, &cid);
                crate::ai::set_meter((cid.clone(), "create"));
                let first = lessons(&c).next().cloned().unwrap_or_default();
                // A long roadmap is saved as an outline: write its first phase now so the learner can start.
                // If that fails, the lesson page asks again when it is opened.
                if truth(&first["stub"]) {
                    c = self.expand(&cid, &first["id"]).unwrap_or(c);
                } else {
                    self.prefetch(cid.clone());
                }
                // Keep the finished course usable when the separate reviewer is unavailable.
                match self.learner_walkthrough(&cid, "") {
                    Ok(report) => c["walkthrough"] = report,
                    Err(e) => eprintln!("learner walkthrough failed: {e}"),
                }
                return Ok(c);
            }
            ("POST", "api/run") => {
                self.may_run()?;
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
            ("GET", "api/import/files") => return self.import_files(),
            ("POST", "api/import") => {
                // A file chosen from the import screen's list; otherwise the system file picker.
                let f = if truth(&b["path"]) { string(&b["path"]) } else { (self.pick)("file") };
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
                let answer = self.tutor(&c, &merge(l.clone(), json!({"watching":b["video"]})), ms.clone(), &mut None, &mut |_, _| {})?;
                self.save_chat(&string(&c["id"]), &string(&l["id"]), &ms, &answer, &[]);
                return Ok(json!({"text":answer}));
            }
            ("POST", "api/opencode/install") => {
                crate::harness::install_opencode().map_err(Error::bad)?;
                return Ok(json!({"opencode":true}));
            }
            ("POST", "api/chat/stop") => {
                self.stop_chat.store(true, Ordering::Relaxed);
                return Ok(json!({}));
            }
            _ => {}
        }
        let re = regex(
            r"^api/courses/([0-9a-f]{12})(/walkthrough|/blueprint|/milestone|/notes|/state|/videos|/video-names|/practice|/paused|/adapt|/revise|/expand|/export|/summary|/setup|/work|/edit|/apply|/prefs|/chat|/exam|/cards|/check|/offline)?$",
        );
        let m = re.captures(path).ok_or_else(Error::missing)?;
        let cid = &m[1];
        let sub = m.get(2).map(|m| m.as_str()).unwrap_or("");
        match (method, sub) {
            ("GET", "") => {
                let mut c = self.mark_known(self.load(cid)?)?;
                // Roadmaps made outside the app (MCP, import) never searched for videos; start when first opened.
                // Obscure picks saved before the view floor are hidden here and removed for good by the prefetch.
                let retry = !self.repicked.lock().unwrap_or_else(|e| e.into_inner()).contains(cid);
                if drop_obscure(&mut c) | lessons(&c).any(|l| needs_videos(l, retry)) {
                    self.prefetch(cid.to_string());
                }
                self.fetch_transcripts(lessons(&c).flat_map(|l| list(&l["videos"]).to_vec()).collect());
                c["chats"] = self.read_side(&self.chat_file(cid)?);
                c["saved_videos"] = json!(self.saved_videos(&c));
                // Phases written since offline was turned on get their videos too.
                if truth(&c["offline"]) {
                    self.save_videos(cid.to_string());
                }
                Ok(c)
            }
            ("POST", "/cards") => self.make_cards(cid, b),
            ("POST", "/check") => self.check_work(cid, b),
            ("GET", "/offline") => self.offline_status(cid),
            ("POST", "/offline") => self.set_offline(cid, b),
            ("DELETE", "") => {
                if truth(&self.load(cid)?["offline"]) {
                    self.set_offline(cid, &json!({"on":false}))?;
                }
                let _guard = self.guard();
                fs::remove_file(self.path_of(cid)?)?;
                let _ = fs::remove_file(self.data.join(self.chat_file(cid)?));
                Ok(json!({}))
            }
            // "New chat": forget one lesson's conversation.
            ("DELETE", "/chat") => {
                let name = self.chat_file(cid)?;
                let _guard = self.guard();
                let mut all = self.read_side(&name);
                if let Some(o) = all.as_object_mut() {
                    o.remove(&cap(&text(&b["lesson"]), 20));
                    self.write_side(&name, &all)?;
                }
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
                st.as_object_mut().unwrap().remove("portfolio");
                if old["portfolio"].is_object() { st["portfolio"] = old["portfolio"].clone(); }
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
                        // A page opened before a lesson was removed may still send its score; it gets no review dates.
                        let Ok(l) = lesson(&c, &json!(lid)) else { continue };
                        if old["quiz"].get(lid).is_none() {
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
                // Newly finished lessons and quizzes count toward the daily goal.
                let done = b["done"].as_object().into_iter().flatten().filter(|(k, v)| truth(v) && !truth(&old["done"][k.as_str()])).map(|(k, _)| format!("{cid}/{k}")).collect::<Vec<_>>();
                let mut steps = 0;
                let (mut right, mut asked) = (0, 0);
                for (lid, q) in b["quiz"].as_object().into_iter().flatten() {
                    if old["quiz"][lid.as_str()] != *q && q["total"].as_u64().is_some_and(|t| t > 0) {
                        steps += 1;
                        right += q["score"].as_u64().unwrap_or(0);
                        asked += q["total"].as_u64().unwrap_or(0);
                    }
                }
                c["state"] = st;
                self.save(&c)?;
                self.log_activity(steps, &done, right, asked);
                Ok(json!({}))
            }
            ("POST", "/notes") => self.rewrite_notes(cid, b),
            ("POST", "/blueprint") => {
                let mut c = self.load(cid)?;
                let outline = lessons(&c).map(|l| format!("{}: {}",text(&l["title"]),text(&l["goal"]))).collect::<Vec<_>>().join("\n");
                let Some(plan) = b.get("blueprint") else {
                    return self.blueprint(&format!("{}\nExisting course: {outline}",text(&c["prompt"])),&string(&c["level"]),"","");
                };
                if c.get("blueprint").is_some() { return Err(Error::bad("This course already has an approved blueprint.")); }
                let plan = crate::ai::clean_blueprint(plan)?;
                let original = c["phases"].clone();
                c["blueprint"] = plan.clone();
                self.audit_milestones(&mut c)?;
                let _guard = self.guard();
                let mut latest = self.load(cid)?;
                if latest.get("blueprint").is_some() || latest["phases"] != original {
                    return Err(Error::bad("The course changed during the audit. Retry with the updated lessons."));
                }
                latest["blueprint"] = plan;
                latest["phases"] = c["phases"].take();
                self.save(&latest)?;
                Ok(latest)
            }
            ("POST", "/milestone") => self.milestone(cid,b),
            ("POST", "/walkthrough") => self.learner_walkthrough(cid, &string(&b["additional"])),
            ("POST", "/adapt") => self.adapt(cid),
            ("POST", "/revise") => {
                let ask = cap(b.get("ask").map(text).unwrap_or_default().trim(), 600);
                if ask.is_empty() {
                    return Err(Error::bad("Say what you want to change first."));
                }
                self.revise_options(cid, &ask)
            }
            ("POST", "/edit") => self.lesson_options(cid, &string(&b["lesson"]), &string(&b["action"])),
            ("POST", "/apply") => {
                if !b["plan"].is_object() {
                    return Err(Error::bad("Choose one of the options first."));
                }
                let ask = cap(b.get("ask").map(text).unwrap_or_default().trim(), 600);
                self.apply_plan(cid, &b["plan"], &ask, true)
            }
            ("PUT", "/prefs") => {
                let refresh = truth(&b["refresh"]);
                {
                    let _guard = self.guard();
                    let mut c = self.load(cid)?;
                    c["prefs"] = clean_prefs(b);
                    // New preferences: unfinished lessons get their videos picked again; the learner's own searches stay.
                    if refresh {
                        let done = c["state"]["done"].clone();
                        for p in c["phases"].as_array_mut().into_iter().flatten() {
                            for l in p["lessons"].as_array_mut().into_iter().flatten() {
                                if truth(&done[string(&l["id"])]) {
                                    continue;
                                }
                                let own = list(&l["videos"]).iter().filter(|v| truth(&v["searched"])).cloned().collect::<Vec<_>>();
                                if own.is_empty() {
                                    if let Some(o) = l.as_object_mut() {
                                        o.remove("videos");
                                        o.remove("novideos");
                                    }
                                }
                            }
                        }
                    }
                    self.save(&c)?;
                }
                if refresh {
                    self.prefetch(cid.to_string());
                }
                Ok(json!({}))
            }
            ("POST", "/expand") => self.expand(cid, &b["lesson"]),
            ("POST", "/export") => self.export_course(&self.load(cid)?, truth(&b["template"])),
            ("POST", "/summary") => self.summary(&self.load(cid)?),
            ("POST", "/setup") => {
                let c = self.load(cid)?;
                // Only a program this roadmap's list names can be installed.
                if let Some(cmd) = b["install"].as_str() {
                    if !list(&c["needs"]).iter().any(|n| n["command"] == cmd) {
                        return Err(Error::missing());
                    }
                    return install_tool(cmd);
                }
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
            ("POST", "/video-names") => self.name_videos(cid, &b["lesson"]),
            ("POST", "/videos") => {
                let c = self.load(cid)?;
                let l = lesson(&c, &b["lesson"])?;
                let old = list(&l["videos"]);
                if truth(&b["plan"]) && !old.is_empty() {
                    let mut planned = self.check_videos(&c, l, old.iter().take(8).cloned().collect(), &self.video_prefs(&c))?;
                    if planned.is_empty() { return Err(Error::bad("The available evidence did not support a viewing plan for these videos.")); }
                    for v in old {
                        if !planned.iter().any(|p| p["id"] == v["id"]) {
                            let mut reference = v.clone();
                            reference["reference"] = json!(true);
                            planned.push(reference);
                        }
                    }
                    let _guard = self.guard();
                    let mut latest = self.load(cid)?;
                    let lesson = lesson_mut(&mut latest, &string(&l["id"]))?;
                    if list(&lesson["videos"]) != old { return Err(Error::bad("The video list changed. Reopen the lesson and try again.")); }
                    lesson["videos"] = json!(planned);
                    self.save(&latest)?;
                    return Ok(json!(planned));
                }
                if let Some(query) = b["query"].as_str() {
                    let query = cap(query.trim(), 200);
                    if query.is_empty() { return Err(Error::bad("Enter a video search query.")); }
                    let found = self.query_videos(&c, &query, &old.iter().map(|v| string(&v["id"])).collect::<Vec<_>>())?;
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
                    self.fetch_transcripts(videos.clone());
                    return Ok(json!(videos));
                }
                let fresh = truth(&b["fresh"]);
                let lid = l["id"].clone();
                // This lesson's search may be running already (the background search, or an earlier open the learner left):
                // wait for it and use what it found instead of starting again. The claim keeps a later open waiting for this one.
                let mut c = c;
                let mut _claim = None;
                while !fresh && { let l = lesson(&c, &lid)?; list(&l["videos"]).is_empty() && !truth(&l["novideos"]) } {
                    if let Some(f) = self.claim_lesson(cid, &string(&lid)) { _claim = Some(f); break; }
                    self.wait_lesson(cid, &string(&lid));
                    c = self.load(cid)?;
                }
                let l = lesson(&c, &lid)?;
                let old = list(&l["videos"]);
                // An earlier search found nothing that fits; only "Search again" (fresh) tries once more.
                if !fresh && old.is_empty() && truth(&l["novideos"]) {
                    return Ok(json!([]));
                }
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
                if !vids.is_empty() && fresh || old.is_empty() {
                    let _guard = self.guard();
                    let mut c = self.load(cid)?;
                    set_videos(lesson_mut(&mut c, &string(&l["id"]))?, vids.clone());
                    self.save(&c)?;
                }
                self.fetch_transcripts(vids.clone());
                Ok(json!(if vids.is_empty() { old.to_vec() } else { vids }))
            }
            ("POST", "/exam") => {
                let stage = b["stage"].as_u64().ok_or_else(|| Error::bad("Choose a stage."))?;
                self.build_exam(cid, stage)
            }
            ("POST", "/practice") => {
                let c = self.load(cid)?;
                let l = lesson(&c, &b["lesson"])?;
                // A bank holding only stage exam questions still needs the lesson's own quiz.
                let own = |p: &Value| list(&p["quiz"]).iter().any(|q| q["exam"] != true);
                let fresh = truth(&b["fresh"]) || (truth(&l["practice"]) && !own(&l["practice"]));
                if truth(&l["practice"]) && !fresh {
                    return Ok(l["practice"].clone());
                }
                if list(&l["practice"]["quiz"]).len() >= 200 {
                    return Err(Error::bad("This lesson already has 200 questions. Try the ones you have again."));
                }
                let mut pr = self.build_practice(&c, l)?;
                let _guard = self.guard();
                let mut c = self.load(cid)?;
                let slot = lesson_mut(&mut c, &string(&l["id"]))?;
                let old = slot["practice"].clone();
                if truth(&old) {
                    if !fresh {
                        return Ok(old);
                    }
                    // "New questions" adds to the lesson's question bank and never replaces it, so review dates,
                    // the mistakes notebook and the learner's exercise code stay valid. "from" is where the newest set starts.
                    let mut quiz = list(&old["quiz"]).to_vec();
                    let from = quiz.len();
                    for q in list(&pr["quiz"]) {
                        if !quiz.iter().any(|o| o["q"] == q["q"]) {
                            quiz.push(q.clone());
                        }
                    }
                    if quiz.len() == from {
                        return Err(Error::bad("The AI repeated questions you already have. Try again."));
                    }
                    let exercise = if truth(&old["exercise"]) { old["exercise"].clone() } else { pr["exercise"].clone() };
                    let recall = if truth(&old["recall"]) { old["recall"].clone() } else { pr["recall"].clone() };
                    pr = json!({"quiz":quiz,"from":from,"exercise":exercise});
                    if truth(&recall) {
                        pr["recall"] = recall;
                    }
                }
                slot["practice"] = pr.clone();
                self.save(&c)?;
                Ok(pr)
            }
            _ => Err(Error::missing()),
        }
    }
    pub fn chat_stream(&self, body: &Value, on_piece: &mut dyn FnMut(&str) -> Result<()>) -> Result<String> {
        self.chat_stream_with(body, on_piece, &mut |_, _| {})
    }
    // on_status hears what the tutor is doing between pieces; with `true` it is a finished step, kept with the reply.
    // "api/chat/stop" ends the reply early.
    pub fn chat_stream_with(
        &self,
        body: &Value,
        on_piece: &mut dyn FnMut(&str) -> Result<()>,
        on_status: &mut dyn FnMut(&str, bool),
    ) -> Result<String> {
        let (c, l, msgs) = self.chat_args(body)?;
        self.stop_chat.store(false, Ordering::Relaxed);
        let mut pieces = String::new();
        let mut started = false;
        let mut steps = vec![];
        let on_status = &mut |s: &str, done: bool| {
            if done {
                steps.push(s.to_owned());
            }
            on_status(s, done)
        };
        let result = {
            let mut out = |piece: &str| {
                if self.stop_chat.load(Ordering::Relaxed) {
                    return Err(Error::stopped());
                }
                on_piece(piece)?;
                started = true;
                pieces.push_str(piece);
                Ok(())
            };
            crate::ai::metering((string(&c["id"]), "tutor"), || self.with_effort(&body["effort"], || self.thinking(&cap(&string(&body["thinking"]), 64), || {
                self.tutor(&c, &merge(l.clone(), json!({"watching":body["video"]})), msgs.clone(), &mut Some(&mut out), on_status)
            })))
        };
        match result {
            // A stopped reply keeps what had arrived and adds no note; the page already knows.
            Err(e) if e.status == 499 => {
                if started {
                    self.save_chat(&string(&c["id"]), &string(&l["id"]), &msgs, &pieces, &steps);
                }
                Err(e)
            }
            Ok(text) => {
                self.save_chat(&string(&c["id"]), &string(&l["id"]), &msgs, &text, &steps);
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
/// Claude Code's sign-in, when it is installed and can say: "ready" or "signed_out", and the email it uses.
fn local_auth(out: &mut Value) {
    if let Some((signed_in, email)) = crate::local_ai::claude_auth() {
        out["claude_auth"] = json!(if signed_in { "ready" } else { "signed_out" });
        out["claude_email"] = json!(email);
    }
}
