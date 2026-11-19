//! The GNU Rust toolchain's bundled MinGW libraries don't include shlwapi, which the
//! window library needs for opening links. MinGW's linker can link straight against a
//! DLL, so put a copy of Windows' own shlwapi.dll where the linker looks.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.ends_with("windows-gnu") {
        return;
    }
    let Ok(out) = std::env::var("OUT_DIR") else { return };
    let out = std::path::PathBuf::from(out);
    let windir = std::env::var("SystemRoot")
        .or_else(|_| std::env::var("windir"))
        .unwrap_or_else(|_| "C:\\Windows".to_string());
    let src = std::path::Path::new(&windir).join("System32").join("shlwapi.dll");
    if src.exists() && std::fs::copy(&src, out.join("shlwapi.dll")).is_ok() {
        println!("cargo:rustc-link-search=native={}", out.display());
    }
}
