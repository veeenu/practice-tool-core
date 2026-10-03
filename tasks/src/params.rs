use std::collections::{BTreeMap, HashSet};
use std::env;
use std::ffi::OsStr;
use std::fmt::Write;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;

use anyhow::{bail, ensure, Context, Result};
use once_cell::sync::Lazy;
use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};
use regex::Regex;

use crate::project_root;

pub fn codegen_param_names<P1: AsRef<Path>, P2: AsRef<Path>>(
    paramdex_path: P1,
    dest_path: P2,
) -> Result<()> {
    let mut data: BTreeMap<String, BTreeMap<usize, String>> = BTreeMap::new();

    let files_with_content = project_root()
        .join(paramdex_path)
        .read_dir()?
        .flat_map(|entry| {
            entry.map(|entry| entry.path()).map(|path| {
                if path.is_file() && Some("txt") == path.extension().and_then(OsStr::to_str) {
                    Some(path)
                } else {
                    None
                }
            })
        })
        .flatten();

    let r = Regex::new(r"^(\d+)\s+(.+)").unwrap();

    for path in files_with_content {
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();

        let data_contents: BTreeMap<_, _> = BufReader::new(File::open(path)?)
            .lines()
            .filter_map(|line| {
                let line = line.ok()?;
                let cap = r.captures(&line)?;

                let id: usize = cap[1].parse().ok()?;
                let name: String = cap[2].to_string();
                Some((id, name))
            })
            .collect();

        data.insert(stem, data_contents);
    }

    serde_json::to_writer_pretty(File::create(project_root().join(dest_path))?, &data)?;

    Ok(())
}

pub fn checkout_paramdex() -> Result<()> {
    let git = env::var("GIT").unwrap_or_else(|_| "git".to_string());

    if project_root().join("target/Paramdex").exists() {
        let status = Command::new(&git)
            .current_dir(project_root().join("target/Paramdex"))
            .args(["fetch"])
            .status()
            .context("git")?;

        if !status.success() {
            bail!("git fetch failed");
        }

        let status = Command::new(&git)
            .current_dir(project_root().join("target/Paramdex"))
            .args(["pull"])
            .status()
            .context("git")?;

        if !status.success() {
            bail!("git pull failed");
        }
    } else {
        let status = Command::new(&git)
            .current_dir(project_root().join("target"))
            .args(["clone", "https://github.com/soulsmods/Paramdex.git"])
            .status()
            .context("git")?;

        if !status.success() {
            bail!("git clone failed");
        }
    }

    Ok(())
}

fn snake_case(name: &str) -> String {
    static CAPITALS: Lazy<Regex> = Lazy::new(|| Regex::new(r"[A-Z]+").unwrap());
    static UNDERSCORES: Lazy<Regex> = Lazy::new(|| Regex::new(r"_+").unwrap());

    let mut result = String::new();

    if let Some(first) = name.chars().next() {
        result.push(first);
        result.push_str(&CAPITALS.replace_all(&name[first.len_utf8()..], "_$0"));
    }

    UNDERSCORES.replace_all(&result.to_lowercase(), "_").into_owned()
}

fn fix_name(name: &str) -> String {
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("field{name}")
    } else if name == "type" {
        "ty".into()
    } else {
        name.into()
    }
}

fn slug(name: &str) -> String {
    name.chars().filter(char::is_ascii_alphabetic).collect::<String>().to_ascii_lowercase()
}

struct Field {
    name: String,
    ty: String,
    /// For bitfield members: their width, and the size of their type, in bits.
    bits: Option<(usize, usize)>,
    /// For bitfields: their one bit members, with the bit offset of each.
    flags: Vec<(usize, Field)>,
}

impl Field {
    fn parse(definition: &str) -> Result<Self> {
        static DEFINITION: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^(\w+)\s+(\w+)(?:\[(\d+)\]|:(\d+))?").unwrap());

        let captures = DEFINITION
            .captures(definition)
            .with_context(|| format!("Couldn't parse field: {definition}"))?;

        let ty = match &captures[1] {
            "s8" => "i8",
            "s16" => "i16",
            "s32" => "i32",
            "u8" | "dummy8" | "fixstr" => "u8",
            "u16" | "fixstrW" => "u16",
            "u32" => "u32",
            "f32" => "f32",
            other => bail!("Unsupported field type {other} in {definition}"),
        };

        let bits = match captures.get(4) {
            Some(width) => {
                let size = match ty {
                    "u8" => 8,
                    "u16" => 16,
                    "u32" => 32,
                    _ => bail!("Unsupported bitfield type in {definition}"),
                };
                let width = width.as_str().parse::<usize>()?;
                ensure!((1..=size).contains(&width), "Invalid bitfield width in {definition}");
                Some((width, size))
            },
            None => None,
        };

        let ty = match captures.get(3) {
            Some(count) => format!("[{ty}; {}]", count.as_str().parse::<usize>()?),
            None => ty.into(),
        };

        Ok(Self { name: captures[2].into(), ty, bits, flags: Vec::new() })
    }
}

