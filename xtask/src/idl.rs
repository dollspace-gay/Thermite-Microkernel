use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

pub struct GeneratedIdl {
    pub rust: String,
    pub c: String,
    pub canonical: String,
}

#[derive(Clone)]
struct TypeInfo {
    size: u64,
    align: u64,
}

struct Field {
    name: String,
    type_name: String,
    count: u64,
    offset: u64,
}

struct StructDef {
    name: String,
    size: u64,
    align: u64,
    fields: Vec<Field>,
}

pub fn generate_file(source: &Path, output: &Path) -> Result<GeneratedIdl, String> {
    let text = fs::read_to_string(source)
        .map_err(|error| format!("read IDL {}: {error}", source.display()))?;
    let value: Value =
        serde_json::from_str(&text).map_err(|error| format!("parse IDL: {error}"))?;
    let generated = generate_value(&value)?;
    fs::create_dir_all(output)
        .map_err(|error| format!("create IDL output {}: {error}", output.display()))?;
    fs::write(output.join("kernel_abi.rs"), &generated.rust)
        .map_err(|error| format!("write generated Rust ABI: {error}"))?;
    fs::write(output.join("kernel_abi.h"), &generated.c)
        .map_err(|error| format!("write generated C ABI: {error}"))?;
    fs::write(
        output.join("kernel_abi.canonical.json"),
        &generated.canonical,
    )
    .map_err(|error| format!("write canonical ABI IDL: {error}"))?;
    Ok(generated)
}

pub fn check_outputs(expected: &GeneratedIdl, output: &Path) -> Result<(), String> {
    let expected_names = BTreeSet::from([
        "kernel_abi.canonical.json".to_string(),
        "kernel_abi.h".to_string(),
        "kernel_abi.rs".to_string(),
    ]);
    let mut actual_names = BTreeSet::new();
    for entry in fs::read_dir(output)
        .map_err(|error| format!("read generated IDL directory {}: {error}", output.display()))?
    {
        let entry = entry.map_err(|error| format!("read generated IDL entry: {error}"))?;
        if !entry
            .file_type()
            .map_err(|error| format!("read generated IDL entry type: {error}"))?
            .is_file()
        {
            return Err(format!(
                "generated IDL directory contains non-file {}",
                entry.path().display()
            ));
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "generated IDL filename is not UTF-8".to_string())?;
        actual_names.insert(name);
    }
    if actual_names != expected_names {
        return Err(format!(
            "generated IDL file set is {actual_names:?}, expected {expected_names:?}"
        ));
    }
    for (name, contents) in [
        ("kernel_abi.rs", expected.rust.as_bytes()),
        ("kernel_abi.h", expected.c.as_bytes()),
        ("kernel_abi.canonical.json", expected.canonical.as_bytes()),
    ] {
        let path = output.join(name);
        let actual = fs::read(&path)
            .map_err(|error| format!("read generated IDL output {}: {error}", path.display()))?;
        if actual != contents {
            return Err(format!(
                "generated IDL output {name} differs from the canonical generator result"
            ));
        }
    }
    Ok(())
}

