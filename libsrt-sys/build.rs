use std::process;

fn main() {
    let mut cfg = pkg_config::Config::new();
    if let Ok(lib) = cfg.atleast_version("1.3.0").probe("srt") {
        for include in &lib.include_paths {
            println!("cargo:root={}", include.display());
        }
        return;
    }
    process::exit(1);
}
