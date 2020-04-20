use rust_shell::rsh;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::path::Path;

fn main() {
    let path = "/bin";
    let mut available_binaries = HashMap::<CString, CString>::new();
    for entry in fs::read_dir(Path::new(path)).unwrap() {
        //assign binaries
        let entry = entry.unwrap().path();
        let stem = entry.file_stem().unwrap();
        let stem_string = stem.to_os_string().into_string().unwrap();
        available_binaries.insert(
            CString::new(stem_string).unwrap(),
            CString::new(entry.to_str().unwrap()).unwrap(),
        );
    }
    rsh::rsh_loop(&available_binaries);
}
