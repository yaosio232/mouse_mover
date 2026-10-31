fn main() {
    let root = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap()
        .replace('\\', "/");
    println!("cargo:rerun-if-changed={root}/mouseIcon.ico");
    println!("cargo:rerun-if-changed={root}/app.manifest");
    let rc = format!("1 24 \"{root}/app.manifest\"\n2 ICON \"{root}/mouseIcon.ico\"\n");
    let path = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("app.rc");
    std::fs::write(&path, rc).unwrap();
    embed_resource::compile(path, embed_resource::NONE)
        .manifest_required()
        .unwrap();
}
