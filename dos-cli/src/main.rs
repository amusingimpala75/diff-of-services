use std::{fs, path::PathBuf};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate_to, Shell};
use clap_mangen;
use dos_lib::{directories, document::Document, revision::Revision};
use rusqlite::Connection;
use time::{macros::format_description, OffsetDateTime};

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
    List {
        document: Option<String>,
        revision: Option<u32>,
    },
    Create {
        name: String,
        text: Option<PathBuf>,
    },
    Update {
        name: String,
        path: PathBuf,
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
        Command::List { document, revision } => {
            let conn = match open_connection() {
                Some(c) => c,
                None => {
                    eprintln!("error opening database");
                    return;
                }
            };

            if let Some(document) = document {
                if let Some(revision) = revision {
                    list_revision(document, revision, &conn);
                } else {
                    list_revisions(document, &conn);
                }
            } else {
                list_documents(&conn);
            }
        }
        Command::Create { name, text } => {
            // Ensure the path exists before we start creating the document
            if let Some(path) = &text {
                if !path.exists() {
                    let missing = path.display();
                    eprintln!("{missing} does not exist");
                    return;
                }
            }

            let conn = match open_connection() {
                Some(c) => c,
                None => {
                    eprintln!("error opening database");
                    return;
                }
            };

            let doc = match Document::insert(&name, &conn) {
                Ok(d) => d,
                Err(_) => {
                    eprintln!("Could not add document");
                    return;
                }
            };

            if let Some(p) = &text {
                let contents = match fs::read_to_string(p) {
                    Ok(s) => s,
                    Err(_) => {
                        let name = p.display();
                        eprintln!("Could not open file {name}");
                        return;
                    }
                };

                if doc.add_new_revision(contents, &conn).is_ok() {
                    println!("Added document {name}")
                } else {
                    eprintln!("Could not add new revision to document")
                };
            }
        }
        Command::Update { name, path } => {
            let conn = match open_connection() {
                Some(c) => c,
                None => {
                    eprintln!("Could not open database");
                    return;
                }
            };

            // Ensure document exists
            let doc = match Document::from_name(&name, &conn) {
                Ok(d) => d,
                Err(_) => {
                    eprintln!("Could not fetch document with name {name}");
                    return;
                }
            };

            // Ensure new revision exists
            let contents = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => {
                    let disp = path.display();
                    eprintln!("Could not read revision at {disp}");
                    return;
                }
            };

            let res = doc.add_new_revision(contents, &conn);
            if res.is_err() {
                eprintln!("Could not add revision to document {name}");
            } else {
                let disp = path.display();
                println!("Added revision {disp} to document {name}");
            }
        }
    };
}

fn list_documents(conn: &Connection) {
    let docs = Document::get_all(&conn);

    if let Err(_) = docs {
        eprintln!("Could not retrieve documents from database");
        return;
    }

    let docs = docs.unwrap();

    for doc in docs.iter() {
        let name = doc.name();
        let num_revisions = match doc.count_revisions(&conn) {
            Ok(n) => n,
            Err(_) => {
                eprintln!("Could not fetch document's revisions");
                continue;
            }
        };
        match doc.last_updated(&conn) {
            Some(t) => match t {
                Ok(t) => {
                    let formatted = format_local_time(t);
                    println!("{name}: {num_revisions} revisions, last updated {formatted}")
                }
                Err(_) => {
                    eprintln!("Could not fetch document's latest update time");
                }
            },
            None => println!("{name}: {num_revisions} revisions, last updated never"),
        };
    }

    let count = docs.len();
    println!("{count} documents");
}

fn list_revisions(document: String, conn: &Connection) {
    let document = match Document::from_name(&document, &conn) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("Could not find document named {document}");
            return;
        }
    };

    let revisions = match document.revisions(&conn) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("Could not fetch the revisions");
            return;
        }
    };

    for (idx, revision) in revisions.iter().enumerate().rev() {
        let formatted = format_local_time(revision.added_on());
        println!("Revision #{idx} created on {formatted}")
    }
}

fn list_revision(document: String, revision: u32, conn: &Connection) {
    let idx = revision;

    let document = match Document::from_name(&document, &conn) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("Could not find document named {document}");
            return;
        }
    };

    let revision = match document.nth_revision(revision, conn) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("Could not find revision for index named {revision}");
            return;
        }
    };

    let revision = match revision.load_text(conn) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("Could not fetch revision {idx}'s text");
            return;
        }
    };

    let name = document.name();
    println!("Document: {name}");
    println!("Revision: {idx}");
    let formatted = format_local_time(revision.added_on());
    println!("Added: {formatted}");
    println!("Text:");
    let text = revision.text.unwrap();
    println!("{text}");
}

fn format_local_time(time: OffsetDateTime) -> String {
    let formatter = format_description!("[year]-[month]-[day] at [hour]:[minute]");
    time.format(formatter).unwrap()
}

fn open_connection() -> Option<Connection> {
    fs::create_dir_all(directories::data_dir()).ok()?;
    let conn = Connection::open(directories::data_dir().join("db.sqlite3")).ok()?;
    Document::ensure_table_exists(&conn).ok()?;
    Revision::ensure_table_exists(&conn).ok()?;
    Some(conn)
}
