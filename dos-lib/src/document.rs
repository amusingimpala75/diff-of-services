use rusqlite::{named_params, Connection, Result};
use time::OffsetDateTime;

use crate::revision::Revision;

pub struct Document {
    id: u32,
    name: String,
    latest_revision: Option<u32>,
}

impl Document {
    pub fn ensure_table_exists(conn: &Connection) -> Result<()> {
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

    pub fn insert(name: &String, conn: &Connection) -> Result<Document> {
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
            name: name.clone(),
            latest_revision: None,
        })
    }

    pub fn from_name(name: &String, conn: &Connection) -> Result<Document> {
        conn.query_one(
            "SELECT id, latest_revision FROM documents WHERE name = :name",
            named_params! { ":name": name.clone() },
            |row| {
                Ok(Document {
                    id: row.get("id")?,
                    name: name.clone(),
                    latest_revision: row.get("latest_revision")?,
                })
            },
        )
    }

    pub fn add_new_revision(&self, text: String, conn: &Connection) -> Result<()> {
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
        Ok(())
    }

    pub fn count_revisions(&self, conn: &Connection) -> Result<u32> {
        conn.query_one(
            "SELECT COUNT(*) FROM revisions WHERE document = :id",
            named_params! {
                ":id": self.id,
            },
            |row| row.get(0),
        )
    }

    pub fn last_updated(&self, conn: &Connection) -> Option<Result<OffsetDateTime>> {
        self.latest_revision
            .map(|rev| Revision::from_id(rev, conn).map(|rev| rev.added_on()))
    }

    pub fn revisions(&self, conn: &Connection) -> Result<Vec<Revision>> {
        Revision::get_all(self.id, conn)
    }

    pub fn nth_revision(&self, idx: u32, conn: &Connection) -> Result<Revision> {
        Revision::get_nth(idx, self.id, conn)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
