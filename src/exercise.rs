use crate::*;
use std::{
    io::Read,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
// name, Fedora package, /usr/bin program, source file, tests file, run, test, seconds
pub(crate) const LANGS: &[(&str, [&str; 7], u64)] = &[
    (
        "python",
        [
            "Python",
            "python3",
            "python3",
            "main.py",
            "test.py",
            "python3 main.py",
            "python3 test.py",
        ],
        10,
    ),
    (
        "javascript",
        [
            "JavaScript",
            "nodejs",
            "node",
            "main.js",
            "test.js",
            "node main.js",
            "node test.js",
        ],
        10,
    ),
    (
        "bash",
        [
            "Bash",
            "bash",
            "bash",
            "main.sh",
            "test.sh",
            "bash main.sh",
            "bash test.sh",
        ],
        10,
    ),
    (
        "ruby",
        [
            "Ruby",
            "ruby",
            "ruby",
            "main.rb",
            "test.rb",
            "ruby main.rb",
            "ruby test.rb",
        ],
        10,
    ),
    (
        "php",
        [
            "PHP",
            "php-cli",
            "php",
            "main.php",
            "test.php",
            "php main.php",
            "php test.php",
        ],
        10,
    ),
    (
        "perl",
        [
            "Perl",
            "perl",
            "perl",
            "main.pl",
            "test.pl",
            "perl main.pl",
            "perl test.pl",
        ],
        10,
    ),
    (
        "r",
        [
            "R",
            "R-core",
            "Rscript",
            "main.R",
            "test.R",
            "Rscript main.R",
            "Rscript test.R",
        ],
        30,
    ),
    (
        "lua",
        [
            "Lua",
            "lua",
            "lua",
            "main.lua",
            "test.lua",
            "lua main.lua",
            "lua test.lua",
        ],
        10,
    ),
    (
        "c",
        [
            "C",
            "gcc",
            "gcc",
            "main.c",
            "test.c",
            "gcc -Wall -o prog main.c -lm && ./prog",
            "gcc -o prog test.c -lm && ./prog",
        ],
        30,
    ),
    (
        "cpp",
        [
            "C++",
            "gcc-c++",
            "g++",
            "main.cpp",
            "test.cpp",
            "g++ -Wall -o prog main.cpp && ./prog",
            "g++ -o prog test.cpp && ./prog",
        ],
        30,
    ),
    (
        "rust",
        [
            "Rust",
            "rust",
            "rustc",
            "main.rs",
            "test.rs",
            // Without --edition rustc compiles as Rust 2015, which rejects async fn and lacks the 2021 prelude.
            "rustc --edition 2021 -o prog main.rs && ./prog",
            "rustc --edition 2021 -o prog test.rs && ./prog",
        ],
        60,
    ),
    (
        "go",
        [
            "Go",
            "golang",
            "go",
            "main.go",
            "main_test.go",
            "go run main.go",
            "go test main.go main_test.go",
        ],
        60,
    ),
    (
        "java",
        [
            "Java",
            "java-latest-openjdk-devel",
            "javac",
            "Main.java",
            "Test.java",
            "javac Main.java && java Main",
            "javac Main.java Test.java && java Test",
        ],
        60,
    ),
];
pub(crate) fn language(lang: &str) -> Result<([&'static str; 7], u64)> {
    LANGS
        .iter()
        .find(|(l, _, _)| *l == lang)
        .map(|(_, p, s)| (*p, *s))
        .ok_or_else(Error::missing)
}
pub(crate) fn clean_practice(d: &Value, max: usize) -> Value {
    let quiz=list(&d["quiz"]).iter().filter_map(|q| {let opts=strs(&q["options"],6);let a=q["answer"].as_u64().or_else(||q["answer"].as_bool().map(u64::from))?;
 (opts.len()>1 && a<opts.len() as u64).then(||{let mut x=json!({"q":cap(&string(&q["q"]),1000),"options":opts,"answer":q["answer"],"why":cap(&string(&q["why"]),1000)});
 // A stage exam question kept in the lesson's bank; the lesson quiz leaves it out.
 if q["exam"]==true {x["exam"]=json!(true);} x}) }).take(max).collect::<Vec<_>>();
    let e = &d["exercise"];
    let exercise = if e.is_object() && language(&string(&e["lang"])).is_ok() && truth(&e["tests"]) {
        json!({"lang":e["lang"],"title":cap(&string(&e["title"]),200),"prompt":cap(&string(&e["prompt"]),4000),"starter":cap(&string(&e["starter"]),8000),"tests":cap(&string(&e["tests"]),20000)})
    } else {
        Value::Null
    };
    let mut out = json!({"quiz":quiz,"exercise":exercise});
    // One open question per concept, asked in the daily review in place of the concept's own text.
    let recall = strs(&d["recall"], 8).iter().map(|q| cap(q, 300)).collect::<Vec<_>>();
    if !recall.is_empty() {
        out["recall"] = json!(recall);
    }
    out
}
// Read pipes concurrently so an exercise cannot block on a full stdout/stderr pipe.
pub(crate) fn process(
    cmd: &mut Command,
    secs: u64,
) -> std::io::Result<(i32, String, String, bool)> {
    process_with(cmd, secs, Stdio::null())
}
// The same, with a standard input: Codex reads its prompt from a file there.
pub(crate) fn process_with(
    cmd: &mut Command,
    secs: u64,
    input: Stdio,
) -> std::io::Result<(i32, String, String, bool)> {
    process_until(cmd, secs, input, None, None)
}
// Like process_with, and also kills the program as soon as `stop` becomes true. `pid` receives the program's process id.
pub(crate) fn process_until(
    cmd: &mut Command,
    secs: u64,
    input: Stdio,
    stop: Option<&AtomicBool>,
    pid: Option<&std::sync::atomic::AtomicU32>,
) -> std::io::Result<(i32, String, String, bool)> {
    process_hooked(cmd, secs, input, stop, pid, &|_| Ok(()))
}
// Like process_until, and runs `spawned` on the new program first; when it fails, the program is stopped.
pub(crate) fn process_hooked(
    cmd: &mut Command,
    secs: u64,
    input: Stdio,
    stop: Option<&AtomicBool>,
    pid: Option<&std::sync::atomic::AtomicU32>,
    spawned: &dyn Fn(&std::process::Child) -> std::io::Result<()>,
) -> std::io::Result<(i32, String, String, bool)> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd
        .stdin(input)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Err(e) = spawned(&child) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e);
    }
    if let Some(p) = pid {
        p.store(child.id(), Ordering::Relaxed);
    }
    // An exercise stuck printing in a loop would otherwise fill memory; past the cap its pipe closes and it stops.
    let read = |pipe: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut data = Vec::new();
            let _ = pipe.take(16 << 20).read_to_end(&mut data);
            // Valid text is kept as it is; only broken bytes make a second, repaired copy.
            String::from_utf8(data).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
        })
    };
    let out = read(Box::new(child.stdout.take().unwrap()));
    let err = read(Box::new(child.stderr.take().unwrap()));
    let start = Instant::now();
    let mut timeout = false;
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if start.elapsed() >= Duration::from_secs(secs) {
            timeout = true;
            child.kill()?;
            break child.wait()?;
        }
        if stop.is_some_and(|s| s.load(Ordering::Relaxed)) {
            child.kill()?;
            break child.wait()?;
        }
        thread::sleep(Duration::from_millis(10));
    };
    Ok((
        status.code().unwrap_or(-1),
        out.join().unwrap_or_default(),
        err.join().unwrap_or_default(),
        timeout,
    ))
}
// How one exercise run is limited beyond its time and output caps. Every limit belongs to the run alone (its own cgroup,
// its own mounts), so the learner's other programs never count against it. Each one is used only where this machine has it.
#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sandbox {
    // A systemd user scope (cgroup) caps processes and memory.
    pub(crate) scope: bool,
    // bubblewrap 0.10+ can size its tmpfs mounts, which caps total disk use; without it the work folder is watched instead.
    pub(crate) sized: bool,
    // Processes and threads together.
    pub(crate) tasks: u64,
    // All memory of the run in bytes, the work tmpfs included: tmpfs pages are charged to the run's cgroup.
    pub(crate) memory: u64,
    // Everything the run writes in bytes, and the most one file may hold.
    pub(crate) disk: u64,
}
#[cfg(target_os = "linux")]
impl Sandbox {
    // Past this the kernel slows the run instead of killing it, and the watcher stops it: a kernel OOM kill logs a call
    // trace that desktop problem reporters show as a crash. `memory` stays as the hard backstop.
    pub(crate) fn high(&self) -> u64 {
        self.memory / 8 * 7
    }
}
// A lesson exercise needs a few dozen MB; 512 MB covers a JVM (its default heap is a quarter of this, 128 MB) or a Go build.
pub(crate) const MEMORY_MAX: u64 = 512 << 20;
pub(crate) const DISK_MAX: u64 = 64 << 20;
// Exit status the sandbox script gives when the run filled its disk; the learner's program would have to pick it on purpose.
#[cfg(target_os = "linux")]
const DISK_FULL: i32 = 199;
// Threads count as tasks too, and Go and Java start a few per core, so many-core machines get more.
#[cfg(target_os = "linux")]
pub(crate) fn tasks_max() -> u64 {
    (thread::available_parallelism().map_or(1, |n| n.get()) as u64 * 16).max(256)
}
// Probed once per app run: what this machine supports does not change while the app is open.
#[cfg(target_os = "linux")]
pub(crate) fn sandbox() -> Sandbox {
    static FOUND: std::sync::OnceLock<Sandbox> = std::sync::OnceLock::new();
    *FOUND.get_or_init(|| {
        let mut sb = Sandbox { scope: false, sized: bwrap_sizes(), tasks: tasks_max(), memory: MEMORY_MAX, disk: DISK_MAX };
        sb.scope = scope_applies(&sb);
        sb
    })
}
// systemd-run can succeed while a cgroup controller is not delegated to the user, so the probe reads every limit back.
#[cfg(target_os = "linux")]
pub(crate) fn scope_applies(sb: &Sandbox) -> bool {
    let probe = process(
        Command::new("systemd-run")
            .args(["--user", "--scope", "--quiet", "--collect"])
            .arg(format!("--property=TasksMax={}", sb.tasks))
            .arg(format!("--property=MemoryHigh={}", sb.high()))
            .arg(format!("--property=MemoryMax={}", sb.memory))
            .args(["--property=MemorySwapMax=0", "--", "sh", "-c"])
            .arg(r#"c=/sys/fs/cgroup$(cut -d: -f3 /proc/self/cgroup); cat "$c/pids.max" "$c/memory.high" "$c/memory.max" "$c/memory.swap.max""#),
        5,
    );
    let want = [sb.tasks, sb.high(), sb.memory, 0].map(|n| n.to_string());
    matches!(probe, Ok((0, out, _, false)) if out.split_whitespace().eq(want.iter().map(String::as_str)))
}
#[cfg(target_os = "linux")]
fn bwrap_sizes() -> bool {
    let Ok((0, out, _, false)) = process(Command::new("bwrap").arg("--version"), 5) else {
        return false;
    };
    let v = out.split_whitespace().last().unwrap_or("").split('.').map(|n| n.parse().unwrap_or(0)).collect::<Vec<u64>>();
    v.len() > 1 && (v[0], v[1]) >= (0, 10)
}
fn dir_size(p: &std::path::Path) -> u64 {
    fs::read_dir(p)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => dir_size(&e.path()),
            Ok(t) if t.is_file() => e.metadata().map_or(0, |m| m.len()),
            _ => 0,
        })
        .sum()
}
// Size and free space of a file system, in bytes.
#[cfg(target_os = "linux")]
#[allow(clippy::unnecessary_cast)] // the field types differ between 32- and 64-bit systems
fn file_system_space(path: &str) -> Option<(u64, u64)> {
    let path = std::ffi::CString::new(path).ok()?;
    // SAFETY: statvfs only writes into the zeroed struct it is given, and the path is NUL-terminated.
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    (unsafe { libc::statvfs(path.as_ptr(), &mut s) } == 0).then(|| (s.f_blocks as u64 * s.f_frsize as u64, s.f_bavail as u64 * s.f_frsize as u64))
}
// A process of the run inside the sandbox (its own mount namespace), found among the descendants of `top`.
#[cfg(target_os = "linux")]
fn sandbox_process(top: u32) -> Option<u32> {
    let ours = fs::read_link("/proc/self/ns/mnt").ok()?;
    let mut next = vec![top];
    while let Some(p) = next.pop() {
        if fs::read_link(format!("/proc/{p}/ns/mnt")).is_ok_and(|ns| ns != ours) {
            return Some(p);
        }
        next.extend(fs::read_to_string(format!("/proc/{p}/task/{p}/children")).unwrap_or_default().split_whitespace().filter_map(|c| c.parse::<u32>().ok()));
    }
    None
}
// Scope names say which app process and thread started them, so a leftover can be traced to its run.
#[cfg(target_os = "linux")]
pub(crate) fn unit_prefix() -> String {
    let thread = format!("{:?}", thread::current().id()).chars().filter(char::is_ascii_digit).collect::<String>();
    format!("roadmap-exercise-{}-{thread}-", std::process::id())
}
// A counter from a cgroup events file, such as "max 3" in pids.events.
#[cfg(target_os = "linux")]
fn cgroup_count(cg: &str, file: &str, key: &str) -> u64 {
    fs::read_to_string(format!("{cg}/{file}")).unwrap_or_default().lines().find_map(|l| l.strip_prefix(key)?.trim().parse().ok()).unwrap_or(0)
}
#[cfg(target_os = "linux")]
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Limit {
    Processes,
    Memory,
    Disk,
}
// What the learner is told when a limit stopped the run.
#[cfg(target_os = "linux")]
pub(crate) fn limit_message(l: Limit, sb: &Sandbox) -> String {
    match l {
        Limit::Processes => format!("Stopped: your program started too many processes or threads (the limit is {}). Look for a loop that keeps starting new ones.", sb.tasks),
        Limit::Memory => format!("Stopped: your program used too much memory (the limit is {} MB). Look for a list, string or recursion that keeps growing.", sb.high() >> 20),
        Limit::Disk => format!("Stopped: your program wrote too much to disk (the limit is {} MB). Look for a loop that keeps writing files.", sb.disk >> 20),
    }
}
// The last n characters of the output followed by the errors, without first joining two outputs of up to 16 MiB each.
#[cfg(target_os = "linux")]
fn tail_both(out: &str, err: &str, n: usize) -> String {
    let err = tail(err, n);
    let left = n.saturating_sub(err.chars().count());
    if left == 0 { err } else { tail(out, left) + &err }
}
#[cfg(target_os = "linux")]
pub(crate) fn run_code(lang: &str, code: &str, tests: Option<&str>) -> Result<Value> {
    run_in(lang, code, tests, sandbox())
}
#[cfg(target_os = "linux")]
pub(crate) fn run_in(lang: &str, code: &str, tests: Option<&str>, sb: Sandbox) -> Result<Value> {
    let ([name, pkg, prog, main, test, run, check], secs) = language(lang)?;
    if !std::path::Path::new("/usr/bin").join(prog).exists() {
        return Ok(
            json!({"ok":false,"missing":lang,"output":format!("{name} is not installed on this computer yet. Install it to run this exercise (package: {pkg}).")}),
        );
    }
    // One run at a time per app, so the limits of two runs can never add up; a second run waits (at most one time limit).
    static ONE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
    // The work folder sits inside a private folder: the run sees only the work folder, never the file naming its cgroup.
    let outer = tempfile::tempdir()?;
    let work = outer.path().join("work");
    fs::create_dir(&work)?;
    fs::write(work.join(main), code)?;
    if let Some(t) = tests {
        fs::write(work.join(test), t)?;
    }
    let cg_file = outer.path().join("cgroup");
    let mut cmd = if sb.scope {
        let mut c = Command::new("systemd-run");
        c.args(["--user", "--scope", "--quiet", "--collect"])
            .arg(format!("--unit={}{}", unit_prefix(), random_hex(6)?))
            .arg(format!("--property=TasksMax={}", sb.tasks))
            .arg(format!("--property=MemoryHigh={}", sb.high()))
            .arg(format!("--property=MemoryMax={}", sb.memory))
            .args(["--property=MemorySwapMax=0", "--property=CPUWeight=50", "--", "sh", "-c"])
            // Notes the scope's cgroup for the watcher below, then becomes bwrap, so a timeout kills the sandbox itself.
            .arg(r#"echo "/sys/fs/cgroup$(cut -d: -f3 /proc/self/cgroup)" > "$0"; exec "$@""#)
            .arg(&cg_file)
            .arg("bwrap");
        c
    } else {
        Command::new("bwrap")
    };
    cmd.args(["--unshare-all", "--die-with-parent", "--new-session", "--ro-bind", "/", "/", "--dev", "/dev", "--proc", "/proc"]);
    // The folders that would show the computer's own files are hidden behind empty read-only mounts, which hold no memory.
    for d in ["/run", "/home", "/root", "/var", "/mnt", "/media"] {
        cmd.args(["--tmpfs", d, "--remount-ro", d]);
    }
    // Shared memory gets 1 MB where mounts can be sized (enough for a lock or a small queue), else none.
    if sb.sized {
        cmd.args(["--size", "1048576", "--tmpfs", "/dev/shm"]);
    } else {
        cmd.args(["--tmpfs", "/dev/shm", "--remount-ro", "/dev/shm"]);
    }
    // With a sized /tmp the files are copied in, so everything the run writes counts against that size.
    if sb.sized {
        cmd.arg("--size").arg(sb.disk.to_string()).args(["--tmpfs", "/tmp"]);
        cmd.arg("--ro-bind").arg(&work).args(["/tmp/.in", "--dir", "/tmp/work"]);
    } else {
        cmd.args(["--tmpfs", "/tmp"]).arg("--bind").arg(&work).arg("/tmp/work");
    }
    let program = if tests.is_some() { check } else { run };
    let blocks = sb.disk / 512;
    // SIGXFSZ is ignored so a write past the one-file limit fails with EFBIG instead of a core dump (and a desktop crash report);
    // a file left at the limit size then means the limit was hit, reported as 153 like the signal would be.
    let file_check = format!("rc=$?; [ -n \"$(find /tmp -type f -size +{} -print -quit 2>/dev/null)\" ] && exit 153", blocks - 1);
    let script = if sb.sized {
        format!("trap '' XFSZ; ulimit -c 0; ulimit -f {blocks}; cp -R /tmp/.in/. . || exit 1\n{program}\n{file_check}; set -- $(df -Pk /tmp | tail -n 1); [ \"$4\" -lt 1024 ] && exit {DISK_FULL}; exit $rc")
    } else {
        format!("trap '' XFSZ; ulimit -c 0; ulimit -f {blocks}; {program}\n{file_check}; exit $rc")
    };
    cmd.args([
        "--chdir", "/tmp/work", "--clearenv", "--setenv", "PATH", "/usr/bin:/bin", "--setenv", "HOME", "/tmp/work",
        "--setenv", "PYTHONDONTWRITEBYTECODE", "1", "sh", "-c", &script,
    ]);
    // Watches the run's cgroup (and, without a sized /tmp, its folder) and stops it at the first limit it reaches.
    let (stop, done, hit) = (AtomicBool::new(false), AtomicBool::new(false), std::sync::Mutex::new(None));
    let top = std::sync::atomic::AtomicU32::new(0);
    let result = thread::scope(|s| {
        let watcher = s.spawn(|| {
            let (mut cg, mut inside) = (String::new(), 0);
            // A full sized /tmp is seen from outside through a sandbox process. Many programs ignore a full disk and go on
            // (Python reports it only when a file is closed, as an ignored error), so waiting for the program to end is not enough.
            let mut disk_full = || {
                let top = top.load(Ordering::Relaxed);
                if top == 0 {
                    return false;
                }
                if inside == 0 || !std::path::Path::new(&format!("/proc/{inside}")).exists() {
                    inside = sandbox_process(top).unwrap_or(0);
                }
                // The size check makes sure this is the run's own /tmp, never the computer's.
                inside != 0 && file_system_space(&format!("/proc/{inside}/root/tmp")).is_some_and(|(size, free)| size <= sb.disk && free < 1 << 20)
            };
            while !done.load(Ordering::Relaxed) {
                if sb.scope && cg.is_empty() {
                    cg = fs::read_to_string(&cg_file).unwrap_or_default().trim().to_string();
                }
                let found = if !cg.is_empty() && cgroup_count(&cg, "pids.events", "max ") > 0 {
                    Some(Limit::Processes)
                } else if !cg.is_empty() && (cgroup_count(&cg, "memory.events", "high ") > 0 || cgroup_count(&cg, "memory.events", "oom_kill ") > 0) {
                    Some(Limit::Memory)
                } else if if sb.sized { disk_full() } else { dir_size(&work) > sb.disk } {
                    Some(Limit::Disk)
                } else {
                    None
                };
                if found.is_some() {
                    *hit.lock().unwrap_or_else(|e| e.into_inner()) = found;
                    stop.store(true, Ordering::Relaxed);
                    break;
                }
                // Woken early when the run ends, so an honest run never waits for the next check.
                thread::park_timeout(Duration::from_millis(50));
            }
        });
        let r = process_until(&mut cmd, secs, Stdio::null(), Some(&stop), Some(&top));
        done.store(true, Ordering::Relaxed);
        watcher.thread().unpark();
        r
    });
    let (status, out, err, timeout) = result.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::bad("bubblewrap is not installed. Run: sudo dnf install bubblewrap")
        } else {
            e.into()
        }
    })?;
    let limit = match *hit.lock().unwrap_or_else(|e| e.into_inner()) {
        Some(l) => Some(l),
        None if timeout => None,
        // By default systemd stops the whole scope when its memory runs out, so the sandbox ends by a signal before the watcher sees it.
        None if sb.scope && status == -1 => Some(Limit::Memory),
        // 153 is a shell's status for a program stopped by the one-file limit (SIGXFSZ).
        None if status == 153 || (sb.sized && status == DISK_FULL) => Some(Limit::Disk),
        None => None,
    };
    Ok(match limit {
        Some(l) => json!({"ok":false,"output":format!("{}\n{}", tail_both(&out, &err, 20000), limit_message(l, &sb))}),
        None if timeout => {
            json!({"ok":false,"output":format!("{}\nStopped: ran longer than {secs} seconds. Look for a loop that never ends.",tail(&out,20000))})
        }
        None => json!({"ok":status==0,"output":tail_both(&out, &err, 20000)}),
    })
}
#[cfg(target_os = "linux")]
pub(crate) fn install_lang(lang: &str) -> Result<Value> {
    let ([name, pkg, prog, ..], _) = language(lang)?;
    let installed = || std::path::Path::new("/usr/bin").join(prog).exists();
    if installed() {
        return Ok(json!({"ok":true,"output":format!("{name} is already installed.")}));
    }
    let (status, out, err, timeout) = process(
        Command::new("pkexec").args(["dnf", "install", "-y", pkg]),
        1800,
    )
    .map_err(|_| {
        Error::bad(format!(
            "Can't install from inside the app on this system. Run: sudo dnf install {pkg}"
        ))
    })?;
    if timeout {
        return Err(Error::bad(format!(
            "The install took too long and was stopped. Run: sudo dnf install {pkg}"
        )));
    }
    if [126, 127].contains(&status) {
        return Err(Error::bad(format!("Nothing was installed, because the password dialog was cancelled. You can also run: sudo dnf install {pkg}")));
    }
    let ok = installed();
    Ok(
        json!({"ok":ok,"output":if ok{format!("{name} is installed.")}else{format!("Install failed. Try: sudo dnf install {pkg}\n{}",tail(&(out+&err),2000))}}),
    )
}

