use crate::*;
use crate::exercise::clean_practice;
use std::{collections::HashSet,io::Write,os::unix::fs::{OpenOptionsExt,PermissionsExt},path::Path};
impl App {
 pub(crate) fn config(&self) -> Value {fs::read(&self.conf).ok().and_then(|b|serde_json::from_slice(&b).ok()).filter(Value::is_object).unwrap_or(json!({}))}
 pub(crate) fn write_config(&self,c:&Value) -> Result<()> {
  fs::create_dir_all(self.conf.parent().unwrap())?;
  let mut f=fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&self.conf)?;
  f.set_permissions(fs::Permissions::from_mode(0o600))?;f.write_all(&serde_json::to_vec(c)?)?;Ok(())
 }
 pub(crate) fn theme(&self) -> String {let t=string(&self.config()["theme"]);if ["light","dark","system"].contains(&t.as_str()){t}else{"light".into()}}
 pub(crate) fn save_config(&self,b:&Value) -> Result<()> {
  let _guard=self.lock.lock().unwrap();let mut c=self.config();let base=string(&b["base"]).trim().to_string();
  if !matches(r"^https?://",&base){return Err(Error::bad("Base URL must start with http:// or https://"));}
  c["base"]=json!(base);c["model"]=json!(string(&b["model"]).trim());if truth(&b["key"]){c["key"]=json!(text(&b["key"]).trim());}self.write_config(&c)
 }
 pub(crate) fn path_of(&self,cid:&str) -> Result<PathBuf> {if !matches(r"^[0-9a-f]{12}$",cid){return Err(Error::missing());}Ok(self.data.join(format!("{cid}.json")))}
 pub(crate) fn load(&self,cid:&str) -> Result<Value> {Ok(serde_json::from_slice(&fs::read(self.path_of(cid)?)?)?)}
 pub(crate) fn save(&self,c:&Value) -> Result<()> {
  fs::create_dir_all(&self.data)?;let path=self.path_of(&string(&c["id"]))?;let tmp=path.with_extension("tmp");fs::write(&tmp,serde_json::to_vec(c)?)?;fs::rename(tmp,path)?;Ok(())
 }
 pub(crate) fn courses(&self) -> Result<Vec<Value>> {
  let mut files=match fs::read_dir(&self.data){Ok(d)=>d.filter_map(|e|e.ok()).map(|e|e.path()).filter(|p|p.extension().is_some_and(|x|x=="json")&&matches(r"^[0-9a-f]{12}$",&p.file_stem().unwrap_or_default().to_string_lossy())).collect::<Vec<_>>(),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>vec![],Err(e)=>return Err(e.into())};
  files.sort_by_cached_key(|p|std::cmp::Reverse(p.metadata().and_then(|m|m.modified()).ok()));files.into_iter().map(|p|Ok(serde_json::from_slice(&fs::read(p)?)?)).collect()
 }
 pub(crate) fn known(&self,skip:&str) -> Result<Vec<(String,String,bool)>> {
  let mut out=vec![];for c in self.courses()?{if c["id"]==skip{continue;}for l in lessons(&c){let q=&c["state"]["quiz"][string(&l["id"])];if truth(&c["state"]["done"][string(&l["id"])]){for x in list(&l["concepts"]){let x=text(x);if !x.starts_with("recap:"){out.push((x,string(&c["title"]),q["total"].as_f64().filter(|n|*n!=0.).is_some_and(|n|q["score"].as_f64().unwrap_or(0.)/n<0.6)));}}}}}Ok(out)
 }
 pub(crate) fn learned(&self,skip:&str,n:usize) -> Result<Vec<String>> {
  let out=self.known(skip)?.into_iter().take(n).map(|(x,t,s)|format!("{x} (from {t}{})",if s{", shaky"}else{""})).collect::<Vec<_>>();Ok(if out.is_empty(){vec![]}else{vec![format!("Concepts already learned in their other roadmaps: {}",out.join("; "))]})
 }
 pub(crate) fn mark_known(&self,mut c:Value) -> Result<Value> {
  fn words(s:&str)->HashSet<String>{let small="a an the of to in on for and or is are how why what with that this it its as by from not when which you your".split_whitespace().collect::<HashSet<_>>();regex::Regex::new(r"\w+").unwrap().find_iter(&s.to_lowercase()).map(|m|m.as_str().to_string()).filter(|s|!small.contains(s.as_str())).collect()}
  let seen=self.known(&string(&c["id"]))?.into_iter().filter(|(_,_,s)|!*s).map(|(x,t,_)|(words(&x),t)).collect::<Vec<_>>();
  for p in c["phases"].as_array_mut().into_iter().flatten(){for l in p["lessons"].as_array_mut().into_iter().flatten(){let mut known=json!({});for (i,x) in list(&l["concepts"]).iter().enumerate(){let w=words(&text(x));if let Some((_,t))=seen.iter().find(|(v,_)|{let n=w.intersection(v).count();n>=2&&n*2>=w.union(v).count()}){known[i.to_string()]=json!(t);}}l["known"]=known;}}Ok(c)
 }
 pub(crate) fn export_course(&self,c:&Value)->Result<Value>{
  let out=if self.home.join("Downloads").is_dir(){self.home.join("Downloads")}else{self.home.clone()};
  let re=regex::Regex::new(r"[^\w-]+").unwrap();let title=string(&c["title"]);let s=re.replace_all(&title,"-");let name=cap(s.trim_matches('-'),60);let name=if name.is_empty(){"roadmap"}else{&name};let f=out.join(format!("{name}.roadmap.json"));let mut c=c.clone();c.as_object_mut().unwrap().remove("folder");c.as_object_mut().unwrap().remove("work");fs::write(&f,serde_json::to_vec_pretty(&c)?)?;Ok(json!({"path":f}))
 }
 pub(crate) fn import_course(&self,path:&str)->Result<Value>{
  let f=Path::new(path);if !f.is_file() || f.metadata()?.len()>20_000_000{return Err(Error::bad("That is not a roadmap file."));}
  let d=fs::read(f).ok().and_then(|b|serde_json::from_slice::<Value>(&b).ok()).filter(Value::is_object).ok_or_else(||Error::bad("That is not a roadmap file."))?;
  let mut phases=vec![];let mut ids=HashSet::new();
  for p in list(&d["phases"]).iter().filter(|p|p.is_object()).take(30){let mut ls=vec![];
   for l in list(&p["lessons"]).iter().filter(|l|l.is_object()).take(40){let lid=text(&l["id"]);if !matches(r"^\d{1,3}\.\d{1,3}$",&lid)||!ids.insert(lid.clone()){continue;}let mut x=clean_lesson(l,&lid,&[]);
    x["videos"]=json!(list(&l["videos"]).iter().filter(|v|v.is_object()&&matches(r"^[\w-]{11}$",&text(&v["id"]))).take(4).map(|v|json!({"id":v["id"],"title":cap(&string(&v["title"]),200),"channel":cap(&string(&v["channel"]),100),"duration":v["duration"].as_i64().unwrap_or(0),"why":cap(&string(&v["why"]),300),"chapters":list(&v["chapters"]).iter().filter(|m|m.is_object()&&m["t"].is_i64()).take(40).map(|m|json!({"t":m["t"],"title":cap(&string(&m["title"]),120)})).collect::<Vec<_>>()})).collect::<Vec<_>>());
    if l["practice"].is_object(){x["practice"]=clean_practice(&l["practice"]);}ls.push(x);
   }if !ls.is_empty(){phases.push(json!({"title":cap(&if truth(&p["title"]){text(&p["title"])}else{"Phase".into()},200),"blurb":cap(&string(&p["blurb"]),600),"lessons":ls}));}
  }
  if phases.is_empty(){return Err(Error::bad("That file has no lessons in it."));}
  let old=&d["state"];let mut state=json!({});for k in ["checks","done","notes","code","quiz","review","tests"]{let mut keep=json!({});if let Some(o)=old[k].as_object(){for (a,b) in o{if !ids.contains(a.split('#').next().unwrap_or("")){continue;}let ok=match k{"checks"|"done"=>*b==true,"notes"|"code"=>b.is_string(),_=>{let pair=match k{"quiz"=>["score","total"],"review"=>["step","due"],_=>["runs","ok"]};b.as_object().is_some_and(|o|o.len()==2&&pair.iter().all(|k|o.contains_key(*k))&&o.values().all(|v|v.is_i64()||v.is_u64()||v.is_boolean()))}};if ok{keep[cap(a,20)]=match k{"notes"=>json!(cap(&text(b),20000)),"code"=>json!(cap(&text(b),100000)),_=>b.clone()};}}}state[k]=keep;}
  state["last"]=if ids.contains(&text(&old["last"])){old["last"].clone()}else{phases[0]["lessons"][0]["id"].clone()};
  let c=json!({"id":random_hex(6)?,"prompt":cap(&string(&d["prompt"]),2000),"level":cap(&string(&d["level"]),3000),"folder":null,"title":cap(&if truth(&d["title"]){text(&d["title"])}else{"Imported roadmap".into()},200),"summary":cap(&string(&d["summary"]),600),"phases":phases,"state":state});self.save(&c)?;Ok(c)
 }
}
