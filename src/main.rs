use rust_shell::rsh;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn main() {
    let mut available_binaries = HashMap::new();
    for entry in fs::read_dir(Path::new("/bin")).unwrap() { //assign binaries
        let entry = entry.unwrap().path();
        let stem = entry.file_stem();
        available_binaries.insert(
            stem.unwrap().to_os_string(),
            entry.to_str().unwrap().to_string(),
        );
    }
    rsh::rsh_loop(&available_binaries);
}
