use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

const ALLOWED_SCHEMA_KEYS: &[&str] = &[
    "$defs",
    "$id",
    "$ref",
    "$schema",
    "additionalProperties",
    "const",
    "description",
    "enum",
    "format",
    "items",
    "maximum",
    "minItems",
    "minLength",
    "minimum",
    "properties",
    "required",
    "title",
    "type",
    "uniqueItems",
];

pub fn validate(schema: &Value, manifest: &Value) -> Result<(), String> {
    validate_schema_node(schema, schema, "#", true)?;
    validate_instance(schema, schema, manifest, "$")?;
    validate_semantics(manifest)
}

pub fn canonical_payload(manifest: &Value) -> Result<String, String> {
    let mut payload = manifest.clone();
    let root = payload
        .as_object_mut()
        .ok_or_else(|| "manifest signing payload must be an object".to_string())?;
    let signing = root
        .get_mut("signing")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "manifest signing payload is missing object `signing`".to_string())?;
    signing
        .remove("payload_sha256")
        .ok_or_else(|| "manifest signing payload is missing `payload_sha256`".to_string())?;
    signing
        .remove("signature")
        .ok_or_else(|| "manifest signing payload is missing `signature`".to_string())?;
    serde_json::to_string(&payload)
        .map(|text| format!("{text}\n"))
        .map_err(|error| format!("canonicalize manifest signing payload: {error}"))
}

pub fn canonical_manifest(manifest: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(manifest)
        .map(|text| format!("{text}\n"))
        .map_err(|error| format!("canonicalize release manifest: {error}"))
}

fn validate_schema_node(
    root: &Value,
    node: &Value,
    path: &str,
    is_root: bool,
) -> Result<(), String> {
    let node = object(node, path)?;
    for key in node.keys() {
        if !ALLOWED_SCHEMA_KEYS.contains(&key.as_str()) {
            return Err(format!(
                "schema node {path} contains unsupported keyword `{key}`"
            ));
        }
    }
    if is_root {
        if string(field(node, "$schema", path)?, "$schema")?
            != "https://json-schema.org/draft/2020-12/schema"
        {
            return Err("manifest schema must use JSON Schema draft 2020-12".to_string());
        }
        if string(field(node, "$id", path)?, "$id")?
            != "https://github.com/dollspace-gay/Thermite-Microkernel/release/manifest.schema.json"
        {
            return Err("manifest schema has an unexpected canonical `$id`".to_string());
        }
        field(node, "$defs", path)?;
    }
    if let Some(reference) = node.get("$ref") {
        let reference = string(reference, &format!("{path}.$ref"))?;
        resolve_reference(root, reference)?;
    }
    if let Some(kind) = node.get("type") {
        let kind = string(kind, &format!("{path}.type"))?;
        if !matches!(kind, "object" | "array" | "string" | "integer" | "boolean") {
            return Err(format!("schema node {path} has unsupported type `{kind}`"));
        }
        if kind == "object" {
            let additional = node
                .get("additionalProperties")
                .and_then(Value::as_bool)
                .ok_or_else(|| {
                    format!("object schema {path} must set `additionalProperties` to false")
                })?;
            if additional {
                return Err(format!(
                    "object schema {path} must reject additional properties"
                ));
            }
            let properties = object(field(node, "properties", path)?, "schema properties")?;
            let required = array(field(node, "required", path)?, "schema required")?;
            let mut required_names = BTreeSet::new();
            for name in required {
                let name = string(name, "required property")?;
                if !properties.contains_key(name) || !required_names.insert(name) {
                    return Err(format!(
                        "object schema {path} has a duplicate or unknown required property `{name}`"
                    ));
                }
            }
            if required_names.len() != properties.len() {
                return Err(format!(
                    "object schema {path} must make every declared property required"
                ));
            }
            for (name, property) in properties {
                validate_schema_node(root, property, &format!("{path}.properties.{name}"), false)?;
            }
        } else if kind == "array" {
            let items = field(node, "items", path)?;
            validate_schema_node(root, items, &format!("{path}.items"), false)?;
            if let Some(unique) = node.get("uniqueItems") {
                boolean(unique, &format!("{path}.uniqueItems"))?;
            }
        }
    }
    if let Some(definitions) = node.get("$defs") {
        for (name, definition) in object(definitions, &format!("{path}.$defs"))? {
            validate_schema_node(root, definition, &format!("{path}.$defs.{name}"), false)?;
        }
    }
    if let Some(values) = node.get("enum") {
        if array(values, &format!("{path}.enum"))?.is_empty() {
            return Err(format!("schema enum {path} must not be empty"));
        }
    }
    for (key, value) in [
        ("minLength", node.get("minLength")),
        ("minimum", node.get("minimum")),
        ("maximum", node.get("maximum")),
        ("minItems", node.get("minItems")),
    ] {
        if let Some(value) = value {
            unsigned(value, &format!("{path}.{key}"))?;
        }
    }
    if let Some(format) = node.get("format") {
        let format = string(format, &format!("{path}.format"))?;
        if !matches!(
            format,
            "tmk-sha256"
                | "tmk-ed25519-signature"
                | "tmk-git-revision"
                | "tmk-identifier"
                | "tmk-version"
        ) {
            return Err(format!(
                "schema node {path} has unsupported format `{format}`"
            ));
        }
    }
    Ok(())
}