pub fn generate_value(root: &Value) -> Result<GeneratedIdl, String> {
    let root = object(root, "IDL root")?;
    expect_exact_keys(
        root,
        &[
            "abi",
            "bitfields",
            "limits",
            "operations",
            "schema",
            "statuses",
            "structs",
            "syscalls",
        ],
        "IDL root",
    )?;
    if string(field(root, "schema", "IDL root")?, "schema")? != "tmk.kernel-idl.v1" {
        return Err("unsupported IDL schema".to_string());
    }

    let abi = object(field(root, "abi", "IDL root")?, "abi")?;
    expect_exact_keys(
        abi,
        &["endian", "major", "minor", "utcb_magic", "word_bits"],
        "abi",
    )?;
    let major = unsigned(field(abi, "major", "abi")?, "abi.major")?;
    let minor = unsigned(field(abi, "minor", "abi")?, "abi.minor")?;
    let word_bits = unsigned(field(abi, "word_bits", "abi")?, "abi.word_bits")?;
    let utcb_magic = unsigned(field(abi, "utcb_magic", "abi")?, "abi.utcb_magic")?;
    if major != 1
        || word_bits != 64
        || string(field(abi, "endian", "abi")?, "abi.endian")? != "little"
    {
        return Err("M0 supports only little-endian 64-bit ABI major 1".to_string());
    }
    if minor > u16::MAX as u64 || utcb_magic > u32::MAX as u64 {
        return Err("ABI version or UTCB magic exceeds its wire type".to_string());
    }

    let limits = object(field(root, "limits", "IDL root")?, "limits")?;
    expect_exact_keys(
        limits,
        &[
            "fast_words",
            "message_words",
            "receive_slots",
            "send_caps",
            "utcb_bytes",
            "utcb_page_alignment",
        ],
        "limits",
    )?;
    let mut limit_values = BTreeMap::new();
    for (name, value) in limits {
        validate_snake_identifier(name, "limit")?;
        limit_values.insert(name.clone(), unsigned(value, &format!("limits.{name}"))?);
    }

    let syscalls = number_constants(field(root, "syscalls", "IDL root")?, "syscalls")?;
    require_dense_numbers(&syscalls, "syscalls")?;
    let statuses = number_constants(field(root, "statuses", "IDL root")?, "statuses")?;
    require_dense_numbers(&statuses, "statuses")?;
    let operations = operations(field(root, "operations", "IDL root")?)?;
    let bitfields = bitfields(field(root, "bitfields", "IDL root")?)?;
    let structs = structs(field(root, "structs", "IDL root")?)?;

    let utcb = structs
        .iter()
        .find(|item| item.name == "TmkUtcbV1")
        .ok_or_else(|| "IDL must define TmkUtcbV1".to_string())?;
    if limit_values.get("utcb_bytes") != Some(&utcb.size) {
        return Err("limits.utcb_bytes must equal sizeof(TmkUtcbV1)".to_string());
    }
    if !limit_values
        .get("utcb_page_alignment")
        .is_some_and(|value| value.is_power_of_two() && *value >= utcb.align)
    {
        return Err(
            "UTCB page alignment must be a power of two covering struct alignment".to_string(),
        );
    }

    let rust = render_rust(
        major,
        minor,
        utcb_magic,
        &limit_values,
        &syscalls,
        &statuses,
        &operations,
        &bitfields,
        &structs,
    );
    let c = render_c(
        major,
        minor,
        utcb_magic,
        &limit_values,
        &syscalls,
        &statuses,
        &operations,
        &bitfields,
        &structs,
    );
    let canonical = format!(
        "{}\n",
        serde_json::to_string_pretty(&Value::Object(root.clone()))
            .map_err(|error| format!("canonicalize IDL: {error}"))?
    );
    Ok(GeneratedIdl { rust, c, canonical })
}

fn number_constants(value: &Value, label: &str) -> Result<Vec<(String, u64)>, String> {
    let mut names = BTreeSet::new();
    let mut numbers = BTreeSet::new();
    let mut result = Vec::new();
    for (index, item) in array(value, label)?.iter().enumerate() {
        let item_label = format!("{label}[{index}]");
        let item = object(item, &item_label)?;
        expect_exact_keys(item, &["name", "number"], &item_label)?;
        let name = string(
            field(item, "name", &item_label)?,
            &format!("{item_label}.name"),
        )?;
        validate_const_identifier(name, &item_label)?;
        let number = unsigned(
            field(item, "number", &item_label)?,
            &format!("{item_label}.number"),
        )?;
        if !names.insert(name.to_string()) || !numbers.insert(number) {
            return Err(format!("{label} contains a duplicate name or number"));
        }
        result.push((name.to_string(), number));
    }
    if result.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(result)
}

