use std::path::PathBuf;

use anyhow::Result;

use crate::{
    packages::gcc::{GccStage, install_gcc},
    packages::glibc::install_glibc_sysroot,
    packages::linux,
    packages::musl::install_musl_sysroot,
    profile::{Libc, Toolchain},
};

/// Create and populate a sysroot for a target.
///
/// This:
///   1. Creates the sysroot directory
///   2. Installs Linux kernel headers into the sysroot
///   3. Builds a stage1 cross-compiler to configure and build glibc into the sysroot
///
/// The caller must already have installed binutils.
pub fn setup_sysroot(toolchain: &Toolchain, jobs: u64) -> Result<PathBuf> {
    log::info!("=> setup sysroot");

    let sysroot = toolchain.sysroot()?;
    std::fs::create_dir_all(&sysroot)?;
    std::fs::create_dir_all(sysroot.join("usr").join("include"))?;
    std::fs::create_dir_all(sysroot.join("usr").join("lib"))?;

    // 1. install linux headers
    linux::install_headers(&toolchain)?;

    install_gcc(&toolchain, jobs, GccStage::Stage1)?;

    match toolchain.libc {
        Libc::Musl(_) => {
            install_musl_sysroot(&toolchain)?;
        }
        _ => {
            install_glibc_sysroot(&toolchain)?;
        }
    }

    Ok(sysroot)
}
