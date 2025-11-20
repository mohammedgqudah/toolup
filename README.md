# Toolup
A tool to manage C toolchains and building Linux kernels.

I built this tool for two main reasons:

1. To have a simple interface for building cross-compilers and managing toolchains for C. (e.g. `toolup install aarch64-unknown-linux-musl`)
2. To quickly build different versions of the Linux kernel and run programs inside a VM using that kernel - for research or compatibility tests. (e.g. `toolup linux 5.11 ./my-bin`)

## Toolchains
A single toochain consists of a **target**, **binutils**, **gcc** and **libc** (musl, or glibc). The install command accepts an optional version for each component, if none are specified, the latest version will be used.
You can have multiple toolchains for the same target (i.e. a different gcc or binutils version), and toolup will read `toolup.toml` to see which toolchain to use when invoking the compiler via `toolup cc`.


## Usage Examples
`toolup install`

```bash
toolup install avr-elf
toolup install x86_64-elf
toolup install i686-elf
toolup install riscv64-unknown-linux-gnu
toolup install armv7-unknown-none-eabihf
toolup install bpf-unknown-none
toolup install aarch64-unknown-none-gnu
```

`toolup linux`

```bash
# quickly build a kernel image and a minimal rootfs and start qemu-system-<arch> in the terminal
toolup linux 6.16 -t riscv64-unknown-linux-gnu

# -m will open the kernel menuconfig, since this is `ppc64-`, we can configure a big endian kernel
toolup linux 6.17 -t ppc64-unknown-linux-gnu -j20 -m
```

qemu userspace emulation
```
aarch64-unknown-linux-gnu-gcc test.c -o test
qemu-aarch64 -L `aarch64-unknown-linux-gnu-gcc -print-sysroot` ./test
```

# TODO
- We still have a dependency on the host (e.g. when compiling kernel host tools) and that's the reason I can't build old kernels or older GCC versions.

## Screenshots

```
λ tree -L 2 ~/.toolup
/home/hyper/.toolup
├── linux-images
│   ├── aarch64-unknown-linux-gnu-5.10
│   ├── x86_64-unknown-linux-gnu-5.10
│   ├── x86_64-unknown-linux-gnu-5.11
│   ├── x86_64-unknown-linux-gnu-6.12
│   └── x86_64-unknown-linux-gnu-6.17
├── sysroot
│   ├── sysroot-aarch64-unknown-linux-gnu-gcc-15.2.0-bin-2.45-glibc-2.42
│   ├── sysroot-aarch64-unknown-linux-gnu-glibc-2.35
│   ├── sysroot-aarch64-unknown-linux-musl-musl-1.2.5
│   ├── sysroot-x86_64-unknown-linux-gnu-glibc-2.35
│   ├── sysroot-x86_64-unknown-linux-gnu-glibc-2.42
│   └── sysroot-x86_64-unknown-linux-musl-gcc-15.2.0-bin-2.45-musl-1.2.5
└── toolchains
    ├── aarch64-elf-gcc-15.2.0-bin-2.45-glibc-2.42
    ├── aarch64-unknown-linux-gnu-gcc-15.2.0-bin-2.34-glibc-2.35
    ├── aarch64-unknown-linux-gnu-gcc-15.2.0-bin-2.45-glibc-2.42
    ├── aarch64-unknown-linux-musl-gcc-15.2.0-bin-2.45-musl-1.2.5
    ├── x86_64-elf-gcc-15.2.0-bin-2.45-glibc-2.42
    ├── x86_64-unknown-linux-gnu-gcc-15.2.0-bin-2.34-glibc-2.35
    ├── x86_64-unknown-linux-gnu-gcc-15.2.0-bin-2.45-glibc-2.42
    └── x86_64-unknown-linux-musl-gcc-15.2.0-bin-2.45-musl-1.2.5
```

```
λ toolup install aarch64-elf
Toolchain: aarch64-elf
├─ GCC: 15.2.0
├─ Binutils: 2.45
├─ Libc: glibc-2.42

/home/hyper/.toolup/toolchains/aarch64-elf-gcc-15.2.0-bin-2.45-glibc-2.42/bin
toolchain is already installed
```
