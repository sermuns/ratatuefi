<div align="center">

# `efimux`

_EFI application for booting other EFI applications_

[![Built With Ratatui](https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff)](https://ratatui.rs/)

</div>

![demo video](media/demo.avif)

## Installation and usage

> _Based on <https://rust-osdev.github.io/uefi-rs/tutorial/hardware.html>_

1. Connect a USB drive (**we will erase all of its data!**).

2. Find its device path, e.g. via `lsblk`.

3. Create the GPT and correct partition layout for EFI, example using `sgdisk`:

    ```sh
    sgdisk \
        --clear \
        --new=1:1M:10M \
        --typecode=1:C12A7328-F81F-11D2-BA4B-00A0C93EC93B \
        /path/to/usb_drive
    ```

4. Format the partition as FAT:

    ```sh
    mkfs.fat /path/to/usb_drive_partition
    ```

5. Mount the partition:

    ```sh
    mount --mkdir /path/to/usb_drive_partition /mnt/usb
    ```

6. Copy in `efimux.efi` as `efi/boot/bootx64.efi`:

    ```sh
    mkdir -p /mnt/usb/efi/boot/
    cp efimux.efi /mnt/usb/efi/boot/bootx64.efi
    ```

7. Done! Now you are ready to boot from the USB.

## Disclaimer

No artificial intelligence was used in the making of this.

<a href="https://brainmade.org/">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://brainmade.org/white-logo.svg">
  <source media="(prefers-color-scheme: light)" srcset="https://brainmade.org/black-logo.svg">
  <img alt="brainmade" src="https://brainmade.org/white-logo.svg">
</picture>
</a>
