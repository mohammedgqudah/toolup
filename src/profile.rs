use anyhow::{Result, anyhow};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    I686,
    Aarch64,
    Armv7,
    Riscv64,
    Ppc64Le,
    Ppc64,
    Avr,
    Bpf,
    Xtensa,
}

impl ToString for Arch {
    fn to_string(&self) -> String {
        match self {
            Arch::X86_64 => "x86_64".into(),
            Arch::I686 => "i686".into(),
            Arch::Aarch64 => "aarch64".into(),
            Arch::Armv7 => "armv7".into(),
            Arch::Riscv64 => "riscv64".into(),
            Arch::Ppc64Le => "ppc64le".into(),
            Arch::Ppc64 => "ppc64".into(),
            Arch::Avr => "avr".into(),
            Arch::Bpf => "bpf".into(),
            Arch::Xtensa => "xtensa".into(),
        }
    }
}

impl Arch {
    /// Return an architecture string to be used the `ARCH` parameter when building the kernel.
    pub fn to_kernel_arch(self) -> &'static str {
        match self {
            Arch::X86_64 => "x86",
            Arch::I686 => "x86",
            Arch::Aarch64 => "arm64",
            Arch::Armv7 => "arm",
            Arch::Riscv64 => "riscv",
            Arch::Ppc64Le => "powerpc",
            Arch::Ppc64 => "powerpc",
            Arch::Xtensa => "xtensa",
            Arch::Avr => unreachable!(),
            Arch::Bpf => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    None, // bare-metal
    Linux,
}

impl ToString for Os {
    fn to_string(&self) -> String {
        match self {
            Os::None => "none".into(),
            Os::Linux => "linux".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abi {
    Gnu,
    Musl,
    Msvc,
    Eabi,
    Eabihf,
    GnuEabi,
    GnuEabihf,
    Elf,
}

impl ToString for Abi {
    fn to_string(&self) -> String {
        match self {
            Abi::Gnu => "gnu".into(),
            Abi::Musl => "musl".into(),
            Abi::Msvc => "msvc".into(),
            Abi::Eabi => "eabi".into(),
            Abi::Eabihf => "eabihf".into(),
            Abi::GnuEabi => "gnueabi".into(),
            Abi::GnuEabihf => "gnueabihf".into(),
            Abi::Elf => "elf".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    Unknown,
    Pc,
    Esp32,
    Esp32S2,
    Esp32S3,
    //Apple,
}

impl ToString for Vendor {
    fn to_string(&self) -> String {
        match self {
            Vendor::Unknown => "unknown".into(),
            Vendor::Pc => "pc".into(),
            Vendor::Esp32 => "esp32".into(),
            Vendor::Esp32S2 => "esp32s2".into(),
            Vendor::Esp32S3 => "esp32s3".into(),
        }
    }
}

impl FromStr for Arch {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "x86_64" => Ok(Arch::X86_64),
            "i686" => Ok(Arch::I686),
            "aarch64" => Ok(Arch::Aarch64),
            "armv7" => Ok(Arch::Armv7),
            "riscv64" => Ok(Arch::Riscv64),
            "ppc64le" => Ok(Arch::Ppc64Le),
            "ppc64" => Ok(Arch::Ppc64),
            "avr" => Ok(Arch::Avr),
            "bpf" => Ok(Arch::Bpf),
            "xtensa" => Ok(Arch::Xtensa),
            _ => Err(anyhow!("unsupported architecture")),
        }
    }
}
impl FromStr for Abi {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "elf" => Ok(Abi::Elf),
            "gnu" => Ok(Abi::Gnu),
            "musl" => Ok(Abi::Musl),
            "msvc" => Ok(Abi::Msvc),
            "eabi" => Ok(Abi::Eabi),
            "gnueabi" => Ok(Abi::GnuEabi),
            "eabihf" => Ok(Abi::Eabihf),
            "gnueabihf" => Ok(Abi::GnuEabihf),
            _ => Err(anyhow!("unsupported abi")),
        }
    }
}

impl FromStr for Vendor {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "unknown" => Ok(Vendor::Unknown),
            "pc" => Ok(Vendor::Pc),
            "esp32" => Ok(Vendor::Esp32),
            "esp32s2" => Ok(Vendor::Esp32S2),
            "esp32s3" => Ok(Vendor::Esp32S3),
            _ => Err(anyhow!("unsupported vendor")),
        }
    }
}

