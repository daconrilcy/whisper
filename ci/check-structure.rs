use std::{
    collections::{HashMap, HashSet},
    env,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
struct TestRecord {
    path: String,
    name: String,
    _old_category: String,
    category: String,
    prerequisites: String,
    reason: String,
}

#[derive(Clone)]
struct TestFunction {
    path: String,
    name: String,
    header: String,
    body: String,
}

fn skip_quoted_or_comment(bytes: &[u8], index: usize) -> Option<usize> {
    let byte = *bytes.get(index)?;

    if byte == b'/' && bytes.get(index + 1) == Some(&b'/') {
        let end = bytes[index + 2..]
            .iter()
            .position(|item| *item == b'\n')
            .map_or(bytes.len(), |offset| index + 2 + offset);
        return Some(end);
    }

    if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
        let mut depth = 1usize;
        let mut cursor = index + 2;
        while cursor + 1 < bytes.len() {
            if bytes[cursor] == b'/' && bytes[cursor + 1] == b'*' {
                depth += 1;
                cursor += 2;
            } else if bytes[cursor] == b'*' && bytes[cursor + 1] == b'/' {
                depth -= 1;
                cursor += 2;
                if depth == 0 {
                    return Some(cursor);
                }
            } else {
                cursor += 1;
            }
        }
        return Some(bytes.len());
    }

    let raw_start = if byte == b'r' {
        Some(index)
    } else if byte == b'b' && bytes.get(index + 1) == Some(&b'r') {
        Some(index + 1)
    } else {
        None
    };
    if let Some(start) = raw_start {
        let mut cursor = start + 1;
        while bytes.get(cursor) == Some(&b'#') {
            cursor += 1;
        }
        if bytes.get(cursor) == Some(&b'"') {
            let hashes = cursor - start - 1;
            cursor += 1;
            while cursor < bytes.len() {
                if bytes[cursor] == b'"'
                    && bytes.get(cursor + 1..cursor + 1 + hashes)
                        == Some(&vec![b'#'; hashes][..])
                {
                    return Some(cursor + 1 + hashes);
                }
                cursor += 1;
            }
            return Some(bytes.len());
        }
    }

    if byte == b'"' {
        let mut cursor = index + 1;
        let mut escaped = false;
        while cursor < bytes.len() {
            let current = bytes[cursor];
            if escaped {
                escaped = false;
            } else if current == b'\\' {
                escaped = true;
            } else if current == b'"' {
                return Some(cursor + 1);
            }
            cursor += 1;
        }
        return Some(bytes.len());
    }

    if byte == b'\'' {
        let mut cursor = index + 1;
        let mut escaped = false;
        while cursor < bytes.len() && bytes[cursor] != b'\n' {
            let current = bytes[cursor];
            if escaped {
                escaped = false;
            } else if current == b'\\' {
                escaped = true;
            } else if current == b'\'' {
                if cursor > index + 1 {
                    return Some(cursor + 1);
                }
                break;
            }
            cursor += 1;
        }
    }

    None
}

