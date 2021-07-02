# build:
# 	[tasks.build-flow]
# dependencies = [
#   { name = "build-flow", path = "../init" },
#   { name = "build", path = "." },
#   { name = "post-build", path = "." },
# ]

# [tasks.build]
# command = "cargo"
# args = ["build"]

# [tasks.post-build]
# env = { PROFILE = { source = "${CARGO_MAKE_CARGO_PROFILE}", mapping = { dev = "debug" } } }
# # command = "llvm-objdump"
# script = '''
# llvm-objdump --section-headers --source -d ../target/aarch64-unknown-none/${PROFILE}/proton > ../target/aarch64-unknown-none/${PROFILE}/proton.S
# '''

build:
	$(MAKE) -C ../init build
	cargo build