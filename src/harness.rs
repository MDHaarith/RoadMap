//! Connects the roadmap-mcp server to the AI coding tools on this computer, finds OpenCode and installs it.
use crate::*;
use std::path::{Path, PathBuf};

/// The server's name in every tool's settings.
const NAME: &str = "roadmap";

/// Where the AI tools keep their settings: the home folder and the system's per-user config folder.
pub struct Dirs {
    pub home: PathBuf,
    pub config: PathBuf,
    pub opencode: bool,
}
impl Dirs {
    pub fn here() -> Option<Self> {
        Some(Self { home: dirs::home_dir()?, config: dirs::config_dir()?, opencode: find_opencode().is_some() })
    }
}

/// One tool's settings file, whether the tool is on this computer, and the entry that starts the server.
struct Tool {
    name: &'static str,
    file: PathBuf,
    present: bool,
    key: &'static str,
    entry: Value,
}
fn tools(d: &Dirs, cmd: &str) -> Vec<Tool> {
    let (h, c) = (&d.home, &d.config);
    let plain = json!({"command":cmd,"args":[]});
    let tool = |name, file: PathBuf, present, key, entry| Tool { name, file, present, key, entry };
    vec![
        // Claude Code writes this file on its first start; without it there is nothing to add to.
        tool("Claude Code", h.join(".claude.json"), h.join(".claude.json").is_file(), "mcpServers", json!({"type":"stdio","command":cmd,"args":[],"env":{}})),
        tool("Claude Desktop", c.join("Claude").join("claude_desktop_config.json"), c.join("Claude").is_dir(), "mcpServers", plain.clone()),
        // OpenCode reads ~/.config/opencode on every system, Windows included.
        tool("OpenCode", h.join(".config").join("opencode").join("opencode.json"), d.opencode || h.join(".config").join("opencode").is_dir(), "mcp", json!({"type":"local","command":[cmd],"enabled":true})),
        tool("Gemini CLI", h.join(".gemini").join("settings.json"), h.join(".gemini").is_dir(), "mcpServers", plain.clone()),
        tool("Cursor", h.join(".cursor").join("mcp.json"), h.join(".cursor").is_dir(), "mcpServers", plain.clone()),
        tool("VS Code", c.join("Code").join("User").join("mcp.json"), c.join("Code").join("User").is_dir(), "servers", json!({"type":"stdio","command":cmd})),
        tool("Windsurf", h.join(".codeium").join("windsurf").join("mcp_config.json"), h.join(".codeium").join("windsurf").is_dir(), "mcpServers", plain),
    ]
}
fn codex_file(d: &Dirs) -> PathBuf {
    std::env::var_os("CODEX_HOME").map(PathBuf::from).unwrap_or_else(|| d.home.join(".codex")).join("config.toml")
}

/// Adds the server (`mcp` set) to every AI tool found here, or removes it (`None`) from every tool that has it.
/// Each result is the tool's name and whether its settings changed; a file that is not plain JSON is left alone.
pub fn connect(d: &Dirs, mcp: Option<&Path>) -> Vec<(String, std::result::Result<bool, String>)> {
    let cmd = mcp.map(|p| p.to_string_lossy().into_owned());
    let mut out = vec![];
    for t in tools(d, cmd.as_deref().unwrap_or("")) {
        if if cmd.is_some() { t.present } else { t.file.is_file() } {
            out.push((t.name.to_string(), edit_json(&t.file, t.key, cmd.as_ref().map(|_| &t.entry))));
        }
    }
    let codex = codex_file(d);
    if if cmd.is_some() { codex.parent().is_some_and(Path::is_dir) } else { codex.is_file() } {
        out.push(("Codex".into(), edit_codex(&codex, cmd.as_deref())));
    }
    out
}

fn edit_json(file: &Path, key: &str, entry: Option<&Value>) -> std::result::Result<bool, String> {
    let mut v: Value = match fs::read(file) {
        Ok(b) if b.iter().all(u8::is_ascii_whitespace) => json!({}),
        Ok(b) => serde_json::from_slice(&b).map_err(|_| "its settings file has comments or is not plain JSON, so it was left as it is".to_string())?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(e) => return Err(e.to_string()),
    };
    if !v.is_object() || v.get(key).is_some_and(|s| !s.is_null() && !s.is_object()) {
        return Err("its settings file has an unexpected shape, so it was left as it is".into());
    }
    let now = v.get(key).and_then(|s| s.get(NAME));
    match entry {
        // An entry that already starts this server stays as it is, with any changes the learner made to it.
        Some(e) if now.is_some_and(|n| n["command"] == e["command"]) => return Ok(false),
        Some(e) => v[key][NAME] = e.clone(),
        None if now.is_none() => return Ok(false),
        None => { v[key].as_object_mut().map(|s| s.remove(NAME)); }
    }
    let mut bytes = serde_json::to_vec_pretty(&v).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    write_atomic(file, &bytes).map_err(|e| e.message)?;
    Ok(true)
}

