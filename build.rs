// Embeds assets/icons/kannada_ime.ico and version info into kannada_ime.exe.
// A no-op on non-Windows hosts. Needs the Windows SDK `rc.exe` (MSVC) or
// `windres` (MinGW) on PATH; if embedding fails the build still succeeds.

fn main() {
    println!("cargo:rerun-if-changed=assets/icons/kannada_ime.ico");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            let mut res = winres::WindowsResource::new();
            res.set_icon("assets/icons/kannada_ime.ico");
            res.set("ProductName", "Kannada IME");
            res.set("FileDescription", "Phonetic Kannada typing agent");
            if let Err(e) = res.compile() {
                println!("cargo:warning=could not embed exe icon: {e}");
            }
        }
    }
}
