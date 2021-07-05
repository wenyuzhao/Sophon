include ./common.mk

build:
	$(call build, init)
	cargo build $(args)
	llvm-objdump --section-headers --source -d ../target/aarch64-unknown-none/debug/proton > ../target/aarch64-unknown-none/debug/proton.S 2> /dev/null
