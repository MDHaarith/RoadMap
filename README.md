# Roadmap

A native GTK3 / WebKitGTK 4.1 learning app. The application and its tests are Rust;
`index.html` is the existing UI. Saved JSON roadmaps and settings from the Python
version load from the same XDG directories without conversion.

Build on Fedora with Rust/Cargo 1.92 or newer, GCC, pkgconf-pkg-config, gtk3-devel,
and webkit2gtk4.1-devel. Run with `make run`; run the tests with `make test`.
The tests use local fake AI and yt-dlp services and real bubblewrap sandboxes,
with Bash exercise fixtures, and need no Python interpreter or external AI key.
They require bubblewrap and permission to create unprivileged user namespaces.

Build the Fedora 44 RPM with:

```sh
podman run --rm --security-opt label=disable -v "$PWD":/src -w /src \
  registry.fedoraproject.org/fedora:44 sh -c \
  'dnf -y -q install rpm-build make cargo rust gcc pkgconf-pkg-config gtk3-devel webkit2gtk4.1-devel >/dev/null 2>&1 && make rpm'
```

The architecture-specific RPM is written under `dist/`. Cargo downloads the
versions in `Cargo.lock`, so the container needs network access; crates are not
vendored. Runtime packages are GTK3, WebKitGTK 4.1, yt-dlp, bubblewrap, Bash, and
the system CA certificates.
Other exercise interpreters/compilers are installed on demand through the same
Polkit / DNF dialog as before. Python exercises still use an optional Python
interpreter; Fedora's yt-dlp package also has its own Python dependency.

The Rust server uses HTTP chunked framing for streamed tutor replies. API
routes, payloads, prompts, schemas, saved data, and the UI are unchanged.
Error diagnostics from Rust's JSON/HTTP libraries can differ in wording.

A new roadmap has a size: about two weeks, a month, three months or six months.
The two long sizes are planned as an outline of stages, phases and lesson titles
(`OUTLINE_PROMPT`). The lessons of a phase are written when the learner reaches it
(`POST api/courses/<id>/expand`), so later phases use their results. "Change my
roadmap" (`POST api/courses/<id>/revise`) lets the learner ask for a new direction
part-way through; new phases arrive as outlines and are written the same way.

## Connect an AI agent (MCP)

`roadmap-mcp` lets an agent list, read, create and fill learning roadmaps, read progress, append ideas and search. It never calls AI or the network and cannot change progress, delete roadmaps, access settings or API keys, run exercises, or read project folders. Search includes learner notes and tutor chats.

Claude Code: `claude mcp add roadmap -- roadmap-mcp`

Codex, in `~/.codex/config.toml`:

```toml
[mcp_servers.roadmap]
command = "roadmap-mcp"
```

From source, run `cargo build --release --locked` and use the absolute path to `target/release/roadmap-mcp`. A roadmap created while the app is open appears after the library page is reopened.
