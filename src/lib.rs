extern crate nix;

pub mod rush {
    use nix::sys::wait::*;
    use nix::unistd::*;
    use std::collections::HashMap;
    use std::ffi::{CStr, CString};
    use std::io;
    use std::io::Write;

    pub fn main_loop(env_path: &[&CStr]) {
        loop {
            print!("> ");
            io::stdout().flush().unwrap(); // make sure "> " above is printed
            let line = read_line();
            let command_configs = split_to_commands(&line);
            match execute(command_configs, env_path) {
                Some(Status::Exit) => break,
                _ => (),
            }
        }
    }

    #[derive(Debug)]
    struct CommandConfig {
        pub command: CString,
        pub argv: Vec<CString>,
    }

    fn read_line() -> String {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        return input;
    }

    fn split_to_commands<'a>(line: &'a str) -> Vec<CommandConfig> {
        let mut vec_of_commands = Vec::new();
        let mut inputs = line.split_ascii_whitespace();
        while let Some(command) = inputs.next() {
            let mut pipe_found = false;
            let mut argv = vec![CString::new(command).unwrap()];
            while let Some(arg) = inputs.next() {
                if arg != "|" {
                    argv.push(CString::new(arg).unwrap());
                } else {
                    pipe_found = true;
                    break;
                }
            }
            vec_of_commands.push(CommandConfig {
                command: CString::new(command).unwrap(),
                argv,
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

    fn execute(command_configs: Vec<CommandConfig>, env_path: &[&CStr]) -> Option<Status> {
        let mut result = None;
        for command_config in command_configs {
            result = match command_config.command.to_str().unwrap() {
                "cd" => rsh_cd(&command_config.argv),
                "help" => rsh_help(&command_config.argv),
                "exit" => rsh_exit(&command_config.argv),
                "pwd" => rsh_pwd(&command_config.argv),
                // Some("which") => rsh_which(&command_config.args, &available_binaries),
                _ => rsh_launch(&command_config, env_path),
            };
        }
        result
    }

    fn rsh_launch(command_configs: &CommandConfig, env_path: &[&CStr]) -> Option<Status> {
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
                if command_configs.argv.len() == 0 {
                    execvpe(&command_configs.command, &[], env_path).unwrap();
                } else {
                    execvpe(
                        &command_configs.command,
                        &command_configs.argv[..]
                            .iter()
                            .map(AsRef::as_ref)
                            .collect::<Vec<&CStr>>()
                            .as_ref(),
                        env_path,
                    )
                    .unwrap();
                }
                Some(Status::Exit)
            }
            Err(_) => None,
        }
    }

    fn print_command(config: CommandConfig) {
        println!(
            "Command is {:?},\n args are {:?}",
            config.command, config.argv
        );
    }

    fn rsh_cd(args: &Vec<CString>) -> Option<Status> {
        assert!(
            args.len() > 1,
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
