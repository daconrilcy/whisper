use std::{fs, path::Path};

#[test]
fn test_sources_create_executable_fixtures_only_through_pe_constructor() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();
    for source_root in [root.join("src"), root.join("tests")] {
        scan_sources(&source_root, &mut violations);
    }
    assert!(
        violations.is_empty(),
        "test sources write .exe files outside the PE constructor: {violations:?}"
    );

    let direct = concat!(
        "fn witness() { let fake = root.join(\"fake",
        ".exe\");\nfs::write(&fake, b\"bad\"); }"
    );
    let multiline = concat!(
        "fn witness() { let fake = root.join(\"fake",
        ".exe\");\nfs::write(\n&fake,\nb\"bad\",\n); }"
    );
    let owned = concat!(
        "fn witness() { let fake = root.join(\"owned",
        ".exe\");\nfs::write(fake, b\"bad\"); }"
    );
    let mutable = concat!(
        "fn witness() { let mut fake = root.join(\"mutable",
        ".exe\");\nfs::write(&fake, b\"bad\"); }"
    );
    let mutant = concat!(
        "fn witness() { let mutant = root.join(\"mutant",
        ".exe\");\nfs::write(&mutant, b\"bad\"); }"
    );
    let worker_path = concat!(
        "fn witness() { let workers = WorkerPaths { cpu: root.join(\"cpu",
        ".exe\"), gpu: root.join(\"gpu.exe\") };\nfs::write(&workers.cpu, b\"bad\"); }"
    );
    assert!(has_executable_write(direct));
    assert!(has_executable_write(multiline));
    assert!(has_executable_write(owned));
    assert!(has_executable_write(mutable));
    assert!(has_executable_write(mutant));
    assert!(has_executable_write(worker_path));
}

fn scan_sources(path: &Path, violations: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_sources(&path, violations);
        } else if path.extension().is_some_and(|extension| extension == "rs")
            && path.file_name().is_some_and(|name| name != "pe_fixture.rs")
            && fs::read_to_string(&path).is_ok_and(|source| has_executable_write(&source))
        {
            violations.push(path.display().to_string());
        }
    }
}

fn has_executable_write(source: &str) -> bool {
    let executable_variables = source
        .split(';')
        .filter_map(executable_binding)
        .collect::<Vec<_>>();
    source.split(';').any(|statement| {
        let compact: String = statement.chars().filter(|c| !c.is_whitespace()).collect();
        let writes = ["write(", "copy(", "create("]
            .iter()
            .any(|call| compact.contains(call));
        writes
            && (compact.contains(".exe\"")
                || executable_variables
                    .iter()
                    .any(|name| writes_executable_variable(&compact, name)))
    })
}

fn executable_binding(statement: &str) -> Option<String> {
    let start = statement.match_indices("let").find_map(|(index, _)| {
        let previous_is_identifier = index > 0
            && statement[..index]
                .chars()
                .next_back()
                .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        (!previous_is_identifier).then_some(index + 3)
    })?;
    let declaration = statement[start..].trim_start();
    let declaration = if let Some(rest) = declaration.strip_prefix("mut")
        && rest.chars().next().is_some_and(char::is_whitespace)
    {
        rest.trim_start()
    } else {
        declaration
    };
    let (binding, value) = declaration.split_once('=')?;
    if !value.contains(".exe\"") {
        return None;
    }
    let binding = binding.trim();
    (!binding.is_empty()
        && binding
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_'))
    .then(|| binding.to_owned())
}

fn writes_executable_variable(statement: &str, name: &str) -> bool {
    let direct_or_owned = [
        format!("write({name},"),
        format!("write(&{name},"),
        format!("create({name})"),
        format!("create(&{name})"),
        format!("copy({name},"),
        format!("copy(&{name},"),
        format!(",{name})"),
        format!(",&{name})"),
        format!(",{name},"),
        format!(",&{name},"),
    ]
    .iter()
    .any(|needle| statement.contains(needle));
    direct_or_owned
        || ["cpu", "gpu", "executable"].iter().any(|field| {
            statement.contains(&format!("{name}.{field}"))
                || statement.contains(&format!("&{name}.{field}"))
        })
}
