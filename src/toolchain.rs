use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::LazyLock;
use tokio::{
    fs::{create_dir_all, remove_dir_all},
    io::AsyncReadExt,
    process::Command,
    sync::OnceCell,
};

pub const TOOLCHAIN_VERSION: &str = env!("RUSTOWL_TOOLCHAIN");
const CONFING_SYSROOTS: Option<&str> = option_env!("RUSTOWL_SYSROOTS");
static FALLBACK_SYSROOT: LazyLock<PathBuf> = LazyLock::new(|| {
    env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("rustowl-runtime")
});
static CONFIG_SYSROOTS: LazyLock<Vec<PathBuf>> = LazyLock::new(|| {
    CONFING_SYSROOTS
        .map(|v| env::split_paths(v).collect())
        .unwrap_or_default()
});
static RUSTUP_SYSROOT: OnceCell<Option<PathBuf>> = OnceCell::const_new();

const TARBALL_NAME: &str = env!("RUSTOWL_TARBALL_NAME");

pub async fn get_sysroot() -> PathBuf {
    let env_var = env::var("RUSTOWL_RUNTIME_DIRS")
        .unwrap_or(env::var("RUSTOWL_SYSROOTS").unwrap_or_default());
    for sysroot in env::split_paths(&env_var) {
        if sysroot.is_dir() {
            log::info!("select sysroot from runtime env var: {}", sysroot.display());
            return sysroot;
        }
    }
    for sysroot in &*CONFIG_SYSROOTS {
        if sysroot.is_dir() {
            log::info!(
                "select sysroot from build time env var: {}",
                sysroot.display()
            );
            return sysroot.clone();
        }
    }

    if let Some(sysroot) = RUSTUP_SYSROOT
        .get_or_init(|| async {
            if let Ok(mut child) = Command::new("rustup")
                .args(["run", TOOLCHAIN_VERSION, "rustc", "--print=sysroot"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                let mut stdout = child.stdout.take().unwrap();
                if let Ok(status) = child.wait().await {
                    if status.success() {
                        let mut output = String::new();
                        if stdout.read_to_string(&mut output).await.is_ok() {
                            return Some(PathBuf::from(output.trim()));
                        }
                    }
                }
            }
            None
        })
        .await
    {
        log::info!("select sysroot from rustup: {}", sysroot.display());
        return sysroot.to_owned();
    }

    match setup_toolchain().await {
        Ok(v) => v,
        Err(_) => {
            std::process::exit(1);
        }
    }
}

pub async fn setup_toolchain() -> Result<PathBuf, ()> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    if !FALLBACK_SYSROOT.is_dir() {
        log::info!("sysroot not found; start downloading {TARBALL_NAME}...");
        let tarball_url = format!(
            "https://github.com/cordx56/rustowl/releases/download/v{}/{TARBALL_NAME}",
            clap::crate_version!(),
        );
        let resp = match reqwest::get(&tarball_url)
            .await
            .and_then(|v| v.error_for_status())
        {
            Ok(v) => v,
            Err(e) => {
                log::error!("failed to download runtime tarball");
                log::error!("{e:?}");
                return Err(());
            }
        };
        let bytes = match resp.bytes().await {
            Ok(v) => v,
            Err(e) => {
                log::error!("failed to download runtime tarball");
                log::error!("{e:?}");
                return Err(());
            }
        };
        log::info!("download finished");
        if create_dir_all(&*FALLBACK_SYSROOT).await.is_err() {
            log::error!("failed to create toolchain directory");
            return Err(());
        }
        let decoder = GzDecoder::new(&*bytes);
        let mut archive = Archive::new(decoder);
        if let Ok(entries) = archive.entries() {
            for mut entry in entries.flatten() {
                if let Ok(path) = entry.path() {
                    let path = path.to_path_buf();
                    if path.as_os_str() != "rustowl" {
                        if !entry.unpack_in(&*FALLBACK_SYSROOT).unwrap_or(false) {
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
        log::info!("toolchain setup done in {}", FALLBACK_SYSROOT.display());
    }
    log::info!("select fallback sysroot: {}", FALLBACK_SYSROOT.display());
    Ok(FALLBACK_SYSROOT.clone())
}
pub async fn uninstall_toolchain() {
    remove_dir_all(&*FALLBACK_SYSROOT).await.unwrap();
}
