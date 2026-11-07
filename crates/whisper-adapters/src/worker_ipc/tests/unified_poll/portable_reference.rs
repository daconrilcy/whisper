#[cfg(windows)]
fn validate_raw_index(index: &[u8], document: &[u8]) -> Result<usize, String> {
    if index.len() < 8 || &index[..8] != b"PUDOC001" || !(index.len() - 8).is_multiple_of(16) {
        return Err("invalid document index header or size".into());
    }
    let mut cursor = 0_usize;
    let mut count = 0;
    for record in index[8..].as_chunks::<16>().0 {
        let start = u64::from_le_bytes(record[..8].try_into().unwrap());
        let length = u64::from_le_bytes(record[8..].try_into().unwrap());
        let start = usize::try_from(start).map_err(|_| "index offset overflow")?;
        let length = usize::try_from(length).map_err(|_| "index length overflow")?;
        let end = start.checked_add(length).ok_or("index range overflow")?;
        if start != cursor || length == 0 || end > document.len() || document[end - 1] != b'\n' {
            return Err("index is not contiguous or points outside a complete JSON line".into());
        }
        if document[start..end - 1].contains(&b'\n') {
            return Err("indexed range contains multiple physical lines".into());
        }
        serde_json::from_slice::<serde_json::Value>(&document[start..end])
            .map_err(|error| format!("invalid indexed JSON: {error}"))?;
        cursor = end;
        count += 1;
    }
    if cursor != document.len() {
        return Err("index does not cover the complete document".into());
    }
    Ok(count)
}

#[cfg(windows)]
fn file_pair_bytes(files: &[serde_json::Value], name: &str) -> Result<Option<Vec<u8>>, String> {
    let item = files
        .iter()
        .find(|item| item[0].as_str() == Some(name))
        .ok_or_else(|| format!("missing {name} entry"))?;
    if item[1].is_null() {
        return Ok(None);
    }
    serde_json::from_value(item[1].clone())
        .map(Some)
        .map_err(|error| error.to_string())
}

#[cfg(windows)]
fn verify_raw_document_files(files: &[serde_json::Value]) -> Result<(), String> {
    match (
        file_pair_bytes(files, "document.idx")?,
        file_pair_bytes(files, "document.jsonl")?,
    ) {
        (None, None) => Ok(()),
        (Some(index), Some(document)) => validate_raw_index(&index, &document).map(|_| ()),
        _ => Err("index/document presence mismatch".into()),
    }
}

#[cfg(windows)]
fn logical_index_replacements(
    value: &serde_json::Value,
    placeholder: &str,
    pid_width: usize,
    replacements: &mut Vec<(String, String)>,
) -> Result<(), String> {
    let Some(array) = value.as_array() else {
        if let Some(object) = value.as_object() {
            for child in object.values() {
                logical_index_replacements(child, placeholder, pid_width, replacements)?;
            }
        }
        return Ok(());
    };
    if array
        .iter()
        .any(|item| item.is_array() && item[0].as_str() == Some("document.idx"))
    {
        let index = file_pair_bytes(array, "document.idx")?;
        let document = file_pair_bytes(array, "document.jsonl")?;
        match (index, document) {
            (None, None) => return Ok(()),
            (Some(index), Some(document)) => {
                let normalized = String::from_utf8(document).map_err(|error| error.to_string())?;
                let marker = format!("\"instance_id\":\"{placeholder}\"");
                let reconstructed = normalized.replace(
                    &marker,
                    &format!("\"instance_id\":{}", "1".repeat(pid_width)),
                );
                let count = validate_raw_index(&index, reconstructed.as_bytes())?;
                let original = array
                    .iter()
                    .find(|item| item[0].as_str() == Some("document.idx"))
                    .unwrap();
                let logical: Vec<_> = (0..count)
                    .map(|position| (position, position + 1))
                    .collect();
                let replacement =
                    serde_json::json!(["document.idx", {"logical_record_ranges": logical}]);
                replacements.push((
                    serde_json::to_string(original).unwrap(),
                    serde_json::to_string(&replacement).unwrap(),
                ));
                return Ok(());
            }
            _ => return Err("index/document presence mismatch".into()),
        }
    }
    for child in array {
        logical_index_replacements(child, placeholder, pid_width, replacements)?;
    }
    Ok(())
}

