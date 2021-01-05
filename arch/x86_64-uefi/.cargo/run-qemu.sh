#!/usr/bin/env bash

set -ex

uefi_bin=$1

# Prepare & cleanup temp files
rm -rf /tmp/proton-*
temp_base=$(mktemp -d /tmp/proton-XXXXXXXX)

# Copy files to a temp folder
tmp_dir=$temp_base
cp $uefi_bin $tmp_dir/proton.efi
cp .cargo/startup.nsh $tmp_dir/

# Create image file
tmp_dmg=$temp_base.dmg
tmp_img=$temp_base.img
hdiutil create -FS fat32 -srcfolder $tmp_dir $tmp_dmg
qemu-img convert -O raw $tmp_dmg $tmp_img
qemu-img resize $tmp_img 128M

# Launch qemu
qemu=qemu-system-x86_64
bios=.cargo/OVMF.fd
machine_args="-cpu qemu64"
$qemu $machine_args -bios $bios -drive file=$tmp_img,index=0,media=disk,format=raw -net none -monitor none -nographic -serial stdio