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
mod profile;
mod qemu;
mod sysroot;

use anyhow::Result;
use binutils::install_binutils;
use gcc::install_gcc;

use crate::{
    gcc::{GccStage, Sysroot},
    profile::{Abi, Target},
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
}

/// Install a toolchain.
///
/// use `force` to forcefully re-install a toolchain if it was already installed.
fn install_toolchain(toolchain_str: String, gcc: String, jobs: u64, force: bool) -> Result<()> {
    let toolchain = Target::from_str(&toolchain_str)?;

    if download::cross_prefix()?
        .join("bin")
        .join(format!("{}-gcc", toolchain_str))
        .exists()
        && !force
    {
        // toolchain already installed
        return Ok(());
    }

    match toolchain {
        // freestanding
        Target {
            abi: Abi::Elf | Abi::Eabihf | Abi::Eabi,
            ..
        } => {
            install_binutils(&toolchain, jobs)?;
            install_gcc(&toolchain, &gcc, jobs, GccStage::Stage1)?;
        }
        // glibc path
        Target {
            abi: Abi::Gnu | Abi::GnuEabi | Abi::GnuEabihf,
            ..
        } => {
            install_binutils(&toolchain, jobs)?;
            let sysroot = setup_sysroot(&toolchain, &gcc, jobs)?;
            install_gcc(
                &toolchain,
                &gcc,
                jobs,
                GccStage::Final(Some(Sysroot(sysroot))),
            )?;
        }
        _ => unimplemented!(),
    };

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Toolchain {
            toolchain,
            gcc,
            jobs,
        } => {
            install_toolchain(toolchain, gcc, jobs, false)?;
        }
        Commands::Linux {
            version,
            toolchain,
            jobs,
            menuconfig,
            defconfig,
        } => {
            let target = Target::from_str(toolchain.as_str())?;
            install_toolchain(toolchain, "15.2.0".into(), jobs, false)?;
            let kernel_image = linux::get_image(&target, &version, jobs, menuconfig, defconfig)?;
            let rootfs = busybox::build_rootfs(&target)?;
            start_vm(&target, kernel_image, rootfs)?;
        }
    };

    Ok(())
}
