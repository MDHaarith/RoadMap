.PHONY: run test rpm

run:
	python3 roadmap_app.py

test:
	python3 test_roadmap_app.py

# Needs: sudo dnf install rpm-build. Output: dist/noarch/roadmap-app-*.rpm
rpm:
	rpmbuild -bb --define "_sourcedir $(CURDIR)" --define "_rpmdir $(CURDIR)/dist" roadmap-app.spec