fn matching_delimiter(text: &str, start: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(start) != Some(&open) {
        return None;
    }
    let mut depth = 0usize;
    let mut index = start;
    while index < bytes.len() {
        if let Some(next) = skip_quoted_or_comment(bytes, index) {
            index = next;
            continue;
        }
        match bytes[index] {
            current if current == open => depth += 1,
            current if current == close => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn function_name(line: &str) -> Option<&str> {
    let marker = line.find("fn ")?;
    let rest = &line[marker + 3..];
    let end = rest
        .find(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .unwrap_or(rest.len());
    (end > 0).then_some(&rest[..end])
}

fn function_body(source: &str, signature_start: usize) -> Option<(usize, usize)> {
    let bytes = source.as_bytes();
    let open_paren = source[signature_start..].find('(')? + signature_start;
    let close_paren = matching_delimiter(source, open_paren, b'(', b')')?;
    let open_brace = source[close_paren + 1..].find('{')? + close_paren + 1;
    let close_brace = matching_delimiter(source, open_brace, b'{', b'}')?;
    let _ = bytes;
    Some((open_brace, close_brace + 1))
}

fn discover_tests(path: &str, source: &str) -> Vec<TestFunction> {
    let lines: Vec<&str> = source.lines().collect();
    let mut line_offsets = Vec::with_capacity(lines.len());
    let mut offset = 0usize;
    for line in &lines {
        let actual = source[offset..].find(line).map_or(offset, |relative| offset + relative);
        line_offsets.push(actual);
        offset = actual + line.len();
        if source.as_bytes().get(offset) == Some(&b'\r') {
            offset += 1;
        }
        if source.as_bytes().get(offset) == Some(&b'\n') {
            offset += 1;
        }
    }

    let mut tests = Vec::new();
    for (attribute_line, line) in lines.iter().enumerate() {
        if line.trim() != "#[test]" {
            continue;
        }

        let mut signature_line = None;
        for candidate in attribute_line + 1..(attribute_line + 100).min(lines.len()) {
            if lines[candidate].trim() == "#[test]" {
                break;
            }
            if function_name(lines[candidate]).is_some() {
                signature_line = Some(candidate);
                break;
            }
        }
        let Some(signature_line) = signature_line else {
            continue;
        };
        let Some(name) = function_name(lines[signature_line]) else {
            continue;
        };
        let signature_start = line_offsets[signature_line] + lines[signature_line].find("fn ").unwrap_or(0);
        let Some((body_start, body_end)) = function_body(source, signature_start) else {
            continue;
        };

        let header_start = line_offsets[attribute_line];
        let header_end = line_offsets[signature_line];
        tests.push(TestFunction {
            path: path.to_owned(),
            name: name.to_owned(),
            header: source[header_start..header_end].to_owned(),
            body: source[body_start..body_end].to_owned(),
        });
    }
    tests
}

fn collect_rs_files(directory: &Path, output: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, output)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
    Ok(())
}

fn parse_test_registry(line: &str) -> Option<TestRecord> {
    let mut fields = line.splitn(7, '|');
    if fields.next()? != "TEST" {
        return None;
    }
    Some(TestRecord {
        path: fields.next()?.to_owned(),
        name: fields.next()?.to_owned(),
        _old_category: fields.next()?.to_owned(),
        category: fields.next()?.to_owned(),
        prerequisites: fields.next()?.to_owned(),
        reason: fields.next()?.to_owned(),
    })
}

#[derive(Debug)]
struct EnvironmentRead {
    variable: String,
    synthetic_default: bool,
}

fn method_chain(source: &str, mut cursor: usize) -> Vec<(String, String)> {
    let bytes = source.as_bytes();
    let mut methods = Vec::new();
    loop {
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'.') {
            break;
        }
        cursor += 1;
        let name_start = cursor;
        while bytes.get(cursor).is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_') {
            cursor += 1;
        }
        if name_start == cursor {
            break;
        }
        let name = source[name_start..cursor].to_owned();
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'(') {
            break;
        }
        let open = cursor;
        let Some(close) = matching_delimiter(source, open, b'(', b')') else {
            break;
        };
        methods.push((name, source[open + 1..close].trim().to_owned()));
        cursor = close + 1;
    }
    methods
}

fn default_is_synthetic(methods: &[(String, String)]) -> bool {
    methods.iter().any(|(name, argument)| match name.as_str() {
        "unwrap_or_default" => argument.is_empty(),
        "unwrap_or_else" | "or_else" => {
            argument.starts_with("||")
                && (argument.contains("std::env::temp_dir()")
                    || argument.contains("env::temp_dir()")
                    || argument.contains('"'))
        }
        "unwrap_or" => {
            argument.starts_with('"')
                || argument.starts_with("PathBuf::from(")
                || argument.starts_with("std::path::PathBuf::from(")
        }
        _ => false,
    })
}

