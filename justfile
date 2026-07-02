set default-list

create-virtual-uefi-layout:
    cp /usr/share/OVMF/x64/OVMF_CODE.4m.fd /usr/share/OVMF/x64/OVMF_VARS.4m.fd .
    mkdir -p esp/efi/boot

symlink package:
    ln -sf /home/sermuns/.cache/cargo/x86_64-unknown-uefi/debug/{{ package }}.efi esp/efi/boot/bootx64.efi

qemu:
    cargo build --workspace
    qemu-system-x86_64 -enable-kvm \
        -display sdl,gl=on \
        -drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.4m.fd \
        -drive if=pflash,format=raw,readonly=on,file=OVMF_VARS.4m.fd \
        -drive format=raw,file=fat:rw:esp

watch:
    watchexec -r -- just qemu
