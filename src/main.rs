mod lib;
use crate::lib::rush;
use std::ffi::CString;

fn main() {
    let env_path = CString::new("PATH=/bin:/usr/bin").unwrap();
    let env_home = CString::new("HOME=/home/woody").unwrap();
    let env_vars = &[env_path.as_ref(), env_home.as_ref()];
    rush::main_loop(env_vars.as_ref());
}