fn env_reads(body: &str) -> Vec<EnvironmentRead> {
    let mut reads = Vec::new();
    let markers = ["env::var_os", "env::var", "option_env!"];
    let mut occurrences = Vec::new();
    for marker in markers {
        let mut start = 0usize;
        while let Some(found) = body[start..].find(marker) {
            let index = start + found;
            occurrences.push((index, marker));
            start = index + marker.len();
        }
    }
    occurrences.sort_by_key(|entry| entry.0);

    for (index, marker) in occurrences {
        if marker == "env::var" && body[index..].starts_with("env::var_os") {
            continue;
        }
        let open = index + marker.len();
        let Some(relative_open) = body[open..].find('(') else {
            continue;
        };
        let open = open + relative_open;
        let Some(close) = matching_delimiter(body, open, b'(', b')') else {
            continue;
        };
        let argument = body[open + 1..close].trim();
        let mut variable = argument
            .strip_prefix('"')
            .and_then(|value| value.find('"').map(|end| value[..end].to_owned()))
            .unwrap_or_else(|| argument.to_owned());
        if !variable.starts_with('"') && variable.chars().all(|character| character.is_ascii_uppercase() || character == '_') {
            let declaration = format!("const {variable}:");
            if let Some(declaration_start) = body.find(&declaration) {
                if let Some(value_start) = body[declaration_start..].find('"') {
                    let value_start = declaration_start + value_start + 1;
                    if let Some(value_end) = body[value_start..].find('"') {
                        variable = body[value_start..value_start + value_end].to_owned();
                    }
                }
            }
        }
        let methods = method_chain(body, close + 1);
        reads.push(EnvironmentRead {
            variable,
            synthetic_default: default_is_synthetic(&methods),
        });
    }
    reads
}

fn has_silent_optional_return(body: &str) -> bool {
    let mut cursor = 0usize;
    while let Some(relative) = body[cursor..].find("is_none()") {
        let found = cursor + relative;
        let Some(open_relative) = body[found..].find('{') else {
            break;
        };
        let open = found + open_relative;
        let Some(close) = matching_delimiter(body, open, b'{', b'}') else {
            break;
        };
        let branch = &body[open + 1..close];
        if branch.contains("return;") && !contains_verification(branch) {
            return true;
        }
        cursor = close + 1;
    }

    cursor = 0;
    while let Some(relative) = body[cursor..].find("let Some") {
        let found = cursor + relative;
        let Some(else_relative) = body[found..].find("else") else {
            break;
        };
        let else_index = found + else_relative;
        let Some(open_relative) = body[else_index..].find('{') else {
            break;
        };
        let open = else_index + open_relative;
        let Some(close) = matching_delimiter(body, open, b'{', b'}') else {
            break;
        };
        let branch = &body[open + 1..close];
        if branch.contains("return;") && !contains_verification(branch) {
            return true;
        }
        cursor = close + 1;
    }
    false
}

fn contains_verification(branch: &str) -> bool {
    [
        "assert!(",
        "assert_eq!(",
        "assert_ne!(",
        "assert_matches!(",
        ".expect(",
        "panic!(",
    ]
    .iter()
    .any(|marker| branch.contains(marker))
}

fn is_env_child_exception(
    exceptions: &HashMap<String, String>,
    test: &TestFunction,
    variable: &str,
) -> bool {
    let key = format!("{}|{}|{}", test.path, test.name, variable);
    exceptions.contains_key(&key)
}

fn test_registry(
    baseline: &str,
) -> (
    Vec<TestRecord>,
    HashMap<String, String>,
    Vec<(String, String, String)>,
) {
    let mut tests = Vec::new();
    let mut child_environment = HashMap::new();
    let mut helper_defaults = Vec::new();
    for line in baseline.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with('#')) {
        if let Some(record) = parse_test_registry(line) {
            tests.push(record);
            continue;
        }
        let fields: Vec<&str> = line.splitn(5, '|').collect();
        match fields.as_slice() {
            ["CHILD_ENV", path, name, variable, reason] => {
                child_environment.insert(
                    format!("{path}|{name}|{variable}"),
                    (*reason).to_owned(),
                );
            }
            ["HELPER_DEFAULT", path, name, variable, reason] => {
                helper_defaults.push((
                    (*path).to_owned(),
                    (*name).to_owned(),
                    (*variable).to_owned(),
                ));
                if variable.contains('*') || variable.contains('?') || reason.is_empty() {
                    eprintln!("invalid wildcard or empty helper exception: {path}::{name}");
                }
            }
            _ => {}
        }
    }
    (tests, child_environment, helper_defaults)
}

fn verify_helper_defaults(
    repo: &Path,
    helpers: &[(String, String, String)],
) -> Vec<String> {
    let mut errors = Vec::new();
    for (path, name, variable) in helpers {
        let source_path = repo.join(path);
        let source = match fs::read_to_string(&source_path) {
            Ok(source) => source,
            Err(error) => {
                errors.push(format!("HELPER missing {path}::{name}: {error}"));
                continue;
            }
        };
        if !helper_has_default(&source, name, variable) {
            errors.push(format!(
                "HELPER synthetic default changed: {path}::{name} {variable}"
            ));
        }
    }
    errors
}

