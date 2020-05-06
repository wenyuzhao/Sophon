arch ?= aarch64
features ?=

profile = $(if $(release),release,debug)
project = $(PWD)

include arch/$(arch)/Build.mk



init: FORCE
	$(MAKE) arch_user_program name=init path=init

drivers: FORCE
	$(MAKE) arch_user_program name=emmc path=drivers/emmc

kernel: init drivers arch_kernel FORCE

run: arch_run

clean:
	@cargo clean



FORCE: ;

test.img: size=64
test.img: FORCE
ifdef folder
	rm -f test.img
	curl -s https://raw.githubusercontent.com/Othernet-Project/dir2fat32/master/dir2fat32.sh | bash -s test.img $(size) $(folder)
endif