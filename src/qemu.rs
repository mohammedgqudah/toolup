use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Result;

use crate::profile::{Arch, Target};

pub fn start_vm(target: &Target, kernel: impl AsRef<Path>, initrd: impl AsRef<Path>) -> Result<()> {
    let kernel = kernel.as_ref();
    let initrd = initrd.as_ref();

    let (qemu, extra, console) = match target.arch {
        Arch::X86_64 => ("qemu-system-x86_64", vec![], "ttyS0"),
        Arch::I686 => ("qemu-system-i386", vec![], "ttyS0"),
        Arch::Riscv64 => (
            "qemu-system-riscv64",
            vec!["-machine", "virt", "-bios", "default"],
            "ttyS0",
        ),
        Arch::Aarch64 => (
            "qemu-system-aarch64",
            vec!["-M", "virt", "-cpu", "cortex-a57"],
            "ttyAMA0",
        ),
        Arch::Ppc64 => ("qemu-system-ppc64", vec!["-machine", "pseries"], "hvc0"),
        Arch::Ppc64Le => ("qemu-system-ppc64le", vec!["-machine", "pseries"], "hvc0"),
        Arch::Armv7 => (
            "qemu-system-arm",
            vec!["-M", "virt", "-cpu", "cortex-a15"],
            "ttyAMA0",
        ),

        _ => unreachable!(),
    };

    let append = format!("console={console},115200 rdinit=/init earlycon");

    let mut cmd = Command::new(qemu);
    cmd.args(&extra)
        .args(["-m", "1G", "-smp", "2", "-nographic"])
        .args([
            "-kernel",
            kernel
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("bad kernel path"))?,
        ])
        .args([
            "-initrd",
            initrd
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("bad initrd path"))?,
        ])
        .args(["-append", &append])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    print!("{} ", qemu);
    for arg in cmd.get_args() {
        print!("{} ", arg.to_str().unwrap());
    }

    //let status = cmd.status()?;
    //if !status.success() {
    //    bail!("QEMU exited with status {status}");
    //}
    Ok(())
}
