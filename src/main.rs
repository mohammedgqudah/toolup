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
    download::cache_dir,
    gcc::{GccStage, Sysroot},
    linux::install_headers,
    sysroot::setup_sysroot,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    architecture: String,
    #[arg(long)]
    libc: Option<String>,
    #[arg(short, long, default_value_t = 10)]
    jobs: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let profile = profile::select_profile(&args.architecture, args.libc.as_ref());

    install_binutils(args.architecture.clone(), args.jobs)?;
    let sysroot = setup_sysroot(&args.architecture, profile, args.jobs)?;
    install_gcc(
        &args.architecture,
        profile,
        args.jobs,
        GccStage::Final(Some(Sysroot(sysroot))),
    )?;

    //println!("=> building binutils");
    //install_binutils(args.architecture.clone(), args.jobs)?;
    //println!("=> building GCC");
    //install_gcc(args.architecture, args.jobs)?;

    Ok(())
}
