include ./common.mk

features=

ifdef disable_log
	features+=disable_log
endif

build:
	cargo build $(args) $(if $(features), --features $(features))
	llvm-objdump --section-headers --source -d ../target/aarch64-unknown-none/$(profile)/sophon > ../target/aarch64-unknown-none/$(profile)/sophon.s 2> /dev/null
