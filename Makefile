# The default make command.
DEFAULT = help

# Use 'VERBOSE=1' to echo all commands, for example 'make help VERBOSE=1'.
ifdef VERBOSE
  Q :=
else
  Q := @
endif

.PHONY: \
		clean \
		run \
		test \
		bench \
		watch

all: $(DEFAULT)

help:
	$(Q)echo "VRRB Dev CLI - v0.0.1"
	$(Q)echo "make run               - Runs main executable"
	$(Q)echo "make build             - Builds the main executable"
	$(Q)echo "make build-dev         - Builds a debug version of the main executable"
	$(Q)echo "make test              - Tests all crates"
	$(Q)echo "make lint              - Runs clippy and formatter"
	$(Q)echo "make fmt               - Runs formatter"
	$(Q)echo "make bench             - Benchmarks all crates"
	$(Q)echo "make dev               - Alias for 'make watch'"
	$(Q)echo "make watch             - Runs main executable in hot-reloading mode for development"
	$(Q)echo "make clean             - Deletes binaries and documentation."
	$(Q)echo "make ci-build   		 - Builds a container for the Node runtime."
	$(Q)echo "make ci-run     		 - Builds and runs a container for the Node runtime."
	$(Q)echo "make ci-run-d   		 - Builds and runs a container for the Node runtime in dettached mode."

build:
	$(Q)cargo build --release
	$(Q)echo "--- Done"

build-dev:
	$(Q)cargo build
	$(Q)echo "--- Done"

ci-build:
	$(Q)sh infra/scripts/build-ci.sh
	$(Q)echo "--- Done"

ci-run: ci-build
	$(Q)docker run --rm --name vrrb-node ghcr.io/vrrb-io/vrrb
	$(Q)echo "--- Done"

ci-run-d:
	$(Q)docker run -d --name vrrb-node ghcr.io/vrrb-io/vrrb
	$(Q)echo "--- Done"

clean: clean-ui
	$(Q)cargo clean
	$(Q)echo "--- Deleted binaries and documentation"

clean-ui:
	$(Q)rm -rf infra/gui/node_modules infra/gui/.next
	$(Q)echo "--- Deleted UI build artifacts"

run:
	# TODO: consider replacing with env aware script instead
	$(Q)sh infra/scripts/run_test_node.sh
	$(Q)echo "--- Done"

run-test-cluster:
	$(Q)sh infra/scripts/run_test_node.sh
	$(Q)echo "--- Done"

test:
	$(Q)sh infra/scripts/run_tests.sh
	$(Q)echo "--- Executed tests on all crates"

lint:
	$(Q)sh infra/scripts/run_lints.sh
	$(Q)echo "--- Ran all lints and formatters"

fmt:
	$(Q)cargo +nightly fmt --all 
	$(Q)echo "--- Ran formatter"

dev: watch 

watch: 
	$(Q)sh infra/scripts/watch.sh
	$(Q)echo "--- Done"

buf-gen:
	$(Q) buf generate infra/proto

buf-push:
	$(Q) buf generate infra/proto
