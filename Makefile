.PHONY: run test build bundle rpm

run:
	npm --prefix ui run tauri -- dev

test:
	cargo test --locked

build:
	npm --prefix ui run tauri -- build --no-bundle -- --locked
	cargo build --release --locked --bin roadmap-mcp

# Native targets: deb/rpm/appimage, msi/nsis, or dmg/app on their respective OS.
bundle:
	npm --prefix ui run tauri -- build -- --locked

# Fedora build dependencies are listed in roadmap-app.spec. Cargo downloads locked crates.
# Output: dist/<architecture>/roadmap-app-*.rpm
rpm:
	rpmbuild -bb --define "_sourcedir $(CURDIR)" --define "_rpmdir $(CURDIR)/dist" roadmap-app.spec
