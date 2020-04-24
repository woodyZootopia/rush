extern crate nix;

pub mod rush {
    use nix::sys::wait::*;
    use nix::unistd::*;
    use std::collections::HashMap;
    use std::ffi::{CStr, CString};
    use std::io;
    use std::io::Write;

    pub fn main_loop(available_binaries: &HashMap<CString, CString>) {
        loop {
            print!("> ");
            io::stdout().flush().unwrap(); // make sure above `> ` is printed
            let line = read_line();
            let configs = split_to_commands(&line);
            match execute(configs, &available_binaries) {
                Some(Status::Exit) => break,
                _ => (),
            }
        }
    }

    struct CommandConfig<'a> {
        pub command: Option<&'a str>,
        pub args: Vec<CString>,
    }

    fn read_line() -> String {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        return input;
    }

    fn split_to_commands<'a>(line: &'a str) -> Vec<CommandConfig<'a>> {
        let mut inputs = line.split_ascii_whitespace();
        let mut vec_of_commands = Vec::new();
        while let Some(command) = inputs.next() {
            let mut pipe_found = false;
            let mut args = vec![CString::new(command).unwrap()];
            while let Some(arg) = inputs.next() {
                if arg != "|" {
                    args.push(CString::new(arg).unwrap());
                } else {
                    pipe_found = true;
                    break;
                }
            }
            vec_of_commands.push(CommandConfig {
                command: Some(command),
                args: args,
            });
            if !pipe_found {
                break;
            }
        }
        vec_of_commands
    }

    enum Status {
        Success,
        Exit,
    }

    fn execute(
        configs: Vec<CommandConfig>,
        available_binaries: &HashMap<CString, CString>,
    ) -> Option<Status> {
        let mut result = None;
        for config in configs {
            result = match config.command {
                Some("cd") => rsh_cd(&config.args),
                Some("help") => rsh_help(&config.args),
                Some("exit") => rsh_exit(&config.args),
                Some("pwd") => rsh_pwd(&config.args),
                Some("which") => rsh_which(&config.args, &available_binaries),
                _ => rsh_launch(&config, &available_binaries),
            };
        }
        result
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
                            execv(command_path, &[CString::new("").unwrap().as_c_str()]).unwrap();
                        } else {
                            execv(
                                command_path,
                                &config.args[..]
                                    .iter()
                                    .map(AsRef::as_ref)
                                    .collect::<Vec<&CStr>>()
                                    .as_ref(),
                            )
                            .unwrap();
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
        chdir(args[1].as_c_str())
            .map(|_| Status::Success)
            .map_err(|err| println!("{}", err.to_string()))
            .unwrap();
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
        match available_binaries.get(&args[1]) {
            // Some(command) => println!("{}", command),
            Some(command) => println!("{:?}", command),
            None => println!("Command {:?} not found.", args[1]),
        }
        Some(Status::Success)
    }
}
