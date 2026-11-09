Name:           roadmap-app
Version:        0.2.0
Release:        1%{?dist}
Summary:        Learn anything by doing: AI-built courses with free YouTube lessons
# No licence chosen yet. Replace before sharing the package.
License:        LicenseRef-Not-Chosen
BuildArch:      noarch
Requires:       python3 python3-gobject gtk3 webkit2gtk4.1 yt-dlp bubblewrap

%description
Describe a project or skill and Roadmap builds a step-by-step course: free
YouTube lessons, tasks you do yourself, quizzes, coding exercises with tests,
checks that gate each lesson, notes,
and an AI tutor that gives hints instead of answers. Works with any
OpenAI-compatible AI service.

%install
install -Dm755 %{_sourcedir}/roadmap_app.py %{buildroot}%{_bindir}/roadmap-app
install -Dm644 %{_sourcedir}/index.html %{buildroot}%{_datadir}/roadmap-app/index.html
install -Dm644 %{_sourcedir}/roadmap-app.desktop %{buildroot}%{_datadir}/applications/roadmap-app.desktop
install -Dm644 %{_sourcedir}/roadmap-app.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/roadmap-app.svg

%files
%{_bindir}/roadmap-app
%{_datadir}/roadmap-app/
%{_datadir}/applications/roadmap-app.desktop
%{_datadir}/icons/hicolor/scalable/apps/roadmap-app.svg
