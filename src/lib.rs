
pub mod rsh_loop {
    use std::io;

    pub struct CommandConfig {
        pub command: String,
        pub args: Vec<String>,
    }

    pub fn rsh_read_line() -> Result<String, io::Error> {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        return Ok(input);
    }

    pub fn rsh_split_line(line: String) -> CommandConfig {
        let mut inputs = line.split_ascii_whitespace();
        let command: String = inputs.next().unwrap().to_string();
        let mut args = Vec::new();
        while let Some(arg) = inputs.next() {
            args.push(arg.to_string());
        }
        CommandConfig { command, args }
    }

    pub enum Status {
        Success,
    }

    pub fn rsh_execute(config: CommandConfig) -> Status {
        print_command(config);
        Status::Success
    }

    pub fn print_command(config: CommandConfig) {
        println!(
            "Command is {},\n args are {:?}",
            config.command, config.args
        );
    }
}
