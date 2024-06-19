use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=m");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_recursively(false)
        .allowlist_item("fetestexcept")
        .allowlist_item("feclearexcept")
        .allowlist_item("fegetround")
        .allowlist_item("fesetround")
        .allowlist_item("fexcept_t")
        .allowlist_item("FE_ALL_EXCEPT")
        .allowlist_item("FE_DIVBYZERO")
        .allowlist_item("FE_INEXACT")
        .allowlist_item("FE_INVALID")
        .allowlist_item("FE_OVERFLOW")
        .allowlist_item("FE_UNDERFLOW")
        .allowlist_item("FE_DOWNWARD")
        .allowlist_item("FE_TONEAREST")
        .allowlist_item("FE_TOWARDZERO")
        .allowlist_item("FE_UPWARD")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