impl App {
    /// Linux runs exercises in a full sandbox. Windows gets only some limits and macOS fewer (see the non-Linux `run_code`), so the learner must allow runs in Settings first.
    pub(crate) fn may_run(&self) -> Result<()> {
        if cfg!(target_os = "linux") || truth(&self.config()["unsandboxed"]) {
            return Ok(());
        }
        Err(Error { status: 403, message: "On this computer exercises run with only some limits, not a full sandbox. Turn on “Run exercises on this computer” in Settings to use them.".into() })
    }
}

// The program a step names, with the Windows spellings: `python` when only that exists, and `.exe` on what the compilers write.
#[cfg(not(target_os = "linux"))]
fn local_program(prog: &str) -> Option<String> {
    let prog = if prog == "python3" && !on_path("python3") && on_path("python") { "python" } else { prog };
    on_path(prog).then(|| prog.to_string())
}

// Windows: a Job object holds every process of one step. It caps their memory together and how many there are, and
// closing it (when the step ends) stops any the program left behind. It has no network or file system walls.
#[cfg(windows)]
struct Job(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl Job {
    fn new() -> std::io::Result<Job> {
        use windows_sys::Win32::System::JobObjects::*;
        // SAFETY: plain Win32 calls on a handle this struct owns; the info struct is fully initialised (zeroed, then set).
        unsafe {
            let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if h.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let job = Job(h);
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
                | JOB_OBJECT_LIMIT_JOB_MEMORY
                | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
            info.BasicLimitInformation.ActiveProcessLimit = 64;
            info.JobMemoryLimit = MEMORY_MAX as usize;
            let size = std::mem::size_of_val(&info) as u32;
            if SetInformationJobObject(h, JobObjectExtendedLimitInformation, &info as *const _ as *const _, size) == 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(job)
        }
    }
    // The program could start another in the moment before this; Windows puts the ones it starts later in the job too.
    fn add(&self, child: &std::process::Child) -> std::io::Result<()> {
        use std::os::windows::io::AsRawHandle;
        // SAFETY: both handles are open: the job is owned by self, the process by `child`.
        if unsafe { windows_sys::Win32::System::JobObjects::AssignProcessToJobObject(self.0, child.as_raw_handle() as _) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}
#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        // SAFETY: the handle came from CreateJobObjectW and is closed only here.
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}
// Windows and macOS, after the learner allows it in Settings: the time and output caps, a temporary folder deleted
// afterwards, and a watch that stops a run writing more than DISK_MAX there. Windows adds a Job object (memory and
// process caps) but can still use the network and write elsewhere. macOS is not released, so it has nothing more.
#[cfg(not(target_os = "linux"))]
pub(crate) fn run_code(lang: &str, code: &str, tests: Option<&str>) -> Result<Value> {
    let ([name, _, prog, main, test, run, check], secs) = language(lang)?;
    let Some(found) = local_program(prog) else {
        return Ok(json!({"ok":false,"missing":lang,"output":format!("{name} is not installed on this computer yet. Install it to run this exercise.")}));
    };
    static ONE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let work = tempfile::tempdir()?;
    fs::write(work.path().join(main), code)?;
    if let Some(t) = tests {
        fs::write(work.path().join(test), t)?;
    }
    let exe = if cfg!(windows) { "prog.exe" } else { "prog" };
    let start = Instant::now();
    let mut output = String::new();
    // Each step runs directly, never through a shell, so a timeout kills the learner's program itself.
    for step in if tests.is_some() { check } else { run }.split(" && ") {
        let mut words = step.split_whitespace().map(|w| match w {
            "prog" => exe.to_string(),
            "./prog" => work.path().join(exe).to_string_lossy().into_owned(),
            w if w == prog => found.clone(),
            w => w.to_string(),
        });
        let Some(program) = words.next() else { continue };
        let left = secs.saturating_sub(start.elapsed().as_secs()).max(1);
        let mut cmd = Command::new(program);
        cmd.args(words).current_dir(work.path());
        #[cfg(windows)]
        let job = Job::new()?;
        #[cfg(windows)]
        let spawned = |c: &std::process::Child| job.add(c);
        #[cfg(not(windows))]
        let spawned = |_: &std::process::Child| Ok(());
        // Stops the step when the folder grows past DISK_MAX.
        let (stop, done, full) = (AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false));
        let (status, out, err, timeout) = thread::scope(|scope| {
            scope.spawn(|| {
                while !done.load(Ordering::Relaxed) {
                    if dir_size(work.path()) > DISK_MAX {
                        full.store(true, Ordering::Relaxed);
                        stop.store(true, Ordering::Relaxed);
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            });
            let r = process_hooked(&mut cmd, left, Stdio::null(), Some(&stop), None, &spawned);
            done.store(true, Ordering::Relaxed);
            r
        })?;
        output.push_str(&out);
        output.push_str(&err);
        if full.load(Ordering::Relaxed) {
            return Ok(json!({"ok":false,"output":format!("{}\nStopped: wrote more than {} MB of files.", tail(&output, 20000), DISK_MAX >> 20)}));
        }
        if timeout {
            return Ok(json!({"ok":false,"output":format!("{}\nStopped: ran longer than {secs} seconds. Look for a loop that never ends.", tail(&output, 20000))}));
        }
        if status != 0 {
            return Ok(json!({"ok":false,"output":tail(&output, 20000)}));
        }
    }
    Ok(json!({"ok":true,"output":tail(&output, 20000)}))
}

// Homebrew formula names of the exercise languages.
#[cfg(not(target_os = "linux"))]
fn brew_name(lang: &str) -> &'static str {
    match lang {
        "javascript" => "node",
        "cpp" | "c" => "gcc",
        "java" => "openjdk",
        "r" => "r",
        _ => LANGS.iter().find(|(l, _, _)| *l == lang).map_or("", |(l, _, _)| *l),
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn install_lang(lang: &str) -> Result<Value> {
    let ([name, _, prog, ..], _) = language(lang)?;
    if local_program(prog).is_some() {
        return Ok(json!({"ok":true,"output":format!("{name} is already installed.")}));
    }
    if cfg!(target_os = "macos") && on_path("brew") {
        let pkg = brew_name(lang);
        let (status, out, err, timeout) = process(Command::new("brew").args(["install", pkg]), 1800)?;
        let ok = !timeout && status == 0 && local_program(prog).is_some();
        return Ok(json!({"ok":ok,"output":if ok {format!("{name} is installed.")} else {format!("Install failed. Try in Terminal: brew install {pkg}\n{}", tail(&(out + &err), 2000))}}));
    }
    Err(Error::bad(format!("Install {name} from its website, make sure “{prog}” is on PATH, then restart Roadmap.")))
}

/// Installs a tool the roadmap's "Before you start" list names, through the system's package manager.
pub(crate) fn install_tool(cmd: &str) -> Result<Value> {
    if !matches(r"^[A-Za-z0-9_+-][A-Za-z0-9_.+-]{0,39}$", cmd) {
        return Err(Error::bad("That is not a program name."));
    }
    if on_path(cmd) {
        return Ok(json!({"ok":true,"output":format!("{cmd} is already installed.")}));
    }
    // DNF finds the package that provides the program file, so the AI's command name never becomes a package name.
    let (program, args, again): (&str, Vec<String>, String) = if cfg!(target_os = "linux") {
        ("pkexec", vec!["dnf".into(), "install".into(), "-y".into(), format!("/usr/bin/{cmd}")], format!("sudo dnf install /usr/bin/{cmd}"))
    } else if cfg!(target_os = "macos") && on_path("brew") {
        ("brew", vec!["install".into(), cmd.into()], format!("brew install {cmd}"))
    } else {
        return Err(Error::bad(format!("Install {cmd} from its website, then press Check again.")));
    };
    let (status, out, err, timeout) = process(Command::new(program).args(&args), 1800)
        .map_err(|_| Error::bad(format!("Can't install from inside the app on this system. Run: {again}")))?;
    if timeout {
        return Err(Error::bad(format!("The install took too long and was stopped. Run: {again}")));
    }
    if cfg!(target_os = "linux") && [126, 127].contains(&status) {
        return Err(Error::bad(format!("Nothing was installed, because the password dialog was cancelled. You can also run: {again}")));
    }
    let ok = on_path(cmd);
    Ok(json!({"ok":ok,"output":if ok {format!("{cmd} is installed.")} else {format!("Install failed. Try: {again}\n{}", tail(&(out + &err), 2000))}}))
}
