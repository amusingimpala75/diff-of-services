use std::{fs, path::PathBuf};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate_to, Shell};
use dos_lib::{document::Document, open_connection_file};
use rusqlite::Connection;
use time::{macros::format_description, OffsetDateTime};

/// Command line interface to the diff of services tool
#[derive(Parser)]
#[command(name = "dos", author, version, about, long_about = None)]
struct Cli {
    /// Every invocation calls a command
    #[command(subcommand)]
    command: Command,
}

/// Various top level commands
#[derive(Subcommand)]
enum Command {
    /// # Utility commands
    ///
    /// At this point, mainly for generating files
    /// such as shell completion and man pages
    Util {
        /// Which utility action should be performed
        #[command(subcommand)]
        action: UtilAction,
    },
    /// # List Commands
    ///
    /// The List commands list various aspects of the data that
    /// currently resides in the database. What action listing
    /// is performed is based on the presence or absence of
    /// target document or revision. If there are no documents
    /// listed, we simply list of the collection of documents,
    /// showing the number of revisions of the document and when
    /// it was last updated. If just a document but no revision
    /// is found, then we list the revisions and their added date
    /// for the requested document. If both the document and
    /// revision are specified, then we show the metadata of the
    /// document and revision, as well as the text of the revision.
    List {
        /// The document that is being queried
        document: Option<String>,
        /// The revision that is being queried
        revision: Option<u32>,
    },
    /// # Create Command
    ///
    /// The create command creates a document with the given name.
    /// If the text argument is passed, then a default revision is
    /// created with the text contents being the contents of the file
    /// specified by the text parameter.
    Create {
        /// Name of the new document to be created
        name: String,
        /// Path to a file containing the text of the first revision
        text: Option<PathBuf>,
    },
    /// # Update command
    ///
    /// Updates the document named `name' by adding a new revision
    /// with a body as supplied by the file specified by `path'
    Update {
        name: String,
        path: PathBuf,
    },
}

/// What utility action is to be performed
#[derive(Subcommand)]
enum UtilAction {
    /// Generate some system utility files. Currently supports generating
    /// shell completions (bash/zsh/fish) and manpages
    Generate {
        /// Type of thing to be generated
        #[arg(value_enum)]
        kind: Generated,
        /// Path to the directory into which to put the
        /// generated files. Has defaults as a new subdirectory
        /// of the current directory
        out: Option<PathBuf>,
    },
}

