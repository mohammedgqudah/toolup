use clap::Parser;

mod binutils;
mod download;
mod gcc;
mod glibc;
mod linux;
mod make;
mod profile;
mod sysroot;

use anyhow::Result;
use binutils::install_binutils;
use gcc::install_gcc;

use crate::{
    gcc::{GccStage, Sysroot},
    profile::Profile,
    sysroot::setup_sysroot,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    architecture: String,
    #[arg(long)]
    libc: Option<String>,
    #[arg(long, help = "gcc version", default_value = "15.2.0")]
    gcc: String,
    #[arg(short, long, default_value_t = 10)]
    jobs: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let profile = profile::select_profile(&args.architecture, args.libc.as_ref());

    match profile {
        Profile::Freestanding => {
            install_binutils(args.architecture.clone(), args.jobs)?;
            install_gcc(
                &args.architecture,
                &args.gcc,
                profile,
                args.jobs,
                GccStage::Stage1,
            )?;
        }
        Profile::LinuxGlibc => {
            install_binutils(args.architecture.clone(), args.jobs)?;
            let sysroot = setup_sysroot(&args.architecture, &args.gcc, profile, args.jobs)?;
            install_gcc(
                &args.architecture,
                &args.gcc,
                profile,
                args.jobs,
                GccStage::Final(Some(Sysroot(sysroot))),
            )?;
        }
        _ => unimplemented!(),
    }

    Ok(())
}
