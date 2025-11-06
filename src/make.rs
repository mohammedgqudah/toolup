use std::{
    ffi::OsStr,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use indicatif::{ProgressBar, ProgressStyle};

use crate::download::cache_dir;

pub fn run_make_in<P: AsRef<Path>>(workdir: P, args: &[&str]) -> Result<()> {
    _run_make_in(workdir, args, None)
}

pub fn run_make_with_env_in<P: AsRef<Path>>(
    workdir: P,
    args: &[&str],
    env: Vec<(String, String)>,
) -> Result<()> {
    _run_make_in(workdir, args, Some(env))
}

pub fn _run_make_in<P: AsRef<Path>>(
    workdir: P,
    args: &[&str],
    env: Option<Vec<(String, String)>>,
) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.dim} {msg:.dim}")?);
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message(format!("make {}", args.join(" ")));

    let mut _cmd = Command::new("make");

    _cmd.args(args)
        .current_dir(workdir.as_ref())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(_env) = env {
        _cmd.envs(_env);
    }

    let mut child = _cmd.spawn().context("spawning `make`")?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let log_path = cache_dir()?.join("run.txt");
    let log = Arc::new(Mutex::new(File::create(&log_path)?));

    // stream stdout
    //let pb_out = pb.clone();
    //let t_out = std::thread::spawn(move || {
    //    let reader = BufReader::new(stdout);
    //    for line in reader.lines().flatten() {
    //        pb_out.set_message(line.chars().take(80).collect::<String>());
    //    }
    //});

    //// stream stderr
    //let pb_err = pb.clone();
    //let t_err = std::thread::spawn(move || {
    //    let reader = BufReader::new(stderr);
    //    for line in reader.lines().flatten() {
    //        //eprintln!("{line}");
    //        //pb_err.set_message(line.chars().take(80).collect::<String>());
    //    }
    //});

    let t_out = {
        // stream stdout
        let pb_out = pb.clone();
        let log_out = log.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                pb_out.set_message(line.chars().take(80).collect::<String>());
                if let Ok(mut f) = log_out.lock() {
                    let _ = f.write_all(line.as_bytes());
                    let _ = f.write_all("\n".as_bytes());
                }
            }
        })
    };

    let t_err = {
        // stream stderr
        let pb_err = pb.clone();
        let log_out = log.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                pb_err.set_message(line.chars().take(80).collect::<String>());
                if let Ok(mut f) = log_out.lock() {
                    let _ = f.write_all(line.as_bytes());
                    let _ = f.write_all("\n".as_bytes());
                }
            }
        })
    };

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

pub fn run_configure_in<P: AsRef<Path>, S: AsRef<OsStr>>(workdir: P, args: &[S]) -> Result<()> {
    _run_configure_in(workdir, args, None)
}

pub fn run_configure_with_env_in<P: AsRef<Path>, S: AsRef<OsStr>>(
    workdir: P,
    args: &[S],
    env: Vec<(String, String)>,
) -> Result<()> {
    _run_configure_in(workdir, args, Some(env))
}

pub fn _run_configure_in<P: AsRef<Path>, S: AsRef<OsStr>>(
    workdir: P,
    args: &[S],
    env: Option<Vec<(String, String)>>,
) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.dim} {msg:.dim}")?);
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message(format!("configure"));

    let mut _cmd = Command::new(workdir.as_ref().join("..").join("configure"));
    _cmd.args(args)
        .current_dir(workdir.as_ref())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(_env) = env {
        _cmd.envs(_env);
    }
    let mut child = _cmd.spawn().context("spawning `configure`")?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let log_path = cache_dir()?.join("run.txt");
    let log = Arc::new(Mutex::new(File::create(&log_path)?));

    let t_out = {
        // stream stdout
        let pb_out = pb.clone();
        let log_out = log.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                pb_out.set_message(line.chars().take(80).collect::<String>());
                if let Ok(mut f) = log_out.lock() {
                    let _ = f.write_all(line.as_bytes());
                    let _ = f.write_all("\n".as_bytes());
                }
            }
        })
    };

    let t_err = {
        // stream stderr
        let pb_err = pb.clone();
        let log_out = log.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                pb_err.set_message(line.chars().take(80).collect::<String>());
                if let Ok(mut f) = log_out.lock() {
                    let _ = f.write_all(line.as_bytes());
                    let _ = f.write_all("\n".as_bytes());
                }
            }
        })
    };

    let status = child.wait().context("waiting for `configure` to finish")?;
    let _ = t_out.join();
    let _ = t_err.join();

    if status.success() {
        pb.finish_with_message("configure finished successfully");
        Ok(())
    } else {
        pb.finish();
        bail!(
            "configure exited with status {}\n out: {}",
            status,
            log_path.display()
        );
    }
}