fn require_dense_numbers(values: &[(String, u64)], label: &str) -> Result<(), String> {
    let mut numbers: Vec<_> = values.iter().map(|(_, number)| *number).collect();
    numbers.sort_unstable();
    for (expected, actual) in numbers.into_iter().enumerate() {
        if actual != expected as u64 {
            return Err(format!("{label} numbers must be dense from zero"));
        }
    }
    Ok(())
}

fn operations(value: &Value) -> Result<Vec<(String, String, u64)>, String> {
    let mut keys = BTreeSet::new();
    let mut result: Vec<(String, String, u64)> = Vec::new();
    for (index, item) in array(value, "operations")?.iter().enumerate() {
        let label = format!("operations[{index}]");
        let item = object(item, &label)?;
        expect_exact_keys(item, &["name", "namespace", "number"], &label)?;
        let namespace = string(field(item, "namespace", &label)?, "operation namespace")?;
        let name = string(field(item, "name", &label)?, "operation name")?;
        validate_const_identifier(namespace, &label)?;
        validate_const_identifier(name, &label)?;
        let number = unsigned(field(item, "number", &label)?, "operation number")?;
        if !keys.insert((namespace.to_string(), name.to_string(), number))
            || result
                .iter()
                .any(|(other_namespace, other_name, other_number)| {
                    other_namespace == namespace && (other_name == name || *other_number == number)
                })
        {
            return Err(format!("{label} duplicates an operation name or number"));
        }
        result.push((namespace.to_string(), name.to_string(), number));
    }
    if result.is_empty() {
        return Err("operations must not be empty".to_string());
    }
    let mut by_namespace: BTreeMap<&str, Vec<u64>> = BTreeMap::new();
    for (namespace, _, number) in &result {
        by_namespace.entry(namespace).or_default().push(*number);
    }
    for (namespace, numbers) in &mut by_namespace {
        numbers.sort_unstable();
        if numbers
            .iter()
            .enumerate()
            .any(|(expected, actual)| *actual != expected as u64)
        {
            return Err(format!(
                "operations in namespace `{namespace}` must be dense from zero"
            ));
        }
    }
    Ok(result)
}

#[derive(Clone)]
struct BitFieldMember {
    name: String,
    lsb: u64,
    width: u64,
    reserved: bool,
}

#[derive(Clone)]
struct BitFieldDef {
    name: String,
    bits: u64,
    fields: Vec<BitFieldMember>,
}

fn bitfields(value: &Value) -> Result<Vec<BitFieldDef>, String> {
    let mut names = BTreeSet::new();
    let mut result = Vec::new();
    for (index, item) in array(value, "bitfields")?.iter().enumerate() {
        let label = format!("bitfields[{index}]");
        let item = object(item, &label)?;
        expect_exact_keys(item, &["bits", "fields", "name"], &label)?;
        let name = string(field(item, "name", &label)?, "bitfield name")?;
        validate_snake_identifier(name, &label)?;
        if !names.insert(name.to_string()) {
            return Err(format!("duplicate bitfield `{name}`"));
        }
        let bits = unsigned(field(item, "bits", &label)?, "bitfield width")?;
        if bits != 64 {
            return Err(format!("{label} must be a 64-bit word"));
        }
        let mut next = 0;
        let mut field_names = BTreeSet::new();
        let mut fields = Vec::new();
        for (field_index, member) in array(field(item, "fields", &label)?, "bitfield fields")?
            .iter()
            .enumerate()
        {
            let member_label = format!("{label}.fields[{field_index}]");
            let member = object(member, &member_label)?;
            expect_allowed_keys(member, &["lsb", "name", "reserved", "width"], &member_label)?;
            for required in ["name", "lsb", "width"] {
                field(member, required, &member_label)?;
            }
            let member_name = string(field(member, "name", &member_label)?, "bitfield member")?;
            validate_snake_identifier(member_name, &member_label)?;
            if !field_names.insert(member_name.to_string()) {
                return Err(format!("duplicate bitfield member `{member_name}`"));
            }
            let lsb = unsigned(field(member, "lsb", &member_label)?, "bitfield lsb")?;
            let width = unsigned(field(member, "width", &member_label)?, "bitfield width")?;
            if width == 0 || lsb != next || lsb + width > bits {
                return Err(format!(
                    "{member_label} overlaps, leaves a gap, or exceeds its word"
                ));
            }
            next += width;
            let reserved = member
                .get("reserved")
                .map(|value| boolean(value, "reserved"))
                .transpose()?
                .unwrap_or(false);
            fields.push(BitFieldMember {
                name: member_name.to_string(),
                lsb,
                width,
                reserved,
            });
        }
        if next != bits {
            return Err(format!("{label} does not cover all {bits} bits"));
        }
        result.push(BitFieldDef {
            name: name.to_string(),
            bits,
            fields,
        });
    }
    Ok(result)
}