/// Codex keeps TOML; the server's table (and any sub-tables) is replaced or removed as text, and the rest of the file is kept as written.
fn edit_codex(file: &Path, cmd: Option<&str>) -> std::result::Result<bool, String> {
    let text = match fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.to_string()),
    };
    let (head, sub) = (format!("[mcp_servers.{NAME}]"), format!("[mcp_servers.{NAME}."));
    let line = cmd.map(|c| format!("command = \"{}\"", c.replace('\\', "\\\\").replace('"', "\\\"")));
    let (mut kept, mut ours, mut at, mut inside) = (vec![], vec![], None, false);
    for l in text.lines() {
        let t = l.trim();
        if t.starts_with('[') {
            inside = t == head || t.starts_with(&sub);
            if inside && at.is_none() {
                at = Some(kept.len());
            }
        }
        if inside { ours.push(l) } else { kept.push(l) }
    }
    match (&line, at) {
        (Some(l), Some(_)) if ours.iter().any(|x| x.trim() == l) => return Ok(false),
        (None, None) => return Ok(false),
        _ => {}
    }
    let block = line.map(|l| format!("{head}\n{l}\n"));
    let mut lines: Vec<String> = kept.iter().map(|s| s.to_string()).collect();
    if let Some(b) = block {
        let at = at.unwrap_or(lines.len());
        // A blank line before the table, unless it starts the file or follows one.
        let gap = at > 0 && !lines[at - 1].trim().is_empty();
        lines.insert(at, format!("{}{}", if gap { "\n" } else { "" }, b.trim_end()));
        if at + 1 < lines.len() && !lines[at + 1].trim().is_empty() {
            lines.insert(at + 1, String::new());
        }
    }
    write_atomic(file, format!("{}\n", lines.join("\n").trim_end()).as_bytes()).map_err(|e| e.message)?;
    Ok(true)
}

/// The installed roadmap-mcp beside this program, unless it is a development build or lives inside an AppImage,
/// whose files move on every start and would leave the tools pointing nowhere.
pub fn mcp_path() -> Option<PathBuf> {
    if cfg!(debug_assertions) || std::env::var_os("APPIMAGE").is_some() {
        return None;
    }
    let p = std::env::current_exe().ok()?.with_file_name(format!("roadmap-mcp{}", std::env::consts::EXE_SUFFIX));
    p.is_file().then_some(p)
}
/// Marks that the learner turned the connection off, so the app stops adding it back on start.
pub fn off_marker(data: &Path) -> PathBuf {
    data.join("mcp-off")
}
/// Run on every app start: keeps the tools pointing at this copy, and reaches tools installed since.
pub fn keep_connected(data: &Path) {
    if off_marker(data).exists() {
        return;
    }
    if let (Some(d), Some(mcp)) = (Dirs::here(), mcp_path()) {
        for (name, r) in connect(&d, Some(&mcp)) {
            if let Err(why) = r {
                eprintln!("Roadmap tools were not added to {name}: {why}");
            }
        }
    }
}

/// The OpenCode program: on the PATH, or where its installers put it (a desktop launch often has a short PATH).
pub fn find_opencode() -> Option<PathBuf> {
    find("opencode", &[".opencode/bin"])
}
/// Claude Code's own installer puts it in ~/.local/bin (Windows too); older ones in ~/.claude/local.
pub fn find_claude() -> Option<PathBuf> {
    find("claude", &[".claude/local"])
}
/// A program on PATH, in one of `own` (folders in the home folder), or where npm, bun or Homebrew put programs.
/// An app opened from the desktop often has a shorter PATH than a terminal, hence the usual places.
fn find(name: &str, own: &[&str]) -> Option<PathBuf> {
    let names: Vec<String> = if cfg!(windows) { vec![format!("{name}.exe"), format!("{name}.cmd")] } else { vec![name.into()] };
    let mut places: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    if let Some(h) = dirs::home_dir() {
        places.extend(own.iter().map(|d| h.join(d)));
        places.extend([h.join(".local").join("bin"), h.join(".bun").join("bin"), h.join(".npm-global").join("bin")]);
    }
    if let Some(a) = std::env::var_os("APPDATA") {
        places.push(PathBuf::from(a).join("npm"));
    }
    places.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from));
    places.iter().flat_map(|d| names.iter().map(move |n| d.join(n))).find(|p| p.is_file())
}

