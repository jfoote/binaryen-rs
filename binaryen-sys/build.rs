extern crate bindgen;
extern crate cmake;
extern crate cc;

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn gen_bindings() {
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        // See https://github.com/rust-lang-nursery/rust-bindgen/issues/947
        .trust_clang_mangling(false)
        .generate_comments(true)
        // https://github.com/rust-lang-nursery/rust-bindgen/issues/947#issuecomment-327100002
        .layout_tests(false)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn main() {
    if !Path::new("binaryen/.git").exists() {
        let _ = Command::new("git")
            .args(&["submodule", "update", "--init"])
            .status();
    }

    gen_bindings();

    let target = env::var("TARGET").ok();
    if target.map_or(false, |target| target.contains("emscripten")) {
        let mut build_wasm_binaryen_args = vec![];
        if get_debug() {
            build_wasm_binaryen_args.push("-g");
        }

        let _ = Command::new("./build-binaryen-bc.sh")
            .args(&build_wasm_binaryen_args)
            .status()
            .unwrap();

        
        println!("cargo:rustc-link-search=native={}", env::var("OUT_DIR").unwrap());
        println!("cargo:rustc-link-lib=static=binaryen-c");
        return;
    }

    let dst = cmake::Config::new("binaryen")
        .define("BUILD_STATIC_LIB", "ON")
        .build();

    println!("cargo:rustc-link-search=native={}/build/lib", dst.display());
    println!("cargo:rustc-link-lib=static=binaryen");
    println!("cargo:rustc-link-lib=static=asmjs");
    println!("cargo:rustc-link-lib=static=cfg");
    println!("cargo:rustc-link-lib=static=ir");
    println!("cargo:rustc-link-lib=static=passes");
    println!("cargo:rustc-link-lib=static=support");
    println!("cargo:rustc-link-lib=static=wasm");
    println!("cargo:rustc-link-lib=static=emscripten-optimizer");

    // We need to link against C++ std lib
    if let Some(cpp_stdlib) = get_cpp_stdlib() {
        println!("cargo:rustc-link-lib={}", cpp_stdlib);
    }

    let mut cfg = cc::Build::new();
    cfg.file("Shim.cpp")
        .include("binaryen/src")
        .cpp_link_stdlib(None)
        .warnings(false)
        .cpp(true)
        .flag("-std=c++11")
        .compile("binaryen_shim");
}

// See https://github.com/alexcrichton/gcc-rs/blob/88ac58e25/src/lib.rs#L1197
fn get_cpp_stdlib() -> Option<String> {
    env::var("TARGET").ok().and_then(|target| {
        if target.contains("msvc") {
            None
        } else if target.contains("darwin") {
            Some("c++".to_string())
        } else if target.contains("freebsd") {
            Some("c++".to_string())
        } else if target.contains("musl") {
            Some("static=stdc++".to_string())
        } else {
            Some("stdc++".to_string())
        }
    })
}

// See https://github.com/alexcrichton/gcc-rs/blob/10871a0e40/src/lib.rs#L1501
fn get_debug() -> bool {
    match env::var("DEBUG").ok() {
        Some(s) => s != "false",
        None => false,
    }
}
