//! "Save videos for offline": the main video of each written lesson is downloaded with yt-dlp into `<data>/videos/<id>.mp4`,
//! and the player server (`player.rs`) plays that copy. Lessons, notes and practice are saved on disk already.
use crate::exercise::process;
use crate::*;
use std::process::Command;

fn video_id(id: &str) -> bool {
    matches(r"^[A-Za-z0-9_-]{11}$", id)
}
// The video each written lesson shows first.
fn wanted(c: &Value) -> Vec<String> {
    let mut ids: Vec<String> = vec![];
    for l in lessons(c) {
        if let Some(id) = list(&l["videos"]).first().map(|v| string(&v["id"])).filter(|id| video_id(id) && !ids.contains(id)) {
            ids.push(id);
        }
    }
    ids
}

impl App {
    fn video_file(&self, id: &str) -> std::path::PathBuf {
        self.data.join("videos").join(format!("{id}.mp4"))
    }
    /// The ids of this roadmap's lesson videos that have a saved copy.
    pub(crate) fn saved_videos(&self, c: &Value) -> Vec<String> {
        let mut ids: Vec<String> = lessons(c).flat_map(|l| list(&l["videos"]).iter().map(|v| string(&v["id"]))).filter(|id| video_id(id)).collect();
        ids.sort();
        ids.dedup();
        ids.retain(|id| self.video_file(id).is_file());
        ids
    }
    pub(crate) fn offline_status(&self, cid: &str) -> Result<Value> {
        let c = self.load(cid)?;
        let want = wanted(&c);
        let sizes = want.iter().filter_map(|id| self.video_file(id).metadata().ok().map(|m| m.len())).collect::<Vec<_>>();
        let job = self.offline.lock().unwrap_or_else(|e| e.into_inner()).get(cid).cloned().unwrap_or_default();
        Ok(json!({"on":truth(&c["offline"]),"saved":sizes.len(),"total":want.len(),"bytes":sizes.iter().sum::<u64>(),
            "running":truth(&job["running"]),"failed":job["failed"].as_u64().unwrap_or(0),"error":string(&job["error"])}))
    }
    /// {"on": true} saves the videos in the background; {"on": false} stops and deletes them.
    pub(crate) fn set_offline(self: &Arc<Self>, cid: &str, b: &Value) -> Result<Value> {
        let on = truth(&b["on"]);
        {
            let _guard = self.guard();
            let mut c = self.load(cid)?;
            c["offline"] = json!(on);
            self.save(&c)?;
        }
        if on {
            self.save_videos(cid.to_string());
        } else {
            if let Some(job) = self.offline.lock().unwrap_or_else(|e| e.into_inner()).get_mut(cid) {
                job["stop"] = json!(true);
            }
            // A video another offline roadmap also shows stays.
            let keep = self.courses()?.iter().filter(|c| c["id"] != cid && truth(&c["offline"])).flat_map(|c| wanted(c)).collect::<HashSet<_>>();
            for id in wanted(&self.load(cid)?) {
                if !keep.contains(&id) {
                    let _ = fs::remove_file(self.video_file(&id));
                }
            }
        }
        self.offline_status(cid)
    }
    /// Starts the download of every missing video of an offline roadmap, one at a time, unless one is running for it already.
    pub(crate) fn save_videos(self: &Arc<Self>, cid: String) {
        {
            let mut jobs = self.offline.lock().unwrap_or_else(|e| e.into_inner());
            if truth(&jobs.get(&cid).map_or(Value::Null, |j| j["running"].clone())) {
                return;
            }
            jobs.insert(cid.clone(), json!({"running":true}));
        }
        let app = self.clone();
        std::thread::spawn(move || {
            let (mut failed, mut error) = (0, String::new());
            let want = app.load(&cid).map(|c| wanted(&c)).unwrap_or_default();
            let _ = fs::create_dir_all(app.data.join("videos"));
            for id in want {
                let stop = app.offline.lock().unwrap_or_else(|e| e.into_inner()).get(&cid).is_some_and(|j| truth(&j["stop"]));
                if stop {
                    break;
                }
                if app.video_file(&id).is_file() {
                    continue;
                }
                if let Err(e) = app.download(&id) {
                    failed += 1;
                    error = e.message;
                }
            }
            app.offline.lock().unwrap_or_else(|e| e.into_inner()).insert(cid, json!({"running":false,"failed":failed,"error":error}));
        });
    }
    // At most 480p in one MP4 file, so no separate audio track has to be merged (that would need ffmpeg).
    fn download(&self, id: &str) -> Result<()> {
        const JS: [&str; 8] = ["--js-runtimes", "deno", "--js-runtimes", "node", "--js-runtimes", "bun", "--remote-components", "ejs:github"];
        let out = self.video_file(id);
        let run = |js: &[&str]| {
            process(
                Command::new(&self.yt_program)
                    .args(["--no-warnings", "--no-playlist", "--no-mtime", "-f", "18/b[height<=480][ext=mp4]/b[ext=mp4]", "-o"])
                    .arg(&out)
                    .args(js)
                    .arg(format!("https://www.youtube.com/watch?v={id}")),
                1800,
            )
        };
        let mut r = run(&JS);
        if matches!(&r, Ok((s, _, e, _)) if *s != 0 && e.contains("no such option")) {
            r = run(&[]);
        }
        match r {
            Ok((0, _, _, false)) if out.is_file() => Ok(()),
            Ok((_, _, _, true)) => Err(Error::bad("A video download took too long and was stopped.")),
            Ok((_, _, err, _)) => Err(Error::bad(format!("yt-dlp said: {}", cap(err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("yt-dlp failed"), 200)))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::bad("yt-dlp is not installed, so videos can't be saved.")),
            Err(e) => Err(e.into()),
        }
    }
}
