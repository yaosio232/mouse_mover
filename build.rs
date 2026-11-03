fn main() {
    let root = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap()
        .replace('\\', "/");
    println!("cargo:rerun-if-changed={root}/mouseIcon.ico");
    println!("cargo:rerun-if-changed={root}/app.manifest");
    let rc = format!(
        r#"1 24 "{root}/app.manifest"
2 ICON "{root}/mouseIcon.ico"

1 VERSIONINFO
 FILEVERSION 1,0,3,0
 PRODUCTVERSION 1,0,3,0
 FILEFLAGSMASK 0x3fL
 FILEFLAGS 0x0L
 FILEOS 0x00040004L
 FILETYPE 0x00000001L
 FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904b0"
        BEGIN
            VALUE "FileDescription", "AZu Sanpo - mouse idle automation utility\0"
            VALUE "FileVersion", "1.0.3.0\0"
            VALUE "OriginalFilename", "MouseMoverApp.exe\0"
            VALUE "ProductName", "AZu Sanpo\0"
            VALUE "ProductVersion", "1.0.3.0\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#
    );
    let path = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("app.rc");
    std::fs::write(&path, rc).unwrap();
    embed_resource::compile(path, embed_resource::NONE)
        .manifest_required()
        .unwrap();
}
