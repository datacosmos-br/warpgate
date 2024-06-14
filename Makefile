# Define variables
CARGO = cargo
YARN = yarn
JUST = just
BUILD_MODE = release

.PHONY: all install_just install_deps build_frontend build_warpgate clean release debug

# Default target
all: install_just install_deps build_frontend build_warpgate

# Install 'just' command
install_just:
	$(CARGO) install just

# Install admin UI dependencies using 'just'
install_deps:
	$(JUST) yarn

# Build the frontend using 'just'
build_frontend:
	$(JUST) yarn build

# Build Warpgate
build_warpgate:
ifeq ($(BUILD_MODE),release)
	$(CARGO) build --release
else
	$(CARGO) build
endif

# Clean build files
clean:
	$(CARGO) clean

# Release build
release: 
	$(eval BUILD_MODE = release)
	$(MAKE) build_warpgate

# Debug build
debug:
	$(eval BUILD_MODE = debug)
	$(MAKE) build_warpgate

