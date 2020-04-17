extern crate nix;

pub mod rsh {
    use nix::sys::wait::*;
    use nix::unistd::*;
    use std::collections::HashMap;
    use std::io;
    use std::io::Write;
    use std::ffi::OsString;

    pub fn rsh_loop(available_binaries: &HashMap<OsString,String>) {
        loop {
            print!("> ");
            io::stdout().flush().unwrap(); // make sure above `> ` is printed
            let line = rsh_read_line();
            let config = rsh_split_line(&line);
            match rsh_execute(&config, &available_binaries) {
                Some(Status::Exit) => break,
                _ => (),
            }
        }
    }

    struct CommandConfig<'a> {
        pub command: Option<&'a str>,
        pub args: Vec<&'a str>,
    }

    fn rsh_read_line() -> String {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        return input;
    }

    fn rsh_split_line<'a>(line: &'a str) -> CommandConfig<'a> {
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

    enum Status {
        Success,
        Exit,
    }

    fn rsh_execute(config: &CommandConfig, available_binaries: &HashMap<OsString,String>) -> Option<Status> {
        match config.command {
            Some("cd") => rsh_cd(&config.args),
            Some("help") => rsh_help(&config.args),
            Some("exit") => rsh_exit(&config.args),
            Some("pwd") => rsh_pwd(&config.args),
            _ => rsh_launch(config, &available_binaries),
        }
    }

    fn rsh_launch(_config: &CommandConfig, available_binaries: &HashMap<OsString,String>) -> Option<Status> {
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
                        _ => (),
                    }
                }
            }
            Ok(ForkResult::Child) => {
                // child
                println!("I'm child");
                //do command
                // execv();
                Some(Status::Exit)
            }
            Err(_) => None,
        }
    }

    fn print_command(config: CommandConfig) {
        println!(
            "Command is {:?},\n args are {:?}",
            config.command, config.args
        );
    }

    fn rsh_cd(args: &Vec<&str>) -> Option<Status> {
        assert!(
            args.len() > 0,
            "going to home directory just by `cd` is not supported for now"
        );
        chdir(args[0])
            .map(|_| Status::Success)
            .map_err(|err| println!("{}", err.to_string()));
        Some(Status::Success)
    }

    fn rsh_help(_args: &Vec<&str>) -> Option<Status> {
        println!("Woody's re-implemantation of lsh, written in Rust.",);
        println!("Type command and arguments, and hit enter.",);
        // println!("The following commands are built in:",);
        Some(Status::Success)
    }

    fn rsh_exit(_args: &Vec<&str>) -> Option<Status> {
        Some(Status::Exit)
    }

    fn rsh_pwd(_args: &Vec<&str>) -> Option<Status> {
        println!("{:?}", getcwd().unwrap());
        Some(Status::Success)
    }
}
