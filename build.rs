// Embeds assets/icons/kannada_ime.ico and version info (from Cargo.toml) into
// kannada_ime.exe, using `winresource` (the maintained fork of `winres`, 0.5.3).
// Runs only when the *target* is Windows, so a cross-build from Linux gets the
// icon too (it then needs `llvm-rc` or `x86_64-w64-mingw32-windres` on PATH).
// Needs the Windows SDK `rc.exe` (MSVC) or `windres` (MinGW) on PATH on Windows;
// if embedding fails the build still succeeds, with a warning.
fn main() {
    println!("cargo:rerun-if-changed=assets/icons/kannada_ime.ico");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icons/kannada_ime.ico");
        res.set("ProductName", "Kannada IME");
        res.set("FileDescription", "Phonetic Kannada typing agent");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed exe icon: {e}");
        }
    }
}
