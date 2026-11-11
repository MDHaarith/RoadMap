use super::*;
use crate::exercise::{clean_practice, run_code};
use std::{
    io::Write,
    os::unix::fs::PermissionsExt,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

fn course() -> Value {
    json!({"title":"Bake bread","summary":"From flour to loaf.","phases":[{"title":"Dough","blurb":"Start here.","lessons":[{"title":"Mix","goal":"Mix a dough.","concepts":["hydration"],"tasks":["Mix 500 g flour"],"done":["Dough holds together"],"query":"how to mix bread dough","lookups":["bakers percentage chart"]},"not a lesson"]},{"title":"Empty phase","lessons":[]}]})
}
// Same exercise outcomes as the original tests, using Bash so the test suite needs no Python runtime.
const STARTER: &str = "double() { :; }\n";
const GOOD: &str = "double() { echo $(( $1 * 2 )); }";
const WRONG: &str = "double() { echo $(( $1 + 1 )); }";
fn practice() -> Value {
    json!({"quiz":[{"q":"What is hydration?","options":["Water to flour ratio","Oven heat"],"answer":0,"why":"It is a ratio."},{"q":"bad answer index","options":["a","b"],"answer":7}],"exercise":{"lang":"bash","title":"Double","prompt":"Write double(x).","starter":STARTER,"tests":"source ./main.sh\ngot=$(double 2)\nif [ \"$got\" != 4 ]; then\n echo \"double(2): expected 4, got $got\"; exit 1\nfi\necho 'All tests passed'"}})
}
fn found() -> Value {
    json!({"entries":[{"id":"aaaaaaaaaaa","title":"Full lesson","channel":"Baker","duration":900.0,"view_count":1200,"description":"Flour and\nwater"},{"id":"bbbbbbbbbbb","title":"A Short","duration":40},{"id":"ccccccccccc","title":"live now","duration":900,"live_status":"is_live"},{"id":"<script>","title":"bad id","duration":900}],"chapters":[{"start_time":0.0,"title":"Intro"},{"start_time":75.5,"title":"Mixing"},{"title":"no time"}]})
}
struct FakeAI {
    base: String,
    seen: Arc<Mutex<Vec<Value>>>,
    reject: Arc<AtomicBool>,
    server: Arc<tiny_http::Server>,
    worker: Option<thread::JoinHandle<()>>,
}
impl FakeAI {
    fn new() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!(
            "http://127.0.0.1:{}/v1/",
            listener.local_addr().unwrap().port()
        );
        let server = Arc::new(tiny_http::Server::from_listener(listener, None).unwrap());
        let s = server.clone();
        let seen = Arc::new(Mutex::new(vec![]));
        let seen2 = seen.clone();
        let reject = Arc::new(AtomicBool::new(false));
        let reject2 = reject.clone();
        let worker = thread::spawn(move || {
            while let Ok(mut req) = s.recv() {
                let mut bytes = String::new();
                req.as_reader().read_to_string(&mut bytes).unwrap();
                let b: Value = serde_json::from_str(&bytes).unwrap();
                assert_eq!(
                    req.headers()
                        .iter()
                        .find(|h| h.field.equiv("Authorization"))
                        .unwrap()
                        .value
                        .as_str(),
                    "Bearer sk-test"
                );
                if reject2.load(Ordering::Relaxed)
                    && (b.get("response_format").is_some() || b.get("tools").is_some())
                {
                    req.respond(tiny_http::Response::from_string("{}").with_status_code(400))
                        .unwrap();
                    continue;
                }
                seen2.lock().unwrap().push(b.clone());
                let system = b["messages"][0]["content"].as_str().unwrap();
                let user = string(&b["messages"][1]["content"]);
                let mut msg = None;
                assert!(
                    system.contains("JSON object")
                        || system.contains("1.1 Mix (completed)")
                            && system.contains("[x] Dough holds together"),
                    "{system}"
                );
                let text = if system.contains("placement interview inside") {
                    assert!(
                        reject2.load(Ordering::Relaxed)
                            || b["response_format"]["json_schema"]["schema"]["properties"]["done"]
                                .is_object()
                    );
                    if user.contains("Used an oven? My answer: Yes") {
                        json!({"done":true,"q":"","options":[]})
                    } else if user.contains("Baked before? My answer: No, never") {
                        json!({"done":false,"q":"Used an oven?","options":["No, never","Yes"]})
                    } else {
                        json!({"done":false,"q":"Baked before?","options":["No, never","Yes",3]})
                    }
                    .to_string()
                } else if system.contains("choose the videos") {
                    assert!(
                        user.contains(
                            "0. Full lesson | Baker | 15 min | 1200 views | Flour and water"
                        ) && !user.contains("A Short")
                            && !user.contains("live now"),
                        "{user}"
                    );
                    json!({"picks":[{"i":0,"why":"Covers hydration."},{"i":99,"why":"made up"},"junk"],"query":""}).to_string()
                } else if system.contains("mark one review answer") {
                    assert!(user.contains("Concept to explain: hydration"));
                    json!({"right":user.split("answer:").nth(1).unwrap().contains("water"),"feedback":"It is the water to flour ratio."}).to_string()
                } else if system.contains("You designed this course earlier") {
                    assert!(
                        user.contains("- 1.1 Mix [completed, quiz 1 of 1")
                            && user.contains("exercise passed after 2 test runs"),
                        "{user}"
                    );
                    let new = json!({"title":"Mix again","goal":"g","concepts":["hydration by feel"],"tasks":["t"],"done":["d"],"query":"q","lookups":[],"warn":""});
                    json!({"note":"Added practice.","drop":["1.1","7.7"],"add":[{"after":"1.1","lesson":new},{"after":"9.9","lesson":new},{"after":"1.1"}]}).to_string()
                } else if system.contains("practice material") {
                    practice().to_string()
                } else if system.contains("existing, working project") {
                    assert!(
                        user.contains("--- app.py ---")
                            && user.contains("print('hi')")
                            && !user.contains("TOKEN")
                            && !user.contains(".git")
                            && !user.contains("node_modules")
                    );
                    json!({"title":"Understand demo","phases":[{"title":"Run it","lessons":[{"title":"Entry","files":["app.py",".env","../../etc/passwd","made_up.py"]}]}]}).to_string()
                } else if system.contains("You plan a long course") && user.contains("--- app.py ---") {
                    assert!(user.contains("Size: about three months") && user.contains("Never pad it"), "{user}");
                    json!({"title":"Demo","stages":[{"title":"S","phases":[{"title":"P","lessons":[{"title":"Entry","files":["app.py","fake.py"]}]},{"title":"Later","lessons":[{"title":"Later"}]}]}]}).to_string()
                } else if system.contains("You plan a long course") {
                    assert!(user.contains("Size: about six months"), "{user}");
                    json!({"title":"Bake bread","summary":"s","stages":[{"title":"Basics","blurb":"Ends with a loaf.","phases":[{"title":"Dough","blurb":"b","lessons":[{"title":"Mix","goal":"Mix a dough."},{"title":"Knead","goal":"Knead it."}]}]},"junk",{"title":"","blurb":"","phases":[{"title":"Bake","blurb":"b","lessons":[{"title":"Heat","goal":"Heat it."}]}]}]}).to_string()
                } else if system.contains("You write the lessons for one phase") {
                    assert!(
                        user.contains("outline only]. Goal: ")
                            && user.contains("Write the lessons of this phase: "),
                        "{user}"
                    );
                    json!({"lessons":[{"title":"","goal":"","concepts":["gluten"],"tasks":["t"],"done":["d"],"query":"how to mix bread dough","lookups":["l"],"warn":""},{"title":"No checks","goal":"g","concepts":[],"tasks":["t"],"done":[],"query":"q","lookups":[],"warn":""}]}).to_string()
                } else if system.contains("has asked you to change it") {
                    assert!(
                        user.contains("My request: industry level")
                            && user.contains("- 1.1 Mix [completed"),
                        "{user}"
                    );
                    json!({"note":"Now industry.","drop":["1.1","2.1"],"add":[],"phases":[{"after":"1.2","title":"In industry","blurb":"b","lessons":[{"title":"Real ovens","goal":"g"},"junk"]},{"after":"","title":"Empty","blurb":"","lessons":[]}]}).to_string()
                } else if system.contains("JSON object") {
                    format!("Here you go:\n```json\n{}\n```", course())
                } else {
                    assert!(
                        system.contains("bakers percentage chart")
                            && system.contains(GOOD)
                            && system.contains("Last quiz score: 1 of 1")
                    );
                    let text = format!(
                        "hint for: {}",
                        cap(system.split("Current lesson ").nth(1).unwrap(), 8)
                    );
                    let names = list(&b["tools"])
                        .iter()
                        .map(|t| string(&t["function"]["name"]))
                        .collect::<Vec<_>>();
                    if !names.is_empty()
                        && b["messages"].as_array().unwrap().last().unwrap()["role"] == "user"
                    {
                        assert_eq!(names, vec!["get_lesson", "run_exercise_tests", "read_video_transcript"]);
                        let calls = [("run_exercise_tests", "{}"), ("get_lesson", "{\"id\":\"1.1\"}"), ("rm_rf", "{}"), ("get_lesson", "not json")]
                            .iter().enumerate().map(|(i,(n,a))| json!({"id":format!("c{i}"),"type":"function","function":{"name":n,"arguments":a}})).collect::<Vec<_>>();
                        msg = Some(json!({"content":null,"tool_calls":calls}));
                    }
                    text
                };
                if truth(&b["stream"]) && msg.is_none() {
                    let mut writer = req.into_writer();
                    writer.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").unwrap();
                    for part in [&text[..5], &text[5..]] {
                        let data = format!(
                            "data: {}\n\n",
                            json!({"choices":[{"delta":{"content":part}}]})
                        );
                        write!(writer, "{:x}\r\n{data}\r\n", data.len()).unwrap();
                        writer.flush().unwrap();
                    }
                    writer
                        .write_all(b"e\r\ndata: [DONE]\n\n\r\n0\r\n\r\n")
                        .unwrap();
                    writer.flush().unwrap();
                } else {
                    req.respond(
                        tiny_http::Response::from_string(
                            json!({"choices":[{"message":msg.unwrap_or(json!({"content":text}))}]})
                                .to_string(),
                        )
                        .with_header(
                            tiny_http::Header::from_bytes("Content-Type", "application/json")
                                .unwrap(),
                        ),
                    )
                    .unwrap();
                }
            }
        });
        Self {
            base,
            seen,
            reject,
            server,
            worker: Some(worker),
        }
    }
    fn last(&self) -> Value {
        self.seen.lock().unwrap().last().unwrap().clone()
    }
}
impl Drop for FakeAI {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(w) = self.worker.take() {
            w.join().unwrap();
        }
    }
}
fn call(app: &Arc<App>, method: &str, path: &str, body: Value) -> (u16, Value) {
    match request_body(body).and_then(|body| app.route(method, path, &body)) {
        Ok(value) => (200, value),
        Err(e) => (e.status, json!({"error":e.message})),
    }
}
fn wait_videos(base: &Arc<App>, cid: &str) {
    let start = Instant::now();
    loop {
        let c = call(base, "GET", &format!("api/courses/{cid}"), json!({})).1;
        if truth(&c["phases"][0]["lessons"][0]["videos"]) {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "videos not prefetched: {c}"
        );
        thread::sleep(Duration::from_millis(20));
    }
}
#[test]
fn python_spec_end_to_end() {
    let tmp = tempfile::tempdir().unwrap();
    let proj = tmp.path().join("demo");
    fs::create_dir_all(proj.join("node_modules/x")).unwrap();
    fs::create_dir_all(proj.join(".git")).unwrap();
    for (n, s) in [
        ("app.py", "print('hi')"),
        (".env", "TOKEN=1"),
        ("node_modules/x/i.js", "x"),
        (".git/config", "x"),
        ("README.md", "Demo"),
    ] {
        fs::write(proj.join(n), s).unwrap();
    }
    let export = tmp.path().join("Bake-bread.roadmap.json");
    let project = proj.to_string_lossy().into_owned();
    let export_s = export.to_string_lossy().into_owned();
    let pick_project = project.clone();
    let app = App {
        home: tmp.path().into(),
        data: tmp.path().join("data/roadmap-app"),
        conf: tmp.path().join("config/roadmap-app/config.json"),
        lock: Mutex::new(()),
        off: Mutex::new(HashSet::new()),
        meter: Mutex::new(()),
        fetching: Mutex::new(std::collections::HashSet::new()),
        day_offset: AtomicI64::new(0),
        pick: Box::new(move |k| {
            if k == "file" {
                export_s.clone()
            } else {
                pick_project.clone()
            }
        }),
        yt_program: tmp.path().join("yt-dlp"),
    };
    fs::write(
        &app.yt_program,
        format!("#!/bin/sh\ncat <<'FIXTURE'\n{}\nFIXTURE\n", found()),
    )
    .unwrap();
    fs::set_permissions(&app.yt_program, fs::Permissions::from_mode(0o755)).unwrap();
    let app = Arc::new(app);
    let ai = FakeAI::new();
    let base = app.clone();
    assert_eq!(
        call(&base, "PUT", "api/theme", json!({"theme":"\"><script>"})).0,
        400
    );
    assert_eq!(
        call(&base, "PUT", "api/theme", json!({"theme":"dark"})).0,
        200
    );
    assert_eq!(call(&base, "GET", "api/settings", Value::Null).1["theme"], "dark");
    assert_eq!(
        fs::metadata(&app.conf).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        call(&base, "POST", "api/courses", json!({"prompt":"bread"})).0,
        400
    );
    assert_eq!(
        call(
            &base,
            "PUT",
            "api/settings",
            json!({"base":"file:///etc/passwd","model":"m"})
        )
        .0,
        400
    );
    assert_eq!(
        call(
            &base,
            "PUT",
            "api/settings",
            json!({"base":ai.base.trim_end_matches('/'),"model":"m","key":"sk-test"})
        )
        .0,
        200
    );
    assert_eq!(
        call(
            &base,
            "PUT",
            "api/settings",
            json!({"base":ai.base,"model":"m"})
        )
        .0,
        200
    );
    assert_eq!(
        call(&base, "GET", "api/settings", json!({})).1,
        json!({"base":ai.base,"model":"m","small":"","usage":{},"key_set":true,"theme":"dark"})
    );
    assert_eq!(
        fs::metadata(&app.conf).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(call(&base, "POST", "api/placement", json!({})).0, 400);
    let first = call(
        &base,
        "POST",
        "api/placement",
        json!({"prompt":"bake bread"}),
    )
    .1;
    assert_eq!(
        first,
        json!({"done":false,"q":"Baked before?","options":["No, never","Yes","3"]})
    );
    assert!(
        string(&ai.last()["messages"][1]["content"]).contains("You have not asked anything yet.")
    );
    let told = json!([{"q":"Baked before?","a":"No, never"},"junk"]);
    assert_eq!(
        call(
            &base,
            "POST",
            "api/placement",
            json!({"prompt":"bake bread","answers":told})
        )
        .1["q"],
        "Used an oven?"
    );
    assert_eq!(call(&base,"POST","api/placement",json!({"prompt":"bake bread","answers":[{"q":"Baked before?","a":"No, never"},"junk",{"q":"Used an oven?","a":"Yes"}]})).1,json!({"done":true}));
    let n = ai.seen.lock().unwrap().len();
    assert_eq!(
        call(
            &base,
            "POST",
            "api/placement",
            json!({"prompt":"bake bread","answers":vec![json!({"q":"x","a":"y"});7]})
        )
        .1,
        json!({"done":true})
    );
    assert_eq!(ai.seen.lock().unwrap().len(), n);
    let (status, c) = call(
        &base,
        "POST",
        "api/courses",
        json!({"prompt":"bake bread","level":"Complete beginner","answers":told}),
    );
    assert_eq!(status, 200, "{c}");
    assert_eq!(c["title"], "Bake bread");
    assert_eq!(
        lessons(&c).map(|l| string(&l["id"])).collect::<Vec<_>>(),
        ["1.1"]
    );
    assert_eq!(list(&c["phases"]).len(), 1);
    assert_eq!(
        c["phases"][0]["lessons"][0]["lookups"],
        json!(["bakers percentage chart"])
    );
    // Prefetch also calls the AI, so select the actual designer request rather than racing its last request.
    let seen = ai.seen.lock().unwrap().clone();
    let designer = seen
        .iter()
        .find(|r| {
            string(&r["messages"][0]["content"]).contains("You design the whole course")
                && string(&r["messages"][1]["content"]).contains("Placement check:")
        })
        .unwrap();
    assert!(string(&designer["messages"][1]["content"])
        .contains("Placement check:\n- Baked before? My answer: No, never"));
    assert_eq!(designer["response_format"]["json_schema"]["strict"], true);
    assert!(designer["response_format"].to_string().contains("lookups"));
    let cid = string(&c["id"]);
    let course_path = format!("api/courses/{cid}");
    wait_videos(&base, &cid);
    let (status, vids) = call(
        &base,
        "POST",
        &format!("{course_path}/videos"),
        json!({"lesson":"1.1"}),
    );
    assert_eq!(status, 200);
    assert_eq!(vids[0]["id"], "aaaaaaaaaaa");
    assert_eq!(list(&vids).len(), 1);
    assert_eq!(vids[0]["why"], "Covers hydration.");
    assert_eq!(
        vids[0]["chapters"],
        json!([{"t":0,"title":"Intro"},{"t":75,"title":"Mixing"}])
    );
    assert_eq!(
        call(
            &base,
            "POST",
            &format!("{course_path}/videos"),
            json!({"lesson":"1.1","fresh":true})
        )
        .1,
        vids
    );
    assert_eq!(
        call(
            &base,
            "POST",
            "api/run",
            json!({"course":cid,"lesson":"1.1","code":""})
        )
        .0,
        404
    );
    let (status, pr) = call(
        &base,
        "POST",
        &format!("{course_path}/practice"),
        json!({"lesson":"1.1"}),
    );
    assert_eq!(status, 200);
    assert_eq!(list(&pr["quiz"]).len(), 1);
    assert_eq!(pr["exercise"]["lang"], "bash");
    for (code, expected) in [
        (
            WRONG,
            json!({"ok":false,"output":"double(2): expected 4, got 3\n"}),
        ),
        (GOOD, json!({"ok":true,"output":"All tests passed\n"})),
    ] {
        assert_eq!(
            call(
                &base,
                "POST",
                "api/run",
                json!({"course":cid,"lesson":"1.1","code":code,"test":true})
            )
            .1,
            expected
        );
    }
    let port = ai
        .base
        .split(':')
        .nth(2)
        .unwrap()
        .split('/')
        .next()
        .unwrap();
    let probe=format!("[ -e '{}' ] && echo leak || printf 'False '; if [ -z \"$(ls -A /home)\" ]; then echo '[]'; else echo leak; fi\nif (echo > /dev/tcp/127.0.0.1/{port}) 2>/dev/null; then echo net; else echo 'no net'; fi",app.conf.display());
    assert_eq!(
        call(
            &base,
            "POST",
            "api/run",
            json!({"course":cid,"lesson":"1.1","code":probe})
        )
        .1,
        json!({"ok":true,"output":"False []\nno net\n"})
    );
    assert_eq!(call(&base, "GET", "api/review", json!({})).1["count"], 0);
    let (status, pc) = call(
        &base,
        "POST",
        "api/courses",
        json!({"folder":project,"level":"I code already"}),
    );
    assert_eq!(status, 200, "{pc}");
    assert_eq!(pc["folder"], project);
    assert_eq!(pc["phases"][0]["lessons"][0]["files"], json!(["app.py"]));
    assert_eq!(
        call(
            &base,
            "POST",
            "api/file",
            json!({"course":pc["id"],"path":"app.py"})
        )
        .1,
        json!({"text":"print('hi')"})
    );
    for p in [
        ".env",
        "../../etc/passwd",
        "/etc/passwd",
        ".git/config",
        "nope.py",
    ] {
        assert_eq!(
            call(
                &base,
                "POST",
                "api/file",
                json!({"course":pc["id"],"path":p})
            )
            .0,
            404
        );
    }
    assert_eq!(
        call(&base, "POST", "api/courses", json!({"folder":app.home})).0,
        400
    );
    wait_videos(&base, pc["id"].as_str().unwrap());
    assert_eq!(
        call(
            &base,
            "DELETE",
            &format!("api/courses/{}", text(&pc["id"])),
            json!({})
        )
        .0,
        200
    );
    assert_eq!(
        call(&base, "POST", "api/install", json!({"lang":"bash"})).1,
        json!({"ok":true,"output":"Bash is already installed."})
    );
    assert_eq!(
        call(
            &base,
            "POST",
            "api/install",
            json!({"lang":"gcc; rm -rf ~"})
        )
        .0,
        404
    );
    // Probe the missing-runtime branch without assuming which optional interpreters are installed.
    if let Some((lang, _, _)) = crate::exercise::LANGS
        .iter()
        .find(|(_, p, _)| !std::path::Path::new("/usr/bin").join(p[2]).exists())
    {
        assert_eq!(run_code(lang, "", None).unwrap()["missing"], *lang);
    }
    let state = json!({"checks":{"1.1#0":true},"done":{"1.1":true},"notes":{"1.1":"sticky"},"last":"1.1","quiz":{"1.1":{"score":1,"total":1}},"code":{"1.1":GOOD}});
    assert_eq!(
        call(&base, "PUT", &format!("{course_path}/state"), state.clone()).0,
        200
    );
    assert_eq!(
        call(
            &base,
            "PUT",
            &format!("{course_path}/state"),
            json!({"checks":[]})
        )
        .0,
        400
    );
    assert_eq!(
        app.learned("", 120).unwrap(),
        ["Concepts already learned in their other roadmaps: hydration (from Bake bread)"]
    );
    assert!(app.learned(&cid, 120).unwrap().is_empty());
    assert_eq!(call(&base, "GET", "api/review", json!({})).1["count"], 0);
    app.day_offset.store(1, Ordering::Relaxed);
    let due = call(&base, "GET", "api/review", json!({})).1;
    assert_eq!(due["count"], 1);
    assert_eq!(due["questions"][0]["q"], "What is hydration?");
    assert_eq!(due["questions"][0]["roadmap"], "Bake bread");
    assert_eq!(
        call(
            &base,
            "POST",
            "api/review",
            json!({"course":cid,"lesson":"1.1","i":0,"right":true})
        )
        .1,
        json!({"step":0,"due":app.today()+1})
    );
    assert_eq!(call(&base, "GET", "api/review", json!({})).1["count"], 0);
    app.day_offset.store(0, Ordering::Relaxed);
    let mut twin = json!({"id":"0123456789ab","title":"Pizza","summary":"","prompt":"pizza","phases":[{"title":"P","blurb":"","lessons":[{"id":"1.1","title":"Dough","goal":"","concepts":["hydration of dough","hydration","Yeast"],"tasks":[],"done":[],"query":"q"}]}],"state":{"checks":{},"done":{},"notes":{}}});
    app.save(&twin).unwrap();
    assert_eq!(
        call(&base, "GET", "api/courses/0123456789ab", json!({})).1["phases"][0]["lessons"][0]
            ["known"],
        json!({})
    );
    twin["phases"][0]["lessons"][0]["concepts"][0] = json!("Hydration: water to flour");
    app.save(&twin).unwrap();
    let mut source = app.load(&cid).unwrap();
    source["phases"][0]["lessons"][0]["concepts"] =
        json!(["hydration: the ratio of water to flour"]);
    {
        let _guard = app.lock.lock().unwrap();
        app.save(&source).unwrap();
    }
    assert_eq!(
        call(&base, "GET", "api/courses/0123456789ab", json!({})).1["phases"][0]["lessons"][0]
            ["known"],
        json!({"0":"Bake bread"})
    );
    source["phases"][0]["lessons"][0]["concepts"] = json!(["hydration"]);
    {
        let _guard = app.lock.lock().unwrap();
        app.save(&source).unwrap();
    }
    fs::remove_file(app.path_of("0123456789ab").unwrap()).unwrap();
    assert_eq!(
        call(&base, "PUT", &format!("{course_path}/state"), state.clone()).0,
        200
    );
    let mut state = state;
    state["review"] = json!({"1.1#0":{"step":0,"due":app.today()+2},"1.1#explain":{"step":-1,"due":app.today()+2},"1.1#code":{"step":-1,"due":app.today()+4}});
    state["tests"] = json!({"1.1":{"runs":2,"ok":true}});
    assert_eq!(
        call(
            &base,
            "PUT",
            &format!("{course_path}/paused"),
            json!({"paused":true})
        )
        .0,
        200
    );
    assert_eq!(
        call(&base, "GET", "api/courses", json!({})).1[0]["paused"],
        true
    );
    assert_eq!(
        call(
            &base,
            "PUT",
            &format!("{course_path}/paused"),
            json!({"paused":false})
        )
        .0,
        200
    );
    let saved = call(&base, "GET", &course_path, json!({})).1;
    assert_eq!(saved["state"], state);
    assert_eq!(saved["phases"][0]["lessons"][0]["videos"], vids);
    assert_eq!(saved["phases"][0]["lessons"][0]["practice"], pr);
    assert_eq!(saved["phases"][0]["lessons"][0]["known"], json!({}));
    assert_eq!(
        call(&base, "GET", "api/courses", json!({})).1,
        json!([{"id":cid,"title":"Bake bread","summary":"From flour to loaf.","total":1,"done":1,"paused":false,"project":false,"next":null}])
    );
    let chat = json!({"course":cid,"lesson":"1.1","messages":[{"role":"user","content":"hint?"}]});
    assert_eq!(
        call(&base, "POST", "api/chat", chat.clone()).1,
        json!({"text":"hint for: 1.1: Mix"})
    );
    let last = ai.last();
    let results = list(&last["messages"])
        .iter()
        .filter(|m| m["role"] == "tool")
        .map(|m| string(&m["content"]))
        .collect::<Vec<_>>();
    assert_eq!(results.len(), 4);
    assert_eq!(results[0], "All tests passed\n");
    assert!(results[1].starts_with("Lesson 1.1: Mix"));
    assert_eq!(results[2], "That tool or argument does not exist.");
    assert_eq!(results[2], results[3]);
    let mut pieces = String::new();
    let answer = app.chat_stream(&chat, &mut |piece| { pieces.push_str(piece); Ok(()) }).unwrap();
    assert_eq!(answer, "hint for: 1.1: Mix");
    assert_eq!(pieces, answer);
    assert_eq!(ai.last()["stream"], true);
    assert_eq!(
        list(&ai.last()["messages"])
            .iter()
            .filter(|m| m["role"] == "tool")
            .count(),
        4
    );
    app.day_offset.store(4, Ordering::Relaxed);
    let due = call(&base, "GET", "api/review", json!({})).1;
    let qs = list(&due["questions"]);
    assert_eq!(qs.len(), 3);
    assert!(qs.iter().any(|q| q["i"] == 0));
    let explain = qs.iter().find(|q| q["i"] == "explain").unwrap();
    assert_eq!(explain["q"], "hydration");
    assert!(explain.get("answer").is_none());
    let code = qs.iter().find(|q| q["i"] == "code").unwrap();
    assert!(truth(&code["starter"]));
    assert!(code.get("tests").is_none());
    assert_eq!(
        call(
            &base,
            "POST",
            "api/review/answer",
            json!({"course":cid,"lesson":"1.1","i":"explain","answer":"no idea"})
        )
        .1,
        json!({"right":false,"feedback":"It is the water to flour ratio.","step":0,"due":app.today()+1})
    );
    assert_eq!(call(&base,"POST","api/review/answer",json!({"course":cid,"lesson":"1.1","i":"explain","answer":"how much water for the flour"})).1["step"],1);
    assert_eq!(
        call(
            &base,
            "POST",
            "api/review/answer",
            json!({"course":cid,"lesson":"1.1","i":0,"answer":"x"})
        )
        .0,
        404
    );
    assert_eq!(
        call(
            &base,
            "POST",
            "api/review/answer",
            json!({"course":cid,"lesson":"1.1","i":"code","answer":"double() { echo 0; }"})
        )
        .1["right"],
        false
    );
    assert_eq!(
        call(
            &base,
            "POST",
            "api/review/answer",
            json!({"course":cid,"lesson":"1.1","i":"code","answer":GOOD})
        )
        .1,
        json!({"right":true,"output":"All tests passed\n","step":1,"due":app.today()+3})
    );
    app.day_offset.store(0, Ordering::Relaxed);
    assert_eq!(
        call(&base, "POST", &format!("{course_path}/export"), json!({})).1["path"],
        export.to_string_lossy().as_ref()
    );
    let back = call(&base, "POST", "api/import", json!({})).1;
    let src = call(&base, "GET", &course_path, json!({})).1;
    assert_ne!(back["id"], cid);
    assert_eq!(back["state"], src["state"]);
    assert_eq!(back["phases"][0]["lessons"][0]["practice"], pr);
    assert_eq!(
        back["phases"][0]["lessons"][0]["videos"][0]["chapters"],
        vids[0]["chapters"]
    );
    let mut evil: Value = serde_json::from_slice(&fs::read(&export).unwrap()).unwrap();
    assert!(evil.get("folder").is_none());
    evil["phases"][0]["lessons"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"../../x","title":"bad id"}));
    evil["phases"][0]["lessons"][0]["videos"] = json!([{"id":"<script>"}]);
    evil["state"] = json!({"checks":{"1.1#0":"<img>","9.9#0":true},"done":{"1.1":true},"notes":{"1.1":5},"review":{"1.1#0":{"step":"x","due":1}},"quiz":[],"last":"<b>"});
    fs::write(&export, evil.to_string()).unwrap();
    let bad = call(&base, "POST", "api/import", json!({})).1;
    assert_eq!(list(&bad["phases"][0]["lessons"]).len(), 1);
    assert_eq!(bad["phases"][0]["lessons"][0]["videos"], json!([]));
    assert_eq!(
        bad["state"],
        json!({"checks":{},"done":{"1.1":true},"notes":{},"code":{},"quiz":{},"review":{},"tests":{},"last":"1.1"})
    );
    fs::write(&export, "not json").unwrap();
    assert_eq!(call(&base, "POST", "api/import", json!({})).0, 400);
    for x in [back, bad] {
        fs::remove_file(app.path_of(x["id"].as_str().unwrap()).unwrap()).unwrap();
    }
    assert_eq!(
        call(&base, "PUT", &format!("{course_path}/work"), json!({})).1,
        json!({"work":project})
    );
    let c = app.load(&cid).unwrap();
    let l = lesson(&c, &json!("1.1")).unwrap();
    assert_eq!(
        app.tool_result(&c, l, "list_project_files", &json!({}))
            .unwrap(),
        "README.md\napp.py"
    );
    assert_eq!(
        app.tool_result(&c, l, "read_project_file", &json!({"path":"app.py"}))
            .unwrap(),
        "print('hi')"
    );
    assert!(app
        .tool_result(&c, l, "read_project_file", &json!({"path":".env"}))
        .unwrap()
        .contains("No such file"));
    assert_eq!(
        call(
            &base,
            "PUT",
            &format!("{course_path}/work"),
            json!({"clear":true})
        )
        .1,
        json!({"work":""})
    );
    let c = app.load(&cid).unwrap();
    assert!(!app
        .tutor_tools(&c, lesson(&c, &json!("1.1")).unwrap())
        .iter()
        .any(|t| t["function"]["name"] == "read_project_file"));
    let (status, ad) = call(&base, "POST", &format!("{course_path}/adapt"), json!({}));
    assert_eq!(status, 200, "{ad}");
    assert_eq!(
        ad["change"],
        json!({"day":app.today(),"note":"Added practice.","added":["Mix again"],"dropped":[]})
    );
    assert_eq!(
        lessons(&ad["course"])
            .map(|l| (string(&l["id"]), string(&l["title"]), l["added"].clone()))
            .collect::<Vec<_>>(),
        vec![
            ("1.1".into(), "Mix".into(), Value::Null),
            ("1.2".into(), "Mix again".into(), json!(true))
        ]
    );
    assert_eq!(ad["course"]["changes"], json!([ad["change"]]));
    let start = Instant::now();
    while !truth(&app.load(&cid).unwrap()["phases"][0]["lessons"][1]["videos"]) {
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(20));
    }
    ai.reject.store(true, Ordering::Relaxed);
    assert_eq!(
        call(&base, "POST", "api/chat", chat).1,
        json!({"text":"hint for: 1.1: Mix"})
    );
    assert_eq!(
        call(
            &base,
            "POST",
            "api/placement",
            json!({"prompt":"bake bread"})
        )
        .0,
        200
    );
    assert!(app.off.lock().unwrap().contains("tools"));
    assert!(app.off.lock().unwrap().contains("response_format"));
    assert!(ai.last().get("tools").is_none() && ai.last().get("response_format").is_none());
    assert_eq!(
        call(
            &base,
            "POST",
            "api/chat",
            json!({"course":"../../etc/passwd","lesson":"1.1","messages":[]})
        )
        .0,
        404
    );
    assert_eq!(
        call(
            &base,
            "POST",
            "api/chat",
            json!({"course":cid,"lesson":"9.9","messages":[]})
        )
        .0,
        404
    );
    assert_eq!(
        call(&base, "GET", "api/courses/..%2Fconfig", json!({})).0,
        404
    );
    assert_eq!(call(&base, "DELETE", &course_path, json!({})).0, 200);
    assert_eq!(call(&base, "GET", &course_path, json!({})).0, 404);
    assert_eq!(call(&base, "GET", "api/courses", json!({})).1, json!([]));
}
#[test]
fn untrusted_values_and_python_exercise_format() {
    let mut pr = practice();
    pr["exercise"] = json!({"lang":"python","title":"Double","prompt":"Write double(x).","starter":"def double(x):\n    pass\n","tests":"import main\nassert main.double(2) == 4"});
    let cleaned = clean_practice(&pr);
    assert_eq!(cleaned["exercise"], pr["exercise"]);
    assert_eq!(list(&cleaned["quiz"]).len(), 1);
    assert!(parse("[]", "a course").is_err());
    assert!(parse("{broken}", "a course").is_err());
    assert_eq!(cap("é🍞abc", 2), "é🍞");
}

