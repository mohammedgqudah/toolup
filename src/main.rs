use std::{ffi::OsString, io::Write, path::PathBuf, process::Command, str::FromStr};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use toolup::{
    config::resolve_target_toolchain,
    download::cache_dir,
    install_toolchain, install_toolchain_str,
    packages::{busybox, linux},
    profile::{Arch, Os, Target, Toolchain, Vendor},
    qemu::start_vm,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(long, short, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a toolchain for target
    Install {
        /// e.g. aarch64-unknown-linux-gnu
        target: String,
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
    /// Invoke the GCC compiler for the selected toolchain
    CC {
        /// e.g. aarch64-unknown-linux-gnu
        target: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        options: Vec<OsString>,
    },
    /// Manage Linux kernel builds
    Linux {
        /// The kernel version to build. e.g. 6.17
        version: String,
        #[arg(long, short, default_value = "x86_64")]
        architecture: String,
        #[arg(short, long, default_value_t = 10)]
        /// The number of threads to use for running commands
        jobs: u64,
        #[arg(short, long, default_value_t = false)]
        /// Open the kernel's menuconfig before building
        menuconfig: bool,
        #[arg(short, long, default_value_t = false)]
        /// Whether to run defconfig or not. This will erase old config.
        defconfig: bool,
        #[arg(short, long, value_name = "PROGRAM_PATH")]
        /// Copy a program from host and run it in a virtual machine using the built kernel.
        ///
        /// The program's output will be streamed live to the host, and the command will exit with the same exit
        /// code as the program running inside the virtual machine.
        ///
        /// Useful for testing a program across different kernel versions and configurations.
        exec: Option<PathBuf>,
    },
    /// Manage cache
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    env_logger::builder()
        .filter_level(match cli.verbose {
            0 => log::LevelFilter::Info,
            1 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        })
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

    match cli.command {
        Commands::Install {
            target: toolchain,
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
            install_toolchain_str(toolchain, gcc, libc, binutils, None, jobs, false)?;
        }
        Commands::CC { target, options } => {
            let toolchain: Toolchain = resolve_target_toolchain(&target)?.into();
            install_toolchain(toolchain.clone(), 10, false)?;
            Command::new(toolchain.gcc_bin()?).args(options).status()?;
        }
        Commands::Linux {
            version,
            architecture,
            jobs,
            menuconfig,
            defconfig,
            exec: _,
        } => {
            let arch = Arch::from_str(architecture.as_str())?;
            let target = Target {
                arch,
                vendor: Vendor::Unknown,
                abi: toolup::profile::Abi::Gnu,
                os: Os::Linux,
            };
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
