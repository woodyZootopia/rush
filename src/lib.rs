extern crate anyhow;
extern crate nix;

pub mod rush {
    use anyhow::Context;
    use nix::sys::wait::*;
    use nix::unistd::*;
    use std::collections::HashMap;
    use std::ffi::{CStr, CString};
    use std::fs;
    use std::io;
    use std::io::Write;
    use std::path::{Path, PathBuf};

    pub fn main_loop(env_vars: &[&CStr]) {
        loop {
            print!("> ");
            io::stdout().flush().unwrap(); // make sure "> " above is printed
            let line = read_line();
            let command_configs = split_to_commands(&line);
            match execute(command_configs, env_vars) {
                Ok(Status::Exit) => break,
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
            let mut argv =
                vec![CString::new(command).expect("Failed to convert your command to CString")];
            while let Some(arg) = inputs.next() {
                if arg != "|" {
                    argv.push(
                        CString::new(arg).expect("Failed to convert your arguments to CString"),
                    );
                } else {
                    break;
                }
            }
            vec_of_commands.push(CommandConfig {
                command: CString::new(command).unwrap(),
                argv,
            });
        }
        vec_of_commands
    }

    enum Status {
        Success,
        Exit,
    }

    fn obtain_env_val_map(env_vars: &[&CStr]) -> HashMap<CString, CString> {
        let mut env_map = HashMap::new();
        for env_var in env_vars.iter() {
            let mut items = env_var
                .to_str()
                .expect("Failed to convert your environment variable to string")
                .splitn(2, "=");
            env_map.insert(
                CString::new(items.next().unwrap())
                    .expect("Unknown error when parsing your envvar. Isn't it empty?"),
                CString::new(items.next().unwrap())
                    .expect("Error when parsing your envvar. Are you sure it contains '='?"),
            );
        }
        env_map
    }

    fn execute(command_configs: Vec<CommandConfig>, env_vars: &[&CStr]) -> anyhow::Result<Status> {
        let env_map = obtain_env_val_map(env_vars);
        let mut result: anyhow::Result<Status> =
            Err(anyhow::Error::new(nix::Error::UnsupportedOperation));
        for command_config in command_configs {
            result = match command_config.command.to_str().unwrap() {
                "cd" => rsh_cd(&command_config.argv, &env_map),
                "help" => rsh_help(&command_config.argv),
                "exit" => rsh_exit(&command_config.argv),
                "pwd" => rsh_pwd(&command_config.argv),
                "which" => rsh_which(&command_config.argv, &env_map),
                _ => rsh_launch(&command_config, env_vars),
            };
        }
        match result {
            Ok(status) => Ok(status),
            Err(error_type) => {
                println!("{:?}", error_type);
                Err(error_type)
            }
        }
    }

    fn rsh_launch(command_configs: &CommandConfig, env_vars: &[&CStr]) -> anyhow::Result<Status> {
        match fork() {
            Ok(ForkResult::Parent { child, .. }) => {
                // parent
                // println!("I'm parent. Child PID is {}", child);
                loop {
                    let waitresult = waitpid(child, Some(WaitPidFlag::WUNTRACED));
                    match waitresult.unwrap() {
                        WaitStatus::Exited(..) | WaitStatus::Signaled(..) => {
                            break Ok(Status::Success)
                        }
                        _ => (),
                    }
                }
            }
            Ok(ForkResult::Child) => {
                // child
                execvpe(
                    &command_configs.command,
                    &command_configs.argv[..]
                        .iter()
                        .map(AsRef::as_ref)
                        .collect::<Vec<&CStr>>()
                        .as_ref(),
                    env_vars,
                )
                .with_context(|| format!("{:?}: command not found", command_configs.command))?;
                Ok(Status::Exit)
            }
            Err(err) => Err(err.into()),
        }
    }

    fn rsh_cd(args: &Vec<CString>, env_map: &HashMap<CString, CString>) -> anyhow::Result<Status> {
        if args.len() > 1 {
            let destination = args[1].as_c_str();
            chdir(destination).with_context(|| {
                format!("You wanted to chdir to {:?} but that failed", destination)
            })?;
        } else {
            chdir(
                env_map
                    .get(&CString::new("HOME").unwrap())
                    .context("You used cd without arguments, but HOME is not specified in the env")?
                    .as_c_str(),
            )
            .map(|_| Status::Success)?;
        }
        Ok(Status::Success)
    }

    fn rsh_help(_args: &Vec<CString>) -> anyhow::Result<Status> {
        println!("Woody's re-implemantation of lsh, written in Rust.",);
        println!("Type command and arguments, and hit enter.",);
        // println!("The following commands are built in:",);
        Ok(Status::Success)
    }

    fn rsh_exit(_args: &Vec<CString>) -> anyhow::Result<Status> {
        Ok(Status::Exit)
    }

    fn rsh_pwd(_args: &Vec<CString>) -> anyhow::Result<Status> {
        println!("{:?}", getcwd()?);
        Ok(Status::Success)
    }

    fn rsh_which(
        argv: &Vec<CString>,
        env_map: &HashMap<CString, CString>,
    ) -> anyhow::Result<Status> {
        let paths = &(env_map
            .get(&CString::new("PATH").unwrap())
            .expect("PATH is not specified in the env!"))
        .to_str()
        .unwrap()
        .split(":")
        .collect::<Vec<&str>>();
        let available_binaries = find_files_in(paths);
        match available_binaries.get(&argv[1]) {
            Some(command) => println!("{:?}", command),
            None => println!("Command {:?} not found.", argv[1]),
        }
        Ok(Status::Success)
    }

    fn find_files_in(paths: &Vec<&str>) -> HashMap<CString, PathBuf> {
        let mut found_files = HashMap::new();
        for path in paths {
            for entry in fs::read_dir(Path::new(path)).expect(&format!("path {} not found!", path))
            {
                let path = entry.unwrap().path();
                let file_name = path.file_name().unwrap();
                found_files.insert(
                    CString::new(file_name.to_str().unwrap().to_owned()).unwrap(),
                    path,
                );
            }
        }
        found_files
    }
}
