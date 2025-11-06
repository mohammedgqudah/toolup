#[derive(Debug, Clone, Copy)]
pub enum Profile {
    // No libc
    Freestanding,
    // Bare metal with newlib
    Newlib,
    // Linux with glibc (*-linux-gnu*)
    LinuxGlibc,
    // Linux with musl (*-linux-musl*)
    LinuxMusl,
    // Windows (*-w64-mingw32)
    MingwW64,
    // sdk/sysroot provided
    ExternalSysroot,
}

// TODO: this auto detection was an AI auto-complete, review later.
pub fn select_profile<S: AsRef<str>>(arch: S, libc: Option<S>) -> Profile {
    // If user explicitly requests a libc:
    if let Some(l) = libc {
        match l.as_ref() {
            "newlib" => return Profile::Newlib,
            "glibc" => return Profile::LinuxGlibc,
            "musl" => return Profile::LinuxMusl,
            "none" => return Profile::Freestanding,
            _ => {} // fallthrough to auto-detect
        }
    }

    // Triple-based automatic detection:
    if arch.as_ref().contains("w64-mingw32") {
        return Profile::MingwW64;
    }

    if arch.as_ref().contains("linux-musl") {
        return Profile::LinuxMusl;
    }

    if arch.as_ref().contains("linux-gnu") || arch.as_ref().contains("gnueabihf") {
        return Profile::LinuxGlibc;
    }

    // Bare-metal targets typically end in -elf, -none-*, -eabi
    let bare = ["-elf", "-eabi", "-none", "-none-eabi", "-unknown-elf"];

    if bare.iter().any(|pat| arch.as_ref().contains(pat)) {
        // If user didn't explicitly request newlib, default = freestanding
        return Profile::Freestanding;
    }

    // Fallback: require external sysroot
    Profile::ExternalSysroot
}

pub fn kernel_arch<'a>(triple: &'a str) -> &'a str {
    let t = triple.to_lowercase();

    if t.contains("x86_64") {
        return "x86";
    }
    if t.contains("i386") || t.contains("i486") || t.contains("i586") || t.contains("i686") {
        return "x86";
    }
    if t.contains("aarch64") {
        return "arm64";
    }
    if t.contains("arm") {
        return "arm";
    }

    if t.contains("riscv") {
        return "riscv";
    }

    if t.contains("mips") {
        return "mips";
    }

    if t.contains("powerpc") || t.contains("ppc") {
        return "powerpc";
    }

    if t.contains("sparc") {
        return "sparc";
    }

    if t.contains("sh4") || t.contains("sh2") || t.contains("sh") {
        return "sh";
    }

    // fallback: assume same as LLVM/GCC arch name segment
    // (safe enough for weird targets; kernel may still not support it)
    triple.split('-').next().unwrap_or(triple)
}