/// What type of generation is to occur
#[derive(Clone, ValueEnum)]
enum Generated {
    /// Generate the manpages
    Manpages,
    /// Generate the shell completions
    ShellCompletions,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Util { action } => match action {
            UtilAction::Generate { kind, out } => match kind {
                Generated::Manpages => {
                    // Generate manpages, default at ./main
                    let out = match out {
                        Some(path) => path,
                        None => PathBuf::from("./man"),
                    };

                    let s = out.display();

                    println!("generating manpages to {s}");

                    // Create directory (ensures that it does not already exist)
                    // prior to generating manpages
                    let err = fs::create_dir(&out)
                        .and_then(|_| clap_mangen::generate_to(Cli::command(), &out));
                    match err {
                        Ok(()) => {}
                        Err(err) => eprintln!("Error writing manpages: {err}"),
                    }
                }
                Generated::ShellCompletions => {
                    // Generate shell completions, into ./completions
                    let out = match out {
                        Some(path) => path,
                        None => PathBuf::from("./completions"),
                    };

                    let s = out.display();

                    println!("generating shell completions to {s}");

                    // Generate completions, ensuring directory exists.
                    // If any error occurs in the process, skip the rest
                    // of the actions / completions.
                    let err =
                        Shell::value_variants()
                            .iter()
                            .fold(fs::create_dir(&out), |err, &sh| {
                                err.and_then(|_| {
                                    generate_to(sh, &mut Cli::command(), "dos", &out).map(|_| ())
                                })
                            });

                    if err.is_err() { eprintln!("Error writing shell completions") }
                }
            },
        },
        Command::List { document, revision } => {
            // List the documents / document's revision / revision's data
            let conn = match open_connection_file() {
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
            if let Some(path) = &text
                && !path.exists() {
                    let missing = path.display();
                    eprintln!("{missing} does not exist");
                    return;
                }

            let conn = match open_connection_file() {
                Some(c) => c,
                None => {
                    eprintln!("error opening database");
                    return;
                }
            };

            // Create a new document
            let mut doc = match Document::insert(&name, &conn) {
                Ok(d) => d,
                Err(_) => {
                    eprintln!("Could not add document");
                    return;
                }
            };

            // If the user provided text of the new document
            // and the file exists, then add that as a new revision
            // on the created document
            if let Some(p) = &text {
                // Fetch the requested file
                let contents = match fs::read_to_string(p) {
                    Ok(s) => s,
                    Err(_) => {
                        let name = p.display();
                        eprintln!("Could not open file {name}");
                        return;
                    }
                };

                // Add a new revision upon the created document
                if doc.add_new_revision(&contents, &conn).is_ok() {
                    println!("Added document {name}")
                } else {
                    eprintln!("Could not add new revision to document")
                };
            }
        }
        Command::Update { name, path } => {
            let conn = match open_connection_file() {
                Some(c) => c,
                None => {
                    eprintln!("Could not open database");
                    return;
                }
            };

            // Ensure document exists
            let mut doc = match Document::from_name(&name, &conn) {
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

            // Add new revision to the document
            let res = doc.add_new_revision(&contents, &conn);
            if res.is_err() {
                eprintln!("Could not add revision to document {name}");
            } else {
                let disp = path.display();
                println!("Added revision {disp} to document {name}");
            }
        }
    };
}

/// Print to stdout all of the documents with some corresponding metadata
fn list_documents(conn: &Connection) {
    // Get all documents
    let docs = Document::get_all(conn);

    if docs.is_err() {
        eprintln!("Could not retrieve documents from database");
        return;
    }

    let docs = docs.unwrap();

    // Iterate over then all, printing name, number of revisions,
    // and localized time of the most recent revision
    for doc in docs.iter() {
        let name = doc.name();
        let num_revisions = match doc.count_revisions(conn) {
            Ok(n) => n,
            Err(_) => {
                eprintln!("Could not fetch document's revisions");
                continue;
            }
        };
        match doc.last_updated(conn) {
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

/// List all of the revisions for the document with the provided name
fn list_revisions(document: String, conn: &Connection) {
    // Fetch document by name. Could fail if they gave a bad name
    let document = match Document::from_name(&document, conn) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("Could not find document named {document}");
            return;
        }
    };

    // Get the revisions of the document.
    let revisions = match document.revisions(conn) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("Could not fetch the revisions");
            return;
        }
    };

    // List the revisions with indices
    for (idx, revision) in revisions.iter().enumerate().rev() {
        let formatted = format_local_time(revision.added_on());
        println!("Revision #{idx} created on {formatted}")
    }
}

/// List the information for a given document's revision
fn list_revision(document: String, revision: u32, conn: &Connection) {
    let idx = revision;

    // Ensure document exists
    let document = match Document::from_name(&document, conn) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("Could not find document named {document}");
            return;
        }
    };

    // Ensure revision exists
    let revision = match document.nth_revision(revision, conn) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("Could not find revision for index named {revision}");
            return;
        }
    };

    // Load the revision's text
    let revision = match revision.load_text(conn) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("Could not fetch revision {idx}'s text");
            return;
        }
    };

    // Print metadata
    let name = document.name();
    println!("Document: {name}");
    println!("Revision: {idx}");
    let formatted = format_local_time(revision.added_on());
    println!("Added: {formatted}");
    println!("Text:");
    let text = revision.text.unwrap();
    println!("{text}");
}

/// Formats the given time as a string. Shows date/time down to the minute
fn format_local_time(time: OffsetDateTime) -> String {
    let formatter = format_description!("[year]-[month]-[day] at [hour]:[minute]");
    time.format(formatter).unwrap()
}
