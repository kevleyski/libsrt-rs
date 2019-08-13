use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    if env::var("LIBSRT_SYS_USE_PKG_CONFIG").is_ok() {
        let mut cfg = pkg_config::Config::new();
        if let Ok(lib) = cfg.atleast_version("1.3.0").probe("srt") {
            for include in &lib.include_paths {
                println!("cargo:root={}", include.display());
            }
            return;
        }
    }

    if !Path::new("srt/.git").exists() {
        let _ = Command::new("git")
            .args(&["submodule", "update", "--init"])
            .status();
    }

    let dst = cmake::Config::new("srt")
        .define("ENABLE_SHARED", "OFF")
        .define("ENABLE_APPS", "OFF")
        .build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=srt");
}
