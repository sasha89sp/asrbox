use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CheckReport {
    crates_checked: usize,
    rust_files_checked: usize,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("atlas-check") if args.len() == 1 => run_atlas_check_cli(),
        Some("--help" | "-h" | "help") => {
            print_usage();
            ExitCode::SUCCESS
        }
        _ => {
            print_usage();
            ExitCode::FAILURE
        }
    }
}

fn print_usage() {
    println!("Usage: cargo run -p xtask -- <command>");
    println!();
    println!("Commands:");
    println!("  atlas-check    Verify ATLAS.md crate list and crate MAP.md .rs coverage");
}

fn run_atlas_check_cli() -> ExitCode {
    let root = match workspace_root() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("atlas-check error: {err}");
            return ExitCode::FAILURE;
        }
    };

    match atlas_check(&root) {
        Ok(report) => {
            println!(
                "atlas-check: OK ({} crates verified, {} .rs files mapped)",
                report.crates_checked, report.rust_files_checked
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("atlas-check: FAILED ({} issue(s))", errors.len());
            for err in errors {
                eprintln!("  - {err}");
            }
            ExitCode::FAILURE
        }
    }
}

fn workspace_root() -> Result<PathBuf, String> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "cannot resolve workspace root from CARGO_MANIFEST_DIR".to_owned())
}

fn atlas_check(root: &Path) -> Result<CheckReport, Vec<String>> {
    let mut errors = Vec::new();

    let atlas_path = root.join("ATLAS.md");
    let atlas_content = match fs::read_to_string(&atlas_path) {
        Ok(content) => content,
        Err(err) => {
            errors.push(format!("cannot read ATLAS.md: {err}"));
            return Err(errors);
        }
    };

    let crates_dir = root.join("crates");
    let crate_dirs = match discover_crates(&crates_dir) {
        Ok(dirs) => dirs,
        Err(err) => {
            errors.push(err);
            return Err(errors);
        }
    };

    if crate_dirs.is_empty() {
        errors.push("no crates with Cargo.toml found under crates/".to_owned());
        return Err(errors);
    }

    let mut crates_checked = 0usize;
    let mut rust_files_checked = 0usize;

    for crate_dir in &crate_dirs {
        let dir_name = match crate_dir.file_name().and_then(OsStr::to_str) {
            Some(name) => name,
            None => {
                errors.push(format!("non-UTF-8 crate dir: {}", crate_dir.display()));
                continue;
            }
        };

        let crate_rel = format!("crates/{dir_name}");
        crates_checked += 1;

        if !atlas_content.contains(&crate_rel) {
            errors.push(format!("crate '{crate_rel}' is not mentioned in ATLAS.md"));
        }

        let mapped_count = check_crate_map(crate_dir, &crate_rel, &mut errors);
        rust_files_checked += mapped_count;
    }

    let xtask_dir = root.join("xtask");
    if xtask_dir.join("Cargo.toml").is_file() {
        crates_checked += 1;
        if !atlas_content.contains("xtask") {
            errors.push("crate 'xtask' is not mentioned in ATLAS.md".to_owned());
        }
        let mapped_count = check_crate_map(&xtask_dir, "xtask", &mut errors);
        rust_files_checked += mapped_count;
    }

    if errors.is_empty() {
        Ok(CheckReport {
            crates_checked,
            rust_files_checked,
        })
    } else {
        Err(errors)
    }
}

fn discover_crates(crates_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(crates_dir)
        .map_err(|err| format!("cannot read {}: {err}", crates_dir.display()))?;

    let mut dirs = Vec::new();
    for entry_res in entries {
        let entry = entry_res
            .map_err(|err| format!("cannot read entry in {}: {err}", crates_dir.display()))?;
        let path = entry.path();
        if path.is_dir() && path.join("Cargo.toml").is_file() {
            dirs.push(path);
        }
    }

    dirs.sort();
    Ok(dirs)
}

fn check_crate_map(crate_dir: &Path, crate_rel: &str, errors: &mut Vec<String>) -> usize {
    let map_path = crate_dir.join("MAP.md");
    let map_content = match fs::read_to_string(&map_path) {
        Ok(content) => content,
        Err(err) => {
            errors.push(format!("cannot read {crate_rel}/MAP.md: {err}"));
            return 0;
        }
    };

    let rs_files = match collect_rs_files(crate_dir, crate_dir) {
        Ok(files) => files,
        Err(err) => {
            errors.push(format!("cannot scan .rs files in {crate_rel}: {err}"));
            return 0;
        }
    };

    for rel_path in &rs_files {
        if !map_content.contains(rel_path) {
            errors.push(format!(
                "file '{rel_path}' in '{crate_rel}' is not mentioned in {crate_rel}/MAP.md"
            ));
        }
    }

    rs_files.len()
}

