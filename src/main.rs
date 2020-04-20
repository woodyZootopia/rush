use rust_shell::rsh;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::path::Path;

fn main() {
    let mut available_binaries = HashMap::<CString, CString>::new();
    for entry in fs::read_dir(Path::new("/bin")).unwrap() {
        //assign binaries
        let entry = entry.unwrap().path();
        let stem = entry.file_stem();
        let stem_string = stem.unwrap().to_os_string().into_string().unwrap();
        available_binaries.insert(
            CString::new(stem_string).unwrap(),
            CString::new(entry.to_str().unwrap()).unwrap(),
        );
    }
    rsh::rsh_loop(&available_binaries);
}
