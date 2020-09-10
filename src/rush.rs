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

const STDOUT_FILENO: i32 = 1;
const STDIN_FILENO: i32 = 0;

pub mod util;

pub fn main_loop(env_vars: &[&CStr]) {
    loop {
        print!("> ");
        io::stdout().flush().unwrap(); // make sure "> " above is printed
        let line = read_line();
        let command_configs = split_out_command(line.split_ascii_whitespace());
        if let Some(command_configs) = command_configs {
            match util::execute(command_configs, env_vars) {
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
pub enum Status {
    Success,
    Exit,
}

mod tests {
    use super::*;
    #[test]
    fn parse_commands() {
        let command_answer_pairs = vec![
            ("ls some_file", (vec!["ls", "some_file"], None)),
            ("pwd some_file", (vec!["pwd", "some_file"], None)),
            ("echo wow", (vec!["echo", "wow"], None)),
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
        let env_map = util::obtain_env_val_map(env_vars);
        assert_eq!(util::rsh_cd(&vec![], &env_map).unwrap(), Status::Success);
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
        let env_map = util::obtain_env_val_map(env_vars);
        (util::rsh_cd(&vec![], &env_map).unwrap());
    }
}
