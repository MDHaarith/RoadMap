//! Line-delimited MCP transport. No AI or network operations.
use crate::*;
use std::io::{BufRead, Write};

fn required<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| Error::bad(format!("{key} must be a nonempty string")))
}
fn cleaned(l: &Value, id: &str) -> Value {
    let mut x = clean_lesson(l, id, &[]);
    if list(&x["tasks"]).is_empty() || list(&x["done"]).is_empty() {
        x["stub"] = json!(true);
    }
    x
}
fn tool(app: &App, name: &str, a: &Value) -> Result<Value> {
    match name {
        "list_roadmaps" => app.library_list(),
        "search" => app.search(required(a, "query")?),
        "add_idea" => {
            let text = required(a, "text")?;
            let _guard = app.guard();
            let mut notes = list(&app.read_side("ideas.json")["notes"]).to_vec();
            if notes.len() >= 200 {
                return Err(Error::bad("Idea limit is 200 notes"));
            }
            notes.push(json!(cap(text, 4000)));
            app.write_side("ideas.json", &json!({"notes":notes}))?;
            Ok(json!({"notes":notes}))
        }
        "create_roadmap" => {
            let d = &a["roadmap"];
            required(d, "title")?;
            let mut changes = vec![];
            let mut input = d.clone();
            let mut phases = list(&d["phases"]).to_vec();
            if phases.is_empty() {
                for (i, s) in list(&d["stages"])
                    .iter()
                    .filter(|s| s.is_object())
                    .take(12)
                    .enumerate()
                {
                    for p in list(&s["phases"]) {
                        let mut p = p.clone();
                        if p.is_object() {
                            p["stage"] = json!(i);
                        }
                        phases.push(p);
                    }
                }
            }
            let mut out = vec![];
            for p in phases.iter().filter(|p| p.is_object()).take(60) {
                let n = out.len() + 1;
                let mut ls = vec![];
                for l in list(&p["lessons"]) {
                    if !l.is_object() {
                        changes.push("Dropped non-object lesson".to_string());
                        continue;
                    }
                    if ls.len() == 40 {
                        changes.push("Dropped lessons beyond phase limit 40".into());
                        break;
                    }
                    let id = format!("{n}.{}", ls.len() + 1);
                    let x = cleaned(l, &id);
                    for k in [
                        "title", "goal", "concepts", "tasks", "done", "query", "lookups", "warn",
                    ] {
                        if !l[k].is_null() && l[k] != x[k] {
                            changes.push(format!("Lesson {id}: cleaned or shortened {k}"));
                        }
                    }
                    ls.push(x);
                }
                if !ls.is_empty() {
                    let mut x = p.clone();
                    x["lessons"] = json!(ls);
                    out.push(x);
                }
            }
            if out.is_empty() {
                return Err(Error::bad("Roadmap must contain at least one lesson"));
            }
            input["phases"] = json!(out);
            input["state"] = json!({});
            // Import owns course limits, stage/needs validation and safe persistence.
            let _guard = app.guard();
            let mut tmp = tempfile::NamedTempFile::new_in(&app.data)?;
            tmp.write_all(&serde_json::to_vec(&input)?)?;
            let c = app.import_course(
                tmp.path()
                    .to_str()
                    .ok_or_else(|| Error::bad("Invalid temporary path"))?,
            )?;
            for k in ["title", "summary", "prompt", "level", "needs"] {
                if !d[k].is_null() && d[k] != c[k] {
                    changes.push(format!("Cleaned, dropped or shortened {k}"));
                }
            }
            if phases.len() > 60 {
                changes.push("Dropped phases beyond limit 60".into());
            }
            for (i, phase) in list(&input["phases"]).iter().enumerate() {
                for key in ["title", "blurb", "stage"] {
                    if phase.get(key).is_some() && phase[key] != c["phases"][i][key] {
                        changes.push(format!("Phase {}: dropped or shortened {key}", i + 1));
                    }
                }
            }
            if d.get("stages").is_some()
                && clean_stages(&d["stages"]).len() != list(&d["stages"]).len()
            {
                changes.push("Dropped invalid stages or stages beyond limit 12".into());
            }
            for (i, stage) in list(&d["stages"])
                .iter()
                .filter(|s| s.is_object())
                .take(12)
                .enumerate()
            {
                for key in ["title", "blurb"] {
                    if stage.get(key).is_some() && stage[key] != c["stages"][i][key] {
                        changes.push(format!("Stage {}: cleaned or shortened {key}", i + 1));
                    }
                }
            }
            let ls = lessons(&c).collect::<Vec<_>>();
            Ok(
                json!({"id":c["id"],"stages":list(&c["stages"]).len(),"phases":list(&c["phases"]).len(),"written":ls.iter().filter(|l|l["stub"]!=true).count(),"outline":ls.iter().filter(|l|l["stub"]==true).count(),"changes":changes}),
            )
        }
        "write_lessons" => {
            let id = required(a, "id")?;
            let rows = a["lessons"]
                .as_array()
                .ok_or_else(|| Error::bad("lessons must be an array"))?;
            let _guard = app.guard();
            let mut c = app.load(id)?;
            let mut written = vec![];
            let mut skipped = vec![];
            for l in rows {
                let lid = string(&l["id"]);
                let reason = if truth(&c["state"]["done"][&lid]) {
                    Some("completed")
                } else {
                    match lesson(&c, &json!(lid)) {
                        Ok(old) if old["stub"] == true => None,
                        Ok(_) => Some("already written"),
                        Err(_) => Some("unknown lesson"),
                    }
                };
                if let Some(reason) = reason {
                    skipped.push(json!({"id":lid,"reason":reason}));
                    continue;
                }
                let mut x = cleaned(l, &lid);
                if x["stub"] == true {
                    skipped
                        .push(json!({"id":lid,"reason":"missing tasks or done; remains outline"}));
                    continue;
                }
                // What the agent left out keeps the outline's value, as in the app's own phase writer.
                let old = lesson_mut(&mut c, &lid)?;
                for k in ["title", "goal", "files"] {
                    if !truth(&l[k]) {
                        x[k] = old[k].clone();
                    }
                }
                if truth(&old["added"]) {
                    x["added"] = json!(true);
                }
                *old = x;
                written.push(lid);
            }
            app.save(&c)?;
            Ok(json!({"written":written,"skipped":skipped}))
        }
        "get_roadmap" | "get_progress" => {
            let c = app.load(required(a, "id")?)?;
            if name == "get_progress" {
                let items = app.mistakes()?;
                return Ok(
                    json!({"lessons":lessons(&c).map(|l|{let id=string(&l["id"]);json!({"id":id,"completed":truth(&c["state"]["done"][&id]),"quiz":c["state"]["quiz"][&id],"test":c["state"]["tests"][&id]})}).collect::<Vec<_>>(),"mistakes":list(&items["items"]).iter().filter(|m|m["course"]==c["id"]).collect::<Vec<_>>()}),
                );
            }
            if a.get("lesson").is_some() {
                return Ok(lesson(&c, &json!(required(a, "lesson")?))?.clone());
            }
            Ok(
                json!({"id":c["id"],"title":c["title"],"summary":c["summary"],"level":c["level"],"stages":c["stages"],"phases":list(&c["phases"]).iter().map(|p|json!({"title":p["title"],"stage":p["stage"],"lessons":list(&p["lessons"]).iter().map(|l|json!({"id":l["id"],"title":l["title"],"goal":l["goal"],"stub":l["stub"]==true,"completed":truth(&c["state"]["done"][string(&l["id"])])})).collect::<Vec<_>>()})).collect::<Vec<_>>()}),
            )
        }
        _ => Err(Error::bad("Unknown tool")),
    }
}
fn tools() -> Value {
    let specs=[
        ("list_roadmaps","List saved roadmaps with id, title, summary, done/total lesson counts and next lesson.",json!({}),vec![]),
        ("get_roadmap","Read an outline without learner state, or one full lesson. id is the 12-hex roadmap id; lesson is an optional 1-based phase.lesson id.",json!({"id":{"type":"string"},"lesson":{"type":"string"}}),vec!["id"]),
        ("create_roadmap","Create a learning-by-doing roadmap. Give title, summary, prompt (learner request), level, phases or stages containing phases, optional needs. Build toward a concrete goal; scaffold from learner level, give actionable tasks and observable done checks, no answers. Limits: 12 stages, 60 phases, 40 lessons/phase; title 200, summary/goal/blurb/warn 600, prompt 2000, level 3000 chars; concepts/tasks/done 8 entries, lookups 4. Lessons without tasks or done become outlines. IDs and empty progress are assigned. Returns id, counts and cleaning changes.",json!({"roadmap":{"type":"object","description":"Course with title, summary, prompt, level, phases [{title,blurb,lessons:[{title,goal,concepts,tasks,done,query,lookups,warn}]}] or stages [{title,blurb,phases}], optional needs [{name,kind,command,why}]. String arrays contain strings. query is a YouTube search; needs.command is a bare executable name, never shell code."}}),vec!["roadmap"]),
        ("write_lessons","Fill only outline lessons. id is roadmap id; lessons are full lesson objects {id,title,goal,concepts,tasks,done,query,lookups,warn} where id is the existing phase.lesson id from get_roadmap (for example 3.2); title and goal may be left out to keep the outline's. Same cleaning limits as create_roadmap. Completed/written lessons are skipped; missing tasks/done remain outlines. Returns written IDs and skipped IDs with reasons.",json!({"id":{"type":"string"},"lessons":{"type":"array","items":{"type":"object"}}}),vec!["id","lessons"]),
        ("get_progress","Read completed flags, quiz scores, exercise test results and recorded mistakes for roadmap id. Excludes notes and tutor chats.",json!({"id":{"type":"string"}}),vec!["id"]),
        ("add_idea","Append nonempty text to idea notes. Maximum 200 notes, 4000 characters per note (longer text shortened). Returns notes; full collection is an error.",json!({"text":{"type":"string"}}),vec!["text"]),
        ("search","Search all words of query across roadmap lessons, learner notes, tutor chats and ideas. Returns the app search result objects.",json!({"query":{"type":"string"}}),vec!["query"]),
    ];
    json!({"tools":specs.into_iter().map(|(name,description,properties,required)|json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false}})).collect::<Vec<_>>()})
}
fn dispatch(app: &App, m: &Value) -> std::result::Result<Value, i32> {
    let method = m["method"].as_str().ok_or(-32602)?;
    let empty = json!({});
    let p = m.get("params").unwrap_or(&empty);
    if !p.is_object() {
        return Err(-32602);
    }
    Ok(match method {
        "initialize" => {
            let v = p["protocolVersion"].as_str().unwrap_or("");
            json!({"protocolVersion":if ["2025-06-18","2025-03-26","2024-11-05"].contains(&v){v}else{"2025-06-18"},"capabilities":{"tools":{},"prompts":{}},"serverInfo":{"name":"roadmap","version":env!("CARGO_PKG_VERSION")}})
        }
        "ping" | "notifications/initialized" => json!({}),
        "tools/list" => tools(),
        "tools/call" => {
            let name = p["name"].as_str().ok_or(-32602)?;
            let a = p.get("arguments").unwrap_or(&empty);
            if !a.is_object() {
                return Err(-32602);
            }
            match tool(app, name, a) {
                Ok(v) => json!({"content":[{"type":"text","text":v.to_string()}]}),
                Err(e) => json!({"isError":true,"content":[{"type":"text","text":e.message}]}),
            }
        }
        "prompts/list" => {
            json!({"prompts":[{"name":"write_roadmap","description":"Design a learning-by-doing roadmap and save it.","arguments":[{"name":"topic","description":"Learner goal and current level","required":true},{"name":"size","description":"sprint, month, deep or mastery","required":false}]}]})
        }
        "prompts/get" => {
            if p["name"] != "write_roadmap" {
                return Err(-32602);
            }
            let a = &p["arguments"];
            let topic = required(a, "topic").map_err(|_| -32602)?;
            let size = a["size"].as_str().unwrap_or("month");
            if !["sprint", "month", "deep", "mastery"].contains(&size) {
                return Err(-32602);
            }
            json!({"messages":[{"role":"user","content":{"type":"text","text":format!("Topic: {topic}\nSize: {}\n\nBuild this course by calling the roadmap tools. The three rule sets below were written for a model that replies with JSON; where they say to reply with JSON, call the tool instead, and where they disagree with this paragraph, this paragraph wins. A short course: call create_roadmap once with every lesson fully written (rules A). A long course: call create_roadmap with titles and goals only (rules B), then call write_lessons one phase at a time (rules C). Set prompt to the learner's request and level to their level.\n\n--- A. Writing a whole short course ---\n{}\n\n--- B. Outlining a long course ---\n{}\n\n--- C. Writing the lessons of one phase ---\n{}",ai::size_rule(size).1,prompts::COURSE_PROMPT,prompts::OUTLINE_PROMPT,prompts::EXPAND_PROMPT)}}]})
        }
        _ => return Err(-32601),
    })
}
pub fn serve(app: &App, reader: impl BufRead, mut writer: impl Write) -> std::io::Result<()> {
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Err(_) => {
                json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}})
            }
            Ok(m) => {
                let Some(id) = m.get("id") else {
                    continue;
                };
                match dispatch(app, &m) {
                    Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
                    Err(code) => {
                        json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":if code==-32601{"Method not found"}else{"Invalid params"}}})
                    }
                }
            }
        };
        serde_json::to_writer(&mut writer, &reply)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }
    Ok(())
}
