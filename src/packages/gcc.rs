use std::{
    ffi::OsString,
    fmt::Display,
    ops::{Deref, DerefMut},
    path::PathBuf,
    str::FromStr,
};

use anyhow::{Context, Result, anyhow};

use crate::{commands::run_command_in, download::download_and_decompress, profile::Toolchain};

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
    /// Build a stage-1 gcc compiler. Either for a freestanding target or to bootstrap GCC (to
    /// build the [`GccStage::Final`] compiler)
    Stage1,
    /// Build a full compiler using a bootstrap compiler from [`GccStage::Stage1`]
    Final(Option<Sysroot>),
}

pub fn install_gcc(toolchain: &Toolchain, jobs: u64, stage: GccStage) -> Result<()> {
    let gcc_name = format!("gcc-{}", toolchain.gcc.version);
    let tarball = if toolchain.gcc.version <= GCCVersion(10, 1, 0) {
        format!("{gcc_name}.tar.gz")
    } else {
        format!("{gcc_name}.tar.xz")
    };

    let gcc_dir = download_and_decompress(
        format!("https://ftp.gnu.org/gnu/gcc/{gcc_name}/{tarball}"),
        gcc_name,
        true,
    )
    .context("failed to download gcc")?;

    let jobs = jobs.to_string();
    match stage {
        GccStage::Stage1 => {
            log::info!("=> stage1 gcc");
            let objdir = gcc_dir.join(format!("objdir-stage1-{}", toolchain.id()));
            std::fs::create_dir_all(&objdir).context("failed to create an objdir for the arch")?;

            let env: Vec<(OsString, OsString)> = vec![("PATH".into(), toolchain.env_path()?)];

            run_command_in(
                &objdir,
                "configure",
                objdir.parent().unwrap().join("configure"),
                &[
                    format!("--target={}", toolchain.target).as_str(),
                    format!("--prefix={}", toolchain.dir()?.display()).as_str(),
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
                Some(env.clone()),
            )?;
            run_command_in(
                &objdir,
                "make",
                "make",
                &["all-gcc", "-j", jobs.as_str()],
                Some(env.clone()),
            )?;
            run_command_in(
                &objdir,
                "make",
                "make",
                &["install-gcc", "-j", jobs.as_str()],
                Some(env.clone()),
            )?;
            run_command_in(
                &objdir,
                "make",
                "make",
                &["all-target-libgcc", "-j", jobs.as_str()],
                Some(env.clone()),
            )?;
            run_command_in(
                &objdir,
                "make",
                "make",
                &["install-target-libgcc", "-j", jobs.as_str()],
                Some(env.clone()),
            )?;
        }
        GccStage::Final(maybe_sysroot) => {
            log::info!("=> final stage gcc");

            let objdir = gcc_dir.join(format!("objdir-final-{}", toolchain.id()));
            std::fs::create_dir_all(&objdir).context("failed to create an objdir for the arch")?;

            let env: Vec<(OsString, OsString)> = vec![("PATH".into(), toolchain.env_path()?)];

            let mut args: Vec<String> = vec![
                format!("--target={}", toolchain.target),
                format!("--prefix={}", toolchain.dir()?.display()),
                "--disable-nls".into(),
                "--enable-languages=c,c++".into(),
                "--disable-multilib".into(),
            ];
            if let Some(sysroot) = maybe_sysroot {
                args.push(format!("--with-sysroot={}", sysroot.display()));
            }

            run_command_in(
                &objdir,
                "configure",
                objdir.parent().unwrap().join("configure"),
                &args,
                Some(env.clone()),
            )?;

            // hosted/newlib: build everything (gcc, libgcc, libstdc++)
            run_command_in(
                &objdir,
                "make",
                "make",
                &["-j", jobs.as_str()],
                Some(env.clone()),
            )?;
            run_command_in(
                &objdir,
                "make",
                "make",
                &["install", "-j", jobs.as_str()],
                Some(env.clone()),
            )?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GCCVersion(pub u64, pub u64, pub u64);

impl FromStr for GCCVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(".").collect();

        fn parse_part(s: &str) -> anyhow::Result<u64> {
            s.parse().context(format!("`{}` is not a number", s))
        }

        match parts.as_slice() {
            [major, minor, patch] => Ok(GCCVersion(
                parse_part(major)?,
                parse_part(minor)?,
                parse_part(patch)?,
            )),
            _ => Err(anyhow!("`{}` is an invalid gcc version", s)),
        }
    }
}

impl Display for GCCVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct GCC {
    pub version: GCCVersion,
}

impl Default for GCC {
    fn default() -> Self {
        Self {
            version: GCCVersion(15, 2, 0),
        }
    }
}

impl GCC {
    pub fn new(version: GCCVersion) -> Self {
        Self { version }
    }
}
