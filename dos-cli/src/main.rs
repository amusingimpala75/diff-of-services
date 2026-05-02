use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Shell, generate_to};
use dos_lib::{document::Document, open_connection_file};
use rusqlite::Connection;
use time::{OffsetDateTime, macros::format_description};

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
    Update { name: String, path: PathBuf },
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

fn main() -> Result<()> {
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
                    fs::create_dir(&out).with_context(|| {
                        format!("creating {} for manpages", out.to_string_lossy())
                    })?;
                    // Generate the manpages
                    clap_mangen::generate_to(Cli::command(), &out).with_context(|| {
                        format!("generating manpages to {}", out.to_string_lossy())
                    })?;
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
                    fs::create_dir(&out).with_context(|| {
                        format!("creating {} for shell completions", out.to_string_lossy())
                    })?;
                    for &shell in Shell::value_variants() {
                        generate_to(shell, &mut Cli::command(), "dos", &out).with_context(
                            || {
                                format!(
                                    "generating shell completion for {} into {}",
                                    shell,
                                    out.to_string_lossy()
                                )
                            },
                        )?;
                    }
                }
            },
        },
        Command::List { document, revision } => {
            // List the documents / document's revision / revision's data
            let conn = open_connection_file().context("opening database")?;

            if let Some(document) = document {
                if let Some(revision) = revision {
                    list_revision(document, revision, &conn)
                } else {
                    list_revisions(document, &conn)
                }
            } else {
                list_documents(&conn)
            }
            .context("running listing")?;
        }
        Command::Create { name, text } => {
            // Ensure the path exists before we start creating the document
            if let Some(path) = &text
                && !path.exists()
            {
                Err(anyhow!("{} does not exist", path.to_string_lossy()))?;
            }

            let conn = open_connection_file().context("opening database")?;
            // Create a new document
            let mut doc = Document::insert(&name, &conn)
                .with_context(|| format!("inserting document named {name}"))?;

            // If the user provided text of the new document
            // and the file exists, then add that as a new revision
            // on the created document
            if let Some(p) = &text {
                // Fetch the requested file
                let contents = fs::read_to_string(p).with_context(|| {
                    format!(
                        "reading revision's text contents at {}",
                        p.to_string_lossy()
                    )
                })?;

                // Add a new revision upon the created document
                doc.add_new_revision(&contents, &conn)
                    .context("updating with new revision")?;
            }
            println!("Created document named {name}");
        }
        Command::Update { name, path } => {
            let conn = open_connection_file().context("opening database")?;

            // Ensure document exists
            let mut doc = Document::from_name(&name, &conn)
                .with_context(|| format!("fetching document named {name}"))?;
            // Ensure new revision exists
            let contents = fs::read_to_string(&path).with_context(|| {
                format!("reading update text contents at {}", path.to_string_lossy())
            })?;

            // Add new revision to the document
            doc.add_new_revision(&contents, &conn)
                .context("updating document with new revision")?;
            println!(
                "Added revision {} to document {name}",
                path.to_string_lossy()
            );
        }
    };

    Ok(())
}

/// Print to stdout all of the documents with some corresponding metadata
fn list_documents(conn: &Connection) -> Result<()> {
    // Get all documents
    let docs = Document::get_all(conn).context("Fetching all documents")?;

    // Iterate over then all, printing name, number of revisions,
    // and localized time of the most recent revision
    for doc in docs.iter() {
        let name = doc.name();
        let num_revisions = doc
            .count_revisions(conn)
            .with_context(|| format!("Counting revisions for {name}"))?;
        match doc.last_updated(conn) {
            Some(t) => {
                let t = t.with_context(|| format!("fetching last updated time for {name}"))?;
                let formatted = format_local_time(t);
                println!("{name}: {num_revisions} revisions, last updated {formatted}");
            }
            None => println!("{name}: {num_revisions} revisions, last updated never"),
        };
    }

    let count = docs.len();
    println!("{count} documents");

    Ok(())
}

/// List all of the revisions for the document with the provided name
fn list_revisions(name: String, conn: &Connection) -> Result<()> {
    // Fetch document by name. Could fail if they gave a bad name
    let document = Document::from_name(&name, conn)
        .with_context(|| format!("fetching document with name {name}"))?;
    // Get the revisions of the document.
    let revisions = document
        .revisions(conn)
        .with_context(|| format!("fetching revisions for {name}"))?;

    // List the revisions with indices
    for (idx, revision) in revisions.iter().enumerate().rev() {
        let formatted = format_local_time(revision.added_on());
        println!("Revision #{idx} created on {formatted}")
    }

    Ok(())
}

/// List the information for a given document's revision
fn list_revision(name: String, nth: u32, conn: &Connection) -> Result<()> {
    // Ensure document exists
    let document = Document::from_name(&name, conn)
        .with_context(|| format!("fetching document named {name}"))?;
    // Ensure revision exists
    let revision = document
        .nth_revision(nth, conn)
        .with_context(|| format!("fetching revision {nth} for {name}"))?;
    // Load the revision's text
    let revision = revision
        .load_text(conn)
        .with_context(|| format!("fetching text for {nth} revision of {name}"))?;

    // Print metadata
    let name = document.name();
    println!("Document: {name}");
    println!("Revision: {nth}");
    let formatted = format_local_time(revision.added_on());
    println!("Added: {formatted}");
    println!("Text:");
    let text = revision
        .text
        .ok_or(anyhow!("revision's text was missing"))?;
    println!("{text}");

    Ok(())
}

/// Formats the given time as a string. Shows date/time down to the minute
fn format_local_time(time: OffsetDateTime) -> String {
    let formatter = format_description!("[year]-[month]-[day] at [hour]:[minute]");
    time.format(formatter).unwrap()
}