#[cfg(windows)]
fn portable_index_trace(
    bytes: &[u8],
    placeholder: &str,
    pid_width: usize,
) -> Result<Vec<u8>, String> {
    let mut lines: Vec<String> =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    if serde_json::to_vec_pretty(&lines).map_err(|error| error.to_string())? != bytes {
        return Err("noncanonical trace encoding".into());
    }
    for line in &mut lines {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let mut replacements = Vec::new();
        logical_index_replacements(&value, placeholder, pid_width, &mut replacements)?;
        for (before, after) in replacements {
            *line = line.replacen(&before, &after, 1);
        }
    }
    serde_json::to_vec_pretty(&lines).map_err(|error| error.to_string())
}

#[cfg(windows)]
fn assert_portable_index_trace(actual: &[u8], frozen: &[u8], pid: u64, placeholder: &str) {
    let actual = portable_index_trace(actual, placeholder, pid.to_string().len())
        .expect("candidate index invalid");
    let expected = portable_index_trace(frozen, placeholder, 5).expect("baseline index invalid");
    assert_eq!(actual, expected, "portable index trace differs");
}

#[cfg(windows)]
#[test]
fn unified_poll_portable_index_validates_offsets_and_pid_widths() {
    fn sample(pid: u64) -> (Vec<u8>, Vec<u8>) {
        let document =
            format!("{{\"segment\":{{\"instance_id\":{pid},\"text\":\"test\"}},\"revision\":1}}\n")
                .into_bytes();
        let mut index = b"PUDOC001".to_vec();
        index.extend_from_slice(&0_u64.to_le_bytes());
        index.extend_from_slice(&(document.len() as u64).to_le_bytes());
        (index, document)
    }
    let (index4, document4) = sample(1234);
    let (index5, document5) = sample(12345);
    assert_eq!(validate_raw_index(&index4, &document4), Ok(1));
    assert_eq!(validate_raw_index(&index5, &document5), Ok(1));
    let mut corrupt = index4.clone();
    corrupt[0] ^= 1;
    assert!(validate_raw_index(&corrupt, &document4).is_err());
    let mut wrong_offset = index4.clone();
    wrong_offset[8] = 1;
    assert!(validate_raw_index(&wrong_offset, &document4).is_err());
    let mut wrong_length = index4.clone();
    wrong_length[16] -= 1;
    assert!(validate_raw_index(&wrong_length, &document4).is_err());
    let document_with_blank = b"{\"a\":1}\n\n{\"a\":2}\n";
    let mut bad_lines = b"PUDOC001".to_vec();
    bad_lines.extend_from_slice(&0_u64.to_le_bytes());
    bad_lines.extend_from_slice(&9_u64.to_le_bytes());
    bad_lines.extend_from_slice(&9_u64.to_le_bytes());
    bad_lines.extend_from_slice(&8_u64.to_le_bytes());
    assert!(validate_raw_index(&bad_lines, document_with_blank).is_err());
    let snapshot = |index: Vec<u8>, document: Vec<u8>, pid: u64| {
        let text = String::from_utf8(document).unwrap().replace(
            &format!("\"instance_id\":{pid}"),
            "\"instance_id\":\"<PID1>\"",
        );
        let entry = serde_json::json!([
            "document files",
            [
                ["document.idx", index],
                ["document.jsonl", text.into_bytes()]
            ],
            null
        ]);
        serde_json::to_vec_pretty(&vec![serde_json::to_string(&entry).unwrap()]).unwrap()
    };
    let mut changed_document = document4.clone();
    let text = changed_document
        .windows(4)
        .position(|part| part == b"test")
        .unwrap();
    changed_document[text] = b'b';
    let changed = snapshot(index4.clone(), changed_document, 1234);
    let a = snapshot(index4, document4, 1234);
    let b = snapshot(index5, document5, 12345);
    assert_eq!(
        portable_index_trace(&a, "<PID1>", 4).unwrap(),
        portable_index_trace(&b, "<PID1>", 5).unwrap()
    );
    assert_ne!(
        portable_index_trace(&a, "<PID1>", 4).unwrap(),
        portable_index_trace(&changed, "<PID1>", 4).unwrap()
    );
    let mut noncanonical = a.clone();
    noncanonical.push(b' ');
    assert!(portable_index_trace(&noncanonical, "<PID1>", 4).is_err());
}
