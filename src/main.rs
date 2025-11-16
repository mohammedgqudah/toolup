use std::io::Write;
use std::str::FromStr;

use clap::{Parser, Subcommand};

mod binutils;
mod busybox;
mod cpio;
mod download;
mod gcc;
mod glibc;
mod linux;
mod make;
mod musl;
mod profile;
mod qemu;
mod sysroot;

use anyhow::{Context, Result};
use binutils::install_binutils;
use gcc::install_gcc;

use crate::{
    binutils::{Binutils, BinutilsVersion},
    download::cache_dir,
    gcc::{GCC, GCCVersion, GccStage, Sysroot},
    glibc::GlibcVersion,
    musl::MuslVersion,
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
        toolchain: String,
        #[arg(long, help = "gcc version", default_value = "15.2.0")]
        gcc: String,
        #[arg(long, help = "libc version", default_value = "2.42")]
        libc: String,
        #[arg(long, help = "binutils version", default_value = "2.45")]
        binutils: String,
        #[arg(short, long, default_value_t = 10)]
        jobs: u64,
    },
    Linux {
        version: String,
        #[arg(long, short, default_value = "x86_64-unknown-linux-gnu")]
        toolchain: String,
        #[arg(short, long, default_value_t = 10)]
        jobs: u64,
        #[arg(short, long, default_value_t = false)]
        menuconfig: bool,
        #[arg(short, long, default_value_t = false)]
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
    let toolchain = Toolchain::new(target, binutils, gcc, libc);

    println!("{}", toolchain);

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
            install_toolchain(toolchain, gcc, libc, binutils, jobs, false)?;
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
                linux::get_image(&target, &version, jobs, menuconfig, defconfig)?;
            let rootfs = busybox::build_rootfs(&toolchain)?;
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
