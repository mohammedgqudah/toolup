use std::path::PathBuf;

use anyhow::Result;

use crate::{
    download::cache_dir,
    gcc::{GccStage, Sysroot, install_gcc},
    glibc::install_glibc_sysroot,
    linux,
    profile::Profile,
};

pub fn setup_sysroot(
    architecture: impl AsRef<str>,
    profile: Profile,
    jobs: u64,
) -> Result<PathBuf> {
    println!("=> setup sysroot");

    let sysroot = cache_dir()?.join(format!(
        "sysroot-{arch}-{profile:#?}",
        arch = architecture.as_ref(),
        profile = profile
    ));
    std::fs::create_dir_all(&sysroot)?;
    std::fs::create_dir_all(sysroot.join("usr").join("include"))?;
    std::fs::create_dir_all(sysroot.join("usr").join("lib"))?;

    // 1. install linux headers
    linux::install_headers(architecture.as_ref(), &sysroot)?;

    install_gcc(architecture.as_ref(), profile, jobs, GccStage::Stage1)?;

    install_glibc_sysroot(Sysroot(sysroot.clone()), profile, architecture)?;

    Ok(sysroot)
}
