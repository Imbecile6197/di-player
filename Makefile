.PHONY: build release install uninstall run test clean

PREFIX ?= $(HOME)/.local

build:
	cargo build

release:
	cargo build --release

install: release
	./scripts/install.sh

uninstall:
	rm -f $(PREFIX)/bin/di_player
	rm -f $(PREFIX)/share/applications/com.alchymista.DiPlayer.desktop
	rm -f $(PREFIX)/share/icons/hicolor/*/apps/com.alchymista.DiPlayer.png
	rm -f $(PREFIX)/share/icons/hicolor/scalable/apps/com.alchymista.DiPlayer.svg
	-gtk4-update-icon-cache -f $(PREFIX)/share/icons/hicolor 2>/dev/null || true
	-update-desktop-database $(PREFIX)/share/applications 2>/dev/null || true

run: release
	./target/release/di_player

test:
	cargo test

clean:
	cargo clean
