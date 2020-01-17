arch ?= aarch64

profile = $(if $(release),release,debug)
cargo_xbuild = cargo xbuild --no-default-features $(if $(release), --release)

include Makefile.$(arch).mk

FORCE: ;