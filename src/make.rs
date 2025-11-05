use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use indicatif::{ProgressBar, ProgressStyle};

pub fn run_make_in<P: AsRef<Path>>(workdir: P, args: &[&str]) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.dim} {msg:.dim}")?);
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message(format!("make {}", args.join(" ")));

    let mut child = Command::new("make")
        .args(args)
        .current_dir(workdir.as_ref())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawning `make`")?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // stream stdout
    let pb_out = pb.clone();
    let t_out = std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            pb_out.set_message(line.chars().take(80).collect::<String>());
        }
    });

    // stream stderr
    let pb_err = pb.clone();
    let t_err = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().flatten() {
            //eprintln!("{line}");
            pb_err.set_message(line.chars().take(80).collect::<String>());
        }
    });

    let status = child.wait().context("waiting for `make` to finish")?;
    let _ = t_out.join();
    let _ = t_err.join();

    if status.success() {
        pb.finish_with_message("make finished successfully");
        Ok(())
    } else {
        pb.finish();
        bail!("make exited with status {}", status);
    }
}

pub fn run_configure_in<P: AsRef<Path>>(workdir: P, args: &[&str]) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.dim} {msg:.dim}")?);
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message(format!("configure {}", args.join(" ")));

    let mut child = Command::new(workdir.as_ref().join("..").join("configure"))
        .args(args)
        .current_dir(workdir.as_ref())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawning `configure`")?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // stream stdout
    let pb_out = pb.clone();
    let t_out = std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            pb_out.set_message(line.chars().take(80).collect::<String>());
        }
    });

    // stream stderr
    let pb_err = pb.clone();
    let t_err = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().flatten() {
            //eprintln!("{line}");
            pb_err.set_message(line.chars().take(80).collect::<String>());
        }
    });

    let status = child.wait().context("waiting for `configure` to finish")?;
    let _ = t_out.join();
    let _ = t_err.join();

    if status.success() {
        pb.finish_with_message("configure finished successfully");
        Ok(())
    } else {
        pb.finish();
        bail!("configure exited with status {}", status);
    }
}