/// Downloads OpenCode from its GitHub releases into ~/.opencode/bin, where its own install script puts it.
pub fn install_opencode() -> std::result::Result<PathBuf, String> {
    if let Some(p) = find_opencode() {
        return Ok(p);
    }
    use std::io::Read;
    // shortcut: always the AVX2 build; a CPU without AVX2 needs the "-baseline" download, add it if someone hits that.
    let target = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "windows-x64.zip",
        ("windows", "aarch64") => "windows-arm64.zip",
        ("macos", "x86_64") => "darwin-x64.zip",
        ("macos", "aarch64") => "darwin-arm64.zip",
        ("linux", "x86_64") => "linux-x64.tar.gz",
        ("linux", "aarch64") => "linux-arm64.tar.gz",
        _ => return Err("OpenCode has no download for this computer.".into()),
    };
    let url = format!("https://github.com/anomalyco/opencode/releases/latest/download/opencode-{target}");
    let r = crate::ai::agent(&url, 600).and_then(|a| a.get(&url).call()).map_err(|e| format!("Could not download OpenCode: {e}"))?;
    let mut archive = vec![];
    r.into_reader().take(400 << 20).read_to_end(&mut archive).map_err(|e| format!("Could not download OpenCode: {e}"))?;
    let dir = dirs::home_dir().ok_or("No home folder.")?.join(".opencode").join("bin");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = format!("opencode{}", std::env::consts::EXE_SUFFIX);
    let bin = if target.ends_with(".zip") {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive)).map_err(|e| e.to_string())?;
        let mut bin = vec![];
        let at = (0..zip.len()).find(|&i| zip.by_index(i).is_ok_and(|f| f.name().ok().is_some_and(|n| n.rsplit('/').next() == Some(name.as_str())))).ok_or("The OpenCode download has no program in it.")?;
        zip.by_index(at).map_err(|e| e.to_string())?.read_to_end(&mut bin).map_err(|e| e.to_string())?;
        bin
    } else {
        // Linux ships a .tar.gz; the system tar unpacks it.
        let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let file = tmp.path().join("opencode.tar.gz");
        fs::write(&file, &archive).map_err(|e| e.to_string())?;
        let ok = std::process::Command::new("tar").arg("-xzf").arg(&file).arg("-C").arg(tmp.path()).status().is_ok_and(|s| s.success());
        fs::read(tmp.path().join(&name)).ok().filter(|_| ok).ok_or("Could not unpack the OpenCode download.")?
    };
    let path = dir.join(&name);
    write_atomic(&path, &bin).map_err(|e| e.message)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    on_path(&dir);
    Ok(path)
}

/// Puts ~/.opencode/bin on the learner's PATH for new terminals, as OpenCode's own install script does.
/// Not worth failing the install over: the app finds OpenCode there anyway.
fn on_path(dir: &Path) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // The user's Path from the registry as written (%VARS% unexpanded). Setting and clearing a variable after it
        // makes Windows tell open programs, so a terminal opened next sees the new Path.
        let d = dir.display().to_string().replace('\'', "''");
        let script = format!("$k=[Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment');$p=[string]$k.GetValue('Path','',[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames);$d='{d}';if(-not(($p -split ';') -contains $d)){{$k.SetValue('Path',((@($p.TrimEnd(';'),$d) -ne '') -join ';'),[Microsoft.Win32.RegistryValueKind]::ExpandString);[Environment]::SetEnvironmentVariable('ROADMAP_PATH','1','User');[Environment]::SetEnvironmentVariable('ROADMAP_PATH',$null,'User')}}");
        let _ = std::process::Command::new("powershell").creation_flags(0x08000000).args(["-NoProfile", "-NonInteractive", "-Command", &script]).status();
    }
    #[cfg(unix)]
    if let Some(home) = dirs::home_dir() {
        let _ = dir;
        let (file, line) = shell_line(&home, &std::env::var("SHELL").unwrap_or_default(), cfg!(target_os = "macos"));
        if let Err(e) = add_line(&file, &line) {
            eprintln!("OpenCode was not added to PATH in {}: {e}", file.display());
        }
    }
}
/// The startup file of the learner's shell and the line that adds OpenCode to PATH there.
#[cfg(unix)]
fn shell_line(home: &Path, shell: &str, mac: bool) -> (PathBuf, String) {
    let export = r#"export PATH="$HOME/.opencode/bin:$PATH""#.to_string();
    match shell.rsplit('/').next().unwrap_or("") {
        "fish" => (home.join(".config").join("fish").join("config.fish"), "fish_add_path $HOME/.opencode/bin".into()),
        "zsh" => (home.join(".zshrc"), export),
        "bash" => (home.join(".bashrc"), export),
        // macOS starts zsh by default.
        _ if mac => (home.join(".zshrc"), export),
        _ => (home.join(".profile"), export),
    }
}
/// Appends the line unless the file already mentions OpenCode's folder; the rest of the file is never rewritten.
#[cfg(unix)]
fn add_line(file: &Path, line: &str) -> std::io::Result<()> {
    use std::io::Write;
    let old = fs::read_to_string(file).unwrap_or_default();
    if old.contains(".opencode/bin") {
        return Ok(());
    }
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    let gap = if old.is_empty() || old.ends_with('\n') { "" } else { "\n" };
    fs::OpenOptions::new().create(true).append(true).open(file)?.write_all(format!("{gap}\n# opencode\n{line}\n").as_bytes())
}

