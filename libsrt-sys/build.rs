use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap();

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

    let mut cmk = cmake::Config::new("srt");

    if target.contains("apple") {
        let output = Command::new("brew")
            .args(&["--prefix", "openssl"])
            .output()
            .expect("failed to execute brew");
        let output_str = String::from_utf8_lossy(&output.stdout);
        let trimmed_str = output_str.trim();

        env::set_var("OPENSSL_ROOT_DIR", trimmed_str.to_string());
        env::set_var("OPENSSL_LIB_DIR", format!("{}/lib", trimmed_str));
        env::set_var("OPENSSL_INCLUDE_DIR", format!("{}/include", trimmed_str));

        println!("cargo:rustc-link-search=native={}/lib", trimmed_str);
    }

    let dst = cmk
        .define("ENABLE_SHARED", "OFF")
        .define("ENABLE_APPS", "OFF")
        .build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-search=native={}/lib64", dst.display());
    println!("cargo:rustc-link-lib=static=srt");

    if target.contains("apple") {
        println!("cargo:rustc-link-lib=c++");
    } else {
        println!("cargo:rustc-link-lib=stdc++");
    }
    println!("cargo:rustc-link-lib=crypto");
}
