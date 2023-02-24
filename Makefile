.PHONY: build
build:
	cargo build --release
	make -f Makefile.plugin

