PREFIX = /usr/local
INSTALL_DIR = $(PREFIX)/bin
DEST = $(INSTALL_DIR)/dorst
BIN = target/release/dorst
SOURCE_FILES = $(shell test -e src/ && find src -type f)

all: build

build: $(BIN)

$(BIN): $(SOURCE_FILES)
	@if [ -n "$(features)" ]; then \
		cargo build --features $(features) --release; \
	else \
		cargo build --release; \
	fi

install:
	@rm -f $(DEST)
	cp $(BIN) $(DEST)

	@if [ -n "$(features)" ] && [ "$(features)" = "gui" ]; then \
		desktop-file-install data/org.hellbyte.dorst.desktop; \
		update-desktop-database; \
		install -Dm644 "data/org.hellbyte.dorst.png" \
			"/usr/local/share/pixmaps/org.hellbyte.dorst.png"; \
	fi

uninstall:
	rm -f $(DEST)

help:
	@echo "Available targets:"
	@echo "build install uninstall"

.PHONY: help install uninstall