struct CheckSummary {
    tests: usize,
    native_gated: usize,
    registered: usize,
    helper_count: usize,
    environment_errors: usize,
    silent_errors: usize,
    registry_errors: usize,
    helper_errors: usize,
    issues: Vec<String>,
}

impl CheckSummary {
    fn error_count(&self) -> usize {
        self.environment_errors + self.silent_errors + self.registry_errors + self.helper_errors
    }
}

fn analyze_repository(repo: &Path, baseline: &str) -> CheckSummary {
    let (records, child_environment, helper_defaults) = test_registry(baseline);
    let mut source_paths = Vec::new();
    if let Err(error) = collect_rs_files(&repo.join("crates"), &mut source_paths) {
        return CheckSummary {
            tests: 0,
            native_gated: 0,
            registered: records.len(),
            helper_count: helper_defaults.len(),
            environment_errors: 0,
            silent_errors: 0,
            registry_errors: 1,
            helper_errors: 0,
            issues: vec![format!("could not enumerate Rust sources: {error}")],
        };
    }

    let mut discovered = HashMap::new();
    let mut native_count = 0usize;
    let mut environment_errors = 0usize;
    let mut silent_errors = 0usize;
    let mut registry_errors = 0usize;
    let mut issues = Vec::new();
    for source_path in source_paths {
        let relative = source_path
            .strip_prefix(repo)
            .expect("workspace source under repository")
            .to_string_lossy()
            .replace('\\', "/");
        let source = match fs::read_to_string(&source_path) {
            Ok(source) => source,
            Err(error) => {
                issues.push(format!("could not read {}: {error}", source_path.display()));
                registry_errors += 1;
                continue;
            }
        };
        for test in discover_tests(&relative, &source) {
            let key = format!("{}|{}", test.path, test.name);
            if discovered.contains_key(&key) {
                issues.push(format!("duplicate source test identity: {key}"));
                registry_errors += 1;
            }
            let native_gate = test.header.contains("feature = \"native-tests\"")
                && test.header.contains("ignore =");
            let feature_only = test.header.contains("#[cfg(")
                && test.header.contains("feature =")
                && !native_gate;
            let ignored = test.header.contains("#[ignore") || native_gate;
            if native_gate {
                native_count += 1;
            }
            discovered.insert(key.clone(), (native_gate, feature_only, ignored));

            if test.body.contains("NOT RUN") {
                issues.push(format!("SILENT-RETURN: {key} contains NOT RUN"));
                silent_errors += 1;
            }
            for call in macro_calls(&test.body, "eprintln!") {
                let lower = call.to_ascii_lowercase();
                if lower.contains("skip") || lower.contains("skipping") {
                    issues.push(format!("SILENT-RETURN: {key} prints a skip marker"));
                    silent_errors += 1;
                }
            }
            if has_silent_optional_return(&test.body) {
                issues.push(format!(
                    "SILENT-RETURN: {key} returns from a missing-input branch"
                ));
                silent_errors += 1;
            }

            if ignored || feature_only {
                continue;
            }
            for read in env_reads(&test.body) {
                let variable = read.variable;
                if [
                    "CARGO_MANIFEST_DIR",
                    "CARGO_BIN_EXE_whisper-worker-cpu",
                    "CARGO_BIN_EXE_whisper-worker-gpu",
                    "CARGO_BIN_EXE_whisper-worker-parakeet-cpu",
                ]
                .contains(&variable.as_str())
                {
                    continue;
                }
                if read.synthetic_default
                    || is_env_child_exception(&child_environment, &test, &variable)
                {
                    continue;
                }
                issues.push(format!(
                    "ENVIRONMENT: {}::{} reads {} without a synthetic default or validated child exception",
                    test.path, test.name, variable
                ));
                environment_errors += 1;
            }
        }
    }

    let mut registered = HashSet::new();
    for record in &records {
        let key = format!("{}|{}", record.path, record.name);
        if !registered.insert(key.clone()) {
            issues.push(format!("REGISTRY duplicate entry: {key}"));
            registry_errors += 1;
            continue;
        }
        let Some((native_gate, _, ignored)) = discovered.get(&key) else {
            issues.push(format!("REGISTRY test not found: {key}"));
            registry_errors += 1;
            continue;
        };
        if record.reason.is_empty() {
            issues.push(format!("REGISTRY reason missing: {key}"));
            registry_errors += 1;
        }
        if record.category == "NATIF" && !native_gate && !ignored {
            issues.push(format!("REGISTRY native test is not explicitly ignored by default: {key}"));
            registry_errors += 1;
        }
        if record.category == "AUTONOME" && *native_gate {
            issues.push(format!("REGISTRY autonomous test is behind native-tests: {key}"));
            registry_errors += 1;
        }
        if record.category == "PENDING_USER" {
            issues.push(format!("REGISTRY pending user decision: {key}"));
            registry_errors += 1;
        }
        if !["AUTONOME", "NATIF", "PENDING_USER"].contains(&record.category.as_str()) {
            issues.push(format!("REGISTRY invalid category {}: {key}", record.category));
            registry_errors += 1;
        }
        let _ = &record.prerequisites;
    }
    if records.len() != discovered.len() {
        issues.push(format!(
            "REGISTRY count mismatch: records={} source_tests={}",
            records.len(),
            discovered.len()
        ));
        registry_errors += 1;
    }

    let helper_errors_list = verify_helper_defaults(repo, &helper_defaults);
    let helper_errors = helper_errors_list.len();
    issues.extend(helper_errors_list);
    CheckSummary {
        tests: discovered.len(),
        native_gated: native_count,
        registered: records.len(),
        helper_count: helper_defaults.len(),
        environment_errors,
        silent_errors,
        registry_errors,
        helper_errors,
        issues,
    }
}

