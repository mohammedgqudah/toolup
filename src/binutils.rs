use anyhow::{Context, Result};

use crate::{
    download::{
        DownloadResult::{Cached, Created, Replaced},
        cache_dir, decompress_tar_xz, download,
    },
    make::{run_configure_in, run_make_in},
};

pub fn install_binutils(architecture: String, jobs: u64) -> Result<()> {
    match download(
        "https://ftp.gnu.org/gnu/binutils/binutils-2.45.tar.xz",
        "binutils-2.45.tar.xz",
        true,
    )
    .context("failed to download binutils")?
    {
        Replaced(path) | Created(path) | Cached(path) => {
            let binutils = cache_dir()?.join("binutils-2.45");
            let arch = binutils.join(format!("objdir-arch-{}", architecture));
            decompress_tar_xz(path, cache_dir()?)?;
            std::fs::create_dir_all(&arch).context("failed to create an objdir for the arch")?;
            run_configure_in(
                &arch,
                &[
                    "--target",
                    architecture.as_str(),
                    "--prefix",
                    "/home/hyper/opt/cross",
                    "--with-sysroot",
                    "--disable-nls",
                    "--disable-werror",
                ],
            )?;
            run_make_in(&arch, &["-j", jobs.to_string().as_str()])?;
            run_make_in(&arch, &["install", "-j", jobs.to_string().as_str()])?;
        }
    }

    Ok(())
}
