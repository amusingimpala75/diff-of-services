import { invoke } from "@tauri-apps/api/core";

let documentSelect: HTMLSelectElement;
let revisionSelect: HTMLSelectElement;
let textElement: HTMLDivElement;
let diffRadios: [HTMLInputElement];

interface Revision {
  id: number;
  date_added: Date;
}

interface Document {
  id: number;
  name: string;
}

async function loadDocuments() {
  const documents: [Document] = await invoke("get_all_documents", {});

  while (documentSelect.children.length > 0) {
    documentSelect.remove(0);
  }

  for (const doc of documents) {
    const option = document.createElement("option");
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

  const revisions: [Revision] = await invoke("get_document_revisions", {
    id: parseInt(documentSelect.value, 10),
  });

  revisions.reverse();

  for (const rev of revisions) {
    const option = document.createElement("option");
    option.value = rev.id.toString();
    option.label = rev.date_added.toString();
    revisionSelect.appendChild(option);
  }

  revisionSelect.selectedIndex = 0;
  updateRevision();
}

async function updateRevision() {
  while (textElement.firstChild) {
    textElement.removeChild(textElement.firstChild);
  }

  if (revisionSelect.value === "") {
    return;
  }

  for (const radio of diffRadios) {
    if (radio.checked) {
      switch (radio.value) {
        case "none":
          updateRevisionPlainText();
          break;
        case "inline":
          updateRevisionInlineDiff();
          break;
        case "2col":
          updateRevisionDiff2Col();
          break;
      }
    }
  }
}

async function updateRevisionPlainText() {
  const text: string = await invoke("get_revision_content", {
    id: parseInt(revisionSelect.value, 10),
  });

  for (const line of text.split("\n")) {
    const p = document.createElement("p");
    p.textContent = line;
    textElement.appendChild(p);
  }
}

enum DiffType {
  ADD = "Add",
  REMOVE = "Remove",
  SAME = "Same",
}

interface DiffSegment {
  text: string;
  type: DiffType;
}

async function updateRevisionInlineDiff() {
  if (revisionSelect.selectedIndex + 1 === revisionSelect.options.length) {
    updateRevisionPlainText();
    return;
  }
  const current = revisionSelect.value;
  const previous =
    revisionSelect.options[revisionSelect.selectedIndex + 1].value;

  const diff: [[DiffSegment]] = await invoke("get_revision_diff", {
    old: parseInt(previous, 10),
    new: parseInt(current, 10),
  });

  for (const line of diff) {
    const p = document.createElement("p");
    for (const segment of line) {
      const span = document.createElement("span");
      span.textContent = segment.text;
      console.log(segment.type);
      switch (segment.type) {
        case DiffType.ADD:
          span.classList.add("diff-add");
          break;
        case DiffType.REMOVE:
          span.classList.add("diff-remove");
          break;
        case DiffType.SAME:
          span.classList.add("diff-same");
          break;
      }
      p.appendChild(span);
    }
    textElement.appendChild(p);
  }
}

async function updateRevisionDiff2Col() {
  if (revisionSelect.selectedIndex + 1 === revisionSelect.options.length) {
    updateRevisionPlainText();
    return;
  }
  const current = revisionSelect.value;
  const previous =
    revisionSelect.options[revisionSelect.selectedIndex + 1].value;

  const diff: [[DiffSegment]] = await invoke("get_revision_diff", {
    old: parseInt(previous, 10),
    new: parseInt(current, 10),
  });

  const left = document.createElement("div");
  const right = document.createElement("div");
  textElement.appendChild(left);
  textElement.appendChild(right);
  left.classList.add("column");
  right.classList.add("column");

  for (const line of diff) {
    const lp = document.createElement("p");
    const rp = document.createElement("p");
    left.appendChild(lp);
    right.appendChild(rp);
    for (const segment of line) {
      const lspan = document.createElement("span");
      const rspan = document.createElement("span");
      lspan.textContent = segment.text;
      rspan.textContent = segment.text;
      lp.appendChild(lspan);
      rp.appendChild(rspan);

      switch (segment.type) {
        case DiffType.ADD:
          lspan.classList.add("hidden");
          rspan.classList.add("diff-add");
          break;
        case DiffType.REMOVE:
          lspan.classList.add("diff-remove");
          rspan.classList.add("hidden");
          break;
        case DiffType.SAME:
          lspan.classList.add("diff-same");
          rspan.classList.add("diff-same");
          break;
      }
    }
  }
}

window.addEventListener("DOMContentLoaded", () => {
  const ds = document.querySelector<HTMLSelectElement>("#documents");
  const rs = document.querySelector<HTMLSelectElement>("#revisions");
  const te = document.querySelector<HTMLDivElement>("#document-revision-text");

  if (!ds || !rs || !te) {
    throw new Error("Invalid document state, missing key elements");
  }

  documentSelect = ds;
  revisionSelect = rs;
  textElement = te;

  diffRadios = <[HTMLInputElement]>(
    Array.from(document.querySelector("#diff-type")?.children)
  );

  documentSelect.addEventListener("change", updateDocument);
  revisionSelect.addEventListener("change", updateRevision);
  diffRadios.forEach((radio) => {
    radio.addEventListener("change", updateRevision);
  });

  document.querySelector("#add-revision")?.addEventListener("click", () => {
    window.location.replace("/revision");
  });

  loadDocuments();
});
