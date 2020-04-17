use rust_shell::rsh_loop;
use std::io;
use std::io::Write;

fn rsh_loop() {
    loop {
        print!("> ");
        io::stdout().flush().unwrap(); // make sure above `> ` is printed
        let line = rsh_loop::rsh_read_line().unwrap();
        let args = rsh_loop::rsh_split_line(&line);
        match rsh_loop::rsh_execute(args) {
            Some(rsh_loop::Status::Exit) => break,
            _ => (),
        }
    }
}

fn main() {
    rsh_loop();
}
