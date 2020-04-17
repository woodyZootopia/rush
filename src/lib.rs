extern crate nix;

pub mod rsh_loop {
    use nix::sys::wait::*;
    use nix::unistd::*;
    use std::io;

    pub struct CommandConfig<'a> {
        pub command: Option<&'a str>,
        pub args: Vec<&'a str>,
    }

    pub fn rsh_read_line() -> String {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        return input;
    }

    pub fn rsh_split_line<'a>(line: &'a str) -> CommandConfig<'a> {
        let mut inputs = line.split_ascii_whitespace();
        if let Some(command) = inputs.next() {
            let mut args = Vec::new();
            while let Some(arg) = inputs.next() {
                args.push(arg);
            }
            CommandConfig {
                command: Some(command),
                args,
            }
        } else {
            CommandConfig {
                command: None,
                args: Vec::new(),
            }
        }
    }

    pub enum Status {
        Success,
        Exit,
    }

    pub fn rsh_execute(config: CommandConfig) -> Option<Status> {
        match config.command {
            Some("cd") => rsh_cd(config.args),
            Some("help") => rsh_help(config.args),
            Some("exit") => rsh_exit(config.args),
            _ => rsh_launch(config),
        }
    }

    fn rsh_launch(config: CommandConfig) -> Option<Status> {
        print_command(config);
        match fork() {
            Ok(ForkResult::Parent { child, .. }) => {
                // parent
                // println!("I'm parent. Child PID is {}", child);
                loop {
                    let waitresult = waitpid(child, Some(WaitPidFlag::WUNTRACED));
                    match waitresult.unwrap() {
                        WaitStatus::Exited(..) | WaitStatus::Signaled(..) => {
                            break Some(Status::Success)
                        }
                        _ => ()
                    }
                }
            }
            Ok(ForkResult::Child) => {
                // child
                println!("I'm child");
                //do command
                Some(Status::Exit)
            }
            Err(_) => {
                None
            }
        }
    }

    fn print_command(config: CommandConfig) {
        println!(
            "Command is {:?},\n args are {:?}",
            config.command, config.args
        );
    }

    fn rsh_cd(args: Vec<&str>) -> Option<Status> {
        assert!(
            args.len() > 0,
            "going to home directory just by `cd` is not supported for now"
        );
        Some(Status::Success)
    }

    fn rsh_help(args: Vec<&str>) -> Option<Status> {
        Some(Status::Success)
    }
    fn rsh_exit(args: Vec<&str>) -> Option<Status> {
        Some(Status::Success)
    }
}
