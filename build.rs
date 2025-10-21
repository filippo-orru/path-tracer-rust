use std::fs;
use wesl::ModulePath;

fn main() {
    // Compile all wesl shaders in the "src/shaders" directory
    let shaders_dir = "src/shaders";
    let entries = fs::read_dir(shaders_dir).expect("Failed to read shaders directory");

    let wesl = wesl::Wesl::new(shaders_dir);

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "wesl") {
            let file_stem = path.file_stem().unwrap().to_string_lossy();
            println!("cargo:rerun-if-changed={}", path.display());

            wesl.build_artifact(
                &ModulePath {
                    origin: wesl::syntax::PathOrigin::Absolute,
                    components: vec![file_stem.to_string()],
                },
                &file_stem,
            );
        }
    }
}
