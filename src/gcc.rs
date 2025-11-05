use anyhow::{Context, Result};

use crate::{
    download::{
        DownloadResult::{Cached, Created, Replaced},
        cache_dir, decompress_tar_xz, download,
    },
    make::{run_configure_in, run_make_in},
};

pub fn install_gcc(architecture: String, jobs: u64) -> Result<()> {
    match download(
        "https://ftp.gnu.org/gnu/gcc/gcc-15.2.0/gcc-15.2.0.tar.xz",
        "gcc-15.2.0.tar.xz",
        true,
    )
    .context("failed to download gcc")?
    {
        Replaced(path) | Created(path) | Cached(path) => {
            let gcc = cache_dir()?.join("gcc-15.2.0");
            let arch = gcc.join(format!("objdir-arch-{}", architecture));
            decompress_tar_xz(path, cache_dir()?)?;
            std::fs::create_dir_all(&arch).context("failed to create an objdir for the arch")?;
            run_configure_in(
                &arch,
                &[
                    "--target",
                    architecture.as_str(),
                    "--prefix",
                    "/home/hyper/opt/cross",
                    "--disable-nls",
                    "--enable-languages=c,c++",
                    "--without-headers",
                    "--disable-hosted-libstdcxx"
                ],
            )?;
            run_make_in(&arch, &["all-gcc", "-j", jobs.to_string().as_str()])?;
            run_make_in(&arch, &["all-target-libgcc", "-j", jobs.to_string().as_str()])?;
            run_make_in(&arch, &["all-target-libstdc++-v3", "-j", jobs.to_string().as_str()])?;
            run_make_in(&arch, &["install-gcc", "-j", jobs.to_string().as_str()])?;
            run_make_in(&arch, &["install-target-libgcc", "-j", jobs.to_string().as_str()])?;
            run_make_in(&arch, &["install-target-libstdc++-v3", "-j", jobs.to_string().as_str()])?;
        }
    }

    Ok(())
}