fn structs(value: &Value) -> Result<Vec<StructDef>, String> {
    let mut types = BTreeMap::from([
        ("u8".to_string(), TypeInfo { size: 1, align: 1 }),
        ("u16".to_string(), TypeInfo { size: 2, align: 2 }),
        ("u32".to_string(), TypeInfo { size: 4, align: 4 }),
        ("u64".to_string(), TypeInfo { size: 8, align: 8 }),
    ]);
    let mut names = BTreeSet::new();
    let mut result = Vec::new();
    for (index, item) in array(value, "structs")?.iter().enumerate() {
        let label = format!("structs[{index}]");
        let item = object(item, &label)?;
        expect_exact_keys(item, &["align", "fields", "name", "size"], &label)?;
        let name = string(field(item, "name", &label)?, "struct name")?;
        validate_type_identifier(name, &label)?;
        if !names.insert(name.to_string()) || types.contains_key(name) {
            return Err(format!("duplicate struct `{name}`"));
        }
        let size = unsigned(field(item, "size", &label)?, "struct size")?;
        let align = unsigned(field(item, "align", &label)?, "struct alignment")?;
        if !align.is_power_of_two() || align > 4096 || size == 0 {
            return Err(format!("{label} has invalid size/alignment"));
        }
        let mut offset = 0;
        let mut max_align = 1;
        let mut field_names = BTreeSet::new();
        let mut fields = Vec::new();
        for (field_index, member) in array(field(item, "fields", &label)?, "struct fields")?
            .iter()
            .enumerate()
        {
            let member_label = format!("{label}.fields[{field_index}]");
            let member = object(member, &member_label)?;
            expect_allowed_keys(member, &["count", "name", "offset", "type"], &member_label)?;
            for required in ["name", "offset", "type"] {
                field(member, required, &member_label)?;
            }
            let member_name = string(field(member, "name", &member_label)?, "field name")?;
            validate_snake_identifier(member_name, &member_label)?;
            if !field_names.insert(member_name.to_string()) {
                return Err(format!("duplicate field `{member_name}` in `{name}`"));
            }
            let type_name = string(field(member, "type", &member_label)?, "field type")?;
            let type_info = types.get(type_name).ok_or_else(|| {
                format!("{member_label} uses unknown or forward type `{type_name}`")
            })?;
            let count = member
                .get("count")
                .map(|value| unsigned(value, "field count"))
                .transpose()?
                .unwrap_or(1);
            if count == 0 {
                return Err(format!("{member_label} has zero count"));
            }
            let declared_offset =
                unsigned(field(member, "offset", &member_label)?, "field offset")?;
            let expected_offset = align_up(offset, type_info.align)?;
            if declared_offset != expected_offset {
                return Err(format!(
                    "{member_label} offset is {declared_offset}, expected {expected_offset}; padding must be explicit"
                ));
            }
            let field_size = type_info
                .size
                .checked_mul(count)
                .ok_or_else(|| format!("{member_label} size overflow"))?;
            offset = declared_offset
                .checked_add(field_size)
                .ok_or_else(|| format!("{member_label} end overflow"))?;
            max_align = max_align.max(type_info.align);
            fields.push(Field {
                name: member_name.to_string(),
                type_name: type_name.to_string(),
                count,
                offset: declared_offset,
            });
        }
        if align != max_align || size != align_up(offset, align)? {
            return Err(format!(
                "{label} declares size/alignment {size}/{align}, computed {}/{}",
                align_up(offset, align)?,
                max_align
            ));
        }
        types.insert(name.to_string(), TypeInfo { size, align });
        result.push(StructDef {
            name: name.to_string(),
            size,
            align,
            fields,
        });
    }
    Ok(result)
}

