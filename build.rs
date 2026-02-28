use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=VECTOR_XLAPI_DIR");

    if env::var_os("CARGO_FEATURE_VXL_CAN").is_none() {
        return;
    }

    let default_dir = r"C:\Users\Public\Documents\Vector\XL Driver Library 25.20.14.0\bin";
    let link_dir = env::var("VECTOR_XLAPI_DIR").unwrap_or_else(|_| default_dir.to_string());
    println!("cargo:rustc-link-search=native={}", link_dir);

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    if target_os == "windows" && target_arch == "x86_64" {
        println!("cargo:rustc-link-lib=dylib=vxlapi64");
    } else {
        println!("cargo:rustc-link-lib=dylib=vxlapi");
    }
}
