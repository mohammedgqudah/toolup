use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use anyhow::{Context, Result};

use crate::{
    download::{cross_prefix, download_and_decompress},
    make::{run_configure_in, run_make_in},
    profile::Target,
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
    target: &Target,
    version: impl AsRef<str>,
    jobs: u64,
    stage: GccStage,
) -> Result<()> {
    let gcc_name = format!("gcc-{}", version.as_ref());
    let tarball = format!("{gcc_name}.tar.xz");

    let gcc_dir = download_and_decompress(
        format!("https://ftp.gnu.org/gnu/gcc/{gcc_name}/{tarball}"),
        gcc_name,
        true,
    )
    .context("failed to download gcc")?;

    let jobs = jobs.to_string();
    match stage {
        GccStage::Stage1 => {
            println!("=> stage1 gcc");
            let objdir = gcc_dir.join(format!("objdir-stage1-arch-{}", target.to_string()));
            std::fs::create_dir_all(&objdir).context("failed to create an objdir for the arch")?;

            let t = format!("--target={}", target.to_string());
            run_configure_in(
                &objdir,
                &[
                    t.as_str(),
                    format!("--prefix={}", cross_prefix()?.display()).as_str(),
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

            let objdir = gcc_dir.join(format!("objdir-final-arch-{}", target.to_string()));
            std::fs::create_dir_all(&objdir).context("failed to create an objdir for the arch")?;

            let mut args: Vec<String> = vec![
                format!("--target={}", target.to_string()),
                format!("--prefix={}", cross_prefix()?.display()),
                "--disable-nls".into(),
                "--enable-languages=c,c++".into(),
                "--disable-multilib".into(),
            ];
            if let Some(sysroot) = maybe_sysroot {
                let p = sysroot.as_os_str().to_str().unwrap().to_string();
                args.push(format!("--with-sysroot={}", p));
            }

            run_configure_in(&objdir, args.as_ref())?;

            // hosted/newlib: build everything (gcc, libgcc, libstdc++)
            run_make_in(&objdir, &["-j", jobs.as_str()])?;
            run_make_in(&objdir, &["install", "-j", jobs.as_str()])?;
        }
    }
    Ok(())
}