fn validate_instance(
    root: &Value,
    schema: &Value,
    instance: &Value,
    path: &str,
) -> Result<(), String> {
    let schema = object(schema, "schema node")?;
    if let Some(reference) = schema.get("$ref") {
        let reference = string(reference, "schema reference")?;
        return validate_instance(root, resolve_reference(root, reference)?, instance, path);
    }
    if let Some(expected) = schema.get("const") {
        if instance != expected {
            return Err(format!("{path} does not equal its required constant"));
        }
    }
    if let Some(choices) = schema.get("enum") {
        if !array(choices, "schema enum")?.contains(instance) {
            return Err(format!("{path} is not an allowed enum value"));
        }
    }
    let Some(kind) = schema.get("type") else {
        return Ok(());
    };
    match string(kind, "schema type")? {
        "object" => {
            let value = object(instance, path)?;
            let properties = object(field(schema, "properties", "object schema")?, "properties")?;
            let required = array(field(schema, "required", "object schema")?, "required")?;
            for name in required {
                let name = string(name, "required property")?;
                if !value.contains_key(name) {
                    return Err(format!("{path} is missing required property `{name}`"));
                }
            }
            if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                for name in value.keys() {
                    if !properties.contains_key(name) {
                        return Err(format!("{path} contains unknown property `{name}`"));
                    }
                }
            }
            for (name, property_schema) in properties {
                if let Some(property) = value.get(name) {
                    validate_instance(root, property_schema, property, &format!("{path}.{name}"))?;
                }
            }
        }
        "array" => {
            let values = array(instance, path)?;
            let minimum = schema.get("minItems").map_or(Ok(0), |value| {
                unsigned(value, "schema minItems").map(|value| value as usize)
            })?;
            if values.len() < minimum {
                return Err(format!("{path} has fewer than {minimum} items"));
            }
            if schema.get("uniqueItems") == Some(&Value::Bool(true)) {
                for index in 0..values.len() {
                    if values[..index].contains(&values[index]) {
                        return Err(format!("{path} contains a duplicate item at index {index}"));
                    }
                }
            }
            let item_schema = field(schema, "items", "array schema")?;
            for (index, value) in values.iter().enumerate() {
                validate_instance(root, item_schema, value, &format!("{path}[{index}]"))?;
            }
        }
        "string" => {
            let value = string(instance, path)?;
            let minimum = schema.get("minLength").map_or(Ok(0), |value| {
                unsigned(value, "schema minLength").map(|value| value as usize)
            })?;
            if value.chars().count() < minimum {
                return Err(format!("{path} is shorter than {minimum} characters"));
            }
            if let Some(format) = schema.get("format") {
                validate_format(string(format, "schema format")?, value, path)?;
            }
        }
        "integer" => {
            let value = unsigned(instance, path)?;
            if let Some(minimum) = schema.get("minimum") {
                let minimum = unsigned(minimum, "schema minimum")?;
                if value < minimum {
                    return Err(format!("{path} is below minimum {minimum}"));
                }
            }
            if let Some(maximum) = schema.get("maximum") {
                let maximum = unsigned(maximum, "schema maximum")?;
                if value > maximum {
                    return Err(format!("{path} exceeds maximum {maximum}"));
                }
            }
        }
        "boolean" => {
            boolean(instance, path)?;
        }
        other => return Err(format!("unsupported schema type `{other}` at {path}")),
    }
    Ok(())
}

