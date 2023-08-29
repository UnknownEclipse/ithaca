use std::{
    env::args,
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
};

use color_eyre::{eyre::bail, Result};
use owo_colors::OwoColorize;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let Some(command) = args().nth(1) else {
        return Ok(());
    };

    match command.as_str() {
        "build" => {
            build()?;
        }
        "run" => {
            let iso = build()?;
            run(&iso)?;
        }
        _ => {
            bail!("invalid subcommand '{}'", command)
        }
    }

    Ok(())
}

fn run(iso: &Path) -> Result<()> {
    println!("{:>12} `kernel.iso`", "Running".bold().green());

    _ = File::create("qemu.log");

    Command::new("qemu-system-x86_64")
        .args(["-M", "q35", "-m", "2G", "-serial", "stdio", "-cdrom"])
        .arg(iso)
        .args(["-boot", "d", "-d", "int,guest_errors", "-D", "qemu.log"])
        .spawn()?
        .wait()?;

    Ok(())
}

fn build() -> Result<PathBuf> {
    let limine = fetch_limine()?;

    let kernel_elf = compile_kernel()?;

    println!("{:>12} `kernel.iso`", "Building".bold().green());
    let iso_path = build_iso(&kernel_elf, &limine)?;
    println!("{:>12} `kernel`", "Finished".bold().green());
    Ok(iso_path)
}

fn compile_kernel() -> Result<PathBuf> {
    Command::new("cargo")
        .args(["build"])
        .current_dir("kernel")
        .spawn()?
        .wait()?;

    Ok(Path::new("kernel/target/x86_64-unknown-none/debug/kernel").canonicalize()?)
}

fn build_iso(kernel_elf: &Path, limine: &Path) -> Result<PathBuf> {
    fs::create_dir_all("build/iso_root/EFI/BOOT")?;

    fs::copy(kernel_elf, "build/iso_root/kernel.elf")?;

    for file in [
        "limine-bios-cd.bin",
        "limine-bios.sys",
        "limine-uefi-cd.bin",
    ] {
        let from = limine.join(file);
        let to = Path::new("build/iso_root").join(file);
        fs::copy(from, to)?;
    }

    fs::copy("kernel/limine.cfg", "build/iso_root/limine.cfg")?;
    fs::copy(
        "build/limine/BOOTX64.EFI",
        "build/iso_root/EFI/BOOT/BOOTX64.EFI",
    )?;
    fs::copy(
        "build/limine/BOOTIA32.EFI",
        "build/iso_root/EFI/BOOT/BOOTIA32.EFI",
    )?;

    Command::new("xorriso")
        .args([
            "-as",
            "mkisofs",
            "-b",
            "limine-bios-cd.bin",
            "-no-emul-boot",
            "-boot-load-size",
            "4",
            "-boot-info-table",
            "--efi-boot",
            "limine-uefi-cd.bin",
            "-efi-boot-part",
            "--efi-boot-image",
            "--protective-msdos-label",
            "build/iso_root",
            "-o",
            "build/kernel.iso",
        ])
        .spawn()?
        .wait()?;

    Command::new("build/limine/limine")
        .args(["bios-install", "build/kernel.iso"])
        .spawn()?
        .wait()?;

    Ok(Path::new("build/kernel.iso").canonicalize()?)
}

fn fetch_limine() -> Result<PathBuf> {
    const LIMINE_GIT_URL: &str = "https://github.com/limine-bootloader/limine.git";

    fs::create_dir_all("build")?;

    let path = Path::new("build").join("limine");

    if path.exists() {
        return Ok(path);
    }

    println!("{:>12} `{}`", "Cloning".bold().green(), LIMINE_GIT_URL);

    Command::new("git")
        .args([
            "clone",
            LIMINE_GIT_URL,
            "--branch=v5.x-branch-binary",
            "--depth=1",
        ])
        .current_dir("build")
        .spawn()?
        .wait()?;

    println!("{:>12} `limine`", "Building".bold().green());

    Command::new("make")
        .current_dir("build/limine")
        .args(["limine"])
        .spawn()?
        .wait()?;

    Ok(path)
}
