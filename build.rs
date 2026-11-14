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

    // GTK and Qt menus are alternative backends: with gtk3/gtk4 also enabled (e.g. through
    // feature unification, or `--all-features`), the GTK backend wins and `qt` only adds the
    // unused `qtshim` dependency.
    if gtk_platform
        && std::env::var_os("CARGO_FEATURE_QT").is_some()
        && (std::env::var_os("CARGO_FEATURE_GTK3").is_some()
            || std::env::var_os("CARGO_FEATURE_GTK4").is_some())
    {
        println!(
            "cargo::warning=features `qt` and `gtk3`/`gtk4` are enabled together; Muda will use the GTK backend"
        );
    }

    if gtk_platform
        && std::env::var_os("CARGO_FEATURE_QT").is_some()
        && std::env::var_os("CARGO_FEATURE_GTK3").is_none()
        && std::env::var_os("CARGO_FEATURE_GTK4").is_none()
    {
        println!("cargo::rustc-cfg=muda_qt");
    }
}
