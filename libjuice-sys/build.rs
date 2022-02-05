use std::env;
use std::path::PathBuf;

#[allow(dead_code)]
fn env_var_rerun(name: &str) -> Result<String, env::VarError> {
    println!("cargo:rerun-if-env-changed={}", name);
    env::var(name)
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    let mut config = cmake::Config::new("libjuice");
    config.build_target("juice-static");
    config.out_dir(&out_dir);
    config.define("NO_EXPORT_HEADER", "ON");
    config.define("NO_TESTS", "ON");
    config.build();

    // Link static libjuice
    println!("cargo:rustc-link-search=native={}/build", out_dir);
    println!("cargo:rustc-link-lib=static=juice-static");

    let bindings = bindgen::Builder::default()
        .header("libjuice/include/juice/juice.h")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(out_dir);
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");
}
