use rusqlite::{Connection, Result, named_params};
use time::OffsetDateTime;

use crate::revision::Revision;

/// # Document
///
/// The document type keeps track of the name and latest revision of this doc.
/// It does not itself keep track of the contents of the revision, or last
/// modified state. That is left to the Revision type, meaning such calls
/// go out to the local sqlite database.
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
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
    /// Last time revision was added or attempted to add but was identical
    last_checked: OffsetDateTime,
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
               latest_revision INTEGER,
               last_checked    TEXT NOT NULL,
               FOREIGN KEY(latest_revision) REFERENCES revisions(id)
             )",
            (),
        )?;
        Ok(())
    }

    /// Selects all documents from the database
    pub fn get_all(conn: &Connection) -> Result<Vec<Document>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, latest_revision, last_checked
             FROM documents",
        )?;
        stmt.query_map([], |row| {
            Ok(Document {
                id: row.get("id")?,
                name: row.get("name")?,
                latest_revision: row.get("latest_revision")?,
                last_checked: row.get("last_checked")?,
            })
        })?
        .collect()
    }

    /// Creates a new document with the given name, returning
    /// the document that has been created
    pub fn insert(name: &str, conn: &Connection) -> Result<Document> {
        let now = OffsetDateTime::now_utc();
        let id = conn.query_one(
            "INSERT INTO documents (name, last_checked)
             VALUES (:name, :last_checked)
             RETURNING id",
            named_params! {
                ":name": &name,
                ":last_checked": &now,
            },
            |row| row.get("id"),
        )?;
        Ok(Document {
            id,
            name: name.to_string(),
            latest_revision: None,
            last_checked: now,
        })
    }

    /// Gets an existing document from the database with the given
    /// name. Currently just doing a direct string comparison, so
    /// entring the name could be a bit jank.
    pub fn from_name(name: &str, conn: &Connection) -> Result<Document> {
        conn.query_one(
            "SELECT id, latest_revision, last_checked FROM documents WHERE name = :name",
            named_params! { ":name": name },
            |row| {
                Ok(Document {
                    id: row.get("id")?,
                    name: name.to_string(),
                    latest_revision: row.get("latest_revision")?,
                    last_checked: row.get("last_checked")?,
                })
            },
        )
    }

    /// Gets an existing document from the database with the given id.
    pub fn from_id(id: u32, conn: &Connection) -> Result<Document> {
        conn.query_one(
            "SELECT name, latest_revision, last_checked FROM documents WHERE id = :id",
            named_params! { ":id": id },
            |row| {
                Ok(Document {
                    id,
                    name: row.get("name")?,
                    latest_revision: row.get("latest_revision")?,
                    last_checked: row.get("last_checked")?,
                })
            },
        )
    }

    /// Adds a new revision with the requested text and updates the latest
    /// rev to refer to it. Also updates the last checked time. Does NOT
    /// create a new revision if the text is identical.
    pub fn add_new_revision(&mut self, text: &str, conn: &Connection) -> anyhow::Result<Revision> {
        // Re-use the old revision if the text is identical
        let rev = if let Some(id) = self.latest_revision
            && let Ok(old) = Revision::from_id(id, conn)
            && let Ok(texted) = old.load_text(conn)
            && texted.text.as_ref().unwrap() == text
        {
            texted
        } else {
            Revision::insert(self.id, text, conn)?
        };

        let now = OffsetDateTime::now_utc();
        conn.execute(
            "UPDATE documents
             SET latest_revision = :rev,
                 last_checked = :now
             WHERE id = :id",
            named_params! {
                ":id": self.id,
                ":rev": rev.id,
                ":now": now,
            },
        )?;
        self.latest_revision = Some(rev.id);
        self.last_checked = now;
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

    pub fn id(&self) -> u32 {
        self.id
    }
}

#[cfg(test)]
mod tests {

    use crate::{document::Document, revision::Revision};

    #[test]
    fn ensure_valid_database_layout() {
        let conn = crate::open_connection_memory().unwrap();

        conn.table_exists(None, "documents").unwrap();

        vec![
            ("id", "INTEGER", "BINARY", false, true, false),
            ("name", "TEXT", "BINARY", true, false, false),
            ("latest_revision", "INTEGER", "BINARY", false, false, false),
        ]
        .iter()
        .for_each(|row| {
            let (name, t, collate, notnull, primary, autoincrement) = row;
            assert!(conn.column_exists(None, "documents", name).unwrap());

            let (decl_t, decl_collate, decl_notnull, decl_primary, decl_autoinc) =
                conn.column_metadata(None, "documents", name).unwrap();

            assert_eq!(&decl_t.unwrap().to_str().unwrap(), t);
            assert_eq!(&decl_collate.unwrap().to_str().unwrap(), collate);
            assert_eq!(&decl_notnull, notnull);
            assert_eq!(&decl_primary, primary);
            assert_eq!(&decl_autoinc, autoincrement);
        });
    }

    #[test]
    fn can_insert_document() {
        let conn = crate::open_connection_memory().unwrap();

        let name = "test";

        let doc = Document::insert(name, &conn).unwrap();

        assert_eq!(doc.name, name);

        assert_eq!(doc.latest_revision, None);
    }