fn render_rust(
    major: u64,
    minor: u64,
    magic: u64,
    limits: &BTreeMap<String, u64>,
    syscalls: &[(String, u64)],
    statuses: &[(String, u64)],
    operations: &[(String, String, u64)],
    bitfields: &[BitFieldDef],
    structs: &[StructDef],
) -> String {
    let mut out = String::new();
    writeln!(
        out,
        "// @generated by `cargo run -p xtask -- m0-idl`; do not edit."
    )
    .unwrap();
    writeln!(out, "pub const TMK_ABI_MAJOR: u16 = {major};").unwrap();
    writeln!(out, "pub const TMK_ABI_MINOR: u16 = {minor};").unwrap();
    writeln!(out, "pub const TMK_UTCB_MAGIC: u32 = {magic};").unwrap();
    for (name, value) in limits {
        writeln!(out, "pub const TMK_LIMIT_{}: usize = {value};", upper(name)).unwrap();
    }
    writeln!(out, "pub type TmkSyscallNumber = u64;").unwrap();
    for (name, number) in syscalls {
        writeln!(out, "pub const {name}: TmkSyscallNumber = {number};").unwrap();
    }
    writeln!(out, "pub type TmkKernelStatus = u64;").unwrap();
    for (name, number) in statuses {
        writeln!(out, "pub const {name}: TmkKernelStatus = {number};").unwrap();
    }
    writeln!(out, "pub type TmkInvocationOperation = u64;").unwrap();
    for (namespace, name, number) in operations {
        writeln!(
            out,
            "pub const TMK_OP_{namespace}_{name}: TmkInvocationOperation = {number};"
        )
        .unwrap();
    }
    render_rust_bitfields(&mut out, bitfields);
    for item in structs {
        writeln!(out, "#[repr(C)]").unwrap();
        writeln!(out, "#[derive(Clone, Copy)]").unwrap();
        writeln!(out, "pub struct {} {{", item.name).unwrap();
        for field in &item.fields {
            let type_name = rust_type(&field.type_name);
            if field.count == 1 {
                writeln!(out, "    pub {}: {type_name},", field.name).unwrap();
            } else {
                writeln!(
                    out,
                    "    pub {}: [{type_name}; {}],",
                    field.name, field.count
                )
                .unwrap();
            }
        }
        writeln!(out, "}}").unwrap();
        writeln!(out, "const _: () = {{").unwrap();
        writeln!(
            out,
            "    assert!(core::mem::size_of::<{}>() == {});",
            item.name, item.size
        )
        .unwrap();
        writeln!(
            out,
            "    assert!(core::mem::align_of::<{}>() == {});",
            item.name, item.align
        )
        .unwrap();
        for field in &item.fields {
            writeln!(
                out,
                "    assert!(core::mem::offset_of!({}, {}) == {});",
                item.name, field.name, field.offset
            )
            .unwrap();
        }
        writeln!(out, "}};").unwrap();
    }
    out
}

