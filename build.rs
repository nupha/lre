use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=lib/");

    let target = env::var("TARGET").unwrap();
    let is_msvc = target.contains("msvc");

    let mut build = cc::Build::new();

    build
        .file("lib/libregexp.c")
        .file("lib/cutils.c")
        .file("lib/libunicode.c")
        .file("lib/stubs.c")
        .include("lib");

    if is_msvc {
        build
            .flag("/std:c11")
            .flag("/W3")
            .flag("/WX")
            .flag("/wd4100")
            .flag("/wd4389")
            .flag("/wd4244")
            .flag("/wd4245")
            .flag("/wd4267")
            .flag("/wd4018")
            .flag("/wd4819")
            .flag("/O2")
            .define("alloca", "_alloca")
            .flag("/FImsvc_compat.h");
    } else {
        build
            .flag_if_supported("-Wall")
            .flag_if_supported("-Wextra")
            .flag_if_supported("-Werror")
            .flag_if_supported("-Wno-unused-parameter")
            .flag_if_supported("-Wno-sign-compare")
            .flag_if_supported("-O2");
    }

    if target.contains("windows") {
        build.define("_WIN32", None);
    }

    build.compile("libregexp");

    // -- bindgen --
    let mut bindings = bindgen::Builder::default()
        .header("lib/libregexp.h")
        .header("lib/cutils.h")
        .header("lib/libunicode.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
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
        .generate_comments(true);

    if is_msvc {
        // libclang needs MSVC include paths to resolve system headers
        if let Some(dirs) = detect_msvc_include_dirs() {
            for d in &dirs {
                bindings = bindings.clang_arg(format!("-isystem{}", d));
            }
        }
        bindings = bindings.clang_arg("-std=c11");
        bindings = bindings.clang_arg("-target");
        bindings = bindings.clang_arg("x86_64-pc-windows-msvc");
    }

    let bindings = bindings.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:rustc-link-lib=static=libregexp");
}

/// Find MSVC include directories via vswhere
fn detect_msvc_include_dirs() -> Option<Vec<String>> {
    let cl_exe = find_msvc_root()?;
    // cl.exe path: <msvc_root>/bin/HostX64/x64/cl.exe
    // Go up 4 levels to reach the MSVC version root
    let msvc_ver_root = cl_exe.parent()?.parent()?.parent()?.parent()?;

    let mut dirs = Vec::new();
    // MSVC built-in include
    let msvc_inc = msvc_ver_root.join("include");
    if msvc_inc.exists() {
        dirs.push(msvc_inc.to_string_lossy().to_string());
    }

    // Windows Kits — under Program Files or Program Files (x86)
    let mut kit_root = None;
    // Try Program Files (x86) first
    if let Ok(base) = env::var("ProgramW6432") {
        let x86 = base.replace("Program Files", "Program Files (x86)");
        for p in [&x86, &base] {
            let k10 = PathBuf::from(p).join("Windows Kits").join("10");
            let k81 = PathBuf::from(p).join("Windows Kits").join("8.1");
            if k10.exists() {
                kit_root = Some(k10);
                break;
            }
            if k81.exists() {
                kit_root = Some(k81);
                break;
            }
        }
    }
    let kit_root = kit_root?;

    let inc_dir = kit_root.join("Include");
    let kit_ver = std::fs::read_dir(&inc_dir).ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .max_by_key(|e| e.file_name())?;

    for sub in &["ucrt", "shared", "um", "winrt"] {
        let p = kit_ver.path().join(sub);
        if p.exists() {
            dirs.push(p.to_string_lossy().to_string());
        }
    }

    Some(dirs)
}

/// Find path to cl.exe via vswhere or PATH
fn find_msvc_root() -> Option<PathBuf> {
    // Method 1: vswhere to locate VS installation
    let pf86 = r"C:\Program Files (x86)";
    let vswhere = PathBuf::from(pf86)
        .join("Microsoft Visual Studio")
        .join("Installer")
        .join("vswhere.exe");
    if vswhere.exists() {
        if let Ok(out) = Command::new(&vswhere)
            .args(["-latest", "-property", "installationPath"])
            .output()
        {
            let vs_path = String::from_utf8(out.stdout).ok()?.trim().to_string();
            if !vs_path.is_empty() {
                let msvc_dir = PathBuf::from(&vs_path).join("VC").join("Tools").join("MSVC");
                if let Ok(entries) = std::fs::read_dir(&msvc_dir) {
                    let latest = entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .max_by_key(|e| e.file_name());
                    if let Some(l) = latest {
                        let cl_path = l.path().join("bin").join("HostX64").join("x64").join("cl.exe");
                        if cl_path.exists() {
                            return Some(cl_path);
                        }
                    }
                }
            }
        }
    }

    // Method 2: find cl.exe via PATH (when vcvars is already sourced)
    if let Ok(paths) = env::var("PATH") {
        for p in env::split_paths(&paths) {
            let cl = p.join("cl.exe");
            if cl.exists() {
                return Some(cl);
            }
        }
    }

    None
}