    #[test]
    fn get_all_no_documents_empty_vec() {
        let conn = crate::open_connection_memory().unwrap();
        let docs = Document::get_all(&conn).unwrap();
        assert_eq!(docs.len(), 0);
    }

    #[test]
    fn get_all_documents_inserted() {
        let conn = crate::open_connection_memory().unwrap();

        let mut docs = vec!["1", "2", "3"]
            .iter()
            .map(|name| Document::insert(name, &conn).unwrap())
            .collect::<Vec<Document>>();
        docs.sort();

        let mut res = Document::get_all(&conn).unwrap();
        res.sort();

        assert_eq!(docs, res);
    }

    #[test]
    fn get_by_name_works() {
        let conn = crate::open_connection_memory().unwrap();

        let name = "test";
        Document::insert(name, &conn).unwrap();

        let res = Document::from_name(name, &conn).unwrap();
        assert_eq!(res.name, name);
    }

    #[test]
    fn by_name_err_missing() {
        let conn = crate::open_connection_memory().unwrap();

        Document::insert("foo", &conn).unwrap();

        assert!(Document::from_name("bar", &conn).is_err());
    }

    #[test]
    fn add_new_revision_works() {
        let conn = crate::open_connection_memory().unwrap();

        let name = "foo";
        let text = "bar baz";

        let rev = Document::insert(name, &conn)
            .unwrap()
            .add_new_revision(text, &conn)
            .unwrap()
            .load_text(&conn)
            .unwrap();

        assert_eq!(rev.text.unwrap(), text);
    }

    #[test]
    fn count_revs_empty() {
        let conn = crate::open_connection_memory().unwrap();

        assert_eq!(
            Document::insert("foo", &conn)
                .unwrap()
                .count_revisions(&conn)
                .unwrap(),
            0
        );
    }

    #[test]
    fn count_revs_some() {
        let conn = crate::open_connection_memory().unwrap();

        let mut doc = Document::insert("foo", &conn).unwrap();

        doc.add_new_revision("bar", &conn).unwrap();
        doc.add_new_revision("baz", &conn).unwrap();

        assert_eq!(doc.count_revisions(&conn).unwrap(), 2);
    }

    #[test]
    fn last_update_none() {
        let conn = crate::open_connection_memory().unwrap();

        let doc = Document::insert("foo", &conn).unwrap();

        assert_eq!(doc.last_updated(&conn), None);
    }

    #[test]
    fn last_update_correct() {
        let conn = crate::open_connection_memory().unwrap();

        let mut doc = Document::insert("foo", &conn).unwrap();

        doc.add_new_revision("bar", &conn).unwrap();
        doc.add_new_revision("baz'", &conn).unwrap();
        let rev3 = doc
            .add_new_revision("quux", &conn)
            .unwrap()
            .load_text(&conn)
            .unwrap();

        assert_eq!(doc.last_updated(&conn).unwrap().unwrap(), rev3.added_on());
    }

    #[test]
    fn revisions_empty_none() {
        let conn = crate::open_connection_memory().unwrap();

        let doc = Document::insert("foo", &conn).unwrap();

        assert_eq!(doc.revisions(&conn).unwrap().len(), 0);
    }

    #[test]
    fn revisions_some_correct() {
        let conn = crate::open_connection_memory().unwrap();

        let doc = Document::insert("foo", &conn).unwrap();

        let mut revs = vec!["1", "2", "3"]
            .iter()
            .map(|t| doc.clone().add_new_revision(t, &conn).unwrap())
            .collect::<Vec<Revision>>();
        revs.sort();

        let mut res = doc
            .revisions(&conn)
            .unwrap()
            .into_iter()
            .map(|r| r.load_text(&conn).unwrap())
            .collect::<Vec<Revision>>();
        res.sort();

        assert_eq!(res, revs);
    }

    #[test]
    fn nth_out_of_range() {
        let conn = crate::open_connection_memory().unwrap();

        assert!(
            Document::insert("foo", &conn)
                .unwrap()
                .nth_revision(0, &conn)
                .is_err()
        );
    }

    #[test]
    fn nth_correct() {
        let conn = crate::open_connection_memory().unwrap();

        let mut doc = Document::insert("foo", &conn).unwrap();

        doc.add_new_revision("bar", &conn).unwrap();
        let rev = doc.add_new_revision("baz", &conn).unwrap();
        doc.add_new_revision("quux", &conn).unwrap();

        assert_eq!(
            doc.nth_revision(1, &conn)
                .unwrap()
                .load_text(&conn)
                .unwrap(),
            rev
        );
    }

    #[test]
    fn name_correct() {
        let conn = crate::open_connection_memory().unwrap();

        let name = "foobarbaz";

        assert_eq!(Document::insert(name, &conn).unwrap().name(), name);
    }

    #[test]
    fn update_identical_only_last_checked() {
        let conn = crate::open_connection_memory().unwrap();

        let mut doc = Document::insert("foo", &conn).unwrap();

        let old = doc.add_new_revision("foo", &conn).unwrap();
        let new = doc.add_new_revision("foo", &conn).unwrap();

        assert_eq!(old.id, new.id);
    }
}
