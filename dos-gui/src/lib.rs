use dos_lib::{diff, document::Document, revision::Revision};

#[derive(serde::Serialize)]
struct DocumentStub {
    id: u32,
    name: String,
}

impl DocumentStub {
    fn from_document(document: Document) -> Self {
        DocumentStub {
            id: document.id(),
            name: document.name().to_string(),
        }
    }
}

#[tauri::command]
fn get_all_documents() -> Result<Vec<DocumentStub>, String> {
    let connection = match dos_lib::open_connection_file() {
        Ok(conn) => conn,
        Err(_) => return Err("could not open connection".to_string()),
    };
    let documents = match Document::get_all(&connection) {
        Ok(docs) => docs,
        Err(_) => return Err("Could not fetch documents".to_string()),
    };
    Ok(documents
        .into_iter()
        .map(DocumentStub::from_document)
        .collect::<Vec<_>>())
}

#[derive(serde::Serialize)]
struct RevisionStub {
    id: u32,
    date_added: String,
}

impl RevisionStub {
    fn from_revision(revision: Revision) -> Self {
        RevisionStub {
            id: revision.id,
            date_added: dos_lib::format_local_time(revision.added_on()),
        }
    }
}

#[tauri::command]
fn get_document_revisions(id: u32) -> Result<Vec<RevisionStub>, String> {
    let connection = match dos_lib::open_connection_file() {
        Ok(conn) => conn,
        Err(_) => return Err("could not open connection".to_string()),
    };

    let document = match Document::from_id(id, &connection) {
        Ok(doc) => doc,
        Err(_) => return Err(format!("no document with id {id}")),
    };

    let revisions = match document.revisions(&connection) {
        Ok(revs) => revs,
        Err(_) => {
            return Err(format!(
                "coudl not fetch the revisions for document with id {id}"
            ));
        }
    };

    Ok(revisions
        .into_iter()
        .map(RevisionStub::from_revision)
        .collect::<Vec<_>>())
}

#[tauri::command]
fn get_revision_content(id: u32) -> Result<String, String> {
    let connection = match dos_lib::open_connection_file() {
        Ok(conn) => conn,
        Err(_) => return Err("could not open connection".to_string()),
    };

    let revision = match Revision::from_id(id, &connection) {
        Ok(rev) => rev,
        Err(_) => return Err(format!("Could not fetch revision with id {id}")),
    };

    let text = match revision.load_text(&connection) {
        Ok(rev) => rev.text.unwrap(),
        Err(_) => return Err(format!("Could not fetch text of revision {id}")),
    };

    Ok(text)
}

#[tauri::command]
fn get_revision_diff(old: u32, new: u32) -> Result<Vec<Vec<diff::DiffSegment>>, String> {
    let connection = match dos_lib::open_connection_file() {
        Ok(conn) => conn,
        Err(_) => return Err("could not open connection".to_string()),
    };

    let old_revision = match Revision::from_id(old, &connection) {
        Ok(rev) => rev,
        Err(_) => return Err(format!("could fetch revision {old}")),
    };

    let new_revision = match Revision::from_id(new, &connection) {
        Ok(rev) => rev,
        Err(_) => return Err(format!("could fetch revision {new}")),
    };

    let old_text = match old_revision.load_text(&connection) {
        Ok(rev) => rev.text.unwrap(),
        Err(_) => return Err(format!("Could not fetch text of revision {old}")),
    };

    let new_text = match new_revision.load_text(&connection) {
        Ok(rev) => rev.text.unwrap(),
        Err(_) => return Err(format!("Could not fetch text of revision {new}")),
    };

    Ok(diff::diff_lines_words(&old_text, &new_text))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_documents,
            get_document_revisions,
            get_revision_content,
            get_revision_diff
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
