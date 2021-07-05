export arch ?= aarch64
export profile ?= debug

export workspace = $(abspath $(dir $(abspath $(lastword $(MAKEFILE_LIST))))/..)
export target = $(workspace)/target

export args ?=

ifneq ($(profile), debug)
    args += --release
endif

define build
    $(MAKE) -C $(workspace)/$(strip $1) -f ./Build.mk build
endef
