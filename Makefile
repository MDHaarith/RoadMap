.PHONY: run test rpm

run:
	cargo run --locked

test:
	cargo test --locked

# Fedora build dependencies are listed in roadmap-app.spec. Cargo downloads locked crates.
# Output: dist/<architecture>/roadmap-app-*.rpm
rpm:
	rpmbuild -bb --define "_sourcedir $(CURDIR)" --define "_rpmdir $(CURDIR)/dist" roadmap-app.spec
