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
    profile::Profile,
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
        architecture: String,
        #[arg(long)]
        libc: Option<String>,
        #[arg(long, help = "gcc version", default_value = "15.2.0")]
        gcc: String,
        #[arg(short, long, default_value_t = 10)]
        jobs: u64,
    },
    Linux {
        version: String,
        #[arg(long, short, default_value = "x86_64-linux-gnu")]
        architecture: String,
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
fn install_toolchain(
    architecture: String,
    libc: Option<String>,
    gcc: String,
    jobs: u64,
    force: bool,
) -> Result<()> {
    if download::cross_prefix()?
        .join("bin")
        .join(format!("{}-gcc", architecture))
        .exists()
        && !force
    {
        // toolchain already installed
        return Ok(());
    }

    let profile = profile::select_profile(&architecture, libc.as_ref());

    match profile {
        Profile::Freestanding => {
            install_binutils(architecture.clone(), jobs)?;
            install_gcc(&architecture, &gcc, profile, jobs, GccStage::Stage1)?;
        }
        Profile::LinuxGlibc => {
            install_binutils(architecture.clone(), jobs)?;
            let sysroot = setup_sysroot(&architecture, &gcc, profile, jobs)?;
            install_gcc(
                &architecture,
                &gcc,
                profile,
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
            architecture,
            libc,
            gcc,
            jobs,
        } => {
            install_toolchain(architecture, libc, gcc, jobs, false)?;
        }
        Commands::Linux {
            version,
            architecture,
            jobs,
            menuconfig,
            defconfig,
        } => {
            install_toolchain(
                architecture.clone(),
                Some("glibc".into()),
                "15.2.0".into(),
                jobs,
                false,
            )?;
            let kernel_image =
                linux::get_image(&version, &architecture, jobs, menuconfig, defconfig)?;
            let rootfs = busybox::build_rootfs(&architecture)?;
            start_vm(architecture, kernel_image, rootfs)?;
        }
    };

    Ok(())
}