impl FromStr for Os {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "none" => Ok(Os::None),
            "linux" => Ok(Os::Linux),
            //"windows" => Ok(Os::Windows),
            //"darwin" => Ok(Os::Darwin),
            //"freebsd" => Ok(Os::FreeBsd),
            //"netbsd" => Ok(Os::NetBsd),
            //"openbsd" => Ok(Os::OpenBsd),
            _ => Err(anyhow!("unsupported os")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub arch: Arch,
    pub vendor: Vendor,
    pub os: Os,
    pub abi: Abi,
}

impl ToString for Target {
    fn to_string(&self) -> String {
        match self {
            Target {
                arch: Arch::Bpf, ..
            } => "bpf-unknown-none".into(),
            Target {
                arch: Arch::Xtensa,
                vendor,
                ..
            } => {
                format!("xtensa-{}-elf", vendor.to_string())
            }
            // GNU tools will not understand the full format for freestanding targets.
            Target {
                arch,
                vendor: Vendor::Unknown,
                os: Os::None,
                abi: Abi::Elf,
            } => {
                format!("{}-elf", arch.to_string())
            }
            Target {
                arch,
                vendor,
                os,
                abi,
            } => {
                format!(
                    "{arch}-{vendor}-{os}-{abi}",
                    arch = arch.to_string(),
                    vendor = vendor.to_string(),
                    os = os.to_string(),
                    abi = abi.to_string()
                )
            }
        }
    }
}

impl FromStr for Target {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('-').collect();

        match parts.as_slice() {
            ["bpf", ..] => {
                if s != "bpf-unknown-none" {
                    Err(anyhow!("use `bpf-unknown-none`",))
                } else {
                    Ok(Target {
                        arch: Arch::Bpf,
                        vendor: Vendor::Unknown,
                        os: Os::None,
                        abi: Abi::Elf,
                    })
                }
            }
            [arch, "elf"] => Ok(Target {
                arch: Arch::from_str(arch)?,
                vendor: Vendor::Unknown,
                os: Os::None,
                abi: Abi::Elf,
            }),
            ["xtensa", vendor @ ("esp32" | "esp32s2" | "esp32s3"), "elf"] => Ok(Target {
                arch: Arch::Xtensa,
                vendor: Vendor::from_str(vendor)?,
                os: Os::None,
                abi: Abi::Elf,
            }),
            ["xtensa", ..] => Err(anyhow!("unknown xtensa toolchain",)),
            // GNU tools will not understand the full format for freestanding targets.
            [arch, "unknown", "none", "elf"] => Err(anyhow!(
                "use <arch>-elf for freestanding targets. use: {}-elf",
                arch
            )),
            [arch, vendor, "none", abi] => {
                let abi = Abi::from_str(abi)?;
                match abi {
                    Abi::Eabi | Abi::Eabihf | Abi::Elf => {}
                    _ => {
                        return Err(anyhow!(
                            "unsupported abi `{}` for os `none`",
                            abi.to_string(),
                        ));
                    }
                };
                Ok(Target {
                    arch: Arch::from_str(arch)?,
                    vendor: Vendor::from_str(vendor)?,
                    os: Os::None,
                    abi,
                })
            }
            // 4 parts: arch-vendor-os-abi
            [arch, vendor, os, abi] => {
                //
                Ok(Target {
                    arch: Arch::from_str(arch)?,
                    vendor: Vendor::from_str(vendor)?,
                    os: Os::from_str(os)?,
                    abi: Abi::from_str(abi)?,
                })
            }
            _ => Err(anyhow!(
                "invalid target format, use the canonical format arch-vendor-os-abi"
            )),
        }
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::{Abi, Arch, Os, Target, Vendor};
    use anyhow::Result;

    #[test]
    pub fn test() -> Result<()> {
        assert_eq!(
            Target::from_str("x86_64-unknown-none-elf")?,
            Target {
                arch: Arch::X86_64,
                vendor: Vendor::Unknown,
                os: Os::None,
                abi: Abi::Elf
            }
        );
        assert_eq!(
            Target::from_str("armv7-unknown-linux-gnueabi")?,
            Target {
                arch: Arch::Armv7,
                vendor: Vendor::Unknown,
                os: Os::Linux,
                abi: Abi::GnuEabi
            }
        );
        assert_eq!(
            Target::from_str("armv7-pc-linux-gnueabi")?,
            Target {
                arch: Arch::Armv7,
                vendor: Vendor::Pc,
                os: Os::Linux,
                abi: Abi::GnuEabi
            }
        );
        assert_eq!(
            Target::from_str("i686-unknown-none-gnu")?,
            Target {
                arch: Arch::I686,
                vendor: Vendor::Unknown,
                os: Os::None,
                abi: Abi::Gnu
            }
        );
        assert_eq!(
            Target::from_str("i686-unknown-linux-gnu")?,
            Target {
                arch: Arch::I686,
                vendor: Vendor::Unknown,
                os: Os::Linux,
                abi: Abi::Gnu
            }
        );
        assert_eq!(
            Target::from_str("ppc64-unknown-linux-gnu")?,
            Target {
                arch: Arch::Ppc64,
                vendor: Vendor::Unknown,
                os: Os::Linux,
                abi: Abi::Gnu
            }
        );
        assert_eq!(
            Target::from_str("ppc64le-unknown-linux-gnu")?,
            Target {
                arch: Arch::Ppc64Le,
                vendor: Vendor::Unknown,
                os: Os::Linux,
                abi: Abi::Gnu
            }
        );

        Ok(())
    }
}
