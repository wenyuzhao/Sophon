#!/usr/bin/env bash

set -e

uefi_bin=$1
boot_dir=$(dirname $(dirname $(dirname $uefi_bin)))/_boot

# Launch qemu
qemu=qemu-system-aarch64
bios=.cargo/QEMU_EFI.fd
machine_args="-M virt -machine virtualization=on -cpu cortex-a72 -smp 4 -m 1G"
# machine_args="-M virt,dumpdtb=$outdir/device-tree.dtb -cpu cortex-a72 -smp 1 -m 1G"
shift
set -ex
$qemu $machine_args -s -semihosting -bios $bios -drive index=0,format=raw,file=fat:rw:$boot_dir -net none -monitor none -nographic -serial stdio $@


# Launch qemu
# qemu=qemu-system-x86_64
# bios=.cargo/OVMF.fd
# machine_args="-cpu qemu64"
# $qemu $machine_args -bios $bios -drive file=$tmp_img,index=0,media=disk,format=raw -net none -monitor none -nographic -serial stdio