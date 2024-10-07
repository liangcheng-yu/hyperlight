#cloud-config

runcmd:
- yum install -y dnf
- dnf install -y 'dnf-command(config-manager)'
- dnf config-manager --add-repo https://pkgs.tailscale.com/stable/fedora/tailscale.repo
- dnf config-manager --add-repo https://cli.github.com/packages/rpm/gh-cli.repo
- dnf install -y tailscale vim kernel-mshv kernel-mshv-devel git clang lldb binutils valgrind glibc-devel kernel-headers nano gh grubby ca-certificates
- update-ca-trust
- systemctl enable --now tailscaled
- tailscale up --ssh --authkey="${tailscale_auth_key}"
- groupadd mshv
- usermod -a -G mshv azureuser
- |
  cat <<EOF >> /etc/udev/rules.d/10-hyperlight.rules
  # hyperlight related rule to change the /dev/mshv group to be "mshv"
  SUBSYSTEM=="misc", KERNEL=="mshv", RUN+="/bin/chown root:mshv /dev/mshv", RUN+="/bin/chmod 0660 /dev/mshv"
  EOF
- sed -i -e 's@menuentry "CBL-Mariner"@menuentry "Dom0" {\n    search --no-floppy --set=root --file /EFI/Microsoft/Boot/bootmgfw.efi\n        chainloader /EFI/Microsoft/Boot/bootmgfw.efi\n}\n\nmenuentry "CBL-Mariner"@'  /boot/grub2/grub.cfg
- sudo -H -u azureuser bash -c 'curl -sSf --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y'
- sudo -H -u azureuser bash -c 'source "$HOME/.cargo/env" && cargo install just'
- curl -sfL https://direnv.net/install.sh | bash
- sudo -H -u azureuser bash -c "echo 'eval \"\$(direnv hook bash)\"' >> /home/azureuser/.bashrc"
- shutdown -r 0
