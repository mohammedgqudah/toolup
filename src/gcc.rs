use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use anyhow::{Context, Result};

use crate::{
    download::{
        DownloadResult::{Cached, Created, Replaced},
        cache_dir, decompress_tar_xz, download,
    },
    make::{run_configure_in, run_make_in},
    profile::Profile,
};

pub struct Sysroot(pub PathBuf);
impl Deref for Sysroot {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Sysroot {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub enum GccStage {
    Stage1,
    Final(Option<Sysroot>),
}

pub fn install_gcc(
    architecture: impl AsRef<str>,
    profile: Profile,
    jobs: u64,
    stage: GccStage,
) -> Result<()> {
    let architecture = architecture.as_ref();
    let gcc_dir = cache_dir()?.join("gcc-15.2.0");

    if !gcc_dir.exists() {
        let download_result = download(
            "https://ftp.gnu.org/gnu/gcc/gcc-15.2.0/gcc-15.2.0.tar.xz",
            "gcc-15.2.0.tar.xz",
            true,
        )
        .context("failed to download gcc")?;

        let path = match download_result {
            Replaced(p) | Created(p) | Cached(p) => p,
        };
        decompress_tar_xz(path, cache_dir()?)?;
    }

    let jobs = jobs.to_string();
    match stage {
        GccStage::Stage1 => {
            println!("=> stage1 gcc");
            let objdir = gcc_dir.join(format!("objdir-stage1-arch-{}", architecture));
            std::fs::create_dir_all(&objdir).context("failed to create an objdir for the arch")?;

            let t = format!("--target={architecture}");
            run_configure_in(
                &objdir,
                &[
                    t.as_str(),
                    "--prefix=/home/hyper/opt/cross",
                    "--disable-nls",
                    "--enable-languages=c,c++".into(),
                    "--without-headers".into(),
                    "--disable-threads".into(),
                    "--disable-shared".into(),
                    "--disable-libssp".into(),
                    "--disable-libgomp".into(),
                    "--disable-libquadmath".into(),
                    "--disable-multilib".into(),
                ],
            )?;
            run_make_in(&objdir, &["all-gcc", "-j", jobs.as_str()])?;
            run_make_in(&objdir, &["install-gcc", "-j", jobs.as_str()])?;
            run_make_in(&objdir, &["all-target-libgcc", "-j", jobs.as_str()])?;
            run_make_in(&objdir, &["install-target-libgcc", "-j", jobs.as_str()])?;
        }
        GccStage::Final(maybe_sysroot) => {
            println!("=> final stage gcc");

            let objdir = gcc_dir.join(format!("objdir-final-arch-{}", architecture));
            std::fs::create_dir_all(&objdir).context("failed to create an objdir for the arch")?;

            let mut args: Vec<String> = vec![
                "--target".into(),
                architecture.to_string(),
                "--prefix".into(),
                "/home/hyper/opt/cross".into(),
                "--disable-nls".into(),
                "--enable-languages=c,c++".into(),
                "--disable-multilib".into(),
            ];
            if let Some(sysroot) = maybe_sysroot {
                let p = sysroot.as_os_str().to_str().unwrap().to_string();
                args.push(format!("--with-sysroot={}", p));
            }

            run_configure_in(&objdir, args.as_ref())?;

            let build_libstdcxx = match profile {
                Profile::Freestanding => false,
                _ => true,
            };

            if build_libstdcxx {
                // hosted/newlib: build everything (gcc, libgcc, libstdc++)
                run_make_in(&objdir, &["-j", jobs.as_str()])?;
                run_make_in(&objdir, &["install", "-j", jobs.as_str()])?;
            } else {
                // pure freestanding: compiler already installed in Stage1; only build libgcc if desired
                run_make_in(&objdir, &["all-target-libgcc", "-j", jobs.as_str()])?;
                run_make_in(&objdir, &["install-target-libgcc", "-j", jobs.as_str()])?;
            }
        }
    }
    Ok(())
}
