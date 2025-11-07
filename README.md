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
toolup toolchain riscv64-linux-gnu
toolup toolchain x86_64-elf
```

```bash
# quickly build a kernel image and a minimal rootfs and start qemu-system-<arch> in the terminal
toolup linux --linux 6.16 -a riscv64-linux-gnu
```

## Screenshots
<img width="500" alt="image" src="https://github.com/user-attachments/assets/a876bfac-97fc-424b-85dc-f92bbbf0c208" />

<img width="500"  alt="image" src="https://github.com/user-attachments/assets/580d9b8b-6f19-4b27-9ae9-4692f63d352a" />
