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
        let out = self.video_file(id);
        let yt = self.yt_path()?;
        // Without a JavaScript runtime YouTube shows yt-dlp no MP4 formats. Use one on PATH, else the app's own Deno.
        let mut js: Vec<String> = ["deno", "node", "bun"].iter().flat_map(|r| ["--js-runtimes".into(), r.to_string()]).collect();
        if !["deno", "node", "bun"].iter().any(|r| on_path(&format!("{r}{}", std::env::consts::EXE_SUFFIX))) {
            if let Some(deno) = self.deno_path() {
                js.extend(["--js-runtimes".into(), format!("deno:{}", deno.display())]);
            }
        }
        js.extend(["--remote-components".into(), "ejs:github".into()]);
        let run = |js: &[String]| {
            process(
                Command::new(&yt)
                    .args(["--no-warnings", "--no-playlist", "--no-mtime", "-f", "18/b[height<=480][ext=mp4]/b[ext=mp4]", "-o"])
                    .arg(&out)
                    .args(js)
                    .arg(format!("https://www.youtube.com/watch?v={id}")),
                1800,
            )
        };
        let mut r = run(&js);
        if matches!(&r, Ok((s, _, e, _)) if *s != 0 && e.contains("no such option")) {
            r = run(&[]);
        }
        match r {
            Ok((0, _, _, false)) if out.is_file() => Ok(()),
            Ok((_, _, _, true)) => Err(Error::bad("A video download took too long and was stopped.")),
            Ok((_, _, err, _)) => Err(Error::bad(format!("yt-dlp said: {}", cap(err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("yt-dlp failed"), 200)))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::bad("yt-dlp could not be started, so videos can't be saved.")),
            Err(e) => Err(e.into()),
        }
    }
    // The yt-dlp to run: the app's own copy in `<data>/bin`, downloaded from yt-dlp's releases on first use and checked for a
    // newer release once a run, so no system needs it installed. Distro packages lag and old copies stop working on YouTube,
    // so one on PATH is used only while the download fails.
    pub(crate) fn yt_path(&self) -> Result<PathBuf> {
        static FETCH: Mutex<()> = Mutex::new(());
        static CHECKED: AtomicBool = AtomicBool::new(false);
        // Tests set a full path to a script.
        if self.yt_program.parent().is_some_and(|p| !p.as_os_str().is_empty()) {
            return Ok(self.yt_program.clone());
        }
        let own = self.data.join("bin").join(&self.yt_program);
        if !own.is_file() {
            let _g = FETCH.lock().unwrap_or_else(|e| e.into_inner());
            if !own.is_file() {
                if let Err(why) = fetch_yt(&own) {
                    if on_path(&self.yt_program.to_string_lossy()) {
                        return Ok(self.yt_program.clone());
                    }
                    return Err(Error::bad(format!("yt-dlp, which finds videos, could not be downloaded: {why}. Check your internet connection and try again.")));
                }
                CHECKED.store(true, Ordering::Relaxed);
            }
        }
        // YouTube changes often and old yt-dlp copies stop working, so look for a new release in the background.
        if !CHECKED.swap(true, Ordering::Relaxed) {
            let own = own.clone();
            std::thread::spawn(move || {
                if let Err(why) = fetch_yt(&own) {
                    eprintln!("yt-dlp update failed: {why}");
                }
            });
        }
        Ok(own)
    }
    // The app's own Deno in `<data>/bin`, downloaded on first need. None when that fails; yt-dlp then reports the missing formats.
    fn deno_path(&self) -> Option<PathBuf> {
        static FETCH: Mutex<()> = Mutex::new(());
        let own = self.data.join("bin").join(format!("deno{}", std::env::consts::EXE_SUFFIX));
        let _g = FETCH.lock().unwrap_or_else(|e| e.into_inner());
        if !own.is_file() {
            if let Err(why) = fetch_deno(&own) {
                eprintln!("Deno download failed: {why}");
                return None;
            }
        }
        Some(own)
    }
}

