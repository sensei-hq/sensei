fn main() {
    // tauri-plugin-playwright registers the `playwright:default` permission only
    // when the `e2e-testing` feature is active.  Without the feature the build
    // system still finds capabilities/e2e.json and fails with "Permission
    // playwright:default not found".  Restrict the capabilities glob to
    // default.json for normal builds; allow all files for e2e builds.
    let attrs = if cfg!(feature = "e2e-testing") {
        tauri_build::Attributes::new()
    } else {
        tauri_build::Attributes::new().capabilities_path_pattern("capabilities/default.json")
    };

    tauri_build::try_build(attrs).expect("failed to run tauri-build");
}
