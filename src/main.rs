use std::io::Write;
use std::str::FromStr;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod commands;
mod cpio;
mod download;
mod packages;
mod profile;
mod qemu;
mod sysroot;

use crate::{
    download::cache_dir,
    packages::binutils::{Binutils, BinutilsVersion, install_binutils},
    packages::gcc::{GCC, GCCVersion, GccStage, Sysroot, install_gcc},
    packages::glibc::GlibcVersion,
    packages::linux::KernelVersion,
    packages::musl::MuslVersion,
    profile::{Abi, Libc, Target, Toolchain},
    qemu::start_vm,
    sysroot::setup_sysroot,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Toolchain {
        /// e.g. aarch64-unknown-linux-gnu
        toolchain: String,
        #[arg(long, default_value = "15.2.0")]
        /// GCC version
        gcc: String,
        #[arg(long)]
        /// glibc or musl version; depending on the target
        libc: Option<String>,
        #[arg(long, default_value = "2.45")]
        /// binutils version
        binutils: String,
        #[arg(short, long, default_value_t = 10)]
        /// The number of threads to use for running commands
        jobs: u64,
    },
    Linux {
        /// The kernel version to build. e.g. 6.17
        version: String,
        #[arg(long, short, default_value = "x86_64-unknown-linux-gnu")]
        toolchain: String,
        #[arg(short, long, default_value_t = 10)]
        /// The number of threads to use for running commands
        jobs: u64,
        #[arg(short, long, default_value_t = false)]
        /// Open the kernel's menuconfig before building
        menuconfig: bool,
        #[arg(short, long, default_value_t = false)]
        /// Whether to run defconfig or not. This will erase old config.
        defconfig: bool,
    },
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}

#[derive(Subcommand)]
enum CacheAction {
    /// Remove cache for a specific toolchain
    Clean {
        toolchain: String,
    },
    Dir {},
    Prune {},
}

/// Install a toolchain.
///
/// use `force` to forcefully re-install a toolchain if it was already installed.
fn install_toolchain(
    target_str: String,
    gcc_str: String,
    libc_str: String,
    binutils_str: String,
    kernel_version: Option<&KernelVersion>,
    jobs: u64,
    force: bool,
) -> Result<Toolchain> {
    let target = Target::from_str(&target_str)?;
    let binutils = Binutils::new(BinutilsVersion::from_str(&binutils_str)?);
    let gcc = GCC::new(GCCVersion::from_str(&gcc_str)?);
    let libc = match target.abi {
        Abi::Musl => Libc::Musl(MuslVersion::from_str(&libc_str)?),
        _ => Libc::Glibc(GlibcVersion::from_str(&libc_str)?),
    };

    let toolchain = if let Some(kernel_version) = kernel_version {
        Toolchain::new_with_kernel(target, binutils, gcc, libc, kernel_version.clone())
    } else {
        Toolchain::new(target, binutils, gcc, libc)
    };

    println!("{}", toolchain);
    log::info!("export PATH=\"{}:$PATH\"", toolchain.bin_dir()?.display());
    log::info!("export SYSROOT={}", toolchain.sysroot()?.display());
    log::info!(
        "export PKG_CONFIG_SYSROOT_DIR={}",
        toolchain.sysroot()?.display()
    );
    log::info!("export TARGET={}", toolchain.target);
    log::info!("");

    if toolchain.gcc_bin()?.exists() && !force {
        log::info!("toolchain is already installed");
        return Ok(toolchain);
    }

    match target {
        // freestanding
        Target {
            abi: Abi::Elf | Abi::Eabihf | Abi::Eabi,
            ..
        } => {
            install_binutils(&toolchain, jobs)?;
            install_gcc(&toolchain, jobs, GccStage::Stage1)?;
        }
        Target {
            abi: Abi::Gnu | Abi::GnuEabi | Abi::GnuEabihf | Abi::Musl,
            ..
        } => {
            install_binutils(&toolchain, jobs)?;
            let sysroot = setup_sysroot(&toolchain, jobs)?;
            install_gcc(&toolchain, jobs, GccStage::Final(Some(Sysroot(sysroot))))?;
        }
        _ => unimplemented!(),
    };

    Ok(toolchain)
}

fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format(|buf, record| {
            let warn_style = buf.default_level_style(log::Level::Warn);
            match record.level() {
                log::Level::Info => {
                    writeln!(buf, "{}", record.args())
                }
                _ => {
                    writeln!(buf, "{warn_style}{}{warn_style:#}", record.args())
                }
            }
        })
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Toolchain {
            toolchain,
            gcc,
            libc,
            binutils,
            jobs,
        } => {
            let libc = libc.unwrap_or(if toolchain.contains("musl") {
                "1.2.5".into()
            } else {
                "2.42".into()
            });
            install_toolchain(toolchain, gcc, libc, binutils, None, jobs, false)?;
        }
        Commands::Linux {
            version,
            toolchain,
            jobs,
            menuconfig,
            defconfig,
        } => {
            let target = Target::from_str(toolchain.as_str())?;
            let (kernel_image, toolchain) =
                packages::linux::get_image(&target, &version, jobs, menuconfig, defconfig)?;
            let rootfs = packages::busybox::build_rootfs(&toolchain)?;
            start_vm(&target, kernel_image, rootfs)?;
        }
        Commands::Cache { action } => match action {
            CacheAction::Clean { toolchain: _ } => {
                // TODO: should each build step expose a clean_cache(target) function? what about
                // different versions? ask to clean the cache for a specific version?
                unimplemented!()
            }
            CacheAction::Dir {} => {
                log::info!("{}", cache_dir()?.display());
            }
            CacheAction::Prune {} => {
                std::fs::remove_dir_all(cache_dir()?).context("failed to prune cache")?;
            }
        },
    };

    Ok(())
}
