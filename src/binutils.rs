use anyhow::{Context, Result};

use crate::{
    download::{cross_prefix, download_and_decompress},
    make::{run_configure_in, run_make_in},
    profile::Target,
};

/// Download and build binutils.
pub fn install_binutils(target: &Target, jobs: u64) -> Result<()> {
    println!("=> install binutils");

    let binutils_dir = download_and_decompress(
        "https://ftp.gnu.org/gnu/binutils/binutils-2.45.tar.xz",
        "binutils-2.45",
        true,
    )
    .context("failed to download binutils")?;

    let arch_dir = binutils_dir.join(format!("objdir-arch-{}", target.to_string()));

    std::fs::create_dir_all(&arch_dir).context("failed to create an objdir for the arch")?;

    run_configure_in(
        &arch_dir,
        &[
            "--target",
            target.to_string().as_str(),
            "--prefix",
            cross_prefix()?.to_str().unwrap(),
            "--disable-nls",
            "--disable-werror",
        ],
    )?;
    let jobs = jobs.to_string();
    run_make_in(&arch_dir, &["-j", jobs.as_str()])?;
    run_make_in(&arch_dir, &["install", "-j", jobs.as_str()])?;
    Ok(())
}
