use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main()
{
    println!("cargo:rerun-if-changed=src/variadic.c");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is always set"));
    let obj_path = out_dir.join("variadic.o");
    let lib_path = out_dir.join("libjson_c_variadic.a");

    let cc = env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let ar = env::var("AR").unwrap_or_else(|_| "ar".to_string());

    let status = Command::new(&cc)
        .arg("-c")
        .arg("-fPIC")
        .arg("-O2")
        .arg("-std=c11")
        .arg("-Iinclude/json-c")
        .arg("src/variadic.c")
        .arg("-o")
        .arg(&obj_path)
        .status()
        .expect("failed to invoke the C compiler");
    assert!(status.success(), "compiling src/variadic.c failed");

    let status = Command::new(&ar)
        .arg("crus")
        .arg(&lib_path)
        .arg(&obj_path)
        .status()
        .expect("failed to invoke ar");
    assert!(status.success(), "archiving variadic helpers failed");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=json_c_variadic");
}
