# Roadmap

A Tauri 2 learning app with a Svelte 5 / TypeScript frontend in `ui/` and a Rust backend in `src/`. Linux keeps the existing XDG data and settings paths without conversion; Windows and macOS use their native data/config directories under `roadmap-app`.

Build on Fedora with Rust/Cargo 1.92 or newer, Node.js 24, npm, GCC/G++, pkgconf-pkg-config, gtk3-devel, webkit2gtk4.1-devel and openssl-devel. First run `npm --prefix ui ci`. Run with `make run`; run the tests with `make test`. `make build` embeds `ui/dist` in the release app and also builds `roadmap-mcp`; `make bundle` creates native installers (deb/rpm/AppImage on Linux, MSI/NSIS on Windows, app/DMG on macOS). Build installers on their target OS. Plain `cargo build` builds both binaries; its debug app expects the Vite dev server.
The tests use local fake AI and yt-dlp services and real bubblewrap sandboxes,
with Bash exercise fixtures, and need no Python interpreter or external AI key.
They require bubblewrap and permission to create unprivileged user namespaces.

Build the Fedora 44 RPM with:

```sh
podman run --rm --security-opt label=disable -v "$PWD":/src -w /src \
  registry.fedoraproject.org/fedora:44 sh -c \
  'dnf -y -q install rpm-build make cargo rust gcc-c++ pkgconf-pkg-config gtk3-devel webkit2gtk4.1-devel openssl-devel nodejs npm >/dev/null 2>&1 && make rpm'
```

The architecture-specific RPM is written under `dist/`. Cargo downloads the
versions in `Cargo.lock`, so the container needs network access; crates are not
vendored. Runtime packages are GTK3, WebKitGTK 4.1, yt-dlp, bubblewrap, Bash, and
the system CA certificates.
On Linux, other exercise interpreters/compilers are installed on demand through the same Polkit / DNF dialog as before. Sandbox execution and runtime installation return an explicit unsupported error on Windows and macOS. Install yt-dlp separately there and put it on PATH. Python exercises still use an optional Python
interpreter; Fedora's yt-dlp package also has its own Python dependency.

The root crate is also the Tauri app; `tauri.conf.json` points at `ui/dist`. The `api(method, path, body)` command runs the existing routes on a blocking worker. `chat_stream(body, onPiece)` sends tutor pieces over a Tauri channel and returns the final text. Both reject invalid bodies and preserve the 1 MiB limit. Errors serialize as `{ "status": 400, "message": "..." }`. File/folder selection uses native dialogs; window controls and HTTP(S) links use scoped Tauri capabilities.

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

From source, run `cargo build --release --locked --bin roadmap-mcp` and use the absolute path to `target/release/roadmap-mcp`. A roadmap created while the app is open appears after the library page is reopened.