fn dedup_fields<'a>(fields: impl IntoIterator<Item = &'a mut Field>) {
    let mut names = HashSet::new();
    let mut index = 0;

    for field in fields {
        let name = snake_case(&field.name);

        if names.contains(&name) {
            field.name = format!("{}_{index}", field.name);
            index += 1;
        }

        names.insert(name);
    }
}

fn parse_layout(xml: &str) -> Result<Vec<Field>> {
    let mut reader = Reader::from_str(xml);
    let mut fields = Vec::new();
    // The bitfield being filled, its size and the bits used so far.
    let mut bitfield: Option<(Field, usize, usize)> = None;
    let mut bitfield_index = 0;
    let mut depth = 0;
    let mut fields_depth = None;

    loop {
        let event = reader.read_event()?;
        let empty = matches!(event, Event::Empty(_));

        match event {
            Event::Start(element) | Event::Empty(element) => {
                if depth == 1 && element.name().as_ref() == b"Fields" && !empty {
                    fields_depth = Some(depth + 1);
                } else if fields_depth == Some(depth) && element.name().as_ref() == b"Field" {
                    let definition = element
                        .try_get_attribute("Def")?
                        .context("Field is missing its Def attribute")?;

                    let definition = definition
                        .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())?;

                    let field = Field::parse(&definition)?;

                    // Bitfield members are packed, in order, into a field of
                    // their type, as the compiler does: it ends when the next
                    // member doesn't fit, has a type of a different size, or
                    // isn't a bitfield member.
                    //
                    // Earlier generators (ER's, and DS3's in Python) assumed
                    // that each member was one bit wide, and ended a field once
                    // it had as many members as its type has bits. Wider
                    // members (e.g. `dummy8 pad:6`) and fields left partly
                    // unused carried members over into the next field, or
                    // dropped the field altogether, shifting every field after
                    // it.
                    match field.bits {
                        Some((width, size)) => {
                            if bitfield.as_ref().is_some_and(|&(_, bitfield_size, used)| {
                                bitfield_size != size || used + width > size
                            }) {
                                close_bitfield(&mut fields, bitfield.take());
                            }

                            let (current, _, used) = bitfield.get_or_insert_with(|| {
                                let name = format!("bitfield{bitfield_index}");
                                bitfield_index += 1;
                                (
                                    Field {
                                        name,
                                        ty: field.ty.clone(),
                                        bits: None,
                                        flags: Vec::new(),
                                    },
                                    size,
                                    0,
                                )
                            });

                            // Wider members can't be represented as flags.
                            if width == 1 {
                                current.flags.push((*used, field));
                            }
                            *used += width;
                        },
                        None => {
                            close_bitfield(&mut fields, bitfield.take());
                            fields.push(field);
                        },
                    }
                }

                if !empty {
                    depth += 1;
                }
            },
            Event::End(_) => {
                if fields_depth == Some(depth) {
                    fields_depth = None;
                }

                depth -= 1;
            },
            Event::Eof => break,
            _ => {},
        }
    }

    ensure!(depth == 0, "Unexpected end of XML document");

    close_bitfield(&mut fields, bitfield);
    dedup_fields(&mut fields);

    Ok(fields)
}

fn close_bitfield(fields: &mut Vec<Field>, bitfield: Option<(Field, usize, usize)>) {
    if let Some((mut bitfield, ..)) = bitfield {
        dedup_fields(bitfield.flags.iter_mut().map(|(_, flag)| flag));
        fields.push(bitfield);
    }
}

/// Slugs of layouts left out of the generated code. The Python generator this
/// replaced marked them as broken.
const SKIPPED_LAYOUTS: &[&str] = &["defaultkeyassign"];

