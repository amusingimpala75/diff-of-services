use rusqlite::{named_params, Connection, Result, Row};
use time::{OffsetDateTime, UtcOffset};

// [TODO] zstd compression on the text
pub struct Revision {
    pub id: u32,
    added: OffsetDateTime,
    pub text: Option<String>,
}

impl Revision {
    pub fn ensure_table_exists(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS revisions (
               id       INTEGER PRIMARY KEY,
               document INTEGER NOT NULL,
               added    TEXT NOT NULL,
               text     BLOB NOT NULL
             )",
            (),
        )?;
        Ok(())
    }

    pub fn insert(document: u32, text: String, conn: &Connection) -> Result<Revision> {
        let added = OffsetDateTime::now_utc();
        let id = conn.query_one(
            "INSERT INTO revisions (document, added, text)
             VALUES (:document, :added, :text)
             RETURNING id",
            named_params! {
                ":document": document,
                ":added": added,
                ":text": text.clone(), // [TODO] better error checking
            },
            |row| row.get("id"),
        )?;

        Ok(Revision {
            id,
            added,
            text: Some(text),
        })
    }

    fn from_metadata_row(row: &Row) -> Result<Revision> {
        Ok(Revision {
            id: row.get("id")?,
            added: row.get("added")?,
            text: None,
        })
    }

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

    pub fn from_id(id: u32, conn: &Connection) -> Result<Revision> {
        conn.query_one(
            "SELECT id, added
             FROM revisions
             WHERE id = :id",
            named_params! { ":id": id },
            Revision::from_metadata_row,
        )
    }

    pub fn load_text(self, conn: &Connection) -> Result<Revision> {
        let text = conn.query_one(
            "SELECT text
             FROM revisions
             WHERE id = :id",
            named_params! {
                ":id": self.id,
            },
            |row| row.get("text").into(),
        );

        Ok(Revision {
            text: text.ok(),
            ..self
        })
    }

    pub fn added_on(&self) -> OffsetDateTime {
        let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        self.added.to_offset(offset)
    }
}
