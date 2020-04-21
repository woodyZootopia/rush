extern crate nix;

pub mod rush {
    use nix::sys::wait::*;
    use nix::unistd::*;
    use std::collections::HashMap;
    use std::ffi::{CStr, CString};
    use std::io;
    use std::io::Write;

    pub fn rsh_loop(available_binaries: &HashMap<CString, CString>) {
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
        pub args: Vec<CString>,
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
                args.push(CString::new(arg).unwrap());
            }
            CommandConfig {
                command: Some(command),
                args: args,
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

    fn rsh_execute(
        config: &CommandConfig,
        available_binaries: &HashMap<CString, CString>,
    ) -> Option<Status> {
        match config.command {
            Some("cd") => rsh_cd(&config.args),
            Some("help") => rsh_help(&config.args),
            Some("exit") => rsh_exit(&config.args),
            Some("pwd") => rsh_pwd(&config.args),
            Some("which") => rsh_which(&config.args, &available_binaries),
            _ => rsh_launch(config, &available_binaries),
        }
    }

    fn rsh_launch(
        config: &CommandConfig,
        available_binaries: &HashMap<CString, CString>,
    ) -> Option<Status> {
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
                match config.command {
                    None => Some(Status::Exit),
                    Some(command) => {
                        let command_path = available_binaries
                            .get(&CString::new(command).unwrap())
                            .expect(&format!("Command not found: {}", command)[..]);

                        if config.args.len() == 0 {
                            execv(command_path, &[CString::new("").unwrap().as_c_str()]);
                        } else {
                            execv(
                                command_path,
                                &config.args[..]
                                    .iter()
                                    .map(AsRef::as_ref)
                                    .collect::<Vec<&CStr>>()
                                    .as_ref(),
                            );
                        }
                        Some(Status::Exit)
                    }
                }
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

    fn rsh_cd(args: &Vec<CString>) -> Option<Status> {
        assert!(
            args.len() > 0,
            "going to home directory just by `cd` is not supported for now"
        );
        chdir(args[0].as_c_str())
            .map(|_| Status::Success)
            .map_err(|err| println!("{}", err.to_string()));
        Some(Status::Success)
    }

    fn rsh_help(_args: &Vec<CString>) -> Option<Status> {
        println!("Woody's re-implemantation of lsh, written in Rust.",);
        println!("Type command and arguments, and hit enter.",);
        // println!("The following commands are built in:",);
        Some(Status::Success)
    }

    fn rsh_exit(_args: &Vec<CString>) -> Option<Status> {
        Some(Status::Exit)
    }

    fn rsh_pwd(_args: &Vec<CString>) -> Option<Status> {
        println!("{:?}", getcwd().unwrap());
        Some(Status::Success)
    }

    fn rsh_which(
        args: &Vec<CString>,
        available_binaries: &HashMap<CString, CString>,
    ) -> Option<Status> {
        match available_binaries.get(&args[0]) {
            // Some(command) => println!("{}", command),
            Some(command) => println!("{:?}", command),
            None => println!("Command {:?} not found.", args[0]),
        }
        Some(Status::Success)
    }
}
