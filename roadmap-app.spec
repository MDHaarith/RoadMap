Name:           roadmap-app
Version:        0.2.0
Release:        12%{?dist}
Summary:        Learn anything by doing: AI-built courses with free YouTube lessons
# No licence chosen yet. Replace before sharing the package.
License:        LicenseRef-Not-Chosen
BuildRequires:  gcc-c++ pkgconf-pkg-config gtk3-devel webkit2gtk4.1-devel openssl-devel
# `make rpm` defines local_toolchain: Rust and Node then come from PATH (rustup, nvm), not Fedora packages.
%if 0%{!?local_toolchain:1}
BuildRequires:  cargo >= 1.92 rust >= 1.92 nodejs npm
%endif
Requires:       gtk3 webkit2gtk4.1 yt-dlp bubblewrap bash ca-certificates
# Optional exercise interpreters are installed on demand; Python does not run the app.
# Cargo downloads Cargo.lock's crates during this local RPM build (not vendored).
%global debug_package %{nil}

%description
Describe a project or skill and Roadmap builds a step-by-step course: free
YouTube lessons, tasks you do yourself, quizzes, coding exercises with tests,
checks that gate each lesson, a placement check, notes,
and an AI tutor that gives hints instead of answers. Works with any
OpenAI-compatible AI service.

%build
cd %{_sourcedir}
npm --prefix ui ci
CARGO_TARGET_DIR=%{_builddir}/roadmap-target npm --prefix ui run tauri -- build --no-bundle -- --locked
CARGO_TARGET_DIR=%{_builddir}/roadmap-target cargo build --release --locked --bin roadmap-mcp

%install
install -Dm755 %{_builddir}/roadmap-target/release/roadmap-app %{buildroot}%{_bindir}/roadmap-app
install -Dm755 %{_builddir}/roadmap-target/release/roadmap-mcp %{buildroot}%{_bindir}/roadmap-mcp
install -Dm644 %{_sourcedir}/roadmap-app.desktop %{buildroot}%{_datadir}/applications/roadmap-app.desktop
install -Dm644 %{_sourcedir}/roadmap-app.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/roadmap-app.svg

%files
%{_bindir}/roadmap-mcp
%{_bindir}/roadmap-app
%{_datadir}/applications/roadmap-app.desktop
%{_datadir}/icons/hicolor/scalable/apps/roadmap-app.svg
