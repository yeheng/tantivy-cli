use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo::rerun-if-changed=web/package.json");
    println!("cargo::rerun-if-changed=web/vite.config.js");
    println!("cargo::rerun-if-changed=web/tailwind.config.js");
    println!("cargo::rerun-if-changed=web/index.html");
    println!("cargo::rerun-if-changed=web/src/");

    let web_dir = Path::new("web");
    let dist_dir = web_dir.join("dist");
    let pkg_json = web_dir.join("package.json");

    // Only trigger npm build if web/ exists and has package.json
    if !pkg_json.exists() {
        if !dist_dir.exists() {
            panic!(
                "web/dist/ not found and web/package.json missing. \
                 Please create the web/ frontend project or provide a pre-built dist/."
            );
        }
        return;
    }

    // Check if dist is older than any source file
    let needs_build = if !dist_dir.exists() {
        true
    } else {
        let dist_meta = dist_dir.metadata().unwrap();
        let dist_mtime = dist_meta.modified().unwrap();

        let src_dir = web_dir.join("src");
        is_any_newer_than(&src_dir, dist_mtime) || {
            let pkg_meta = pkg_json.metadata().unwrap();
            pkg_meta.modified().unwrap() > dist_mtime
        }
    };

    if needs_build {
        println!("cargo::warning=Building web frontend...");

        let npm = if Command::new("npm").arg("--version").output().is_ok() {
            "npm"
        } else {
            panic!("npm not found. Please install Node.js and npm to build the web frontend.");
        };

        // npm install
        let status = Command::new(npm)
            .args(["install", "--prefix", "web"])
            .status()
            .expect("failed to run npm install");
        if !status.success() {
            panic!("npm install failed");
        }

        // npm run build
        let status = Command::new(npm)
            .args(["run", "build", "--prefix", "web"])
            .status()
            .expect("failed to run npm run build");
        if !status.success() {
            panic!("npm run build failed");
        }
    }
}

fn is_any_newer_than(dir: &Path, cutoff: std::time::SystemTime) -> bool {
    if !dir.exists() {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if is_any_newer_than(&path, cutoff) {
                return true;
            }
        } else {
            if let Ok(meta) = path.metadata() {
                if let Ok(mtime) = meta.modified() {
                    if mtime > cutoff {
                        return true;
                    }
                }
            }
        }
    }
    false
}
