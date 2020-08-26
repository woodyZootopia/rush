use rust_shell::rush;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::path::Path;

fn main() {
    let env_path = CString::new("PATH=/bin:/usr/bin").unwrap();
    let env_home = CString::new("HOME=/home/woody").unwrap();
    let env_path = &[env_path.as_ref(), env_home.as_ref()];
    rush::main_loop(env_path.as_ref());
}