#[cfg(all(test, unix))]
#[test]
fn opencode_goes_on_path_once() {
    let tmp = tempfile::tempdir().unwrap();
    let (file, line) = shell_line(tmp.path(), "/usr/bin/zsh", false);
    assert_eq!(file, tmp.path().join(".zshrc"));
    fs::write(&file, "alias ll='ls -l'").unwrap();
    add_line(&file, &line).unwrap();
    add_line(&file, &line).unwrap();
    assert_eq!(fs::read_to_string(&file).unwrap(), "alias ll='ls -l'\n\n# opencode\nexport PATH=\"$HOME/.opencode/bin:$PATH\"\n");
    let (fish, line) = shell_line(tmp.path(), "/usr/bin/fish", false);
    add_line(&fish, &line).unwrap();
    assert_eq!(fs::read_to_string(&fish).unwrap(), "\n# opencode\nfish_add_path $HOME/.opencode/bin\n");
    assert_eq!(shell_line(tmp.path(), "", true).0, tmp.path().join(".zshrc"));
    assert_eq!(shell_line(tmp.path(), "/bin/sh", false).0, tmp.path().join(".profile"));
}

#[cfg(test)]
#[test]
fn connects_every_tool_and_removes_only_its_own_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let (home, config) = (tmp.path().join("home"), tmp.path().join("config"));
    let d = Dirs { home: home.clone(), config: config.clone(), opencode: false };
    for dir in [home.join(".gemini"), home.join(".codex"), config.join("Code").join("User"), home.join(".cursor")] {
        fs::create_dir_all(dir).unwrap();
    }
    fs::write(home.join(".claude.json"), r#"{"numStartups":3,"mcpServers":{"other":{"command":"x"}}}"#).unwrap();
    fs::write(home.join(".codex").join("config.toml"), "model = \"m\"\n\n[mcp_servers.other]\ncommand = \"x\"\n").unwrap();
    // A settings file with comments is not rewritten.
    fs::write(config.join("Code").join("User").join("mcp.json"), "// mine\n{}").unwrap();
    let mcp = Path::new(r"C:\Apps\Roadmap\roadmap-mcp.exe");
    let done: std::collections::BTreeMap<_, _> = connect(&d, Some(mcp)).into_iter().collect();
    assert_eq!(done.keys().cloned().collect::<Vec<_>>(), ["Claude Code", "Codex", "Cursor", "Gemini CLI", "VS Code"]);
    assert!(done["VS Code"].is_err() && done["Cursor"] == Ok(true), "{done:?}");
    let claude: Value = serde_json::from_slice(&fs::read(home.join(".claude.json")).unwrap()).unwrap();
    assert_eq!((claude["numStartups"].clone(), claude["mcpServers"]["other"]["command"].clone(), claude["mcpServers"]["roadmap"]["command"].clone()), (json!(3), json!("x"), json!(mcp.to_string_lossy())));
    let toml = fs::read_to_string(home.join(".codex").join("config.toml")).unwrap();
    assert_eq!(toml, "model = \"m\"\n\n[mcp_servers.other]\ncommand = \"x\"\n\n[mcp_servers.roadmap]\ncommand = \"C:\\\\Apps\\\\Roadmap\\\\roadmap-mcp.exe\"\n");
    // A second run changes nothing; a moved app updates the path in place.
    assert!(connect(&d, Some(mcp)).iter().all(|(n, r)| n == "VS Code" || *r == Ok(false)));
    connect(&d, Some(Path::new("/opt/roadmap-mcp")));
    assert_eq!(fs::read_to_string(home.join(".codex").join("config.toml")).unwrap(), "model = \"m\"\n\n[mcp_servers.other]\ncommand = \"x\"\n\n[mcp_servers.roadmap]\ncommand = \"/opt/roadmap-mcp\"\n");
    // Removing takes out only this server.
    connect(&d, None);
    assert_eq!(fs::read_to_string(home.join(".codex").join("config.toml")).unwrap(), "model = \"m\"\n\n[mcp_servers.other]\ncommand = \"x\"\n");
    let claude: Value = serde_json::from_slice(&fs::read(home.join(".claude.json")).unwrap()).unwrap();
    assert_eq!(claude["mcpServers"], json!({"other":{"command":"x"}}));
    assert_eq!(fs::read_to_string(config.join("Code").join("User").join("mcp.json")).unwrap(), "// mine\n{}");
}
