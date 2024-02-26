set -o errexit
set -o nounset
set -o pipefail

dnf install -y 'dnf-command(config-manager)'
dnf config-manager --add-repo https://cli.github.com/packages/rpm/gh-cli.repo
dnf install -y vim kernel-mshv git lldb binutils glibc-devel kernel-headers nano gh grubby ca-certificates mshv-bootloader-lx mshv hvloader azure-cli
dnf remove -y llvm
dnf install -y clang16 clang16-tools-extra
update-ca-trust
groupadd mshv

cat <<EOF >> /etc/udev/rules.d/10-hyperlight.rules
  # hyperlight related rule to change the /dev/mshv group to be "mshv"
  SUBSYSTEM=="misc", KERNEL=="mshv", RUN+="/bin/chown root:mshv /dev/mshv", RUN+="/bin/chmod 0660 /dev/mshv"
EOF

boot_uuid=$(sudo grep -o -m 1 '[0-9a-f]\{8\}-[0-9a-f]\{4\}-[0-9a-f]\{4\}-[0-9a-f]\{4\}-[0-9a-f]\{12\}' /boot/efi/boot/grub2/grub.cfg)
export boot_uuid

sudo sed -i -e 's@load_env -f \$bootprefix\/mariner.cfg@load_env -f \$bootprefix\/mariner-mshv.cfg\nload_env -f $bootprefix\/mariner.cfg\n@'  /boot/grub2/grub.cfg
sudo sed -i -e 's@menuentry "CBL-Mariner"@menuentry "Dom0" {\n    search --no-floppy --set=root --file /HvLoader.efi\n    chainloader /HvLoader.efi lxhvloader.dll MSHV_ROOT=\\\\Windows MSHV_ENABLE=TRUE MSHV_SCHEDULER_TYPE=ROOT MSHV_X2APIC_POLICY=ENABLE MSHV_SEV_SNP=TRUE MSHV_LOAD_OPTION=INCLUDETRACEMETADATA=1\n    boot\n    search --no-floppy --fs-uuid '"$boot_uuid"' --set=root\n    linux $bootprefix/$mariner_linux_mshv $mariner_cmdline_mshv $systemd_cmdline root=$rootdevice\n    if [ -f $bootprefix/$mariner_initrd_mshv ]; then\n    initrd $bootprefix/$mariner_initrd_mshv\n    fi\n}\n\nmenuentry "CBL-Mariner"@'  /boot/grub2/grub.cfg