fn isolated_app(tmp: &std::path::Path) -> Arc<App> {
    let mut app = App::new(|_| String::new()).unwrap();
    let a = Arc::get_mut(&mut app).unwrap();
    a.home = tmp.into();
    a.data = tmp.join("data");
    a.conf = tmp.join("config.json");
    app
}
#[test]
fn sse_tool_fragments_stream_fallback_and_ai_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let app = isolated_app(tmp.path());
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let address = server.server_addr().to_string();
    app.save_config(&json!({"base":format!("http://{address}/v1"),"model":"m"}))
        .unwrap();
    let worker = thread::spawn(move || {
        for i in 0..5 {
            let mut req = server.recv().unwrap();
            let mut data = String::new();
            req.as_reader().read_to_string(&mut data).unwrap();
            let body: Value = serde_json::from_str(&data).unwrap();
            let (status, ct, response) = match i {
                0 => {
                    assert_eq!(body["stream"], true);
                    let deltas = [
                        json!({"content":"hi ","tool_calls":[{"index":1,"id":"b","function":{"name":"get_","arguments":"{\"id\":"}},{"index":0,"id":"a","function":{"name":"run_exercise_","arguments":"{"}}]}),
                        json!({"content":"there","tool_calls":[{"index":0,"function":{"name":"tests","arguments":"}"}},{"index":1,"function":{"name":"lesson","arguments":"\"1.1\"}"}}]}),
                    ];
                    let frames = deltas
                        .iter()
                        .map(|d| format!("data: {}\n\n", json!({"choices":[{"delta":d}]})))
                        .collect::<String>();
                    (
                        200,
                        "text/event-stream",
                        format!(": comment\n\n{frames}data: [DONE]\n\n"),
                    )
                }
                1 => {
                    assert_eq!(body["stream"], true);
                    assert!(body.get("response_format").is_some());
                    (422, "application/json", "unsupported".into())
                }
                2 => {
                    assert!(body.get("stream").is_none() && body.get("response_format").is_none());
                    (
                        200,
                        "application/json",
                        json!({"choices":[{"message":{"content":"fallback"}}]}).to_string(),
                    )
                }
                3 => (200, "application/json", json!({"choices":[]}).to_string()),
                _ => (503, "text/plain", "service unavailable".into()),
            };
            req.respond(
                tiny_http::Response::from_string(response)
                    .with_status_code(status)
                    .with_header(tiny_http::Header::from_bytes("Content-Type", ct).unwrap()),
            )
            .unwrap();
        }
    });
    let mut chunks = String::new();
    let m = app
        .ask(
            &[],
            &mut Some(&mut |s| {
                chunks.push_str(s);
                Ok(())
            }),
            json!({"tools":[{}]}),
        )
        .unwrap();
    assert_eq!(chunks, "hi there");
    assert_eq!(m["content"], "hi there");
    assert_eq!(
        m["tool_calls"],
        json!([{"id":"a","type":"function","function":{"name":"run_exercise_tests","arguments":"{}"}},{"id":"b","type":"function","function":{"name":"get_lesson","arguments":"{\"id\":\"1.1\"}"}}])
    );
    let mut chunks = String::new();
    assert_eq!(
        app.ask(
            &[],
            &mut Some(&mut |s| {
                chunks.push_str(s);
                Ok(())
            }),
            json!({"response_format":{"type":"json_schema"}})
        )
        .unwrap()["content"],
        "fallback"
    );
    assert_eq!(chunks, "fallback");
    assert!(app.off.lock().unwrap().contains("stream"));
    assert_eq!(
        app.ask(&[], &mut None, json!({})).unwrap_err().message,
        "The AI service sent an answer this app can't read."
    );
    assert_eq!(
        app.ask(&[], &mut None, json!({})).unwrap_err().message,
        "AI service error 503: service unavailable"
    );
    worker.join().unwrap();
}
#[test]
fn request_limits_legacy_files_and_project_symlinks() {
    let tmp = tempfile::tempdir().unwrap();
    let app = isolated_app(tmp.path());
    let base = app.clone();
    assert_eq!(call(&base, "PUT", "api/settings", json!([])).0, 400);
    assert_eq!(
        call(
            &base,
            "PUT",
            "api/settings",
            json!({"pad":"x".repeat((1<<20)+1)})
        )
        .0,
        413
    );
    assert_eq!(request_body(Value::Null).unwrap(), json!({}));
    assert_eq!(serde_json::to_value(Error::bad("Bad request.")).unwrap(), json!({"status":400,"message":"Bad request."}));
    // These are the literal file formats the old app wrote, including nullable folder and a Python exercise.
    let legacy = r#"{"id":"0123456789ab","title":"Existing roadmap","summary":"saved before the port","prompt":"learn","level":"beginner","folder":null,"phases":[{"title":"One","blurb":"","lessons":[{"id":"1.1","title":"Basics","goal":"g","concepts":["one"],"tasks":[],"done":[],"query":"q","practice":{"quiz":[],"exercise":{"lang":"python","title":"Double","prompt":"p","starter":"def double(x): pass","tests":"import main"}}}]}],"state":{"checks":{},"done":{},"notes":{"1.1":"old note"},"code":{"1.1":"def double(x): return x * 2"},"last":"1.1"}}"#;
    fs::create_dir_all(&app.data).unwrap();
    fs::write(app.path_of("0123456789ab").unwrap(), legacy).unwrap();
    let loaded = app.load("0123456789ab").unwrap();
    assert_eq!(loaded, serde_json::from_str::<Value>(legacy).unwrap());
    app.save(&loaded).unwrap();
    assert_eq!(app.load("0123456789ab").unwrap(), loaded);
    fs::write(
        &app.conf,
        r#"{"base":"http://127.0.0.1:1/v1","model":"old-model","key":"old-key","theme":"dark"}"#,
    )
    .unwrap();
    assert_eq!(app.theme(), "dark");
    assert_eq!(app.config()["key"], "old-key");
    app.save_config(&json!({"base":"http://127.0.0.1:1/v1","model":"old-model"}))
        .unwrap();
    assert_eq!(
        fs::metadata(&app.conf).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(app.config()["key"], "old-key");
    let root = tmp.path().join("project");
    fs::create_dir(&root).unwrap();
    fs::write(tmp.path().join("outside.txt"), "private").unwrap();
    std::os::unix::fs::symlink(tmp.path().join("outside.txt"), root.join("escape.txt")).unwrap();
    assert!(app
        .project_file(root.to_str().unwrap(), &json!("escape.txt"))
        .is_none());
    fs::write(root.join("password.txt"), "secret").unwrap();
    assert!(app
        .project_file(root.to_str().unwrap(), &json!("password.txt"))
        .is_none());
    fs::write(root.join("huge.txt"), vec![b'x'; 200001]).unwrap();
    assert!(app
        .project_file(root.to_str().unwrap(), &json!("huge.txt"))
        .is_none());
    fs::write(root.join("binary"), [0xff, 0xfe]).unwrap();
    assert!(app
        .project_file(root.to_str().unwrap(), &json!("binary"))
        .is_none());
    assert_eq!(
        strs(&json!([null,true,["x"],{"x":false}]), 8),
        vec!["None", "True", "['x']", "{'x': False}"]
    );
}

#[test]
fn long_roadmap_outline_expand_and_revise() {
    let tmp = tempfile::tempdir().unwrap();
    let mut app = isolated_app(tmp.path());
    let a = Arc::get_mut(&mut app).unwrap();
    a.yt_program = tmp.path().join("yt-dlp");
    fs::write(
        &a.yt_program,
        format!("#!/bin/sh\ncat <<'FIXTURE'\n{}\nFIXTURE\n", found()),
    )
    .unwrap();
    fs::set_permissions(&a.yt_program, fs::Permissions::from_mode(0o755)).unwrap();
    let ai = FakeAI::new();
    app.write_config(&json!({"base":ai.base,"model":"m","key":"sk-test"}))
        .unwrap();
    // A broken file is skipped instead of hiding the library, and a panic does not block later saves.
    fs::create_dir_all(&app.data).unwrap();
    fs::write(app.data.join("aaaaaaaaaaaa.json"), "{broken").unwrap();
    assert!(app.courses().unwrap().is_empty());
    let held = app.clone();
    let _ = thread::spawn(move || {
        let _guard = held.guard();
        panic!("expected: poisons the lock");
    })
    .join();
    drop(app.guard());

    let made = json!({"prompt":"bake bread","level":"Complete beginner","size":"mastery"});
    let c = app.route("POST", "api/courses", &made).unwrap();
    let cid = string(&c["id"]);
    assert_eq!(
        c["stages"],
        json!([{"title":"Basics","blurb":"Ends with a loaf."},{"title":"Stage 2","blurb":""}])
    );
    assert_eq!(
        list(&c["phases"])
            .iter()
            .map(|p| p["stage"].clone())
            .collect::<Vec<_>>(),
        [json!(0), json!(1)]
    );
    // The first phase is written at once. The empty title falls back to the outline; a lesson without checks stays an outline.
    let first = &c["phases"][0]["lessons"];
    assert_eq!(first[0]["title"], "Mix");
    assert_eq!(first[0]["goal"], "Mix a dough.");
    assert_eq!(first[0]["done"], json!(["d"]));
    assert!(first[0].get("stub").is_none());
    assert_eq!(first[1]["title"], "Knead");
    assert_eq!(first[1]["stub"], true);
    assert_eq!(c["phases"][1]["lessons"][0]["stub"], true);
    let start = Instant::now();
    while !truth(&app.load(&cid).unwrap()["phases"][0]["lessons"][0]["videos"]) {
        assert!(start.elapsed() < Duration::from_secs(5), "no videos");
        thread::sleep(Duration::from_millis(20));
    }
    assert!(!truth(&app.load(&cid).unwrap()["phases"][0]["lessons"][1]["videos"]));

    let file = app.export_course(&app.load(&cid).unwrap(), false).unwrap();
    let back = app.import_course(&string(&file["path"])).unwrap();
    assert_eq!(back["stages"], c["stages"]);
    assert_eq!(back["phases"][1]["stage"], 1);
    assert_eq!(back["phases"][1]["lessons"][0]["stub"], true);
    fs::remove_file(app.path_of(&string(&back["id"])).unwrap()).unwrap();

    let path = format!("api/courses/{cid}");
    let state = json!({"checks":{},"done":{"1.1":true},"notes":{}});
    app.route("PUT", &format!("{path}/state"), &state).unwrap();
    assert_eq!(
        app.route("POST", &format!("{path}/revise"), &json!({"ask":" "}))
            .unwrap_err()
            .status,
        400
    );
    let r = app
        .route("POST", &format!("{path}/revise"), &json!({"ask":"industry level"}))
        .unwrap();
    assert_eq!(r["change"]["added"], json!(["In industry"]));
    assert_eq!(r["change"]["dropped"], json!(["Heat"]));
    let c = &r["course"];
    assert_eq!(
        text(&c["prompt"]),
        "bake bread\nLater I also asked: industry level"
    );
    assert_eq!(
        list(&c["phases"])
            .iter()
            .map(|p| (string(&p["title"]), p["stage"].clone()))
            .collect::<Vec<_>>(),
        [("Dough".to_string(), json!(0)), ("In industry".to_string(), json!(0))]
    );
    let new = &c["phases"][1]["lessons"];
    assert_eq!(new, &json!([{"id":"3.1","title":"Real ovens","goal":"g","concepts":[],"tasks":[],"done":[],"query":"Real ovens","lookups":[],"files":[],"warn":"","charts":[],"stub":true,"added":true,"known":{}}]));

    let c = app
        .route("POST", &format!("{path}/expand"), &json!({"lesson":"3.1"}))
        .unwrap();
    let l = &c["phases"][1]["lessons"][0];
    assert_eq!(l["title"], "Real ovens");
    assert_eq!(l["lookups"], json!(["l"]));
    assert_eq!(l["added"], true);
    assert!(l.get("stub").is_none());
    assert_eq!(
        app.route("POST", &format!("{path}/expand"), &json!({"lesson":"9.9"}))
            .unwrap_err()
            .status,
        404
    );
}
#[test]
fn ideas_mistakes_search_setup_and_exports() {
    let tmp = tempfile::tempdir().unwrap();
    let app = isolated_app(tmp.path());
    let cid = "0123456789ab";
    let note = "Remember the Autolyse step";
    app.save(&json!({"id":cid,"title":"Bake bread","summary":"s","prompt":"bread","level":"new",
        "needs":clean_needs(&json!([{"name":"Shell","kind":"tool","command":"sh"},{"name":"Missing","kind":"tool","command":"no-such-tool-xyz"},{"name":"Oven","command":"rm -rf /"}])),
        "phases":[{"title":"P","blurb":"","lessons":[{"id":"1.1","title":"Starter","goal":"Feed a starter","concepts":["hydration ratio"],"tasks":[],"done":["bubbles"],
            "practice":{"quiz":[{"q":"Q?","options":["a","b"],"answer":1,"why":"because"}],"exercise":null}}]}],
        "state":{"checks":{},"done":{},"notes":{"1.1":note},"last":"1.1"}}))
        .unwrap();
    let r = |m: &str, p: &str, b: Value| app.route(m, p, &b);
    let at = |sub: &str| format!("api/courses/{cid}{sub}");

    assert!(r("PUT", "api/ideas", json!({})).is_err());
    r("PUT", "api/ideas", json!({"notes":["Rye autolyse experiment", 5]})).unwrap();
    assert_eq!(
        r("GET", "api/ideas", json!({})).unwrap(),
        json!({"notes":["Rye autolyse experiment"]})
    );

    let miss = |i: u64| r("POST", "api/mistake", json!({"course":cid,"lesson":"1.1","i":i,"picked":0}));
    assert!(miss(9).is_err());
    miss(0).unwrap();
    // The page saves its state often and must not be able to wipe the notebook.
    r("PUT", &at("/state"), json!({"checks":{},"done":{},"notes":{"1.1":note},"mistakes":{}})).unwrap();
    let m = r("GET", "api/mistakes", json!({})).unwrap()["items"][0].clone();
    assert_eq!((&m["picked"], &m["answer"], &m["n"], &m["fixed"]), (&json!(0), &json!(1), &json!(1), &json!(false)));
    assert_eq!(app.load(cid).unwrap()["state"]["review"]["1.1#0"]["due"], app.today());
    app.reviewed(cid, "1.1", "0", true).unwrap();
    assert_eq!(r("GET", "api/mistakes", json!({})).unwrap()["items"][0]["fixed"], true);

    app.save_chat(cid, "1.1", &[json!({"role":"user","content":"why autolyse?"})], "It hydrates the flour.");
    let kinds = |q: &str| {
        list(&r("POST", "api/search", json!({"q":q})).unwrap()["results"])
            .iter()
            .map(|x| string(&x["kind"]))
            .collect::<Vec<_>>()
    };
    assert_eq!(kinds("AUTOLYSE"), ["idea", "note", "chat"]);
    assert_eq!(kinds("hydration starter"), ["lesson"]);
    assert!(kinds("autolyse nowhere").is_empty() && kinds(" ").is_empty());
    assert_eq!(list(&r("GET", &at(""), json!({})).unwrap()["chats"]["1.1"]).len(), 2);

    assert_eq!(
        r("POST", &at("/setup"), json!({})).unwrap(),
        json!({"found":{"sh":true,"no-such-tool-xyz":false}})
    );
    let file = |v: Value| fs::read_to_string(v["path"].as_str().unwrap()).unwrap();
    let t: Value = serde_json::from_str(&file(r("POST", &at("/export"), json!({"template":true})).unwrap())).unwrap();
    assert_eq!((&t["title"], &t["state"]["notes"]), (&json!("Bake bread"), &json!({})));
    let md = file(r("POST", &at("/summary"), json!({})).unwrap());
    assert!(md.starts_with("# Bake bread") && md.contains(note));

    assert_eq!(
        crate::ai::transcript_lines(&json!({"events":[{"tStartMs":0,"segs":[{"utf8":"hello"}]},{"tStartMs":5000,"segs":[{"utf8":"\n"}]},{"tStartMs":9000,"segs":[{"utf8":" there"}]},{"tStartMs":25000,"segs":[{"utf8":"next"}]}]})),
        [(0, "hello there".to_string()), (25, "next".to_string())]
    );
    r("DELETE", &at(""), json!({})).unwrap();
    assert!(!app.data.join(format!("{cid}.chat.json")).exists());
}


#[test]
fn deep_project() {
    let tmp = tempfile::tempdir().unwrap();
    let app = isolated_app(tmp.path());
    let ai = FakeAI::new();
    app.write_config(&json!({"base":ai.base,"model":"m","key":"sk-test"})).unwrap();
    let root = tmp.path().join("project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("app.py"), "print('hi')").unwrap();
    let mut c = app.build_course("understand", "beginner", root.to_str().unwrap(), "deep").unwrap();
    assert!(c["stages"].is_array());
    assert_eq!(c["phases"][0]["lessons"][0]["files"], json!(["app.py"]));
    let cid = string(&c["id"]);
    app.save(&c).unwrap();
    c = app.expand(&cid, &json!("1.1")).unwrap();
    assert!(c["phases"][0]["lessons"][0].get("stub").is_none());
    assert_eq!(c["phases"][0]["lessons"][0]["files"], json!(["app.py"]));
    assert_eq!(c["phases"][1]["lessons"][0]["stub"], true);
}

#[test]
fn topic_videos() {
    let tmp = tempfile::tempdir().unwrap();
    let mut app = isolated_app(tmp.path());
    let a = Arc::get_mut(&mut app).unwrap();
    a.yt_program = tmp.path().join("yt-dlp");
    fs::write(&a.yt_program, format!("#!/bin/sh\ncat <<'FIXTURE'\n{}\nFIXTURE\n", found())).unwrap();
    fs::set_permissions(&a.yt_program, fs::Permissions::from_mode(0o755)).unwrap();
    let cid = "0123456789ab";
    let mut videos = vec![json!({"id":"original"})];
    for i in 0..7 { videos.push(json!({"id":format!("old{i}"),"searched":true})); }
    app.save(&json!({"id":cid,"phases":[{"lessons":[{"id":"1.1","videos":videos}]}]})).unwrap();
    let path = format!("api/courses/{cid}/videos");
    assert_eq!(app.route("POST",&path,&json!({"lesson":"1.1","query":"  "})).unwrap_err().status,400);
    let v = app.route("POST",&path,&json!({"lesson":"1.1","query":"some topic"})).unwrap();
    assert_eq!(v[0]["id"], "original");
    assert_eq!(list(&v).len(),8);
    assert!(list(&v).iter().skip(1).all(|x| truth(&x["searched"])));
    assert!(!list(&v).iter().any(|x| x["id"] == "old0"));
}

#[test]
fn mcp_transport_and_tools() {
    let dir = tempfile::tempdir().unwrap();
    let app = isolated_app(dir.path());
    let input = concat!(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2024-11-05\"}}\n",
        "{\"method\":\"notifications/initialized\"}\n",
        "{\"id\":2,\"method\":\"ping\"}\n",
        "{\"id\":3,\"method\":\"tools/list\"}\n",
        "{\"id\":4,\"method\":\"prompts/get\",\"params\":{\"name\":\"write_roadmap\",\"arguments\":{\"topic\":\"Bread\"}}}\n",
        "bad json\n",
        "{\"id\":5,\"method\":\"unknown\"}\n",
        "{\"id\":6,\"method\":\"tools/call\",\"params\":{\"name\":\"unknown\"}}\n",
        "{\"id\":7,\"method\":\"ping\"}\n"
    );
    let mut output=vec![]; mcp::serve(&app,input.as_bytes(),&mut output).unwrap();
    let replies=String::from_utf8(output).unwrap().lines().map(|l|serde_json::from_str::<Value>(l).unwrap()).collect::<Vec<_>>();
    assert_eq!(replies.len(),8);
    assert_eq!(replies[0]["result"]["protocolVersion"],"2024-11-05");
    assert_eq!(replies[0]["result"]["serverInfo"]["name"],"roadmap");
    assert_eq!(replies[1]["result"],json!({}));
    let names=list(&replies[2]["result"]["tools"]).iter().map(|t|string(&t["name"])).collect::<HashSet<_>>();
    assert_eq!(names, ["list_roadmaps","get_roadmap","create_roadmap","write_lessons","get_progress","add_idea","search"].into_iter().map(String::from).collect());
    assert!(replies[3].to_string().contains("Bread"));
    assert_eq!(replies[4]["error"]["code"],-32700);
    assert_eq!(replies[5]["error"]["code"],-32601);
    assert_eq!(replies[6]["result"]["isError"],true);
    assert_eq!(replies[7]["result"],json!({}));
}

#[test]
fn mcp_roadmap_lifecycle() {
    let dir=tempfile::tempdir().unwrap();let app=isolated_app(dir.path());
    let call=|name:&str,args:Value|->Value {
        let input=json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args}}).to_string()+"\n";
        let mut out=vec![];mcp::serve(&app,input.as_bytes(),&mut out).unwrap();
        let reply:Value=serde_json::from_slice(&out).unwrap();
        if reply["result"]["isError"]==true{return reply["result"].clone();}
        serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
    };
    let full=json!({"title":"Mix","tasks":["Mix flour"],"done":["Dough mixed"]});
    let created=call("create_roadmap",json!({"roadmap":{"title":"x".repeat(250),"needs":[{"name":"unsafe","kind":"tool","command":"sh; bad"}],"stages":[{"title":"Bread","phases":[{"title":"Dough","lessons":[full,{"title":"Bake"},false,{"title":"Serve"}]}]}]}}));
    let id=created["id"].as_str().unwrap();let mut c=app.load(id).unwrap();
    assert_eq!(string(&c["title"]).len(),200);assert_eq!(c["needs"][0]["command"],"");
    assert_ne!(c["phases"][0]["lessons"][0]["stub"],true);
    assert_eq!(c["phases"][0]["lessons"][1]["stub"],true);
    assert_eq!(c["phases"][0]["lessons"][2]["id"],"1.3");
    assert!(created["changes"].to_string().contains("non-object"));
    assert_eq!(call("list_roadmaps",json!({}))[0]["id"],id);
    assert_eq!(call("get_roadmap",json!({"id":id}))["phases"][0]["lessons"][1]["stub"],true);
    c["state"]["done"]["1.3"]=json!(true);c["state"]["notes"]["1.3"]=json!("PRIVATE");app.save(&c).unwrap();
    let result=call("write_lessons",json!({"id":id,"lessons":[{"id":"1.1","tasks":["a"],"done":["b"]},{"id":"1.2","tasks":["a"],"done":["b"]},{"id":"1.3","tasks":["a"],"done":["b"]}]}));
    assert_eq!(result["written"],json!(["1.2"]));assert_eq!(list(&result["skipped"]).len(),2);
    // A lesson written without a title keeps the outline's title, and a second write cannot replace it.
    assert_eq!(app.load(id).unwrap()["phases"][0]["lessons"][1]["title"],"Bake");
    assert_eq!(app.load(id).unwrap()["phases"][0]["lessons"][1]["query"],"Bake");
    let kept=call("write_lessons",json!({"id":id,"lessons":[{"id":"1.2","title":"Changed"}]}));
    assert_eq!(kept["written"],json!([]));assert_eq!(app.load(id).unwrap()["phases"][0]["lessons"][1]["title"],"Bake");
    let progress=call("get_progress",json!({"id":id}));assert_eq!(progress["lessons"][2]["completed"],true);assert!(!progress.to_string().contains("PRIVATE"));
    assert_eq!(call("add_idea",json!({"text":"BREAD IDEA"}))["notes"][0],"BREAD IDEA");
    assert_eq!(call("add_idea",json!({"text":" "}))["isError"],true);
    assert!(call("search",json!({"query":"Mix"})).to_string().contains("Mix"));
}

