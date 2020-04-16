extern crate nix;

pub mod rsh_loop {
    use nix::sys::wait::*;
    use nix::unistd::*;
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
        if let Some(command) = inputs.next() {
            let command = command.to_string();
            let mut args = Vec::new();
            while let Some(arg) = inputs.next() {
                args.push(arg.to_string());
            }
            CommandConfig { command, args }
        } else {
            CommandConfig {
                command: String::new(),
                args: Vec::new(),
            }
        }
    }

    pub enum Status {
        Success,
    }

    pub fn rsh_execute(config: CommandConfig) -> Status {
        print_command(config);
        match fork() {
            Ok(ForkResult::Parent { child, .. }) => {
                // parent
                println!("I'm parent. Child PID is {}", child);
                loop {
                    let waitresult = waitpid(child, Some(WaitPidFlag::WUNTRACED));
                    match waitresult.unwrap() {
                        WaitStatus::Exited(..) | WaitStatus::Signaled(..) => break,
                        _ => (),
                    }
                }
            }
            Ok(ForkResult::Child) => {
                // child
                println!("I'm child");
                //do command
            }
            Err(_) => panic!("Fork failed",),
        }
        Status::Success
    }

    fn print_command(config: CommandConfig) {
        println!(
            "Command is {},\n args are {:?}",
            config.command, config.args
        );
    }
}
