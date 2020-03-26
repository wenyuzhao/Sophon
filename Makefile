arch ?= aarch64

profile = $(if $(release),release,debug)
cargo_xbuild = cargo xbuild --no-default-features $(if $(release), --release)

include Makefile.$(arch).mk

FORCE: ;

test.img: size=64
test.img: FORCE
ifdef folder
	rm -f test.img
	curl -s https://raw.githubusercontent.com/Othernet-Project/dir2fat32/master/dir2fat32.sh | bash -s test.img $(size) $(folder)
endif