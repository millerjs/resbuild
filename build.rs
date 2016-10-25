extern crate walkdir;

use walkdir::WalkDir;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;


// Get absolute path to the "target" directory ("build" dir)
fn get_target_dir() -> PathBuf {
    let bin = env::current_exe().expect("exe path");
    let mut target_dir = PathBuf::from(bin.parent().expect("bin parent"));
    while target_dir.file_name() != Some(OsStr::new("target")) {
        target_dir.pop();
    }
    target_dir
}

// Get absolute path to the project's top dir, given target dir
fn get_top_dir<'a>(target_dir: &'a Path) -> &'a Path {
    target_dir.parent().expect("target parent")
}


fn main() {
    let mut f = File::create(&Path::new("src/dictionary.rs")).unwrap();
    let mut schema_dir = get_top_dir(get_target_dir().as_path()).to_path_buf();
    schema_dir.push(Path::new("src/gdcdictionary/gdcdictionary/schemas"));

    let entries = WalkDir::new(schema_dir).into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().file_name().is_some())
        .filter(|e| e.path().file_name().unwrap().to_str().is_some())
        .filter(|e| format!("{}", e.path().display()).ends_with(".yaml"))
        .filter(|e| !format!("{}", e.path().display()).contains("metaschema.yaml"))
        .filter(|e| !format!("{}", e.path().display()).contains("projects"));

    let mut schemas = Vec::new();

    for entry in entries {
        schemas.push(format!("    include_str!(\"{}\"),\n", entry.path().to_str().unwrap()));
    }

    f.write_all(&*format!("pub static SCHEMAS: [&'static str; {}] = [\n", schemas.len())
                .as_bytes()).unwrap();

    for schema in schemas {
        f.write_all(schema.as_bytes()).unwrap();
    }
    f.write_all(b"];").unwrap();
}
