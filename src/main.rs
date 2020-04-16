use rust_shell::rsh_loop;

fn rsh_loop() {
    loop {
        println!("> ");
        let line = rsh_loop::rsh_read_line();
        let args = rsh_loop::rsh_split_line(line.unwrap());
        let status = rsh_loop::rsh_execute(args);
    }
}

fn main() {
    rsh_loop();
}
