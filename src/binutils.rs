use anyhow::{Context, Result};

use crate::{
    download::{
        DownloadResult::{Cached, Created, Replaced},
        cache_dir, cross_prefix, decompress_tar_xz, download,
    },
    make::{run_configure_in, run_make_in},
};

pub fn install_binutils(architecture: String, jobs: u64) -> Result<()> {
    println!("=> install binutils");

    let binutils_dir = cache_dir()?.join("binutils-2.45");
    let arch_dir = binutils_dir.join(format!("objdir-arch-{}", architecture));

    // download the binutils source if the directory doesn't exist.
    if !binutils_dir.exists() {
        let download_result = download(
            "https://ftp.gnu.org/gnu/binutils/binutils-2.45.tar.xz",
            "binutils-2.45.tar.xz",
            true,
        )
        .context("failed to download binutils")?;

        let path = match download_result {
            Replaced(p) | Created(p) | Cached(p) => p,
        };

        decompress_tar_xz(path, cache_dir()?)?;
    }

    std::fs::create_dir_all(&arch_dir).context("failed to create an objdir for the arch")?;
    run_configure_in(
        &arch_dir,
        &[
            "--target",
            architecture.as_str(),
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
