use rust_shell::rush;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::path::Path;

fn main() {
    let path = "/bin";
    let mut available_binaries = HashMap::<CString, CString>::new();
    for entry in fs::read_dir(Path::new(path)).unwrap() {
        // assign binaries
        let entry_path = entry.unwrap().path();
        let stem_string = entry_path
            .file_stem()
            .unwrap()
            .to_os_string()
            .into_string()
            .unwrap();
        available_binaries.insert(
            CString::new(stem_string).unwrap(),
            CString::new(entry_path.to_str().unwrap()).unwrap(),
        );
    }
    rush::main_loop(&available_binaries);
}