fn helper_has_default(source: &str, name: &str, variable: &str) -> bool {
    let marker = format!("fn {name}");
    let Some(start) = source.find(&marker) else {
        return false;
    };
    let Some((body_start, end)) = function_body(source, start) else {
        return false;
    };
    let reads = env_reads(&source[body_start..end]);
    reads.iter().any(|read| read.variable == variable && read.synthetic_default)
}

fn run_checker_self_tests(root: &Path) -> std::io::Result<()> {
    fs::create_dir_all(root)?;

    let helper_missing = root.join("helper-missing-default.rs");
    fs::write(
        &helper_missing,
        "fn root() { std::env::var_os(\"SYNTHETIC_ROOT\"); }",
    )?;
    let missing_source = fs::read_to_string(&helper_missing)?;
    if helper_has_default(&missing_source, "root", "SYNTHETIC_ROOT") {
        return Err(std::io::Error::other("helper without a default was accepted"));
    }

    let helper_valid = root.join("helper-temp-default.rs");
    fs::write(
        &helper_valid,
        "fn root() { std::env::var_os(\"SYNTHETIC_ROOT\").unwrap_or_else(|| std::env::temp_dir().into_os_string()); }",
    )?;
    let valid_source = fs::read_to_string(&helper_valid)?;
    if !helper_has_default(&valid_source, "root", "SYNTHETIC_ROOT") {
        return Err(std::io::Error::other("temporary-directory default was rejected"));
    }

    let silent_return = root.join("silent-is-none-return.rs");
    fs::write(
        &silent_return,
        "fn sample() { if std::env::var_os(\"INPUT\").is_none() { return; } }",
    )?;
    let silent_source = fs::read_to_string(&silent_return)?;
    let Some((body_start, body_end)) = function_body(&silent_source, 0) else {
        return Err(std::io::Error::other("could not parse is_none fixture"));
    };
    if !has_silent_optional_return(&silent_source[body_start..body_end]) {
        return Err(std::io::Error::other("is_none return was not detected"));
    }

    let unrelated_default = root.join("unrelated-default.rs");
    fs::write(
        &unrelated_default,
        "fn sample() { let _input = std::env::var(\"INPUT\"); let _fallback = other.unwrap_or_else(|| std::env::temp_dir()); }",
    )?;
    let unrelated_source = fs::read_to_string(&unrelated_default)?;
    if env_reads(&unrelated_source)
        .iter()
        .any(|read| read.variable == "INPUT" && read.synthetic_default)
    {
        return Err(std::io::Error::other(
            "unrelated unwrap_or_else was accepted as an environment default",
        ));
    }

    let global_baseline = concat!(
        "HELPER_DEFAULT|crates/fixture.rs|root|REQUIRED_ROOT|synthetic temp-dir fallback\n",
        "TEST|crates/fixture.rs|calls_helper|AUTONOME|AUTONOME||synthetic checker fixture\n",
    );
    let missing_helper_repo = root.join("global-helper-missing");
    write_global_fixture(
        &missing_helper_repo,
        concat!(
            "#[test]\nfn calls_helper() { let _ = root(); }\n",
            "fn root() -> std::path::PathBuf { std::env::var_os(\"REQUIRED_ROOT\").unwrap().into() }\n",
        ),
    )?;
    let missing_helper_result = analyze_repository(&missing_helper_repo, global_baseline);
    if missing_helper_result.helper_errors == 0 {
        return Err(std::io::Error::other(
            "global checker accepted an un-defaulted env read hidden in a called helper",
        ));
    }

    let valid_helper_repo = root.join("global-helper-temp-default");
    write_global_fixture(
        &valid_helper_repo,
        concat!(
            "#[test]\nfn calls_helper() { let _ = root(); }\n",
            "fn root() -> std::path::PathBuf { std::env::var_os(\"REQUIRED_ROOT\").map(std::path::PathBuf::from).unwrap_or_else(|| std::env::temp_dir()) }\n",
        ),
    )?;
    let valid_helper_result = analyze_repository(&valid_helper_repo, global_baseline);
    if valid_helper_result.error_count() != 0 {
        return Err(std::io::Error::other(format!(
            "global checker rejected helper with temp-dir default: {:?}",
            valid_helper_result.issues
        )));
    }

    let tuple_repo = root.join("global-tuple-unrelated-default");
    write_global_fixture(
        &tuple_repo,
        concat!(
            "#[test]\nfn calls_helper() {\n",
            "let pair = (std::env::var_os(\"REQUIRED\").expect(\"required\"), other.unwrap_or_else(|| std::env::temp_dir()));\n",
            "drop(pair);\n}\n",
        ),
    )?;
    let tuple_baseline =
        "TEST|crates/fixture.rs|calls_helper|AUTONOME|AUTONOME||tuple association fixture\n";
    let tuple_result = analyze_repository(&tuple_repo, tuple_baseline);
    if tuple_result.environment_errors == 0 {
        return Err(std::io::Error::other(
            "global checker accepted an unrelated tuple fallback for a required env read",
        ));
    }
    Ok(())
}

