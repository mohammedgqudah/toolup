use std::str::FromStr;

use crate::{
    packages::{
        binutils::{Binutils, BinutilsVersion, install_binutils},
        gcc::{GCC, GCCVersion, GccStage, Sysroot, install_gcc},
        glibc::GlibcVersion,
        linux::KernelVersion,
        musl::MuslVersion,
    },
    profile::{Abi, Libc, Target, Toolchain},
    sysroot::setup_sysroot,
};
use anyhow::Result;

pub mod commands;
pub mod config;
pub mod cpio;
pub mod download;
pub mod packages;
pub mod profile;
pub mod qemu;
pub mod sysroot;

/// Similar to `install_toolchain` but will parse the toolchain from strings.
pub fn install_toolchain_str(
    target_str: String,
    gcc_str: String,
    libc_str: String,
    binutils_str: String,
    kernel_version: Option<&KernelVersion>,
    jobs: u64,
    force: bool,
) -> Result<Toolchain> {
    let target = Target::from_str(&target_str)?;
    let binutils = Binutils::new(BinutilsVersion::from_str(&binutils_str)?);
    let gcc = GCC::new(GCCVersion::from_str(&gcc_str)?);
    let libc = match target.abi {
        Abi::Musl => Libc::Musl(MuslVersion::from_str(&libc_str)?),
        _ => Libc::Glibc(GlibcVersion::from_str(&libc_str)?),
    };

    let toolchain = if let Some(kernel_version) = kernel_version {
        Toolchain::new_with_kernel(target, binutils, gcc, libc, kernel_version.clone())
    } else {
        Toolchain::new(target, binutils, gcc, libc)
    };

    install_toolchain(toolchain, jobs, force)
}

/// Install a toolchain.
///
/// use `force` to forcefully re-install a toolchain if it was already installed.
pub fn install_toolchain(toolchain: Toolchain, jobs: u64, force: bool) -> Result<Toolchain> {
    println!("{}", toolchain);

    log::info!("export PATH=\"{}:$PATH\"", toolchain.bin_dir()?.display());
    log::info!("export SYSROOT={}", toolchain.sysroot()?.display());
    log::info!(
        "export PKG_CONFIG_SYSROOT_DIR={}",
        toolchain.sysroot()?.display()
    );
    log::info!("export TARGET={}", toolchain.target);
    log::info!("");

    if toolchain.gcc_bin()?.exists() && !force {
        log::info!("toolchain is already installed");
        return Ok(toolchain);
    }

    match toolchain.target {
        // freestanding
        Target {
            abi: Abi::Elf | Abi::Eabihf | Abi::Eabi,
            ..
        } => {
            install_binutils(&toolchain, jobs)?;
            install_gcc(&toolchain, jobs, GccStage::Stage1)?;
        }
        Target {
            abi: Abi::Gnu | Abi::GnuEabi | Abi::GnuEabihf | Abi::Musl,
            ..
        } => {
            install_binutils(&toolchain, jobs)?;
            let sysroot = setup_sysroot(&toolchain, jobs)?;
            install_gcc(&toolchain, jobs, GccStage::Final(Some(Sysroot(sysroot))))?;
        }
        _ => unimplemented!(),
    };

    Ok(toolchain)
}
