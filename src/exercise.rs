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
            "rustc -o prog main.rs && ./prog",
            "rustc -o prog test.rs && ./prog",
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
pub(crate) fn clean_practice(d: &Value) -> Value {
    let quiz=list(&d["quiz"]).iter().filter_map(|q| {let opts=strs(&q["options"],6);let a=q["answer"].as_u64().or_else(||q["answer"].as_bool().map(u64::from))?;
 (opts.len()>1 && a<opts.len() as u64).then(||json!({"q":cap(&string(&q["q"]),1000),"options":opts,"answer":q["answer"],"why":cap(&string(&q["why"]),1000)})) }).take(10).collect::<Vec<_>>();
    let e = &d["exercise"];
    let exercise = if e.is_object() && language(&string(&e["lang"])).is_ok() && truth(&e["tests"]) {
        json!({"lang":e["lang"],"title":cap(&string(&e["title"]),200),"prompt":cap(&string(&e["prompt"]),4000),"starter":cap(&string(&e["starter"]),8000),"tests":cap(&string(&e["tests"]),20000)})
    } else {
        Value::Null
    };
    json!({"quiz":quiz,"exercise":exercise})
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
    let read = |mut pipe: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut data = Vec::new();
            let _ = pipe.read_to_end(&mut data);
            String::from_utf8_lossy(&data).into_owned()
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
        thread::sleep(Duration::from_millis(10));
    };
    Ok((
        status.code().unwrap_or(-1),
        out.join().unwrap_or_default(),
        err.join().unwrap_or_default(),
        timeout,
    ))
}
#[cfg(target_os = "linux")]
pub(crate) fn run_code(lang: &str, code: &str, tests: Option<&str>) -> Result<Value> {
    let ([name, pkg, prog, main, test, run, check], secs) = language(lang)?;
    if !std::path::Path::new("/usr/bin").join(prog).exists() {
        return Ok(
            json!({"ok":false,"missing":lang,"output":format!("{name} is not installed on this computer yet. Install it to run this exercise (package: {pkg}).")}),
        );
    }
    let d = tempfile::tempdir()?;
    fs::write(d.path().join(main), code)?;
    if let Some(t) = tests {
        fs::write(d.path().join(test), t)?;
    }
    let mut cmd = Command::new("bwrap");
    cmd.args([
        "--unshare-all",
        "--die-with-parent",
        "--new-session",
        "--ro-bind",
        "/",
        "/",
        "--dev",
        "/dev",
        "--proc",
        "/proc",
        "--tmpfs",
        "/tmp",
        "--tmpfs",
        "/run",
        "--tmpfs",
        "/home",
        "--tmpfs",
        "/root",
        "--tmpfs",
        "/var",
        "--tmpfs",
        "/mnt",
        "--tmpfs",
        "/media",
        "--bind",
    ])
    .arg(d.path())
    .args([
        "/tmp/work",
        "--chdir",
        "/tmp/work",
        "--clearenv",
        "--setenv",
        "PATH",
        "/usr/bin:/bin",
        "--setenv",
        "HOME",
        "/tmp/work",
        "--setenv",
        "PYTHONDONTWRITEBYTECODE",
        "1",
        "sh",
        "-c",
        if tests.is_some() { check } else { run },
    ]);
    let (status, out, err, timeout) = process(&mut cmd, secs).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::bad("bubblewrap is not installed. Run: sudo dnf install bubblewrap")
        } else {
            e.into()
        }
    })?;
    Ok(if timeout {
        json!({"ok":false,"output":format!("{}\nStopped: ran longer than {secs} seconds. Look for a loop that never ends.",tail(&out,20000))})
    } else {
        json!({"ok":status==0,"output":tail(&(out+&err),20000)})
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

#[cfg(not(target_os = "linux"))]
pub(crate) fn run_code(_lang: &str, _code: &str, _tests: Option<&str>) -> Result<Value> {
    Err(Error { status: 501, message: "Running exercises in a sandbox works only on Linux for now.".into() })
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn install_lang(_lang: &str) -> Result<Value> {
    Err(Error { status: 501, message: "Installing exercise runtimes works only on Linux for now.".into() })
}
