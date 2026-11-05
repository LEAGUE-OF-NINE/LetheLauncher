fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");

    // Embed the exe icon and version info. The release workflow reads the exe's
    // ProductVersion to write latest.json, so it must match CARGO_PKG_VERSION.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let version = std::env::var("CARGO_PKG_VERSION").unwrap();
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico")
            .set("ProductName", "Lethe Launcher")
            .set("FileDescription", "Lethe Launcher")
            .set("ProductVersion", &version)
            .set("FileVersion", &version);
        res.compile().expect("failed to embed Windows resources");
    }
}
