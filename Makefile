.PHONY: all build-starpls install-starpls

BAZEL ?= bazelisk
PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
STARPLS_TARGET ?= //crates/starpls:starpls

all: install-starpls

build-starpls:
	$(BAZEL) build $(STARPLS_TARGET)

install-starpls: build-starpls
	@mkdir -p "$(BINDIR)"
	@execroot="$$($(BAZEL) info execution_root)"; \
	output="$$($(BAZEL) cquery $(STARPLS_TARGET) --output=files 2>/dev/null)"; \
	install -m 0755 "$$execroot/$$output" "$(BINDIR)/starpls"; \
	echo "Installed $(BINDIR)/starpls"
