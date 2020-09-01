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
    use std::str::SplitAsciiWhitespace;

    pub fn main_loop(env_vars: &[&CStr]) {
        loop {
            print!("> ");
            io::stdout().flush().unwrap(); // make sure "> " above is printed
            let line = read_line();
            let command_configs = split_out_command(line.split_ascii_whitespace());
            if let Some(command_configs) = command_configs {
                match execute(command_configs, env_vars) {
                    Ok(Status::Exit) => break,
                    _ => (),
                }
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct CommandConfig<'a> {
        pub command: CString,
        pub argv: Vec<CString>,
        pub successive_command: Option<SuccessiveCommand<'a>>,
    }

    #[derive(Debug, Clone)]
    pub struct SuccessiveCommand<'a> {
        controlflow: ControlFlow,
        commands: SplitAsciiWhitespace<'a>,
    }

    #[derive(Debug, PartialEq, Eq, Clone)]
    enum ControlFlow {
        PIPE,
        OR,
        AND,
        SIMUL,
        BOTH,
    }

    fn read_line() -> String {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        return input;
    }

    fn split_out_command(mut inputs: SplitAsciiWhitespace) -> Option<CommandConfig> {
        if let Some(command) = inputs.next() {
            let mut argv =
                vec![CString::new(command).expect("Failed to convert your command to CString")];
            while let Some(arg) = inputs.next() {
                if ["|", "||", "&", "&&", ";"].contains(&arg) {
                    let controlflow = match arg {
                        "|" => ControlFlow::PIPE,
                        "||" => ControlFlow::OR,
                        "&&" => ControlFlow::AND,
                        "&" => ControlFlow::SIMUL,
                        ";" => ControlFlow::BOTH,
                        _ => panic!(),
                    };
                    return Some(CommandConfig {
                        command: CString::new(command).unwrap(),
                        argv,
                        successive_command: Some(SuccessiveCommand {
                            controlflow,
                            commands: inputs,
                        }),
                    });
                }
                argv.push(CString::new(arg).expect("Failed to convert your arguments to CString"));
            }
            return Some(CommandConfig {
                command: CString::new(command).unwrap(),
                argv,
                successive_command: None,
            });
        } else {
            return None;
        }
    }

    #[derive(Debug, PartialEq, Eq)]
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

    fn execute(command_config: CommandConfig, env_vars: &[&CStr]) -> anyhow::Result<Status> {
        let env_map = obtain_env_val_map(env_vars);
        let mut result: anyhow::Result<Status> =
            Err(anyhow::Error::new(nix::Error::UnsupportedOperation));

        result = match command_config.command.to_str().unwrap() {
            "cd" => rsh_cd(&command_config.argv, &env_map),
            "help" => rsh_help(&command_config.argv),
            "exit" => rsh_exit(&command_config.argv),
            "pwd" => rsh_pwd(&command_config.argv),
            "which" => rsh_which(&command_config.argv, &env_map),
            _ => rsh_launch(&command_config, env_vars),
        };

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
                    command_configs.command.as_ref(),
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

    mod tests {
        use super::*;
        #[test]
        fn parse_commands() {
            let command_answer_pairs = vec![
                ("ls some_file", (vec!["ls", "some_file"], None)),
                ("pwd some_file", (vec!["pwd", "some_file"], None)),
                (
                    "cat some_file | less",
                    (
                        vec!["cat", "some_file"],
                        Some(SuccessiveCommand {
                            controlflow: ControlFlow::PIPE,
                            commands: "less".split_ascii_whitespace(),
                        }),
                    ),
                ),
                (
                    "cat some_file ||    ",
                    (
                        vec!["cat", "some_file"],
                        Some(SuccessiveCommand {
                            controlflow: ControlFlow::OR,
                            commands: "".split_ascii_whitespace(),
                        }),
                    ),
                ),
                (
                    "cat some_file > out.txt",
                    (vec!["cat", "some_file", ">", "out.txt"], None),
                ),
                (
                    "cat some_file > out.txt && ls || cat out.txt",
                    (
                        vec!["cat", "some_file", ">", "out.txt"],
                        Some(SuccessiveCommand {
                            controlflow: ControlFlow::AND,
                            commands: "ls || cat out.txt".split_ascii_whitespace(),
                        }),
                    ),
                ),
                (
                    "cat some_file > out.txt & ls || cat   out.txt",
                    (
                        vec!["cat", "some_file", ">", "out.txt"],
                        Some(SuccessiveCommand {
                            controlflow: ControlFlow::SIMUL,
                            commands: "ls || cat out.txt".split_ascii_whitespace(),
                        }),
                    ),
                ),
                (
                    "cat some_file > out.txt ; ls || cat out.txt",
                    (
                        vec!["cat", "some_file", ">", "out.txt"],
                        Some(SuccessiveCommand {
                            controlflow: ControlFlow::BOTH,
                            commands: "ls || cat out.txt".split_ascii_whitespace(),
                        }),
                    ),
                ),
            ];
            for (command, answer) in command_answer_pairs.iter() {
                let command_config =
                    split_out_command(command.split_ascii_whitespace().to_owned().into());
                assert_eq!(
                    command_config.clone().unwrap().command,
                    CString::new(answer.0[0]).unwrap(),
                );
                assert_eq!(
                    command_config.unwrap().argv,
                    answer
                        .0
                        .iter()
                        .map(|x| CString::new(*x).unwrap())
                        .collect::<Vec<_>>()
                );
            }
        }

        #[test]
        fn correct_cd() {
            let env_path = CString::new("PATH=/bin:/usr/bin").unwrap();
            let env_home = CString::new("HOME=/home/woody").unwrap();
            let env_vars = &[env_path.as_ref(), env_home.as_ref()];
            let env_map = obtain_env_val_map(env_vars);
            assert_eq!(rsh_cd(&vec![], &env_map).unwrap(), Status::Success);
        }

        // When HOME doesn't exist...
        #[test]
        #[should_panic(
            expected = "called `Result::unwrap()` on an `Err` value: You used cd without arguments, but HOME is not specified in the env"
        )]
        fn panic_cd() {
            let env_path = CString::new("PATH=/bin:/usr/bin").unwrap();
            let env_home = CString::new("aOME=/home/woody").unwrap();
            let env_vars = &[env_path.as_ref(), env_home.as_ref()];
            let env_map = obtain_env_val_map(env_vars);
            (rsh_cd(&vec![], &env_map).unwrap());
        }
    }
}
