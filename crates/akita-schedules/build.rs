use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=AKITA_PREVALIDATED_VIEW_RS");
    let source = match env::var_os("AKITA_PREVALIDATED_VIEW_RS") {
        Some(path) => {
            let path = PathBuf::from(path);
            println!("cargo:rerun-if-changed={}", path.display());
            fs::read_to_string(path).expect("read trusted prevalidated verifier view")
        }
        None => String::from(
            "pub(super) static VIEW: Option<super::PrevalidatedVerifierView<'static>> = None;\n",
        ),
    };
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo OUT_DIR"));
    fs::write(output.join("prevalidated_view.rs"), source)
        .expect("write trusted prevalidated verifier view");
}