fn write_global_fixture(root: &Path, source: &str) -> std::io::Result<()> {
    let source_dir = root.join("crates");
    fs::create_dir_all(&source_dir)?;
    fs::write(source_dir.join("fixture.rs"), source)
}

fn main() {
    let mut args = env::args().skip(1);
    let first_arg = args.next().expect("usage: check-structure <baseline>");
    if first_arg == "--self-test" {
        let root = PathBuf::from(args.next().expect("self-test root is required"));
        run_checker_self_tests(&root).expect("checker self-tests failed");
        println!("PASS global checker fixtures: helper missing/temp defaults, is_none return and unrelated tuple fallback");
        return;
    }
    let baseline = fs::read_to_string(&first_arg).expect("read test baseline");
    let current_dir = env::current_dir().expect("repository working directory");
    let summary = analyze_repository(current_dir.as_path(), &baseline);
    if summary.error_count() != 0 {
        for issue in &summary.issues {
            eprintln!("{issue}");
        }
        eprintln!(
            "FAIL structure checks: tests={} native-gated={} environment-errors={} silent-errors={} registry-errors={} helper-errors={}",
            summary.tests,
            summary.native_gated,
            summary.environment_errors,
            summary.silent_errors,
            summary.registry_errors,
            summary.helper_errors
        );
        std::process::exit(1);
    }
    println!(
        "PASS structure checks: tests={} native-gated={} registry={} synthetic-env-helpers={} silent-return-patterns=0",
        summary.tests,
        summary.native_gated,
        summary.registered,
        summary.helper_count
    );
}

fn macro_calls(source: &str, macro_name: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let mut cursor = 0usize;
    while let Some(found) = source[cursor..].find(macro_name) {
        let start = cursor + found + macro_name.len();
        let Some(open_relative) = source[start..].find('(') else {
            break;
        };
        let open = start + open_relative;
        let Some(close) = matching_delimiter(source, open, b'(', b')') else {
            break;
        };
        calls.push(source[open + 1..close].to_owned());
        cursor = close + 1;
    }
    calls
}
