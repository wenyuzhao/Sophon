include ./common.mk

build:
	cargo build --features=kernel $(args)
	llvm-objdump --section-headers --source -d ../target/aarch64-unknown-none/$(profile)/sophon > ../target/aarch64-unknown-none/$(profile)/sophon.s 2> /dev/null
