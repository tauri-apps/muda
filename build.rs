fn main() {
    println!("cargo::rerun-if-env-changed=CARGO_FEATURE_GTK3");
    println!("cargo::rerun-if-env-changed=CARGO_FEATURE_GTK4");

    let gtk_platform = matches!(
        std::env::var("CARGO_CFG_TARGET_OS").as_deref(),
        Ok("linux" | "dragonfly" | "freebsd" | "netbsd" | "openbsd")
    );

    if gtk_platform
        && std::env::var_os("CARGO_FEATURE_GTK3").is_some()
        && std::env::var_os("CARGO_FEATURE_GTK4").is_some()
    {
        println!(
            "cargo::warning=features `gtk3` and `gtk4` are enabled together; Muda will use its no-op backend"
        );
    }
}
