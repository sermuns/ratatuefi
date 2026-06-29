<div align="center">

![ratatuefi](./media/banner.svg)
_Zero-bloat Ratatui in UEFI, no OS_

<https://github.com/user-attachments/assets/7f05d64a-27ca-4deb-954a-3c2c53765832>

</div>

## Running in QEMU

```sh
pacman -S qemu-system-x86_64 edk2-ovmf
```

(optionally some graphic backend, but you can also VNC into the VM)

```sh
pacman -S qemu-ui-sdl
```

then

```sh
just create-virtual-uefi-layout
```

and then

```sh
just qemu
```

## Running on a physical machine

> from <https://rust-osdev.github.io/uefi-rs/tutorial/hardware.html>

Connect a USB drive. Follow steps below.

```sh
# Create the GPT, create a 9MB partition starting at 1MB, and set the
# partition type to EFI System.
sgdisk \
    --clear \
    --new=1:1M:10M \
    --typecode=1:C12A7328-F81F-11D2-BA4B-00A0C93EC93B \
    /path/to/disk

# Format the partition as FAT.
mkfs.fat /path/to/disk_partition

# Mount the partition.
mount --mkdir /path/to/disk_partition /mnt/ratatuefi

# Create the boot directory.
mkdir -p /mnt/ratatuefi/EFI/BOOT

# Copy in the boot executable.
cp $CARGO_TARGET_DIR/x86_64-unknown-uefi/debug/ratatuefi.efi /mnt/ratatuefi/EFI/BOOT/BOOTX64.EFI

# Eject the USB drive
eject /path/to/disk
```

> [!IMPORTANT]
> disable secure boot on machine before trying to boot

try booting it!

## Ideas for improving

it is slow as hell to draw. could possibly be improved by

- adding a buffer, and rendering full lines? currently we are rawdoggin character-for-character.
- switching to manual VGA graphics (use mousefood?)
