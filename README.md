
## Running in QEMU

```
pacman -S qemu-system-x86_64 edk2-ovmf
```
(optionally some graphic backend, but you can also VNC into the VM)
```
pacman -S qemu-ui-sdl
```

then

```
just create-virtual-uefi-layout
```

and then

```
just qemu
```