fn validate_semantics(manifest: &Value) -> Result<(), String> {
    let root = object(manifest, "manifest")?;
    let release = object(field(root, "release", "manifest")?, "release")?;
    let development = boolean(field(release, "development", "release")?, "development")?;
    let release_eligible = boolean(
        field(release, "release_eligible", "release")?,
        "release_eligible",
    )?;

    ensure_sorted_unique_strings(
        array(
            field(
                object(field(root, "platform", "manifest")?, "platform")?,
                "cpu_features",
                "platform",
            )?,
            "cpu_features",
        )?,
        "platform.cpu_features",
    )?;
    for (field_name, key) in [
        ("repositories", "name"),
        ("tools", "name"),
        ("functions", "semantic_address"),
        ("forge_receipts", "name"),
        ("direct_verus", "name"),
        ("capsules", "name"),
        ("artifacts", "name"),
        ("tests", "name"),
        ("assumptions", "id"),
    ] {
        ensure_sorted_unique_objects(
            array(field(root, field_name, "manifest")?, field_name)?,
            key,
            field_name,
        )?;
    }
    ensure_sorted_unique_strings(
        array(
            field(root, "known_limitations", "manifest")?,
            "known_limitations",
        )?,
        "known_limitations",
    )?;

    let artifacts = array(field(root, "artifacts", "manifest")?, "artifacts")?;
    let mut artifact_digests = BTreeMap::new();
    for artifact in artifacts {
        let artifact = object(artifact, "artifact")?;
        let name = string(field(artifact, "name", "artifact")?, "artifact.name")?;
        let digest = string(field(artifact, "sha256", "artifact")?, "artifact.sha256")?;
        artifact_digests.insert(name, digest);
    }
    let artifact_names: BTreeSet<&str> = artifact_digests.keys().copied().collect();

    let receipts = array(field(root, "forge_receipts", "manifest")?, "forge_receipts")?;
    let direct_verus = array(field(root, "direct_verus", "manifest")?, "direct_verus")?;
    let capsules = array(field(root, "capsules", "manifest")?, "capsules")?;
    let tests = array(field(root, "tests", "manifest")?, "tests")?;
    let mut accepted_bindings = BTreeSet::new();

    for receipt in receipts {
        let receipt = object(receipt, "forge receipt")?;
        require_artifact_reference(receipt, &artifact_names, "forge receipt")?;
        let kind = string(field(receipt, "kind", "forge receipt")?, "receipt.kind")?;
        let schema = string(field(receipt, "schema", "forge receipt")?, "receipt.schema")?;
        let expected_schema = match kind {
            "standalone" => "thermite.verified-build-receipt.v1",
            "composition" => "thermite.verified-composition-receipt.v1",
            _ => unreachable!("schema enum checked"),
        };
        if schema != expected_schema {
            return Err(format!(
                "{kind} receipt uses schema `{schema}`, expected `{expected_schema}`"
            ));
        }
        if !boolean(
            field(receipt, "replay_passed", "forge receipt")?,
            "replay_passed",
        )? {
            return Err("every included Forge receipt must have passed replay".to_string());
        }
        accepted_bindings.insert(string(
            field(receipt, "binding_sha256", "forge receipt")?,
            "binding_sha256",
        )?);
    }

    for result in direct_verus {
        let result = object(result, "direct Verus result")?;
        require_artifact_reference(result, &artifact_names, "direct Verus result")?;
        let artifact_name = string(
            field(result, "artifact_name", "direct Verus result")?,
            "artifact_name",
        )?;
        let result_artifact_sha = string(
            field(result, "artifact_sha256", "direct Verus result")?,
            "artifact_sha256",
        )?;
        if artifact_digests.get(artifact_name) != Some(&result_artifact_sha) {
            return Err(format!(
                "direct Verus result `{}` artifact digest does not match artifact `{artifact_name}`",
                string(field(result, "name", "direct Verus result")?, "result.name")?
            ));
        }
        if unsigned(field(result, "errors", "direct Verus result")?, "errors")? != 0
            || !boolean(
                field(result, "no_cheating", "direct Verus result")?,
                "no_cheating",
            )?
        {
            return Err(
                "every included direct-Verus result must have zero errors and no-cheating"
                    .to_string(),
            );
        }
        accepted_bindings.insert(string(
            field(result, "result_sha256", "direct Verus result")?,
            "result_sha256",
        )?);
    }

    for capsule in capsules {
        let capsule = object(capsule, "capsule")?;
        require_artifact_reference(capsule, &artifact_names, "capsule")?;
        if field(capsule, "emitted_sha256", "capsule")?
            != field(capsule, "linked_sha256", "capsule")?
        {
            return Err("capsule emitted and linked digests must match".to_string());
        }
        accepted_bindings.insert(string(
            field(capsule, "proof_result_sha256", "capsule")?,
            "proof_result_sha256",
        )?);
    }

    for result in tests {
        let result = object(result, "test result")?;
        let status = string(field(result, "status", "test result")?, "test status")?;
        let passed = unsigned(field(result, "passed", "test result")?, "passed")?;
        let failed = unsigned(field(result, "failed", "test result")?, "failed")?;
        let skipped = unsigned(field(result, "skipped", "test result")?, "skipped")?;
        let consistent = match status {
            "pass" => passed > 0 && failed == 0 && skipped == 0,
            "fail" => failed > 0,
            "incomplete" => skipped > 0 && failed == 0,
            _ => unreachable!("schema enum checked"),
        };
        if !consistent {
            return Err(format!(
                "test status `{status}` disagrees with passed/failed/skipped counts"
            ));
        }
        accepted_bindings.insert(string(
            field(result, "result_sha256", "test result")?,
            "result_sha256",
        )?);
    }

    for function in array(field(root, "functions", "manifest")?, "functions")? {
        let function = object(function, "function")?;
        require_artifact_reference(function, &artifact_names, "function")?;
        let origin = string(field(function, "origin", "function")?, "origin")?;
        let assurance = string(field(function, "assurance", "function")?, "assurance")?;
        let scope = string(field(function, "scope", "function")?, "scope")?;
        let valid = match origin {
            "thermite" => matches!(assurance, "l3" | "l4") && scope == "end_to_end",
            "direct_verus" => assurance == "direct_verus" && scope == "whole_body",
            "capsule" => assurance == "capsule_refinement" && scope == "exact_bytes",
            _ => false,
        };
        if !valid {
            return Err(format!(
                "function origin `{origin}` is inconsistent with assurance `{assurance}` and scope `{scope}`"
            ));
        }
    }

    for artifact in artifacts {
        let artifact = object(artifact, "artifact")?;
        let path = string(field(artifact, "path", "artifact")?, "artifact.path")?;
        if path.starts_with('/') || path.split('/').any(|part| part == ".." || part.is_empty()) {
            return Err(format!(
                "artifact path `{path}` is not a normalized relative path"
            ));
        }
        let bindings = array(
            field(artifact, "source_bindings", "artifact")?,
            "source_bindings",
        )?;
        ensure_sorted_unique_strings(bindings, "artifact.source_bindings")?;
        for binding in bindings {
            let binding = string(binding, "artifact source binding")?;
            if !accepted_bindings.contains(binding) {
                return Err(format!(
                    "artifact source binding `{binding}` is not supplied by a receipt, proof, capsule, or test result"
                ));
            }
        }
    }

    let signing = object(field(root, "signing", "manifest")?, "signing")?;
    let key_id = string(field(signing, "key_id", "signing")?, "key_id")?;
    if key_id == "m0-development-test-key" && (!development || release_eligible) {
        return Err(
            "the committed M0 development key cannot authorize a production or release-eligible manifest"
                .to_string(),
        );
    }

    if release_eligible {
        if development {
            return Err("a release-eligible manifest cannot be a development build".to_string());
        }
        for repository in array(field(root, "repositories", "manifest")?, "repositories")? {
            let repository = object(repository, "repository")?;
            if boolean(field(repository, "dirty", "repository")?, "dirty")? {
                return Err(
                    "a release-eligible manifest cannot name a dirty repository".to_string()
                );
            }
        }
        if !receipts.iter().any(|receipt| {
            receipt
                .get("kind")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind == "composition")
        }) {
            return Err("a release-eligible manifest requires a composition receipt".to_string());
        }
        if artifacts
            .iter()
            .filter(|artifact| artifact.get("kind").and_then(Value::as_str) == Some("boot_image"))
            .count()
            != 1
        {
            return Err("a release-eligible manifest requires exactly one boot image".to_string());
        }
        if artifacts
            .iter()
            .filter(|artifact| artifact.get("kind").and_then(Value::as_str) == Some("link_receipt"))
            .count()
            != 1
        {
            return Err(
                "a release-eligible manifest requires exactly one final-link receipt artifact"
                    .to_string(),
            );
        }
        if tests
            .iter()
            .any(|test| test.get("status").and_then(Value::as_str) != Some("pass"))
        {
            return Err("every release test suite must pass without skips".to_string());
        }
    }
    Ok(())
}