fn collect_rs_files(crate_root: &Path, current_dir: &Path) -> Result<Vec<String>, String> {
    let entries = fs::read_dir(current_dir)
        .map_err(|err| format!("cannot read {}: {err}", current_dir.display()))?;

    let mut files = Vec::new();
    for entry_res in entries {
        let entry = entry_res
            .map_err(|err| format!("cannot read entry in {}: {err}", current_dir.display()))?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path.file_name().and_then(OsStr::to_str);
            if dir_name == Some("target") {
                continue;
            }
            let nested = collect_rs_files(crate_root, &path)?;
            files.extend(nested);
        } else if path.extension().and_then(OsStr::to_str) == Some("rs") {
            let rel = path
                .strip_prefix(crate_root)
                .map_err(|err| format!("cannot strip prefix for {}: {err}", path.display()))?;
            files.push(normalize_rel_path(rel));
        }
    }

    files.sort();
    Ok(files)
}

fn normalize_rel_path(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        parts.push(component.as_os_str().to_string_lossy().into_owned());
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        fn new(prefix: &str) -> Result<Self, String> {
            let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
            let dir_name = format!("asrbox-xtask-{prefix}-{}-{id}", std::process::id());
            let path = env::temp_dir().join(dir_name);
            fs::create_dir_all(&path)
                .map_err(|err| format!("cannot create temp dir {}: {err}", path.display()))?;
            Ok(Self { path })
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_file(path: &Path, content: &str) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("cannot create dir {}: {err}", parent.display()))?;
        }
        fs::write(path, content)
            .map_err(|err| format!("cannot write file {}: {err}", path.display()))
    }

    #[test]
    fn real_workspace_passes_atlas_check() -> Result<(), String> {
        let root = workspace_root()?;
        let report = atlas_check(&root).map_err(|errs| errs.join("; "))?;
        if report.crates_checked < 12 {
            let count = report.crates_checked;
            return Err(format!("expected at least 12 crates, got {count}"));
        }
        if report.rust_files_checked < 12 {
            let count = report.rust_files_checked;
            return Err(format!("expected at least 12 .rs files, got {count}"));
        }
        Ok(())
    }

    #[test]
    fn detects_missing_crate_in_atlas() -> Result<(), String> {
        let temp = TempDirGuard::new("missing-atlas")?;
        let root = temp.path();
        let map_md = "# MAP\n\n- `src/lib.rs`\n";

        write_file(&root.join("ATLAS.md"), "# Atlas\n\n- crates/alpha\n")?;
        write_file(
            &root.join("crates/alpha/Cargo.toml"),
            "[package]\nname = \"alpha\"\n",
        )?;
        write_file(&root.join("crates/alpha/MAP.md"), map_md)?;
        write_file(&root.join("crates/alpha/src/lib.rs"), "")?;

        write_file(
            &root.join("crates/beta/Cargo.toml"),
            "[package]\nname = \"beta\"\n",
        )?;
        write_file(&root.join("crates/beta/MAP.md"), map_md)?;
        write_file(&root.join("crates/beta/src/lib.rs"), "")?;

        let result = atlas_check(root);
        match result {
            Ok(_) => Err("expected failure for missing crate in ATLAS.md".to_owned()),
            Err(errors) => {
                if errors.iter().any(|e| e.contains("crates/beta")) {
                    Ok(())
                } else {
                    Err(format!("unexpected errors: {}", errors.join("; ")))
                }
            }
        }
    }

    #[test]
    fn detects_unmapped_rust_file_in_map() -> Result<(), String> {
        let temp = TempDirGuard::new("missing-map")?;
        let root = temp.path();
        let map_md = "# MAP\n\n- `src/lib.rs`\n";

        write_file(&root.join("ATLAS.md"), "# Atlas\n\n- crates/alpha\n")?;
        write_file(
            &root.join("crates/alpha/Cargo.toml"),
            "[package]\nname = \"alpha\"\n",
        )?;
        write_file(&root.join("crates/alpha/MAP.md"), map_md)?;
        write_file(&root.join("crates/alpha/src/lib.rs"), "")?;
        write_file(&root.join("crates/alpha/src/extra.rs"), "")?;

        let result = atlas_check(root);
        match result {
            Ok(_) => Err("expected failure for unmapped .rs file".to_owned()),
            Err(errors) => {
                if errors.iter().any(|e| e.contains("src/extra.rs")) {
                    Ok(())
                } else {
                    Err(format!("unexpected errors: {}", errors.join("; ")))
                }
            }
        }
    }
}
