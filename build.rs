fn main() {
    // Only embed the icon when compiling *for* Windows (native or cross-compile).
    // CARGO_CFG_TARGET_OS is set by Cargo to the target OS, not the host OS.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("jsd.ico");

        // Optional version metadata shown in Explorer's Properties → Details tab
        res.set("ProductName",      "JioSaavn Downloader");
        res.set("FileDescription",  "High-performance JioSaavn music downloader");
        res.set("LegalCopyright",   "© habitual69");
        res.set("ProductVersion",   "1.0.0");
        res.set("FileVersion",      "1.0.0");

        res.compile().unwrap_or_else(|e| {
            // Don't hard-fail when windres / llvm-rc isn't installed on the host.
            // The binary will still build; it just won't have an embedded icon.
            eprintln!("cargo:warning=Could not embed Windows icon: {e}");
            eprintln!("cargo:warning=Install mingw-w64 (windres) or LLVM (llvm-rc) to fix this.");
        });
    }
}