fn render_rust_bitfields(out: &mut String, bitfields: &[BitFieldDef]) {
    for bitfield in bitfields {
        let prefix = format!("TMK_{}", upper(&bitfield.name));
        writeln!(out, "pub const {prefix}_BITS: u32 = {};", bitfield.bits).unwrap();
        let mut reserved_mask = 0u64;
        for field in &bitfield.fields {
            let field_prefix = format!("{prefix}_{}", upper(&field.name));
            let unshifted = mask(field.width);
            let shifted = unshifted << field.lsb;
            writeln!(out, "pub const {field_prefix}_SHIFT: u32 = {};", field.lsb).unwrap();
            writeln!(
                out,
                "pub const {field_prefix}_WIDTH: u32 = {};",
                field.width
            )
            .unwrap();
            writeln!(
                out,
                "pub const {field_prefix}_MASK: u64 = 0x{shifted:016x};"
            )
            .unwrap();
            writeln!(
                out,
                "pub const fn tmk_{}_{}(value: u64) -> u64 {{ (value & {field_prefix}_MASK) >> {field_prefix}_SHIFT }}",
                bitfield.name, field.name
            )
            .unwrap();
            if field.reserved {
                reserved_mask |= shifted;
            }
        }
        writeln!(
            out,
            "pub const {prefix}_RESERVED_BITS_MASK: u64 = 0x{reserved_mask:016x};"
        )
        .unwrap();
        writeln!(
            out,
            "pub const fn tmk_{}_reserved_zero(value: u64) -> bool {{ value & {prefix}_RESERVED_BITS_MASK == 0 }}",
            bitfield.name
        )
        .unwrap();
    }
}

fn render_c(
    major: u64,
    minor: u64,
    magic: u64,
    limits: &BTreeMap<String, u64>,
    syscalls: &[(String, u64)],
    statuses: &[(String, u64)],
    operations: &[(String, String, u64)],
    bitfields: &[BitFieldDef],
    structs: &[StructDef],
) -> String {
    let mut out = String::new();
    writeln!(
        out,
        "/* @generated by `cargo run -p xtask -- m0-idl`; do not edit. */"
    )
    .unwrap();
    writeln!(out, "#ifndef TMK_KERNEL_ABI_V1_H").unwrap();
    writeln!(out, "#define TMK_KERNEL_ABI_V1_H").unwrap();
    writeln!(out, "#include <stddef.h>").unwrap();
    writeln!(out, "#include <stdint.h>").unwrap();
    writeln!(out, "#define TMK_ABI_MAJOR UINT16_C({major})").unwrap();
    writeln!(out, "#define TMK_ABI_MINOR UINT16_C({minor})").unwrap();
    writeln!(out, "#define TMK_UTCB_MAGIC UINT32_C({magic})").unwrap();
    for (name, value) in limits {
        writeln!(out, "#define TMK_LIMIT_{} UINT64_C({value})", upper(name)).unwrap();
    }
    writeln!(out, "typedef uint64_t TmkSyscallNumber;").unwrap();
    for (name, number) in syscalls {
        writeln!(out, "#define {name} UINT64_C({number})").unwrap();
    }
    writeln!(out, "typedef uint64_t TmkKernelStatus;").unwrap();
    for (name, number) in statuses {
        writeln!(out, "#define {name} UINT64_C({number})").unwrap();
    }
    writeln!(out, "typedef uint64_t TmkInvocationOperation;").unwrap();
    for (namespace, name, number) in operations {
        writeln!(out, "#define TMK_OP_{namespace}_{name} UINT64_C({number})").unwrap();
    }
    for bitfield in bitfields {
        let prefix = format!("TMK_{}", upper(&bitfield.name));
        let mut reserved_mask = 0u64;
        for field in &bitfield.fields {
            let field_prefix = format!("{prefix}_{}", upper(&field.name));
            let shifted = mask(field.width) << field.lsb;
            writeln!(out, "#define {field_prefix}_SHIFT {}u", field.lsb).unwrap();
            writeln!(out, "#define {field_prefix}_WIDTH {}u", field.width).unwrap();
            writeln!(
                out,
                "#define {field_prefix}_MASK UINT64_C(0x{shifted:016x})"
            )
            .unwrap();
            writeln!(
                out,
                "static inline uint64_t tmk_{}_{}(uint64_t value) {{ return (value & {field_prefix}_MASK) >> {field_prefix}_SHIFT; }}",
                bitfield.name, field.name
            )
            .unwrap();
            if field.reserved {
                reserved_mask |= shifted;
            }
        }
        writeln!(
            out,
            "#define {prefix}_RESERVED_BITS_MASK UINT64_C(0x{reserved_mask:016x})"
        )
        .unwrap();
        writeln!(
            out,
            "static inline int tmk_{}_reserved_zero(uint64_t value) {{ return (value & {prefix}_RESERVED_BITS_MASK) == 0; }}",
            bitfield.name
        )
        .unwrap();
    }
    for item in structs {
        writeln!(out, "typedef struct {} {{", item.name).unwrap();
        for field in &item.fields {
            let type_name = c_type(&field.type_name);
            if field.count == 1 {
                writeln!(out, "    {type_name} {};", field.name).unwrap();
            } else {
                writeln!(out, "    {type_name} {}[{}];", field.name, field.count).unwrap();
            }
        }
        writeln!(out, "}} {};", item.name).unwrap();
        writeln!(
            out,
            "_Static_assert(sizeof({}) == {}, \"{} size\");",
            item.name, item.size, item.name
        )
        .unwrap();
        writeln!(
            out,
            "_Static_assert(_Alignof({}) == {}, \"{} alignment\");",
            item.name, item.align, item.name
        )
        .unwrap();
        for field in &item.fields {
            writeln!(
                out,
                "_Static_assert(offsetof({}, {}) == {}, \"{}.{} offset\");",
                item.name, field.name, field.offset, item.name, field.name
            )
            .unwrap();
        }
    }
    writeln!(out, "#endif").unwrap();
    out
}

