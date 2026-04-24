use std::{fs, path::PathBuf};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate_to, Shell};
use clap_mangen;

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

                    let err = fs::create_dir(&out)
                        .and_then(|_| clap_mangen::generate_to(Cli::command(), &out));
                    match err {
                        Ok(()) => {}
                        Err(err) => eprintln!("Error writing manpages: {err}"),
                    }
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
                            .fold(fs::create_dir(&out), |err, &sh| {
                                err.and_then(|_| {
                                    generate_to(sh, &mut Cli::command(), "dos", &out).map(|_| ())
                                })
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
