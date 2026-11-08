fn main() {
    println!("cargo::rerun-if-env-changed=CARGO_FEATURE_GTK3");
    println!("cargo::rerun-if-env-changed=CARGO_FEATURE_GTK4");
    println!("cargo::rerun-if-env-changed=CARGO_FEATURE_QT");
    println!("cargo::rustc-check-cfg=cfg(muda_qt)");

    let gtk_platform = matches!(
        std::env::var("CARGO_CFG_TARGET_OS").as_deref(),
        Ok("linux" | "dragonfly" | "freebsd" | "netbsd" | "openbsd")
    );

    if gtk_platform
        && std::env::var_os("CARGO_FEATURE_GTK3").is_some()
        && std::env::var_os("CARGO_FEATURE_GTK4").is_some()
    {
        println!(
            "cargo::warning=features `gtk3` and `gtk4` are enabled together; Muda will use the `gtk4` backend"
        );
    }

    // With gtk3/gtk4 also enabled, lib.rs emits a compile error instead.
    #[cfg(feature = "qt")]
    if gtk_platform
        && std::env::var_os("CARGO_FEATURE_GTK3").is_none()
        && std::env::var_os("CARGO_FEATURE_GTK4").is_none()
    {
        println!("cargo::rustc-cfg=muda_qt");
        qt::build();
    }
}

#[cfg(feature = "qt")]
mod qt {
    use std::process::Command;

    const MODULES: &[&str] = &["Qt6Widgets"];

    fn pkg_config(args: &[&str]) -> String {
        let out = Command::new("pkg-config")
            .args(args)
            .output()
            .expect("failed to run pkg-config (needed by muda's `qt` feature)");
        if !out.status.success() {
            panic!(
                "pkg-config {args:?} failed: {}\nmuda's `qt` feature needs the Qt 6 Widgets development \
                 package (Fedora: qt6-qtbase-devel, Debian/Ubuntu: qt6-base-dev, Arch: qt6-base).",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    }

    pub fn build() {
        println!("cargo::rerun-if-changed=src/platform_impl/qt/muda_qt.cpp");
        println!("cargo::rerun-if-env-changed=PKG_CONFIG_PATH");
        let mut build = cc::Build::new();
        build
            .cpp(true)
            .std("c++17")
            .file("src/platform_impl/qt/muda_qt.cpp");
        let mut args = vec!["--cflags"];
        args.extend_from_slice(MODULES);
        for tok in pkg_config(&args).split_whitespace() {
            if let Some(inc) = tok.strip_prefix("-I") {
                build.include(inc);
            } else if let Some(def) = tok.strip_prefix("-D") {
                match def.split_once('=') {
                    Some((k, v)) => build.define(k, v),
                    None => build.define(def, None),
                };
            } else {
                build.flag(tok);
            }
        }
        build.compile("muda_qt");
        let mut args = vec!["--libs"];
        args.extend_from_slice(MODULES);
        for tok in pkg_config(&args).split_whitespace() {
            if let Some(lib) = tok.strip_prefix("-l") {
                println!("cargo::rustc-link-lib=dylib={lib}");
            } else if let Some(dir) = tok.strip_prefix("-L") {
                println!("cargo::rustc-link-search=native={dir}");
            }
        }
    }
}
