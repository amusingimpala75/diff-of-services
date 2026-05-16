import { invoke } from "@tauri-apps/api/core";

let documentSelect: HTMLSelectElement;
let revisionSelect: HTMLSelectElement;
let textElement: HTMLDivElement;

interface Revision {
  id: number,
  date_added: Date,
}

interface Document {
  id: number,
  name: string,
}

async function loadDocuments() {
  let documents: [Document] = await invoke("get_all_documents", {});

  while (documentSelect.children.length > 0) {
    documentSelect.remove(0);
  }

  for (const doc of documents) {
    let option = document.createElement("option");
    option.value = doc.id.toString();
    option.label = doc.name;
    documentSelect.appendChild(option);
  }

  documentSelect.selectedIndex = 0;
  updateDocument();
}

async function updateDocument() {
  while (revisionSelect.children.length > 0) {
    revisionSelect.remove(0);
  }

  if (documentSelect.value === "") {
    return;
  }

  let revisions: [Revision] = await invoke("get_document_revisions", {
    id: parseInt(documentSelect.value),
  });

  revisions.reverse();

  for (const rev of revisions) {
    let option = document.createElement("option");
    option.value = rev.id.toString();
    option.label = rev.date_added.toString();
    revisionSelect.appendChild(option);
  }

  revisionSelect.selectedIndex = 0;
  updateRevision();
}

async function updateRevision() {
  while (textElement.children.length > 0) {
    textElement.removeChild(textElement.firstChild!);
  }

  if (revisionSelect.value === "") {
    return;
  }

  const text: string = await invoke("get_revision_content", {
    id: parseInt(revisionSelect.value),
  });

  for (const line of text.split("\n")) {
    let p = document.createElement("p");
    p.textContent = line;
    textElement.appendChild(p);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  documentSelect = document.querySelector("#documents")!;
  revisionSelect = document.querySelector("#revisions")!;
  textElement = document.querySelector("#document-revision-text")!;

  documentSelect.addEventListener("change", updateDocument);
  revisionSelect.addEventListener("change", updateRevision);

  loadDocuments();
});