fn rust_type(name: &str) -> &str {
    name
}

fn c_type(name: &str) -> &str {
    match name {
        "u8" => "uint8_t",
        "u16" => "uint16_t",
        "u32" => "uint32_t",
        "u64" => "uint64_t",
        other => other,
    }
}

fn mask(width: u64) -> u64 {
    if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}

fn align_up(value: u64, align: u64) -> Result<u64, String> {
    value
        .checked_add(align - 1)
        .map(|sum| sum & !(align - 1))
        .ok_or_else(|| "layout alignment overflow".to_string())
}

fn upper(name: &str) -> String {
    name.to_ascii_uppercase()
}

fn validate_const_identifier(name: &str, label: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        || name.as_bytes()[0].is_ascii_digit()
    {
        return Err(format!(
            "{label} contains invalid constant identifier `{name}`"
        ));
    }
    Ok(())
}

fn validate_snake_identifier(name: &str, label: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        || name.as_bytes()[0].is_ascii_digit()
    {
        return Err(format!(
            "{label} contains invalid snake identifier `{name}`"
        ));
    }
    Ok(())
}

fn validate_type_identifier(name: &str, label: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        || !name.as_bytes()[0].is_ascii_uppercase()
    {
        return Err(format!("{label} contains invalid type identifier `{name}`"));
    }
    Ok(())
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

fn expect_exact_keys(
    object: &Map<String, Value>,
    expected: &[&str],
    label: &str,
) -> Result<(), String> {
    expect_allowed_keys(object, expected, label)?;
    for key in expected {
        if !object.contains_key(*key) {
            return Err(format!("{label} is missing `{key}`"));
        }
    }
    Ok(())
}

fn expect_allowed_keys(
    object: &Map<String, Value>,
    allowed: &[&str],
    label: &str,
) -> Result<(), String> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("{label} contains unknown key `{key}`"));
        }
    }
    Ok(())
}
