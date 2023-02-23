use std::env::var;
use std::path::PathBuf;

static CARGO_PKG_NAME: &str = "CARGO_PKG_NAME";
static PROFILE: &str = "PROFILE";
static OUT_DIR: &str = "OUT_DIR";
static CARGO_MANIFEST_DIR: &str = "CARGO_MANIFEST_DIR";

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_name = var(CARGO_PKG_NAME).map_err(|_| "Could not find crate name".to_owned())?;
    let profile = var(PROFILE)?;

    let out_dir = PathBuf::from(var(OUT_DIR)?);
    let manifest_dir = PathBuf::from(var(CARGO_MANIFEST_DIR)?);

    // The directory where the plugin will be deployed
    let lua_dir = manifest_dir.join("lua");
    let deps_dir = lua_dir.join("deps");

    // Get the correct extension for the platform
    let in_ext;
    let mut out_ext = None;
    if cfg!(target_os = "windows") {
        in_ext = "dll";
    } else if cfg!(target_os = "macos") {
        in_ext = "dylib";
        out_ext = Some("so");
    } else {
        in_ext = "so";
    };

    // The name of the plugin library
    // Libs are named lib<crate_name>.<ext>
    // We output to the lua folder as <crate_name>.<ext> using the generated Makefile
    let lib_name = format!("lib{}.{}", crate_name, in_ext);
    let plugin_name = format!("{}.{}", crate_name, out_ext.unwrap_or(in_ext));

    // Get the location (idx) of the actual target dir (target/debug or target/release)
    // By using the $PROFILE and $OUT_DIR env variables we can get the correct target dir regardless of the build type
    // By searching for the profile in the out dir we can get the idx of the target dir in the path, and trim everything after that leaving the target
    let prof_idx = out_dir
        .ancestors()
        .position(|p| p.ends_with(&PathBuf::from(&profile)))
        .ok_or(format!(
            "Could not find target directory in {}",
            out_dir.display()
        ))?;
    // Get the path to the target dir
    let target_dir = out_dir.ancestors().take(prof_idx + 1).collect::<PathBuf>();

    if lua_dir.join(&plugin_name).exists() {
        // Remove old compiled plugin file
        std::fs::remove_file(lua_dir.join(&plugin_name))?;
    }
    if deps_dir.exists() {
        // Remove old deps
        std::fs::remove_dir_all(&deps_dir)?;
    }
    // Ensure the lua dir exists and recreate the deps dir
    std::fs::create_dir_all(&deps_dir)?;

    let makefile = format!(
        "\
LIB_NAME={lib_name}
PLUGIN_NAME={plugin_name}
LUA_DIR ={lua_dir}
DEPS_DIR={deps_dir}

TARGET_DIR={target_dir}

.PHONY: deploy
deploy:
\tcp ${{TARGET_DIR}}/${{LIB_NAME}} ${{LUA_DIR}}/${{PLUGIN_NAME}}
\tcp ${{TARGET_DIR}}/deps/*.rlib ${{DEPS_DIR}}
",
        // Perform path to string conversions
        lua_dir = lua_dir.to_string_lossy().to_string(),
        deps_dir = deps_dir.to_string_lossy().to_string(),
        target_dir = target_dir.to_string_lossy().to_string(),
    );

    // Write the makefile
    std::fs::write(manifest_dir.join("Makefile.plugin"), makefile)?;
    Ok(())
}