#[test]
fn mcp_data_lock_is_shared_between_app_instances() {
    let dir=tempfile::tempdir().unwrap();let app=isolated_app(dir.path());
    let other=isolated_app(dir.path());
    let held=app.guard();
    let (tx,rx)=std::sync::mpsc::channel();
    let worker=thread::spawn(move||{let _guard=other.guard();tx.send(()).unwrap();});
    assert!(rx.recv_timeout(Duration::from_millis(50)).is_err());
    drop(held);rx.recv_timeout(Duration::from_secs(2)).unwrap();worker.join().unwrap();
}

#[test]
fn chat_stream_fallback_empty_partial_and_early_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let app = isolated_app(tmp.path());
    let cid = "0123456789ab";
    let mut c = course();
    c["id"] = json!(cid);
    c["phases"][0]["lessons"][0]["id"] = json!("1.1");
    app.save(&c).unwrap();
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    app.save_config(&json!({"base":format!("http://{}/v1", server.server_addr()),"model":"m"})).unwrap();
    let worker = thread::spawn(move || {
        for (status, content_type, body) in [
            (200, "application/json", json!({"choices":[{"message":{"content":"fallback"}}]}).to_string()),
            (200, "application/json", json!({"choices":[{"message":{"content":""}}]}).to_string()),
            (200, "text/event-stream", format!("data: {}\n\ndata: broken\n\n", json!({"choices":[{"delta":{"content":"partial"}}]}))),
            (503, "text/plain", "unavailable".into()),
        ] {
            let req = server.recv_timeout(Duration::from_secs(10)).unwrap().expect("chat request");
            req.respond(tiny_http::Response::from_string(body).with_status_code(status)
                .with_header(tiny_http::Header::from_bytes("Content-Type", content_type).unwrap())).unwrap();
        }
    });
    let body = json!({"course":cid,"lesson":"1.1","messages":[{"role":"user","content":"help"}]});
    for expected in ["fallback", "(No answer.)", "partial\n\n(The answer was cut off: The AI service sent an answer this app can't read.)"] {
        let mut pieces = String::new();
        let answer = app.chat_stream(&body, &mut |piece| { pieces.push_str(piece); Ok(()) }).unwrap();
        assert_eq!(answer, expected);
        assert_eq!(pieces, answer);
        if expected == "fallback" {
            assert_eq!(app.read_side(&app.chat_file(cid).unwrap())["1.1"][1]["content"], expected);
        }
    }
    let mut pieces = String::new();
    assert_eq!(app.chat_stream(&body, &mut |piece| { pieces.push_str(piece); Ok(()) }).unwrap_err().status, 400);
    assert!(pieces.is_empty());
    worker.join().unwrap();
}
#[test]
fn charts_are_cleaned() {
    let c = clean_charts(&json!([
        {"title":"Break-even","kind":"line","x":"Units","y":"£","labels":["0","100","200"],
         "series":[{"name":"Revenue","values":[0,2500,5000]},{"name":"Short","values":[1]},{"name":"Bad","values":[1,"x",3]}],"note":"n"},
        {"title":"One point","labels":["a"],"series":[{"name":"s","values":[1]}]},
        {"title":"Kind falls back","kind":"pie","labels":["a","b"],"series":[{"name":"s","values":[1.5,2]}]},
        "not a chart"
    ]));
    assert_eq!(c.len(), 2);
    assert_eq!(c[0]["series"], json!([{"name":"Revenue","values":[0.0,2500.0,5000.0]}]));
    assert_eq!(c[1]["kind"], "line");
    assert!(clean_charts(&json!(null)).is_empty());
}
