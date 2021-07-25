include ./common.mk

build:
	$(call build, init)
	cargo build --features=kernel $(args)
	llvm-objdump --section-headers --source -d ../target/aarch64-unknown-none/$(profile)/proton > ../target/aarch64-unknown-none/$(profile)/proton.s 2> /dev/null