fn require_artifact_reference(
    object: &Map<String, Value>,
    artifact_names: &BTreeSet<&str>,
    label: &str,
) -> Result<(), String> {
    let name = string(field(object, "artifact_name", label)?, "artifact_name")?;
    if artifact_names.contains(name) {
        Ok(())
    } else {
        Err(format!("{label} references unknown artifact `{name}`"))
    }
}

fn ensure_sorted_unique_objects(values: &[Value], key: &str, label: &str) -> Result<(), String> {
    let mut previous = None;
    for value in values {
        let value = object(value, label)?;
        let current = string(field(value, key, label)?, &format!("{label}.{key}"))?;
        if previous.is_some_and(|previous| previous >= current) {
            return Err(format!(
                "{label} must be strictly sorted and unique by `{key}`"
            ));
        }
        previous = Some(current);
    }
    Ok(())
}

fn ensure_sorted_unique_strings(values: &[Value], label: &str) -> Result<(), String> {
    let mut previous = None;
    for value in values {
        let current = string(value, label)?;
        if previous.is_some_and(|previous| previous >= current) {
            return Err(format!("{label} must be strictly sorted and unique"));
        }
        previous = Some(current);
    }
    Ok(())
}

fn validate_format(format: &str, value: &str, path: &str) -> Result<(), String> {
    let valid = match format {
        "tmk-sha256" => is_lower_hex(value, 64),
        "tmk-ed25519-signature" => is_lower_hex(value, 128),
        "tmk-git-revision" => is_lower_hex(value, 40),
        "tmk-identifier" => {
            !value.is_empty()
                && !value.starts_with(['-', '.', '_'])
                && value.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'-' | b'.' | b'_')
                })
        }
        "tmk-version" => {
            !value.is_empty()
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'+'))
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(format!("{path} does not satisfy format `{format}`"))
    }
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn resolve_reference<'a>(root: &'a Value, reference: &str) -> Result<&'a Value, String> {
    let name = reference.strip_prefix("#/$defs/").ok_or_else(|| {
        format!("only local `$defs` references are supported, found `{reference}`")
    })?;
    if name.is_empty() || name.contains('/') {
        return Err(format!("invalid local schema reference `{reference}`"));
    }
    root.pointer(&format!("/$defs/{name}"))
        .ok_or_else(|| format!("schema reference `{reference}` does not resolve"))
}

fn object<'a>(value: &'a Value, label: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))
}

fn array<'a>(value: &'a Value, label: &str) -> Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{label} must be an array"))
}

fn string<'a>(value: &'a Value, label: &str) -> Result<&'a str, String> {
    value
        .as_str()
        .ok_or_else(|| format!("{label} must be a string"))
}

fn unsigned(value: &Value, label: &str) -> Result<u64, String> {
    value
        .as_u64()
        .ok_or_else(|| format!("{label} must be an unsigned integer"))
}

fn boolean(value: &Value, label: &str) -> Result<bool, String> {
    value
        .as_bool()
        .ok_or_else(|| format!("{label} must be a boolean"))
}

fn field<'a>(object: &'a Map<String, Value>, name: &str, label: &str) -> Result<&'a Value, String> {
    object
        .get(name)
        .ok_or_else(|| format!("{label} is missing `{name}`"))
}
