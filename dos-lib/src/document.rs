use rusqlite::{named_params, Connection, Result};
use time::OffsetDateTime;

use crate::revision::Revision;

/// # Document
///
/// The document type keeps track of the name and latest revision of this doc.
/// It does not itself keep track of the contents of the revision, or last
/// modified state. That is left to the Revision type, meaning such calls
/// go out to the local sqlite database.
pub struct Document {
    /// Id of the document, would do u64 if sqlite let me
    id: u32,
    /// Name of the document. Can contain any spaces, for now is what the
    /// user will enter for selection. May at some pointer add a user-friendly
    /// id field, so that the display name could be Apple Terms of Service
    /// but it could be referred to as apple_tos or something
    name: String,
    /// Reference to the most recent revision of the document. Probably doesn't
    /// *need* to be an option, but we currently allow the user to create the
    /// document without a revision yet. Otherwise it really should be non-null.
    latest_revision: Option<u32>,
}

impl Document {
    /// Make sure the documents table exists in the connection provided.
    /// All fields of the document field are part of the table, with the
    /// id as the key. Seeing as the name is required, it is listed as so.
    /// Returns a result indicating the success of the operation.
    pub fn ensure_table_exists(conn: &Connection) -> Result<()> {
        // Create the table only if it doesn't yet exist, this allows
        // us to unconditionally call this.
        conn.execute(
            "CREATE TABLE IF NOT EXISTS documents (
               id              INTEGER PRIMARY KEY,
               name            TEXT NOT NULL,
               latest_revision INTEGER
             )",
            (),
        )?;
        Ok(())
    }

    /// Selects all documents from the database
    pub fn get_all(conn: &Connection) -> Result<Vec<Document>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, latest_revision
             FROM documents",
        )?;
        stmt.query_map([], |row| {
            Ok(Document {
                id: row.get("id")?,
                name: row.get("name")?,
                latest_revision: row.get("latest_revision")?,
            })
        })?
        .collect()
    }

    /// Creates a new document with the given name, returning
    /// the document that has been created
    pub fn insert(name: &str, conn: &Connection) -> Result<Document> {
        let id = conn.query_one(
            "INSERT INTO documents (name)
             VALUES (:name)
             RETURNING id",
            named_params! {
                ":name": &name,
            },
            |row| row.get("id"),
        )?;
        Ok(Document {
            id,
            name: name.to_string(),
            latest_revision: None,
        })
    }

    /// Gets an existing document from the database with the given
    /// name. Currently just doing a direct string comparison, so
    /// entring the name could be a bit jank.
    pub fn from_name(name: &str, conn: &Connection) -> Result<Document> {
        conn.query_one(
            "SELECT id, latest_revision FROM documents WHERE name = :name",
            named_params! { ":name": name },
            |row| {
                Ok(Document {
                    id: row.get("id")?,
                    name: name.to_string(),
                    latest_revision: row.get("latest_revision")?,
                })
            },
        )
    }

    /// Adds a new revision with the requested text and updates the latest
    /// rev to refer to it.
    pub fn add_new_revision(&mut self, text: &str, conn: &Connection) -> Result<Revision> {
        let rev = Revision::insert(self.id, text, conn)?;
        conn.execute(
            "UPDATE documents
             SET latest_revision = :rev
             WHERE id = :id",
            named_params! {
                ":id": self.id,
                ":rev": rev.id
            },
        )?;
        self.latest_revision = Some(rev.id);
        Ok(rev)
    }

    /// Gets the number of revision currently for the document
    pub fn count_revisions(&self, conn: &Connection) -> Result<u32> {
        conn.query_one(
            "SELECT COUNT(*) FROM revisions WHERE document = :id",
            named_params! {
                ":id": self.id,
            },
            |row| row.get(0),
        )
    }

    /// Fetches the time relative to the current timezone of the latest
    /// revision if present
    pub fn last_updated(&self, conn: &Connection) -> Option<Result<OffsetDateTime>> {
        self.latest_revision
            .map(|rev| Revision::from_id(rev, conn).map(|rev| rev.added_on()))
    }

    /// Gets the list of the revision of this current document
    pub fn revisions(&self, conn: &Connection) -> Result<Vec<Revision>> {
        Revision::get_all(self.id, conn)
    }

    /// Gets the nth revision of this document
    pub fn nth_revision(&self, idx: u32, conn: &Connection) -> Result<Revision> {
        Revision::get_nth(idx, self.id, conn)
    }

    /// Get our name
    pub fn name(&self) -> &str {
        &self.name
    }
}