/// Generates Rust structs for the params of `game`, the Paramdex directory
/// (e.g. `"ER"`), from the layouts in `<paramdex>/<game>/Defs`.
pub fn codegen_param_data(paramdex: &Path, game: &str) -> Result<String> {
    let definitions = paramdex.join(game).join("Defs");
    let mut layouts = BTreeMap::new();

    for entry in
        fs::read_dir(&definitions).with_context(|| format!("Reading {}", definitions.display()))?
    {
        let path = entry?.path();

        if path.extension().and_then(|s| s.to_str()) != Some("xml") {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .context("Invalid XML filename")?
            .replace("_ST", "");

        let key = slug(&name);

        if SKIPPED_LAYOUTS.contains(&key.as_str()) {
            continue;
        }

        let fields = fs::read_to_string(&path)
            .map_err(anyhow::Error::from)
            .and_then(|xml| parse_layout(&xml))
            .with_context(|| format!("Parsing {}", path.display()))?;

        layouts.insert(key, (name, fields));
    }

    ensure!(!layouts.is_empty(), "No parameter definitions in {}", definitions.display());

    let mut source = String::from(
        r#"// **********************************
// *** AUTOGENERATED, DO NOT EDIT ***
// **********************************
use std::collections::HashMap;
use std::ffi::c_void;

use once_cell::sync::Lazy;
use macro_param::ParamStruct;
use crate::prelude::*;

unsafe fn get_lambda<T: ParamStruct>() -> BoxedVisitorLambda {
    Box::new(|ptr, v| {
        if let Some(r) = (ptr as *mut T).as_mut() {
            r.visit(&mut *v);
        }
    })
}

type BoxedVisitorLambda = Box<dyn Fn(*const c_void, &mut dyn ParamVisitor) + Send + Sync>;

pub static PARAM_VTABLE: Lazy<HashMap<String, BoxedVisitorLambda>> = Lazy::new(|| {
    [
"#,
    );

    for (name, _) in layouts.values() {
        writeln!(source, "        (\"{name}\".to_string(), unsafe {{ get_lambda::<{name}>() }}),")?;
    }

    source.push_str("    ].into_iter().collect()\n});");

    for (name, fields) in layouts.values() {
        writeln!(source, "\n#[derive(ParamStruct, Debug)]\n#[repr(C)]\npub struct {name} {{")?;

        for field in fields {
            for (offset, flag) in &field.flags {
                writeln!(source, "    #[bitflag({}, {offset})]", fix_name(&flag.name))?;
            }

            writeln!(source, "    pub {}: {},", fix_name(&snake_case(&field.name)), field.ty)?;
        }

        source.push_str("}\n");
    }
    Ok(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_xml_fields_and_preserves_names() -> Result<()> {
        let fields = parse_layout(
            r#"<?xml version="1.0"?>
            <PARAMDEF><Fields>
                <Field Def="s32 type = -1"><Description>ignored</Description></Field>
                <Field Def="fixstrW title[16]" />
                <Field Def="f32 2Speed = 1" />
                <Field Def="u8 fooBar" />
                <Field Def="u8 foo_bar" />
            </Fields></PARAMDEF>"#,
        )?;

        assert_eq!(fields.len(), 5);
        assert_eq!(fix_name(&fields[0].name), "ty");
        assert_eq!(fields[0].ty, "i32");
        assert_eq!(fields[1].ty, "[u16; 16]");
        assert_eq!(fix_name(&snake_case(&fields[2].name)), "field2_speed");
        assert_eq!(fields[4].name, "foo_bar_0");
        assert_eq!(snake_case("AIAttackParam"), "a_iattack_param");

        Ok(())
    }

    #[test]
    fn groups_bitflags_in_order() -> Result<()> {
        let mut xml = String::from("<PARAMDEF><Fields>");

        for index in 0..8 {
            write!(xml, "<Field Def=\"u8 flag{index}:1\" />")?;
        }

        xml.push_str("<Field Def=\"s32 value\" /></Fields></PARAMDEF>");

        let fields = parse_layout(&xml)?;

        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "bitfield0");
        assert_eq!(fields[0].ty, "u8");
        assert_eq!(fields[0].flags.len(), 8);
        assert_eq!(fields[0].flags[7].0, 7);
        assert_eq!(fields[0].flags[7].1.name, "flag7");
        assert_eq!(fields[1].name, "value");

        Ok(())
    }

    #[test]
    fn packs_bitfields_by_width() -> Result<()> {
        let fields = parse_layout(
            r#"<PARAMDEF><Fields>
                <Field Def="u8 a:1" /><Field Def="u8 b:1" /><Field Def="dummy8 pad:6" />
                <Field Def="u8 c:1" /><Field Def="u8 d:2" /><Field Def="u8 e:1" />
                <Field Def="u16 f:1" />
                <Field Def="s32 value" />
                <Field Def="u8 g:6" /><Field Def="u8 h:4" />
            </Fields></PARAMDEF>"#,
        )?;

        let layout = fields
            .iter()
            .map(|field| {
                let flags = field.flags.iter().map(|(offset, flag)| (*offset, flag.name.as_str()));
                (field.name.as_str(), field.ty.as_str(), flags.collect::<Vec<_>>())
            })
            .collect::<Vec<_>>();

        assert_eq!(layout, [
            // Full, with a wider member.
            ("bitfield0", "u8", vec![(0, "a"), (1, "b")]),
            // Partly unused, ended by a type of a different size.
            ("bitfield1", "u8", vec![(0, "c"), (3, "e")]),
            // Ended by a member that isn't a bitfield.
            ("bitfield2", "u16", vec![(0, "f")]),
            ("value", "i32", vec![]),
            // Ended by a member that doesn't fit, and by the end of the layout.
            ("bitfield3", "u8", vec![]),
            ("bitfield4", "u8", vec![]),
        ]);

        Ok(())
    }

    #[test]
    fn reports_invalid_definitions() {
        assert!(Field::parse("invalid").is_err());
        assert!(Field::parse("unknown value").is_err());
        assert!(Field::parse("u8 wide:9").is_err());
        assert!(parse_layout("<PARAMDEF><Fields><Field /></Fields></PARAMDEF>").is_err());
        assert!(parse_layout("<PARAMDEF><Fields></PARAMDEF>").is_err());
    }

    #[test]
    #[ignore = "requires a local target/Paramdex checkout"]
    fn generates_local_paramdex() -> Result<()> {
        let source = codegen_param_data(&project_root().join("target/Paramdex"), "ER")?;

        assert!(source.contains("pub struct EquipParamWeapon"));
        assert!(!source.contains("pub struct DefaultKeyAssign"));

        Ok(())
    }
}
