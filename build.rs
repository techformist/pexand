fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        // Try to set icon if icon.ico exists, otherwise skip
        if std::path::Path::new("assets/icon.ico").exists() {
            res.set_icon("assets/icon.ico");
            if let Err(e) = res.compile() {
                println!("cargo:warning=Could not embed icon: {}", e);
            }
        } else {
            println!("cargo:warning=No assets/icon.ico found, executable will have default icon");
        }
    }
}
