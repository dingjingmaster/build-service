.PHONY:clean release debug test deb rpm emerge

release:
	cargo build --release

debug:
	cargo build

test:
	cargo test

deb: release
	sh packaging/package.sh deb

rpm: release
	sh packaging/package.sh rpm

emerge: release
	sh packaging/package.sh emerge

clean:
	-rm -f target/debug/buildsvc
	-rm -f target/release/buildsvc
