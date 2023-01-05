#![warn(rust_2018_idioms)]

use arch::Arch;
use config::Config;
use platform::{Platform, PlatformDetectError};

fn main() -> anyhow::Result<()> {
    let app_dir = app_dir::get()?;

    let arch = Arch::detect()?;

    let platform = match Platform::detect() {
        Ok(platform) => platform,
        Err(PlatformDetectError::NoPlatform) => {
            println!("cargo:warning=No platform has been selected. Not building Unikraft.");
            return Ok(());
        }
        Err(err) => {
            return Err(err.into());
        }
    };

    let config = Config::new(app_dir, arch, platform);
    config.build_unikraft()?;
    config.create_static_library()?;
    config.create_linker_scripts()?;
    Ok(())
}

mod app_dir {
    use std::path::PathBuf;
    use std::{env, error, fmt};

    #[derive(Debug)]
    pub struct AppDirError;

    impl fmt::Display for AppDirError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("APP_DIR was not set and could not be inferred")
        }
    }

    impl error::Error for AppDirError {}

    // This is a hack.
    // There is no stable way of getting the current binary's Cargo manifest directory.
    pub fn get() -> anyhow::Result<PathBuf> {
        println!("cargo:rerun-if-env-changed=APP_DIR");
        if let Some(app_dir) = env::var_os("APP_DIR").map(PathBuf::from) {
            return Ok(app_dir);
        }

        let mut app_dir = env::current_exe()?;
        for _ in 0..5 {
            app_dir.pop();
        }
        if app_dir.join("Cargo.toml").try_exists()? {
            Ok(app_dir)
        } else {
            Err(AppDirError.into())
        }
    }
}

mod arch {
    use std::{env, error, fmt};

    pub enum Arch {
        X86_64,
    }

    #[derive(Debug)]
    pub struct ArchDetectError {
        target_arch: String,
    }

    impl fmt::Display for ArchDetectError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "architecture not supported: {}", self.target_arch)
        }
    }

    impl error::Error for ArchDetectError {}

    impl Arch {
        pub fn detect() -> Result<Self, ArchDetectError> {
            let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
            match target_arch.as_str() {
                "x86_64" => Ok(Self::X86_64),
                _ => Err(ArchDetectError { target_arch }),
            }
        }

        pub fn as_str(&self) -> &str {
            match self {
                Self::X86_64 => "x86_64",
            }
        }
    }
}

mod platform {
    use std::{env, error, fmt};

    pub enum Platform {
        Kvm,
        Linuxu,
    }

    #[derive(Debug)]
    pub enum PlatformDetectError {
        TooManyPlatforms,
        NoPlatform,
    }

    impl fmt::Display for PlatformDetectError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let s = match self {
                PlatformDetectError::TooManyPlatforms => "Too many platforms selected",
                PlatformDetectError::NoPlatform => "No platform selected",
            };
            f.write_str(s)
        }
    }

    impl error::Error for PlatformDetectError {}

    impl Platform {
        pub fn detect() -> Result<Self, PlatformDetectError> {
            let kvm = env::var_os("CARGO_FEATURE_KVM").is_some();
            let linuxu = env::var_os("CARGO_FEATURE_LINUXU").is_some();
            match (kvm, linuxu) {
                (true, true) => Err(PlatformDetectError::TooManyPlatforms),
                (true, false) => Ok(Self::Kvm),
                (false, true) => Ok(Self::Linuxu),
                (false, false) => Err(PlatformDetectError::NoPlatform),
            }
        }

        pub fn as_str(&self) -> &str {
            match self {
                Self::Kvm => "kvm",
                Self::Linuxu => "linuxu",
            }
        }
    }

    impl fmt::Display for Platform {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.as_str())
        }
    }
}

mod config {
    use std::fs::{self, DirEntry, File, ReadDir};
    use std::io::{self, BufReader, BufWriter, ErrorKind};
    use std::path::PathBuf;
    use std::process::Command;
    use std::{env, str};

    use anyhow::Context;

    use crate::arch::Arch;
    use crate::platform::Platform;

    fn not_found_ok(err: io::Error) -> io::Result<()> {
        match err.kind() {
            ErrorKind::NotFound => Ok(()),
            _ => Err(err),
        }
    }

    pub struct Config {
        out_dir: PathBuf,
        app_dir: PathBuf,
        arch: Arch,
        platform: Platform,
    }

