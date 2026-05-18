use similar::{ChangeTag, TextDiff};

#[derive(serde::Serialize)]
pub enum DiffType {
    Add,
    Remove,
    Same,
}

impl DiffType {
    fn from_change_tag(tag: ChangeTag) -> Self {
        match tag {
            ChangeTag::Equal => DiffType::Same,
            ChangeTag::Delete => DiffType::Remove,
            ChangeTag::Insert => DiffType::Add,
        }
    }
}

#[derive(serde::Serialize)]
pub struct DiffSegment {
    pub text: String,
    pub r#type: DiffType,
}

pub fn diff_lines_words(old: &str, new: &str) -> Vec<Vec<DiffSegment>> {
    let line_diff = TextDiff::from_lines(old, new);
    let mut skip = false;
    let mut lines = Vec::new();

    for pair in line_diff.iter_all_changes().collect::<Vec<_>>().windows(2) {
        let (change, next) = (pair[0], pair[1]);
        if skip {
            skip = false;
            continue;
        }
        match change.tag() {
            ChangeTag::Equal | ChangeTag::Insert => lines.push(vec![DiffSegment {
                text: change.value().to_string(),
                r#type: DiffType::from_change_tag(change.tag()),
            }]),
            ChangeTag::Delete => {
                if next.tag() == ChangeTag::Insert {
                    let word_diff = TextDiff::from_words(change.value(), next.value());
                    if word_diff.ratio() > 0.75 {
                        let mut line = Vec::new();
                        for change in word_diff.iter_all_changes() {
                            line.push(DiffSegment {
                                text: change.value().to_string(),
                                r#type: DiffType::from_change_tag(change.tag()),
                            });
                        }
                        lines.push(line);
                        skip = true;
                        continue;
                    }
                }
                lines.push(vec![DiffSegment {
                    text: change.value().to_string(),
                    r#type: DiffType::Remove,
                }])
            }
        }
    }

    if !skip && let Some(change) = line_diff.iter_all_changes().last() {
        lines.push(vec![DiffSegment {
            text: change.value().to_string(),
            r#type: DiffType::from_change_tag(change.tag()),
        }]);
    }

    lines
}
