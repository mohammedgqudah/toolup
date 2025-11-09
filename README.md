This is my personal toolchain installer that I use to easily build cross compilers.

It doesn't do anything fancy besides running `make` and `configure` with some defaults that make sense to me. I'm 100% it only works on my machine (x86_64 GNU/Linux Archlinux) 

it manages:
1. bunutils
2. gcc
3. glibc
4. linux kernel

## Usage Examples
```bash
toolup toolchain avr-elf
toolup toolchain x86_64-elf
toolup toolchain i686-elf
toolup toolchain riscv64-unknown-linux-gnu
toolup toolchain armv7-unknown-none-eabihf
```

```bash
# quickly build a kernel image and a minimal rootfs and start qemu-system-<arch> in the terminal
toolup linux 6.16 -t riscv64-unknown-linux-gnu

# -m will open the kernel menuconfig, since this is `ppc64-`, we can configure a big endian kernel
toolup linux 6.17 -t ppc64-unknown-linux-gnu -j20 -m
```

## Screenshots
<img width="500" alt="image" src="https://github.com/user-attachments/assets/a876bfac-97fc-424b-85dc-f92bbbf0c208" />

<img width="500"  alt="image" src="https://github.com/user-attachments/assets/580d9b8b-6f19-4b27-9ae9-4692f63d352a" />
