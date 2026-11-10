use crate::*;
use std::path::Path;
const SKIP: &[&str] = &[
    "node_modules",
    "target",
    "build",
    "dist",
    "out",
    "venv",
    "env",
    "__pycache__",
    "vendor",
    "Pods",
    "DerivedData",
    "coverage",
];
impl App {
    pub(crate) fn project_file(&self, folder: &str, rel: &Value) -> Option<String> {
        let root = fs::canonicalize(folder).ok()?;
        let f = fs::canonicalize(root.join(text(rel))).ok()?;
        let parts = f.strip_prefix(&root).ok()?;
        if parts.as_os_str().is_empty()
            || parts.components().any(|x| {
                let s = x.as_os_str().to_string_lossy();
                s.starts_with('.') || SKIP.contains(&s.as_ref())
            })
            || matches(
                r"(?i)\.env|secret|credential|password|id_rsa|id_ed25519|\.(pem|key|p12|pfx|keystore|jks)$",
                &f.file_name()?.to_string_lossy(),
            )
            || !f.is_file()
            || f.metadata().ok()?.len() > 200_000
        {
            return None;
        }
        fs::read_to_string(f)
            .ok()
            .map(|s| s.replace("\r\n", "\n").replace('\r', "\n"))
    }
    pub(crate) fn read_project(&self, folder: &str) -> Result<(String, Vec<String>, String)> {
        let folder = if folder == "~" {
            self.home.clone()
        } else if let Some(rest) = folder.strip_prefix("~/") {
            self.home.join(rest)
        } else {
            PathBuf::from(folder)
        };
        let root = fs::canonicalize(folder).map_err(|_| {
            Error::bad("Choose the folder of one project, not your whole home folder.")
        })?;
        if !root.is_dir()
            || root == Path::new("/")
            || root == fs::canonicalize(&self.home).unwrap_or(self.home.clone())
        {
            return Err(Error::bad(
                "Choose the folder of one project, not your whole home folder.",
            ));
        }
        fn walk(root: &Path, d: &Path, files: &mut Vec<String>) -> std::io::Result<()> {
            let mut entries = fs::read_dir(d)?.collect::<std::io::Result<Vec<_>>>()?;
            entries.sort_by_key(|e| e.file_name());
            for e in &entries {
                if !e.file_type()?.is_dir() {
                    files.push(
                        e.path()
                            .strip_prefix(root)
                            .unwrap()
                            .to_string_lossy()
                            .into(),
                    );
                }
            }
            if files.len() > 3000 {
                return Ok(());
            }
            for e in entries {
                let s = e.file_name().to_string_lossy().into_owned();
                if e.file_type()?.is_dir() && !s.starts_with('.') && !SKIP.contains(&s.as_str()) {
                    walk(root, &e.path(), files)?;
                    if files.len() > 3000 {
                        break;
                    }
                }
            }
            Ok(())
        }
        let mut files = vec![];
        walk(&root, &root, &mut files)?;
        files.truncate(3000);
        let root = root.to_string_lossy().into_owned();
        files.retain(|f| self.project_file(&root, &json!(f)).is_some());
        if files.is_empty() {
            return Err(Error::bad("No readable text files in that folder."));
        }
        files.sort_by_key(|f| {
            (
                !f.to_lowercase().starts_with("readme"),
                f.matches('/').count(),
                f.clone(),
            )
        });
        let mut bodies = vec![];
        let mut room = 60000;
        for f in &files {
            let body = cap(
                &self.project_file(&root, &json!(f)).unwrap_or_default(),
                6000,
            );
            let n = body.chars().count();
            if n > room {
                break;
            }
            room -= n;
            bodies.push(format!("--- {f} ---\n{body}"));
        }
        let digest = format!(
            "File list:\n{}\n\nContents of some files:\n{}",
            files
                .iter()
                .take(600)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n"),
            bodies.join("\n")
        );
        Ok((root, files, digest))
    }
    pub(crate) fn lesson_files(&self, c: &Value, l: &Value) -> Vec<String> {
        let mut room = 12000usize;
        let mut out = vec![];
        for f in list(&l["files"]) {
            if let Some(body) = self.project_file(&string(&c["folder"]), f) {
                if room > 0 {
                    out.push(format!("--- {} ---\n{}", text(f), cap(&body, room)));
                    room = room.saturating_sub(body.chars().count());
                }
            }
        }
        out
    }
}
pub(crate) fn root_of(c: &Value) -> String {
    if truth(&c["folder"]) {
        string(&c["folder"])
    } else {
        string(&c["work"])
    }
}
