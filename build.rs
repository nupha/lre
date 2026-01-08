use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=lib/");

    let mut build = cc::Build::new();

    build
        .file("lib/libregexp.c")
        .file("lib/cutils.c")
        .file("lib/libunicode.c")
        .file("lib/stubs.c")
        .include("lib")
        .flag_if_supported("-Wall")
        .flag_if_supported("-Wextra")
        .flag_if_supported("-Werror")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-sign-compare")
        .flag_if_supported("-O2");

    let target = env::var("TARGET").unwrap();
    if target.contains("windows") {
        build.define("_WIN32", None);
    }

    build.compile("libregexp");

    let bindings = bindgen::Builder::default()
        .header("lib/libregexp.h")
        .header("lib/cutils.h")
        .header("lib/libunicode.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .allowlist_function("lre_.*")
        .allowlist_type("LRE_.*")
        .allowlist_var("LRE_.*")
        .size_t_is_usize(true)
        .opaque_type("DynBuf")
        .opaque_type("CharRange")
        .opaque_type("REParseState")
        .opaque_type("REExecContext")
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .generate_comments(true)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:rustc-link-lib=static=libregexp");
}