fn on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(name).is_file()))
}
fn get(url: &str, max: u64) -> std::result::Result<Vec<u8>, String> {
    use std::io::Read;
    let r = crate::ai::agent(url, 600).and_then(|a| a.get(url).call()).map_err(|e| e.to_string())?;
    let mut b = vec![];
    r.into_reader().take(max).read_to_end(&mut b).map_err(|e| e.to_string())?;
    Ok(b)
}
fn sha256(b: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(b))
}
// Atomic, so a program starting meanwhile runs the old copy or the new one. On Windows a running copy can't be replaced; the next run retries.
fn install(own: &std::path::Path, bin: &[u8]) -> std::result::Result<(), String> {
    write_atomic(own, bin).map_err(|e| e.message)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(own, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// yt-dlp's standalone build for this system; it needs no Python.
const YT_ASSET: &str = if cfg!(windows) {
    "yt-dlp.exe"
} else if cfg!(target_os = "macos") {
    "yt-dlp_macos"
} else if cfg!(target_arch = "aarch64") {
    "yt-dlp_linux_aarch64"
} else {
    "yt-dlp_linux"
};
// Downloads the latest release to `own` unless `own` already is it. The file must match the release's SHA2-256SUMS.
fn fetch_yt(own: &std::path::Path) -> std::result::Result<(), String> {
    const BASE: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/";
    let sums = String::from_utf8_lossy(&get(&format!("{BASE}SHA2-256SUMS"), 1 << 20)?).into_owned();
    let want = sums
        .lines()
        .find_map(|l| l.split_once(char::is_whitespace).filter(|(_, n)| n.trim() == YT_ASSET).map(|(h, _)| h.to_lowercase()))
        .ok_or("this system has no yt-dlp build")?;
    if fs::read(own).is_ok_and(|b| sha256(&b) == want) {
        return Ok(());
    }
    let bin = get(&format!("{BASE}{YT_ASSET}"), 300 << 20)?;
    if sha256(&bin) != want {
        return Err("the download was damaged".into());
    }
    install(own, &bin)
}

const DENO_ASSET: &str = if cfg!(windows) {
    "deno-x86_64-pc-windows-msvc.zip"
} else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
    "deno-aarch64-apple-darwin.zip"
} else if cfg!(target_os = "macos") {
    "deno-x86_64-apple-darwin.zip"
} else if cfg!(target_arch = "aarch64") {
    "deno-aarch64-unknown-linux-gnu.zip"
} else {
    "deno-x86_64-unknown-linux-gnu.zip"
};
// The latest Deno release, checked against its .sha256sum (a `sha256sum` line, or PowerShell's table for Windows).
fn fetch_deno(own: &std::path::Path) -> std::result::Result<(), String> {
    use std::io::Read;
    const BASE: &str = "https://github.com/denoland/deno/releases/latest/download/";
    let sum = String::from_utf8_lossy(&get(&format!("{BASE}{DENO_ASSET}.sha256sum"), 4096)?).into_owned();
    let want = regex::Regex::new("[0-9a-fA-F]{64}").unwrap().find(&sum).ok_or("Deno published no checksum")?.as_str().to_lowercase();
    let zip = get(&format!("{BASE}{DENO_ASSET}"), 300 << 20)?;
    if sha256(&zip) != want {
        return Err("the download was damaged".into());
    }
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip)).map_err(|e| e.to_string())?;
    let name = own.file_name().and_then(|n| n.to_str()).ok_or("bad path")?;
    let mut bin = vec![];
    archive.by_name(name).map_err(|e| e.to_string())?.read_to_end(&mut bin).map_err(|e| e.to_string())?;
    install(own, &bin)
}

// Real downloads from GitHub: cargo test --lib tool_download -- --ignored --nocapture
#[cfg(test)]
#[test]
#[ignore]
fn tool_download() {
    let tmp = tempfile::tempdir().unwrap();
    let own = tmp.path().join("bin").join("yt-dlp");
    fetch_yt(&own).unwrap();
    let t = fs::metadata(&own).unwrap().modified().unwrap();
    fetch_yt(&own).unwrap(); // already the latest: not downloaded again
    assert_eq!(fs::metadata(&own).unwrap().modified().unwrap(), t);
    let deno = tmp.path().join("bin").join("deno");
    fetch_deno(&deno).unwrap();
    for p in [&own, &deno] {
        let (status, out, _, _) = process(Command::new(p).arg("--version"), 60).unwrap();
        assert_eq!(status, 0, "{out}");
        println!("{out}");
    }
    // With only the app's own Deno, YouTube offers the 480p MP4 that offline saving downloads.
    let (status, out, err, _) = process(
        Command::new(&own)
            .env("PATH", "/nonexistent")
            .args(["-s", "--no-warnings", "--print", "format_id", "-f", "18/b[height<=480][ext=mp4]/b[ext=mp4]", "--js-runtimes"])
            .arg(format!("deno:{}", deno.display()))
            .args(["--remote-components", "ejs:github", "https://www.youtube.com/watch?v=dQw4w9WgXcQ"]),
        300,
    )
    .unwrap();
    assert_eq!(status, 0, "{err}");
    println!("format {out}");
}
