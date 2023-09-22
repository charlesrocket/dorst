PREFIX=/usr/local
INSTALL_DIR=$(PREFIX)/bin
DEST=$(INSTALL_DIR)/dorst
BIN=target/release/dorst
SOURCE_FILES = $(shell test -e src/ && find src -type f)

all: build

build: $(BIN)

$(BIN): $(SOURCE_FILES)
.ifdef features
	@cargo build --features $(features) --release
.else
	@cargo build --release
.endif

install:
	@rm -f $(DEST)
	@cp $(BIN) $(DEST)
	@echo "Installed!"

.ifdef features
.if $(features) == gui
	desktop-file-install data/org.hellbyte.dorst.desktop
	update-desktop-database
	install -Dm644 "data/org.hellbyte.dorst.png"\
		"/usr/local/share/pixmaps/org.hellbyte.dorst.png"
.endif
.endif

uninstall:
	@rm -f $(DEST)
	@echo "Removed!"

help:
	@echo "Available targets:"
	@echo "build install uninstall"

.PHONY: help install uninstall
