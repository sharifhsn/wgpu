use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    println!("cargo:rerun-if-env-changed=DEVKITPRO");
    println!("cargo:rerun-if-env-changed=DEKO3D_SYS_DEVKITPRO");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("horizon") {
        return;
    }

    if env::var_os("CARGO_FEATURE_DEBUG_DEKO3D").is_some()
        && env::var_os("CARGO_FEATURE_RELEASE_DEKO3D").is_some()
    {
        panic!("enable only one of debug-deko3d or release-deko3d");
    }

    let devkitpro = find_devkitpro().unwrap_or_else(|| {
        panic!(
            "could not find devkitPro; set DEVKITPRO or DEKO3D_SYS_DEVKITPRO to a prefix containing libnx"
        )
    });

    let libnx = devkitpro.join("libnx");
    let include_dir = libnx.join("include");
    let lib_dir = libnx.join("lib");
    let deko_lib = if env::var_os("CARGO_FEATURE_RELEASE_DEKO3D").is_some() {
        "deko3d"
    } else {
        "deko3dd"
    };

    require_file(&include_dir.join("deko3d.h"));
    require_file(&include_dir.join("switch.h"));
    require_file(&lib_dir.join(format!("lib{deko_lib}.a")));
    require_file(&lib_dir.join("libnx.a"));

    println!("cargo:include={}", include_dir.display());
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static={deko_lib}");
    println!("cargo:rustc-link-lib=static=nx");
}

fn find_devkitpro() -> Option<PathBuf> {
    [
        env::var_os("DEKO3D_SYS_DEVKITPRO").map(PathBuf::from),
        env::var_os("DEVKITPRO").map(PathBuf::from),
        Some(PathBuf::from("/opt/devkitpro")),
        Some(PathBuf::from("/tmp/devkitpro-switch1/opt/devkitpro")),
    ]
    .into_iter()
    .flatten()
    .find(|path| path.join("libnx/include/deko3d.h").is_file())
}

fn require_file(path: &Path) {
    if let Err(err) = fs::metadata(path) {
        panic!(
            "required devkitPro file is missing: {} ({err})",
            path.display()
        );
    }
}
