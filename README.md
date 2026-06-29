<div align="center">

# `ratatuefi`

https://github.com/user-attachments/assets/7f05d64a-27ca-4deb-954a-3c2c53765832

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

TBD..

## Ideas for improving

it is slow as hell to draw. could possibly be improved by

- adding a buffer, and rendering full lines? currently we are rawdoggin character-for-character.
- switching to manual VGA graphics (use mousefood?)
