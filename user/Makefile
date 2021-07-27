include ../sophon/common.mk

USER_PROGRAMS = init

# Build a user program. e.g. `make build.init`
build.%:
	cd $(@:build.%=%) && cargo build $(args) --target ../../sophon/aarch64-sophon.json
	llvm-objdump --section-headers --source -d ../target/aarch64-sophon/$(profile)/$(@:build.%=%) > ../target/aarch64-sophon/$(profile)/$(@:build.%=%).s 2> /dev/null

# Build all user programs.
build: $(addprefix build., $(USER_PROGRAMS))

# Build init.fs
init.fs: build
	cd ../tools/initfs_builder && cargo run $(args)

.PHONY: build.% build