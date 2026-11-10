use crate::*;
use crate::{exercise::{clean_practice,language,process,run_code,LANGS},project::root_of,prompts::*};
use std::{collections::BTreeMap,io::{BufRead,BufReader},process::Command,time::Duration};
pub(crate) type Sink<'a> = Option<&'a mut dyn FnMut(&str)->Result<()>>;
fn s()->Value{json!({"type":"string"})}
fn arr(v:Value)->Value{json!({"type":"array","items":v})}
fn obj(p:Value)->Value{let keys=p.as_object().unwrap().keys().cloned().collect::<Vec<_>>();json!({"type":"object","properties":p,"required":keys,"additionalProperties":false})}
fn lesson_schema()->Value{json!({"title":s(),"goal":s(),"concepts":arr(s()),"tasks":arr(s()),"done":arr(s()),"query":s(),"lookups":arr(s()),"warn":s()})}
fn course_schema(project:bool)->Value{let mut l=lesson_schema();if project{l["files"]=arr(s());}obj(json!({"title":s(),"summary":s(),"phases":arr(obj(json!({"title":s(),"blurb":s(),"lessons":arr(obj(l))})))}))}
fn practice_schema()->Value{obj(json!({"quiz":arr(obj(json!({"q":s(),"options":arr(s()),"answer":{"type":"integer"},"why":s()}))),"exercise":{"anyOf":[{"type":"null"},obj(json!({"lang":{"type":"string","enum":LANGS.iter().map(|(l,_,_)|*l).collect::<Vec<_>>()},"title":s(),"prompt":s(),"starter":s(),"tests":s()}))]}}))}
pub(crate) fn said(v:&Value)->Vec<String>{list(v).iter().filter(|a|a.is_object()).take(7).map(|a|format!("- {} My answer: {}",cap(&text(&a["q"]),400),cap(&text(&a["a"]),300))).collect()}
fn messages(system:&str,user:String)->Vec<Value>{vec![json!({"role":"system","content":system}),json!({"role":"user","content":user})]}
impl App {
 pub(crate) fn ask(&self,msgs:&[Value],on_text:&mut Sink<'_>,extra:Value)->Result<Value>{
  let c=self.config();if !truth(&c["base"])||!truth(&c["model"]){return Err(Error::bad("Set up the AI service in Settings first."));}
  let mut body=json!({"model":c["model"],"messages":msgs});let mut extra=extra.as_object().cloned().unwrap_or_default();extra.insert("stream".into(),json!(on_text.is_some()));
  {let off=self.off.lock().unwrap();extra.retain(|k,v|truth(v)&&!off.contains(k));}body.as_object_mut().unwrap().extend(extra.clone());
  let agent=ureq::AgentBuilder::new().timeout(Duration::from_secs(300)).redirects(0).build();
  let url=format!("{}/chat/completions",string(&c["base"]).trim_end_matches('/'));let mut req=agent.post(&url).set("Content-Type","application/json").set("User-Agent","roadmap-app");
  let key=string(&c["key"]);let auth=format!("Bearer {key}");if !key.is_empty(){req=req.set("Authorization",&auth);}
  let r=match req.send_string(&body.to_string()){
   Ok(r)=>r,Err(ureq::Error::Status(code,r))=>{if !extra.is_empty()&&[400,404,422,501].contains(&code){self.off.lock().unwrap().extend(extra.keys().cloned());return self.ask(msgs,on_text,json!({}));}let mut bytes=vec![];let _=r.into_reader().take(400).read_to_end(&mut bytes);return Err(Error::bad(format!("AI service error {code}: {}",String::from_utf8_lossy(&bytes))));},
   Err(e)=>return Err(Error::bad(format!("Can't reach the AI service: {e}"))),
  };
  let unreadable=||Error::bad("The AI service sent an answer this app can't read.");
  if !r.header("Content-Type").unwrap_or("").contains("event-stream"){
   let d:Value=serde_json::from_reader(r.into_reader()).map_err(|_|unreadable())?;
   let m=d["choices"][0].get("message").filter(|v|v.is_object()).ok_or_else(unreadable)?.clone();
   if let Some(out)=on_text.as_mut(){if let Some(t)=m["content"].as_str().filter(|s|!s.is_empty()){out(t)?;}}return Ok(m);
  }
  let mut content=String::new();let mut calls=BTreeMap::new();
  for line in BufReader::new(r.into_reader()).split(b'\n'){
   let line=line.map_err(|_|unreadable())?;let line=String::from_utf8_lossy(&line);let line=line.trim();let Some(data)=line.strip_prefix("data:")else{continue;};if data.trim()=="[DONE]"{continue;}
   let d:Value=serde_json::from_str(data).map_err(|_|unreadable())?;let delta=&d["choices"][0]["delta"];
   if let Some(t)=delta["content"].as_str(){content.push_str(t);if let Some(out)=on_text.as_mut(){out(t)?;}}
   for k in list(&delta["tool_calls"]){let t=calls.entry(k["index"].as_u64().unwrap_or(0)).or_insert_with(||json!({"id":"","type":"function","function":{"name":"","arguments":""}}));if truth(&k["id"]){t["id"]=k["id"].clone();}for f in ["name","arguments"]{let joined=string(&t["function"][f])+&string(&k["function"][f]);t["function"][f]=json!(joined);}}
  }Ok(json!({"content":content,"tool_calls":if calls.is_empty(){Value::Null}else{json!(calls.into_values().collect::<Vec<_>>())}}))
 }
 fn llm(&self,msgs:Vec<Value>,name:&str,schema:Value)->Result<String>{let m=self.ask(&msgs,&mut None,json!({"response_format":{"type":"json_schema","json_schema":{"name":name,"strict":true,"schema":schema}}}))?;Ok(string(&m["content"]))}
 pub(crate) fn placement(&self,prompt:&str,folder:&str,answers:&Value)->Result<Value>{
  let asked=said(answers);if asked.len()>=7{return Ok(json!({"done":true}));}let mut rows=vec![format!("What I want: {}",if prompt.is_empty(){"Understand this project so I can change it."}else{prompt})];
  if !folder.is_empty(){rows.push(format!("The project's files: {}",self.read_project(folder)?.1.into_iter().take(200).collect::<Vec<_>>().join(", ")));}
  if asked.is_empty(){rows.push("You have not asked anything yet.".into());}else{rows.push("Your questions and my answers so far:".into());rows.extend(asked);}
  let d=parse(&self.llm(messages(PLACEMENT_PROMPT,rows.join("\n")),"placement",obj(json!({"done":{"type":"boolean"},"q":s(),"options":arr(s())})))?,"a question")?;
  let q=cap(&string(&d["q"]),400);Ok(if d["done"]==true||q.is_empty(){json!({"done":true})}else{json!({"done":false,"q":q,"options":strs(&d["options"],5)})})
 }
 pub(crate) fn build_course(&self,prompt:&str,level:&str,folder:&str)->Result<Value>{
  let mut known=vec![];let mut root=folder.to_string();let mut rows;let system;
  if !folder.is_empty(){let (r,f,digest)=self.read_project(folder)?;root=r;known=f;system=PROJECT_PROMPT;rows=vec![format!("What I want from this project: {}",if prompt.is_empty(){"Understand how it works so I can change it."}else{prompt}),format!("My level: {level}")];rows.extend(self.learned("",120)?);rows.extend([String::new(),digest]);}
  else{system=COURSE_PROMPT;rows=vec![format!("What I want to learn or build: {prompt}"),format!("My level: {level}")];rows.extend(self.learned("",120)?);}
  let d=parse(&self.llm(messages(system,rows.join("\n")),"course",course_schema(!folder.is_empty()))?,"a course")?;let mut phases=vec![];
  for p in list(&d["phases"]).iter().filter(|p|p.is_object()) {let n=phases.len()+1;let ls=list(&p["lessons"]).iter().filter(|l|l.is_object()).enumerate().map(|(i,l)|clean_lesson(l,&format!("{n}.{}",i+1),&known)).collect::<Vec<_>>();if !ls.is_empty(){phases.push(json!({"title":cap(&if truth(&p["title"]){text(&p["title"])}else{format!("Phase {n}")},200),"blurb":cap(&string(&p["blurb"]),600),"lessons":ls}));}}
  if phases.is_empty(){return Err(Error::bad("The AI returned a course with no lessons. Try again, or use a stronger model."));}
  Ok(json!({"id":random_hex(6)?,"prompt":prompt,"level":level,"folder":root,"title":cap(&if truth(&d["title"]){text(&d["title"])}else{prompt.into()},200),"summary":cap(&string(&d["summary"]),600),"phases":phases,"state":{"checks":{},"done":{},"notes":{},"last":"1.1"}}))
 }
 pub(crate) fn build_practice(&self,c:&Value,l:&Value)->Result<Value>{
  let mut rows=vec![format!("Course: {}",text(&c["title"])),format!("The learner's goal: {}",text(&c["prompt"])),format!("Level: {}",level(c)),format!("Lesson {}: {}",text(&l["id"]),text(&l["title"])),format!("Goal: {}",text(&l["goal"])),format!("Concepts: {}",joined(&l["concepts"])),format!("Tasks (do not reuse): {}",joined(&l["tasks"]))];
  if truth(&l["files"]){rows.push("This lesson is about reading the learner's existing project. Ask the quiz about what this code does and why. The exercise is null.".into());rows.extend(self.lesson_files(c,l));}
  let pr=clean_practice(&parse(&self.llm(messages(PRACTICE_PROMPT,rows.join("\n")),"practice",practice_schema())?,"a quiz")?);
  if list(&pr["quiz"]).is_empty(){return Err(Error::bad("The AI returned a quiz with no usable questions. Try again, or use a stronger model."));}Ok(pr)
 }
 fn yt(&self,args:&[String])->Result<Value>{
  let (status,out,err,timeout)=process(Command::new(&self.yt_program).args(["-J","--no-warnings"]).args(args),60).map_err(|e|if e.kind()==std::io::ErrorKind::NotFound{Error::bad("yt-dlp is not installed. Run: sudo dnf install yt-dlp")}else{e.into()})?;
  if timeout{return Err(Error::bad("YouTube search timed out. Check your connection and try again."));}if status!=0{return Err(Error::bad(format!("YouTube search failed: {}",tail(err.trim(),300))));}Ok(serde_json::from_str::<Value>(&out).ok().filter(Value::is_object).unwrap_or(json!({})))
 }
 fn search_videos(&self,query:&str,skip:&[String])->Result<Vec<Value>>{
  let d=self.yt(&["--flat-playlist".into(),format!("ytsearch12:{query}")])?;let mut out=vec![];
  for e in list(&d["entries"]){let id=text(&e["id"]);if !e.is_object()||!matches(r"^[\w-]{11}$",&id)||!matches!(&e["live_status"],Value::Null)&&e["live_status"]!="not_live"&&e["live_status"]!="was_live"{continue;}
   let duration=e["duration"].as_f64().unwrap_or(0.) as i64;if skip.contains(&id)||(duration!=0&&duration<120){continue;}
   out.push(json!({"id":id,"title":cap(&string(&e["title"]),200),"channel":cap(&if truth(&e["channel"]){text(&e["channel"])}else{string(&e["uploader"])},100),"duration":duration,"views":e["view_count"].as_f64().unwrap_or(0.) as i64,"about":cap(&string(&e["description"]).split_whitespace().collect::<Vec<_>>().join(" "),200)}));
  }Ok(out)
 }
 fn pick_videos(&self,c:&Value,l:&Value,vids:&[Value],query:&str)->Result<(Vec<Value>,String)>{
  let mut rows=vec![format!("Course: {}",text(&c["title"])),format!("Level: {}",level(c)),format!("Lesson: {}",text(&l["title"])),format!("Goal: {}",text(&l["goal"])),format!("Concepts: {}",joined(&l["concepts"])),format!("The search that was used: {query}"),"Results:".into()];
  rows.extend(vids.iter().enumerate().map(|(i,v)|format!("{i}. {} | {} | {} min | {} views | {}",text(&v["title"]),text(&v["channel"]),v["duration"].as_i64().unwrap_or(0)/60,text(&v["views"]),text(&v["about"]))));
  let d=parse(&self.llm(messages(VIDEO_PROMPT,rows.join("\n")),"videos",obj(json!({"picks":arr(obj(json!({"i":{"type":"integer"},"why":s()}))),"query":s()})))?,"a video choice")?;let mut out=vec![];let mut used=HashSet::new();
  for k in list(&d["picks"]){if let Some(i)=k["i"].as_u64().map(|i|i as usize).filter(|i|*i<vids.len()){if used.insert(i){let mut v=vids[i].clone();v["why"]=json!(cap(&string(&k["why"]),300));out.push(v);}}}out.truncate(4);Ok((out,cap(&string(&d["query"]),200)))
 }
 pub(crate) fn find_videos(&self,c:&Value,l:&Value,skip:&[String])->Result<Vec<Value>>{
  let query=string(&l["query"]);let vids=self.search_videos(&query,skip)?;
  let pick=||->Result<Vec<Value>>{let (mut picks,better)=if vids.is_empty(){(vec![],String::new())}else{self.pick_videos(c,l,&vids,&query)?};if picks.len()<2&&!better.is_empty()&&better!=query{let more=self.search_videos(&better,skip)?.into_iter().filter(|v|!vids.iter().any(|x|x["id"]==v["id"])).collect::<Vec<_>>();if !more.is_empty(){picks.extend(self.pick_videos(c,l,&more,&better)?.0);}picks.truncate(4);}Ok(picks)};
  let mut picks=pick().unwrap_or_default();if picks.is_empty(){picks=vids.into_iter().take(4).collect();}
  std::thread::scope(|scope|{let handles=picks.into_iter().map(|mut v|scope.spawn(move||{if let Ok(d)=self.yt(&["--skip-download".into(),format!("https://www.youtube.com/watch?v={}",text(&v["id"]))]){v["chapters"]=json!(list(&d["chapters"]).iter().filter_map(|m|m["start_time"].as_f64().map(|n|json!({"t":n as i64,"title":cap(&string(&m["title"]),120)}))).take(40).collect::<Vec<_>>());}v})).collect::<Vec<_>>();Ok(handles.into_iter().map(|h|h.join().unwrap()).collect())})
 }
 pub(crate) fn prefetch(self:&Arc<Self>,cid:String){let app=self.clone();std::thread::spawn(move||{let _=app.prefetch_inner(&cid);});}
 fn prefetch_inner(&self,cid:&str)->Result<()>{
  let c=self.load(cid)?;let ls=lessons(&c).filter(|l|!truth(&l["videos"])).cloned().collect::<Vec<_>>();let mut found=vec![];
  // Four searches at once, matching the original desktop app's worker limit.
  for chunk in ls.chunks(4){let more=std::thread::scope(|scope|{let handles=chunk.iter().map(|l|{let c=&c;scope.spawn(move||(string(&l["id"]),self.find_videos(c,l,&[]).unwrap_or_default()))}).collect::<Vec<_>>();handles.into_iter().map(|h|h.join().unwrap()).collect::<Vec<_>>()});found.extend(more);}
  let _guard=self.lock.lock().unwrap();let mut c=self.load(cid)?;for (lid,vids) in found{if let Ok(l)=lesson_mut(&mut c,&lid){if !vids.is_empty()&&!truth(&l["videos"]){l["videos"]=json!(vids);}}}self.save(&c)
 }
 pub(crate) fn tutor_system(&self,c:&Value,l:&Value)->Result<String>{
  let st=&c["state"];let outline=lessons(c).map(|x|format!("{} {}{}",text(&x["id"]),text(&x["title"]),if truth(&st["done"][string(&x["id"])]){" (completed)"}else{""})).collect::<Vec<_>>();
  let checks=list(&l["done"]).iter().enumerate().map(|(i,d)|format!("{}{}",if truth(&st["checks"][format!("{}#{i}",text(&l["id"]))]){"[x] "}else{"[ ] "},text(d))).collect::<Vec<_>>();
  let mut rows=vec![TUTOR_PROMPT.into(),String::new(),format!("Course: {}",text(&c["title"])),format!("The learner's goal in their own words: {}",text(&c["prompt"])),format!("Level: {}",level(c)),format!("Lessons: {}",outline.join("; ")),String::new(),format!("Current lesson {}: {}",text(&l["id"]),text(&l["title"])),format!("Goal: {}",text(&l["goal"])),format!("Concepts: {}",joined(&l["concepts"])),format!("Tasks: {}",joined(&l["tasks"])),format!("Done when, with the learner's ticks: {}",checks.join("; ")),format!("Searches the course told them to run themselves: {}",if truth(&l["lookups"]){joined(&l["lookups"])}else{"none".into()}),format!("Safety note: {}",if truth(&l["warn"]){text(&l["warn"])}else{"none".into()})];
  let ex=&l["practice"]["exercise"];if truth(ex){let code=cap(&string(&st["code"][string(&l["id"])]),6000);rows.extend([format!("Coding exercise ({}): {}",text(&ex["lang"]),text(&ex["prompt"])),format!("Their code for it right now:\n{}",if code.is_empty(){"(nothing typed yet)"}else{&code})]);}
  let q=&st["quiz"][string(&l["id"])];if q.is_object(){rows.push(format!("Last quiz score: {} of {}",text(&q["score"]),text(&q["total"])));}
  let root=root_of(c);if !root.is_empty(){rows.push(format!("They connected their project folder ({}). Use list_project_files and read_project_file to look at their real work.",std::path::Path::new(&root).file_name().unwrap_or_default().to_string_lossy()));}
  rows.extend(self.learned(&string(&c["id"]),40)?);rows.extend(self.lesson_files(c,l));Ok(rows.join("\n"))
 }
 pub(crate) fn tutor_tools(&self,c:&Value,l:&Value)->Vec<Value>{
  let tool=|name:&str,description:&str,params:Value|json!({"type":"function","function":{"name":name,"description":description,"parameters":obj(params)}});
  let mut out=vec![tool("get_lesson","Read the goal, concepts and tasks of another lesson of this course. Call it before telling the learner which lesson covers a topic, or when they ask about earlier or later material.",json!({"id":{"type":"string","description":"The lesson number as written in the lesson list, for example 2.3"}}))];
  if truth(&l["practice"]["exercise"]){out.push(tool("run_exercise_tests","Run the code the learner has typed for this lesson's coding exercise against the exercise tests, in a sandbox, and return what the tests printed. Call it before saying whether their exercise code works or what is wrong with it. Takes no arguments.",json!({})));}
  if !root_of(c).is_empty(){out.push(tool("list_project_files","List the text files in the learner's project folder, as paths relative to it. Call it before read_project_file when you do not know the path. Takes no arguments.",json!({})));out.push(tool("read_project_file","Read one text file from the learner's project, as it is on disk now. Call it when the lesson's files, already given to you, are not enough to answer. Dependency folders, hidden files and secrets cannot be read.",json!({"path":{"type":"string","description":"Path relative to the project folder, for example src/app.py"}})));}out
 }
 pub(crate) fn tool_result(&self,c:&Value,l:&Value,name:&str,a:&Value)->Result<String>{
  if !a.is_object()||!self.tutor_tools(c,l).iter().any(|t|t["function"]["name"]==name){return Err(Error::missing());}
  match name{
   "get_lesson"=>{let x=lesson(c,&json!(text(&a["id"])))?;Ok(format!("Lesson {}: {}\nGoal: {}\nConcepts: {}\nTasks: {}",text(&x["id"]),text(&x["title"]),text(&x["goal"]),joined(&x["concepts"]),joined(&x["tasks"])))},
   "run_exercise_tests"=>{let ex=&l["practice"]["exercise"];let saved=string(&c["state"]["code"][string(&l["id"])]);let out=run_code(&string(&ex["lang"]),if saved.is_empty(){ex["starter"].as_str().unwrap_or("")}else{&saved},Some(ex["tests"].as_str().unwrap_or("")))?;Ok(if truth(&out["output"]){text(&out["output"])}else{"(no output)".into()})},
   "list_project_files"=>Ok(self.read_project(&root_of(c))?.1.into_iter().take(400).collect::<Vec<_>>().join("\n")),
   "read_project_file"=>Ok(cap(&self.project_file(&root_of(c),&a["path"]).filter(|s|!s.is_empty()).unwrap_or_else(||"No such file, or it is one the app does not read (hidden, binary, too large or a secret).".into()),12000)),_=>Err(Error::missing())
  }
 }
 pub(crate) fn tutor(&self,c:&Value,l:&Value,msgs:Vec<Value>,on_text:&mut Sink<'_>)->Result<String>{
  let tools=self.tutor_tools(c,l);let mut all=vec![json!({"role":"system","content":self.tutor_system(c,l)?})];all.extend(msgs);
  for _ in 0..4{let m=self.ask(&all,on_text,json!({"tools":tools}))?;let calls=list(&m["tool_calls"]);if calls.is_empty(){return Ok(string(&m["content"]));}all.push(json!({"role":"assistant","content":m["content"],"tool_calls":calls}));
   for k in calls{let f=&k["function"];let out=serde_json::from_str::<Value>(&if truth(&f["arguments"]){text(&f["arguments"])}else{"{}".into()}).ok().and_then(|a|self.tool_result(c,l,&string(&f["name"]),&a).ok()).unwrap_or_else(||"That tool or argument does not exist.".into());all.push(json!({"role":"tool","tool_call_id":k["id"],"content":cap(&out,12000)}));}}
  let answer="The tutor did not finish its answer. Ask again.";if let Some(out)=on_text.as_mut(){out(answer)?;}Ok(answer.into())
 }
 pub(crate) fn chat_args(&self,b:&Value)->Result<(Value,Value,Vec<Value>)>{let c=self.load(&text(&b["course"]))?;let l=lesson(&c,&b["lesson"])?.clone();let ms=list(&b["messages"]);let ms=ms[ms.len().saturating_sub(30)..].iter().filter(|m|m.is_object()&&(m["role"]=="user"||m["role"]=="assistant")).map(|m|json!({"role":m["role"],"content":cap(&text(&m["content"]),20000)})).collect();Ok((c,l,ms))}
 pub(crate) fn review_answer(&self,b:&Value)->Result<Value>{
  let c=self.load(&text(&b["course"]))?;let l=lesson(&c,&b["lesson"])?;let answer=b.get("answer").map(text).unwrap_or_default();
  if b["i"]=="code"{let ex=&l["practice"]["exercise"];language(&string(&ex["lang"]))?;let out=run_code(&string(&ex["lang"]),&cap(&answer,100000),Some(ex["tests"].as_str().unwrap_or("")))?;return if truth(&out["missing"]){Ok(out)}else{Ok(merge(json!({"right":out["ok"],"output":out["output"]}),self.reviewed(&string(&c["id"]),&string(&l["id"]),"code",truth(&out["ok"]))?))};}
  if b["i"]!="explain"||list(&l["concepts"]).is_empty(){return Err(Error::missing());}let step=c["state"]["review"][format!("{}#explain",text(&l["id"]))]["step"].as_i64().unwrap_or(-1);let cs=list(&l["concepts"]);let concept=&cs[(step+1).rem_euclid(cs.len() as i64) as usize];let answer=cap(&answer,4000);
  let user=format!("Course: {}\nLesson: {}\nGoal: {}\nConcept to explain: {}\nThe learner's answer:\n{}",text(&c["title"]),text(&l["title"]),text(&l["goal"]),text(concept),if answer.is_empty(){"(empty)"}else{&answer});
  let d=parse(&self.llm(messages(GRADE_PROMPT,user),"mark",obj(json!({"right":{"type":"boolean"},"feedback":s()})))?,"a mark")?;let right=d["right"]==true;Ok(merge(json!({"right":right,"feedback":cap(&string(&d["feedback"]),1500)}),self.reviewed(&string(&c["id"]),&string(&l["id"]),"explain",right)?))
 }
 pub(crate) fn adapt(self:&Arc<Self>,cid:&str)->Result<Value>{
  let c=self.load(cid)?;let st=&c["state"];let mut rows=vec![format!("Course: {}",text(&c["title"])),format!("The learner's goal: {}",text(&c["prompt"])),format!("Level: {}",level(&c))];
  for (n,p) in list(&c["phases"]).iter().enumerate(){rows.push(format!("Phase {}: {}",n+1,text(&p["title"])));for l in list(&p["lessons"]){let lid=string(&l["id"]);let done=truth(&st["done"][&lid]);let q=&st["quiz"][&lid];let t=&st["tests"][&lid];let mut facts=vec![if done{"completed"}else if untouched(st,l){"not started"}else{"in progress"}.into()];if q.is_object(){facts.push(format!("quiz {} of {}",text(&q["score"]),text(&q["total"])));}if t.is_object(){facts.push(format!("exercise {} after {} test runs",if truth(&t["ok"]){"passed"}else{"never passed"},text(&t["runs"])));}if done&&!(0..list(&l["done"]).len()).all(|i|truth(&st["checks"][format!("{lid}#{i}")])){facts.push("skipped the tasks by passing the quiz".into());}rows.push(format!("- {lid} {} [{}]. Concepts: {}. Tasks: {}",text(&l["title"]),facts.join(", "),joined(&l["concepts"]),joined(&l["tasks"])));}}
  rows.extend(self.learned(cid,60)?);let d=parse(&self.llm(messages(ADAPT_PROMPT,rows.join("\n")),"changes",obj(json!({"note":s(),"drop":arr(s()),"add":arr(obj(json!({"after":s(),"lesson":obj(lesson_schema())})))})))?,"a plan")?;
  let mut c;let change;{
   let _guard=self.lock.lock().unwrap();c=self.load(cid)?;let st=c["state"].clone();let mut ids=lessons(&c).map(|l|string(&l["id"])).collect::<HashSet<_>>();let mut added=vec![];let mut dropped=vec![];
   for a in list(&d["add"]).iter().filter(|a|a.is_object()&&a["lesson"].is_object()).take(2){for (n,p) in c["phases"].as_array_mut().unwrap().iter_mut().enumerate(){let ls=p["lessons"].as_array_mut().unwrap();if let Some(at)=ls.iter().position(|l|l["id"]==text(&a["after"])){let lid=(ls.len()+1..999).map(|i|format!("{}.{i}",n+1)).find(|s|!ids.contains(s)).ok_or_else(||Error::bad("Too many lessons."))?;ids.insert(lid.clone());let mut l=clean_lesson(&a["lesson"],&lid,&[]);l["added"]=json!(true);if truth(&a["lesson"]["title"]){added.push(l["title"].clone());}ls.insert(at+1,l);}}}
   let gone=strs(&d["drop"],20);let mut left=lessons(&c).count();for p in c["phases"].as_array_mut().unwrap(){p["lessons"].as_array_mut().unwrap().retain(|l|{if gone.contains(&string(&l["id"]))&&untouched(&st,l)&&!truth(&l["added"])&&left>1{dropped.push(l["title"].clone());left-=1;false}else{true}});}c["phases"].as_array_mut().unwrap().retain(|p|!list(&p["lessons"]).is_empty());
   change=json!({"day":self.today(),"note":cap(&string(&d["note"]),600),"added":added,"dropped":dropped});if !added.is_empty()||!dropped.is_empty(){let mut changes=list(&c["changes"]).to_vec();changes.push(change.clone());let start=changes.len().saturating_sub(20);c["changes"]=json!(&changes[start..]);self.save(&c)?;}
  }if truth(&change["added"]){self.prefetch(cid.into());}Ok(json!({"change":change,"course":self.mark_known(c)?}))
 }
}
fn level(c:&Value)->String{if truth(&c["level"]){text(&c["level"])}else{"not stated".into()}}
fn untouched(st:&Value,l:&Value)->bool{!truth(&st["done"][string(&l["id"])])&&!(0..list(&l["done"]).len()).any(|i|truth(&st["checks"][format!("{}#{i}",text(&l["id"]))]))}
