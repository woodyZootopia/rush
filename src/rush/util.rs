use super::*;
pub fn obtain_env_val_map(env_vars: &[&CStr]) -> HashMap<CString, CString> {
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

pub(crate) fn execute(command_config: CommandConfig, env_vars: &[&CStr]) -> anyhow::Result<Status> {
    let env_map = obtain_env_val_map(env_vars);
    let mut result: anyhow::Result<Status> =
        Err(anyhow::Error::new(nix::Error::UnsupportedOperation));

    if let Some(succ) = command_config.successive_command {
        match succ.controlflow {
            ControlFlow::PIPE => {
                let fd = pipe2(nix::fcntl::OFlag::O_CLOEXEC)?;
                match fork() {
                    Ok(ForkResult::Parent { child: child1 }) => match fork() {
                        Ok(ForkResult::Parent { child: child2 }) => {
                            loop {
                                let waitresult = waitpid(child2, Some(WaitPidFlag::WUNTRACED));
                                match waitresult.unwrap() {
                                    WaitStatus::Exited(..) | WaitStatus::Signaled(..) => {
                                        break;
                                    }
                                    _ => (),
                                }
                            }
                            loop {
                                let waitresult = waitpid(child1, Some(WaitPidFlag::WUNTRACED));
                                match waitresult.unwrap() {
                                    WaitStatus::Exited(..) | WaitStatus::Signaled(..) => {
                                        break Ok(Status::Success)
                                    }
                                    _ => (),
                                }
                            }
                        }

                        Ok(ForkResult::Child) => {
                            // child 2
                            // only write to successive_command
                            close(fd.0)?;
                            dup2(fd.1, STDOUT_FILENO)?;
                            close(fd.1)?;
                            execute(
                                CommandConfig {
                                    successive_command: None,
                                    ..command_config
                                },
                                env_vars,
                            )?;
                            Ok(Status::Exit)
                        }
                        Err(err) => Err(err.into()),
                    },

                    Ok(ForkResult::Child) => {
                        // child 1
                        // only read from previous command
                        close(fd.1)?;
                        dup2(fd.0, STDIN_FILENO)?;
                        close(fd.0)?;
                        let command_config = split_out_command(succ.commands).unwrap();
                        execute(command_config, env_vars)?;
                        Ok(Status::Exit)
                    }
                    Err(err) => Err(err.into()),
                }
            }
            _ => Ok(Status::Success),
        }
    } else {
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
}

fn rsh_launch(command_config: &CommandConfig, env_vars: &[&CStr]) -> anyhow::Result<Status> {
    match fork() {
        Ok(ForkResult::Parent { child, .. }) => {
            // parent
            // println!("I'm parent. Child PID is {}", child);
            loop {
                let waitresult = waitpid(child, Some(WaitPidFlag::WUNTRACED));
                match waitresult.unwrap() {
                    WaitStatus::Exited(..) | WaitStatus::Signaled(..) => break Ok(Status::Success),
                    _ => (),
                }
            }
        }
        Ok(ForkResult::Child) => {
            // child
            execvpe(
                command_config.command.as_ref(),
                &command_config.argv[..]
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<&CStr>>()
                    .as_ref(),
                env_vars,
            )
            .with_context(|| format!("{:?}: command not found", command_config.command))?;
            Ok(Status::Exit)
        }
        Err(err) => Err(err.into()),
    }
}

pub(crate) fn rsh_cd(
    args: &Vec<CString>,
    env_map: &HashMap<CString, CString>,
) -> anyhow::Result<Status> {
    if args.len() > 1 {
        let destination = args[1].as_c_str();
        chdir(destination)
            .with_context(|| format!("You wanted to chdir to {:?} but that failed", destination))?;
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

fn rsh_which(argv: &Vec<CString>, env_map: &HashMap<CString, CString>) -> anyhow::Result<Status> {
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
        for entry in fs::read_dir(Path::new(path)).expect(&format!("path {} not found!", path)) {
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
