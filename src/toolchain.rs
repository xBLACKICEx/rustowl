use std::env;
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tokio::fs::{create_dir_all, remove_dir_all};

#[cfg(not(windows))]
use flate2::read::GzDecoder;
#[cfg(not(windows))]
use tar::Archive;
#[cfg(windows)]
use zip::ZipArchive;

pub const TOOLCHAIN: &str = env!("RUSTOWL_TOOLCHAIN");
const BUILD_RUNTIME_DIRS: Option<&str> = option_env!("RUSTOWL_RUNTIME_DIRS");

static FALLBACK_RUNTIME_DIRS: LazyLock<Vec<PathBuf>> = LazyLock::new(|| {
    let exec_dir = env::current_exe().unwrap().parent().unwrap().to_path_buf();
    vec![exec_dir.join("rustowl-runtime"), exec_dir]
});

static CONFIG_RUNTIME_DIRS: LazyLock<Vec<PathBuf>> = LazyLock::new(|| {
    BUILD_RUNTIME_DIRS
        .map(|v| env::split_paths(v).collect())
        .unwrap_or_default()
});

const ARCHIVE_NAME: &str = env!("RUSTOWL_ARCHIVE_NAME");

pub const RUSTC_DRIVER_NAME: &str = env!("RUSTC_DRIVER_NAME");
fn recursive_read_dir(path: impl AsRef<Path>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if path.as_ref().is_dir() {
        for entry in read_dir(&path).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                paths.extend_from_slice(&recursive_read_dir(&path));
            } else {
                paths.push(path);
            }
        }
    }
    paths
}
pub fn rustc_driver_path(sysroot: impl AsRef<Path>) -> Option<PathBuf> {
    for file in recursive_read_dir(sysroot) {
        if file.file_name().unwrap().to_string_lossy() == RUSTC_DRIVER_NAME {
            log::info!("rustc_driver found: {}", file.display());
            return Some(file);
        }
    }
    None
}

fn sysroot_from_runtime(runtime: impl AsRef<Path>) -> PathBuf {
    runtime.as_ref().join("sysroot").join(TOOLCHAIN)
}
fn is_valid_runtime_dir(runtime: impl AsRef<Path>) -> bool {
    rustc_driver_path(sysroot_from_runtime(runtime)).is_some()
}
pub fn get_configured_runtime_dir() -> Option<PathBuf> {
    let env_var = env::var("RUSTOWL_RUNTIME_DIRS").unwrap_or_default();

    for runtime in env::split_paths(&env_var) {
        if is_valid_runtime_dir(&runtime) {
            log::info!("select runtime dir from env var: {}", runtime.display());
            return Some(runtime);
        }
    }

    for runtime in &*CONFIG_RUNTIME_DIRS {
        if is_valid_runtime_dir(runtime) {
            log::info!(
                "select runtime dir from build time env var: {}",
                runtime.display()
            );
            return Some(runtime.clone());
        }
    }
    None
}
pub async fn get_runtime_dir() -> PathBuf {
    if let Some(runtime) = get_configured_runtime_dir() {
        return runtime;
    }
    for fallback in &*FALLBACK_RUNTIME_DIRS {
        if is_valid_runtime_dir(fallback) {
            log::info!("select runtime from fallback: {}", fallback.display());
            return fallback.clone();
        }
    }

    log::info!("rustc_driver not found; start setup toolchain");
    match setup_toolchain().await {
        Ok(v) => v,
        Err(e) => {
            log::error!("{e:?}");
            std::process::exit(1);
        }
    }
}
pub async fn get_sysroot() -> PathBuf {
    sysroot_from_runtime(get_runtime_dir().await)
}

pub async fn setup_toolchain() -> Result<PathBuf, ()> {
    log::info!("start downloading {ARCHIVE_NAME}...");
    let tarball_url = format!(
        "https://github.com/cordx56/rustowl/releases/download/v{}/{ARCHIVE_NAME}",
        clap::crate_version!(),
    );

    let resp = match reqwest::get(&tarball_url)
        .await
        .and_then(|v| v.error_for_status())
    {
        Ok(v) => v,
        Err(e) => {
            log::error!("failed to download runtime archive");
            log::error!("{e:?}");
            return Err(());
        }
    };

    let bytes = match resp.bytes().await {
        Ok(v) => v,
        Err(e) => {
            log::error!("failed to download runtime archive");
            log::error!("{e:?}");
            return Err(());
        }
    };
    log::info!("download finished");

    let fallback = FALLBACK_RUNTIME_DIRS[0].clone();
    if create_dir_all(&fallback).await.is_err() {
        log::error!("failed to create toolchain directory");
        return Err(());
    }

    #[cfg(windows)]
    {
        let cursor = std::io::Cursor::new(&*bytes);
        let mut archive = match ZipArchive::new(cursor) {
            Ok(archive) => archive,
            Err(e) => {
                log::error!("failed to read ZIP archive");
                log::error!("{e:?}");
                return Err(());
            }
        };

        for i in 0..archive.len() {
            let mut file = match archive.by_index(i) {
                Ok(file) => file,
                Err(e) => {
                    log::error!("failed to read ZIP entry");
                    log::error!("{e:?}");
                    continue;
                }
            };

            let outpath = match file.enclosed_name() {
                Some(path) => fallback.join(path),
                None => continue,
            };

            if file.is_dir() {
                log::info!("File {} extracted to \"{}\"", i, outpath.display());
                std::fs::create_dir_all(&outpath).unwrap();
            } else {
                log::info!(
                    "File {} extracted to \"{}\" ({} bytes)",
                    i,
                    outpath.display(),
                    file.size()
                );
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(p).unwrap();
                    }
                }
                let mut outfile = std::fs::File::create(&outpath).unwrap();
                std::io::copy(&mut file, &mut outfile).unwrap();
            }

            log::info!("{} unpacked", outpath.display());
        }
    }

    #[cfg(not(windows))]
    {
        let decoder = GzDecoder::new(&*bytes);
        let mut archive = Archive::new(decoder);
        if let Ok(entries) = archive.entries() {
            for mut entry in entries.flatten() {
                if let Ok(path) = entry.path() {
                    let path = path.to_path_buf();
                    if path.as_os_str() != "rustowl" {
                        if !entry.unpack_in(&fallback).unwrap_or(false) {
                            log::error!("failed to unpack runtime tarball");
                            return Err(());
                        }
                        log::info!("{} unpacked", path.display());
                    }
                }
            }
        } else {
            log::error!("failed to unpack runtime tarball");
            return Err(());
        }
    }

    log::info!("runtime setup done in {}", fallback.display());
    Ok(fallback)
}

pub async fn uninstall_toolchain() {
    for fallback in &*FALLBACK_RUNTIME_DIRS {
        let sysroot = sysroot_from_runtime(fallback);
        if sysroot.is_dir() {
            log::info!("remove sysroot: {}", sysroot.display());
            remove_dir_all(&sysroot).await.unwrap();
        }
    }
}
