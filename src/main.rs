use rust_shell::rsh_loop;
use std::io;
use std::io::Write;

fn rsh_loop() {
    loop {
        print!("> ");
        io::stdout().flush().unwrap(); // make sure above `> ` is printed
        let line = rsh_loop::rsh_read_line();
        let args = rsh_loop::rsh_split_line(line.unwrap());
        let status = rsh_loop::rsh_execute(args);
    }
}

fn main() {
    rsh_loop();
}