    impl Config {
        pub fn new(app_dir: PathBuf, arch: Arch, platform: Platform) -> Self {
            let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
            Self {
                out_dir,
                app_dir,
                arch,
                platform,
            }
        }

        pub fn build_unikraft(&self) -> anyhow::Result<()> {
            let copy_to_out = |file_name| {
                let path = self.app_dir.join(file_name);
                fs::copy(&path, self.out_dir.join(file_name))
                    .map(|_len| println!("cargo:rerun-if-changed={}", path.display()))
            };

            copy_to_out("Kraftfile")?;
            copy_to_out("Makefile.uk").or_else(not_found_ok)?;

            let output = Command::new("kraft")
                .arg("build")
                .arg("--arch")
                .arg(self.arch.as_str())
                .arg("--plat")
                .arg(self.platform.as_str())
                .arg("--fast")
                .arg(&self.out_dir)
                .output()
                .context("Failed to execute kraft")?;
            assert!(
                output.status.success(),
                "kraft build was not successful:\n{}",
                str::from_utf8(&output.stdout).unwrap()
            );

            Ok(())
        }

        fn find_object_files(&self) -> io::Result<Vec<PathBuf>> {
            let build_dir = self.out_dir.join("build");

            let dir_entry_then_to_object_file = |dir_entry: DirEntry| {
                dir_entry.file_type().map(|file_type| {
                    let file_name = dir_entry.file_name().into_string().unwrap();
                    (file_type.is_file()
                        && file_name.starts_with("lib")
                        && file_name.ends_with(".o")
                        && !file_name.ends_with(".ld.o"))
                    .then_some(dir_entry.path())
                })
            };

            let read_dir_then_to_object_files = |read_dir: ReadDir| {
                read_dir
                    .filter_map(|entry| entry.and_then(dir_entry_then_to_object_file).transpose())
                    .collect()
            };

            fs::read_dir(build_dir)
                .and_then(read_dir_then_to_object_files)
                .or_else(|err| {
                    (err.kind() == ErrorKind::NotFound)
                        .then_some(Vec::new())
                        .ok_or(err)
                })
        }

        pub fn create_static_library(&self) -> io::Result<()> {
            let object_files = self.find_object_files()?;

            let archive = self.out_dir.join("libunikraft.a");

            fs::remove_file(&archive).or_else(not_found_ok)?;

            let status = Command::new("ar")
                .arg("rcs")
                .arg(&archive)
                .args(object_files)
                .status()?;
            assert!(status.success());

            println!("cargo:rustc-link-search=native={}", self.out_dir.display());
            println!("cargo:rustc-link-lib=static:-bundle,+whole-archive=unikraft");

            Ok(())
        }

        fn default_linker_script(&self) -> PathBuf {
            let mut path = self.out_dir.clone();
            path.push("build");
            path.push(format!("lib{}plat", self.platform));
            path.push("link64.lds");
            path
        }

        fn linker_scripts(&self) -> io::Result<Vec<PathBuf>> {
            let lib = {
                let mut lib = self.out_dir.clone();
                lib.push(".unikraft");
                lib.push("unikraft");
                lib.push("lib");
                lib
            };

            let lib_dir_then_to_linker_script = |path: PathBuf| {
                path.is_dir()
                    .then(|| {
                        let linker_script = {
                            let mut linker_script = path;
                            linker_script.push("extra.ld");
                            linker_script
                        };
                        linker_script
                            .try_exists()
                            .map(|exists| exists.then_some(linker_script))
                            .transpose()
                    })
                    .flatten()
                    .transpose()
            };

            fs::read_dir(lib)?
                .filter_map(|entry| {
                    entry
                        .map(|entry| entry.path())
                        .and_then(lib_dir_then_to_linker_script)
                        .transpose()
                })
                .collect()
        }

        pub fn create_linker_scripts(&self) -> io::Result<()> {
            fs::copy(
                self.default_linker_script(),
                self.out_dir.join("default_unikraft_linker_script.ld"),
            )?;

            let mut out_linker_script = BufWriter::new(File::create(
                self.out_dir.join("unikraft_linker_script.ld"),
            )?);

            for in_linker_script in self.linker_scripts()? {
                let mut in_linker_script = BufReader::new(File::open(in_linker_script)?);
                io::copy(&mut in_linker_script, &mut out_linker_script)?;
            }

            Ok(())
        }
    }
}
