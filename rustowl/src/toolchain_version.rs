use std::env;
use std::path::PathBuf;

pub fn setup_dylib_var() {
    let toolchain_path = PathBuf::from(env!("RUSTOWL_TOOLCHAIN_PATH"));

    #[cfg(windows)]
    {
        let mut paths: std::collections::VecDeque<_> =
            env::split_paths(&env::var_os("Path").unwrap()).collect();
        paths.push_front(toolchain_path.join("bin"));
        unsafe {
            env::set_var("Path", env::join_paths(paths).unwrap());
        }
    }
    #[cfg(unix)]
    {
        let mut paths: std::collections::VecDeque<_> =
            env::split_paths(&env::var("LD_LIBRARY_PATH").unwrap_or("".to_owned())).collect();
        paths.push_front(toolchain_path.join("lib"));
        unsafe {
            env::set_var("LD_LIBRARY_PATH", env::join_paths(paths).unwrap());
        }
    }
}
