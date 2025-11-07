use std::path::Path;
use std::process::{Command, Stdio};

pub fn pack_rootfs(rootfs: &Path, out: &Path) -> std::io::Result<()> {
    let mut cpio = Command::new("cpio")
        .args(["-o", "-H", "newc"])
        .current_dir(rootfs)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    // feed file list from `find .`
    let mut find = Command::new("find")
        .arg(".")
        .current_dir(rootfs)
        .stdout(cpio.stdin.take().unwrap())
        .spawn()?;

    // gzip the cpio output
    let mut gz = Command::new("gzip")
        .arg("-9")
        .stdin(cpio.stdout.take().unwrap())
        .stdout(Stdio::from(std::fs::File::create(out)?))
        .spawn()?;

    find.wait()?;
    cpio.wait()?;
    gz.wait()?;
    Ok(())
}
