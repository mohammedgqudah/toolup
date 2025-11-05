use clap::Parser;

mod binutils;
mod download;
mod gcc;
mod make;

use anyhow::Result;
use binutils::install_binutils;
use gcc::install_gcc;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg()]
    architecture: String,
    #[arg(short, long, default_value_t = 10)]
    jobs: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("=> building binutils");
    install_binutils(args.architecture.clone(), args.jobs)?;
    println!("=> building GCC");
    install_gcc(args.architecture, args.jobs)?;

    Ok(())
}
