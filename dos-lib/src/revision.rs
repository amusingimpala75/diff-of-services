use rusqlite::{named_params, Connection, Result, Row};
use time::{OffsetDateTime, UtcOffset};

/// # Revision
///
/// The revision type keeps track of the revision's id, date added,
/// and text. Additionally in the database it keeps the id of the doc
/// for which it is a revision, but at this point in time there wasn't
/// any use case for exposing that directly in the Rust struct.
// [TODO] zstd compression on the text
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Revision {
    /// Id of the revision in the database
    pub id: u32,
    /// Time when the revision was added. Although we
    /// show it as the local time when user asks for
    /// the time, we internally store it local to the
    /// timezone that it was created. We may take
    /// advantage of this later.
    added: OffsetDateTime,
    /// The text of the revision. This is an option since we
    /// would like to be able to load the revision metadata
    /// without loading the whole document's text into memory.
    /// The database always requires the document to have a text,
    /// we just fetch it lazily.
    pub text: Option<String>,
}

impl Revision {
    /// Create the table for the connection if it doesn't already exist.
    ///
    /// The document entry is only for internal storage and isn't
    /// exposed in the struct, although we use it when collecting
    /// Revision metadata for a given document.
    ///
    /// All field must be present.
    pub fn ensure_table_exists(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS revisions (
               id       INTEGER PRIMARY KEY,
               document INTEGER NOT NULL,
               added    TEXT NOT NULL,
               text     BLOB NOT NULL,
               FOREIGN KEY(document) REFERENCES documents(id)
                 ON DELETE CASCADE
                 ON UPDATE CASCADE
             )",
            (),
        )?;
        Ok(())
    }

    /// Add a new revision for document with the provided document id
    /// and revision text. Returns a result of the revision that was
    /// created from the database call.
    pub fn insert(document: u32, text: &str, conn: &Connection) -> Result<Revision> {
        let added = OffsetDateTime::now_utc();
        let id = conn.query_one(
            "INSERT INTO revisions (document, added, text)
             VALUES (:document, :added, :text)
             RETURNING id",
            named_params! {
                ":document": document,
                ":added": added,
                ":text": text,
            },
            |row| row.get("id"),
        )?;

        Ok(Revision {
            id,
            added,
            text: Some(text.to_string()),
        })
    }

    /// Internal way to turn a metadata Row into the actual revision
    fn from_metadata_row(row: &Row) -> Result<Revision> {
        Ok(Revision {
            id: row.get("id")?,
            added: row.get("added")?,
            text: None,
        })
    }

    /// Fetches all revisions for a given document
    pub fn get_all(document: u32, conn: &Connection) -> Result<Vec<Revision>> {
        let mut stmt = conn.prepare(
            "SELECT id, added
             FROM revisions
             WHERE document = :document",
        )?;
        stmt.query_map(
            named_params! { ":document": document },
            Revision::from_metadata_row,
        )?
        .collect()
    }

    /// Gets the nth revision of provided document
    ///
    /// Because we order on the date field this generally works,
    /// but it is theoretically possible it could mess up since
    /// we aren't turning all of the dates UTC, even though they
    /// keep their timezone metadata. Thus we may need to change
    /// it in the future.
    pub fn get_nth(idx: u32, document: u32, conn: &Connection) -> Result<Revision> {
        conn.query_one(
            "SELECT id, added
             FROM revisions
             WHERE document = :document
             ORDER BY added
             LIMIT 1
             OFFSET :offset",
            named_params! {
                ":document": document,
                ":offset": idx
            },
            Revision::from_metadata_row,
        )
    }

    /// Gets the revision with the given id.
    pub fn from_id(id: u32, conn: &Connection) -> Result<Revision> {
        conn.query_one(
            "SELECT id, added
             FROM revisions
             WHERE id = :id",
            named_params! { ":id": id },
            Revision::from_metadata_row,
        )
    }

    /// Loads the Revision `metadata' into the full struct
    /// with it's corresponding text.
    pub fn load_text(self, conn: &Connection) -> Result<Revision> {
        let text = conn.query_one(
            "SELECT text
             FROM revisions
             WHERE id = :id",
            named_params! {
                ":id": self.id,
            },
            |row| row.get("text"),
        );

        Ok(Revision {
            text: text.ok(),
            ..self
        })
    }

    /// Get the time this revision was added. This localizes the timezone
    /// to the current timezone of the user.
    pub fn added_on(&self) -> OffsetDateTime {
        let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        self.added.to_offset(offset)
    }
}

#[cfg(test)]
mod tests {
    use crate::{document::Document, revision::Revision};

    #[test]
    fn ensure_table_correct_layout() {
        let conn = crate::open_connection_memory().unwrap();

        conn.table_exists(None, "revisions").unwrap();

        vec![
            ("id", "INTEGER", "BINARY", false, true, false),
            ("document", "INTEGER", "BINARY", true, false, false),
            ("added", "TEXT", "BINARY", true, false, false),
            ("text", "BLOB", "BINARY", true, false, false),
        ]
        .iter()
        .for_each(|(name, t, collate, notnull, primary, autoinc)| {
            assert!(conn.column_exists(None, "revisions", name).unwrap());

            let (decl_t, decl_collate, decl_notnull, decl_primary, decl_autoinc) =
                conn.column_metadata(None, "revisions", name).unwrap();

            assert_eq!(&decl_t.unwrap().to_str().unwrap(), t);
            assert_eq!(&decl_collate.unwrap().to_str().unwrap(), collate);
            assert_eq!(&decl_notnull, notnull);
            assert_eq!(&decl_primary, primary);
            assert_eq!(&decl_autoinc, autoinc);
        });
    }

    #[test]
    fn insert_nodoc_fails() {
        let conn = crate::open_connection_memory().unwrap();

        assert!(Revision::insert(0, "foo", &conn).is_err());
    }

    // Can't do tests atm for ones that take doc_id as a parameter
    // - insert
    // - get_all
    // - get_nth

    #[test]
    fn from_id_invalid_fails() {
        let conn = crate::open_connection_memory().unwrap();

        assert!(Revision::from_id(0, &conn).is_err());
    }

    #[test]
    fn from_id_works() {
        let conn = crate::open_connection_memory().unwrap();

        let mut doc = Document::insert("foo", &conn).unwrap();
        let rev = doc.add_new_revision("bar baz", &conn).unwrap();

        let res = Revision::from_id(rev.id, &conn).unwrap();

        assert_eq!(rev.id, res.id);
        assert_eq!(rev.added, res.added);
        assert_eq!(None, res.text);
    }

    #[test]
    fn load_text_works() {
        let conn = crate::open_connection_memory().unwrap();

        let rev = Document::insert("foo", &conn)
            .unwrap()
            .add_new_revision("bar baz", &conn)
            .unwrap();

        let res = Revision::from_id(rev.id, &conn).unwrap();

        assert_eq!(res.text, None);

        let res = res.load_text(&conn).unwrap();

        assert_eq!(res.text, rev.text);
    }
}
