use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Result, bail};

pub fn start_vm(
    architecture: impl AsRef<str>,
    kernel: impl AsRef<Path>,
    initrd: impl AsRef<Path>,
) -> Result<()> {
    let arch = architecture.as_ref().split("-").next().unwrap();

    let kernel = kernel.as_ref();
    let initrd = initrd.as_ref();

    let (qemu, extra, console) = match arch {
        "x86_64" => ("qemu-system-x86_64", vec![], "ttyS0"),
        "i386" | "i686" => ("qemu-system-i386", vec![], "ttyS0"),
        "riscv64" => (
            "qemu-system-riscv64",
            vec!["-machine", "virt", "-bios", "default"],
            "ttyS0",
        ),
        "aarch64" => (
            "qemu-system-aarch64",
            vec!["-M", "virt", "-cpu", "cortex-a57"],
            "ttyAMA0",
        ),
        "powerpc64" => (
            "qemu-system-ppc64",
            vec!["-machine", "pseries", "-bios", "default"],
            "hvc0",
        ),
        "arm" => (
            "qemu-system-arm",
            vec!["-M", "virt", "-cpu", "cortex-a15"],
            "ttyAMA0",
        ),
        "mips" => (
            "qemu-system-mipsel",
            vec!["-M", "malta", "-nographic"],
            "ttyS0",
        ),
        "mips64" => (
            "qemu-system-mips64",
            vec!["-M", "malta", "-nographic"],
            "ttyS0",
        ),
        _ => bail!("unsupported arch: {arch}"),
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
