use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command()]
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
                        None => PathBuf::from("./completion"),
                    };

                    let s = out.display();

                    println!("generating shell completions to {s}")
                }
            },
        },
    };
}
