use std::path::PathBuf;

use anyhow::Result;

use crate::{
    download::sysroots_dir,
    gcc::{GccStage, Sysroot, install_gcc},
    glibc::install_glibc_sysroot,
    linux,
    musl::install_musl_sysroot,
    profile::{Abi, Target},
};

/// Create and populate a sysroot for a target.
///
/// This:
///   1. Creates the sysroot directory
///   2. Installs Linux kernel headers into the sysroot
///   3. Builds a stage1 cross-compiler to configure and build glibc into the sysroot
///
/// The caller must already have installed binutils.
pub fn setup_sysroot(target: &Target, gcc_version: impl AsRef<str>, jobs: u64) -> Result<PathBuf> {
    log::info!("=> setup sysroot");

    let sysroot = sysroots_dir()?.join(format!("sysroot-{}", target.to_string()));
    std::fs::create_dir_all(&sysroot)?;
    std::fs::create_dir_all(sysroot.join("usr").join("include"))?;
    std::fs::create_dir_all(sysroot.join("usr").join("lib"))?;

    // 1. install linux headers
    linux::install_headers(target, &sysroot)?;

    install_gcc(target, gcc_version, jobs, GccStage::Stage1)?;

    match target.abi {
        Abi::Musl => {
            install_musl_sysroot(target, Sysroot(sysroot.clone()))?;
        }
        _ => {
            install_glibc_sysroot(target, Sysroot(sysroot.clone()))?;
        }
    }

    Ok(sysroot)
}
