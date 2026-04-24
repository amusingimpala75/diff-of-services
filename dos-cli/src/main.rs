use std::{fs, path::PathBuf};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate_to, Shell};

#[derive(Parser)]
#[command(name = "dos", author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Util {
        #[command(subcommand)]
        action: UtilAction,
    },
}

#[derive(Subcommand)]
enum UtilAction {
    Generate {
        #[arg(value_enum)]
        kind: Generated,
        out: Option<PathBuf>,
    },
}

#[derive(Clone, ValueEnum)]
enum Generated {
    Manpages,
    ShellCompletions,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Util { action } => match action {
            UtilAction::Generate { kind, out } => match kind {
                Generated::Manpages => {
                    let out = match out {
                        Some(path) => path,
                        None => PathBuf::from("./man"),
                    };

                    let s = out.display();

                    println!("generating manpages to {s}");
                }
                Generated::ShellCompletions => {
                    let out = match out {
                        Some(path) => path,
                        None => PathBuf::from("./completions"),
                    };

                    let s = out.display();

                    println!("generating shell completions to {s}");

                    let err =
                        Shell::value_variants()
                            .iter()
                            .fold(fs::create_dir(&out), |err, &sh| match err {
                                Ok(()) => {
                                    generate_to(sh, &mut Cli::command(), "dos", &out).map(|_| ())
                                }
                                Err(_) => err,
                            });

                    match err {
                        Err(_) => eprintln!("Error writing shell completions"),
                        Ok(()) => {}
                    }
                }
            },
        },
    };
}
