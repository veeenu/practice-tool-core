use std::cmp::PartialOrd;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use heck::AsSnakeCase;
use pelite::{FileMap, PeFile};

#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Version(u32, u32, u32);

impl Version {
    fn to_fromsoft_string(self) -> String {
        format!("{}.{:02}.{}", self.0, self.1, self.2)
    }
}

struct VersionData<'a> {
    version: Version,
    aobs: Vec<(&'a str, usize)>,
}

fn into_needle(pattern: &str) -> Vec<Option<u8>> {
    pattern
        .split(' ')
        .map(|byte| match byte {
            "?" | "??" => None,
            x => u8::from_str_radix(x, 16).ok(),
        })
        .collect::<Vec<_>>()
}

fn naive_search(bytes: &[u8], pattern: &[Option<u8>]) -> Option<usize> {
    bytes.windows(pattern.len()).position(|wnd| {
        wnd.iter().zip(pattern.iter()).all(|(byte, pattern)| match pattern {
            Some(x) => byte == x,
            None => true,
        })
    })
}

pub trait Aob {
    fn find(&self, pe_file: &PeFile) -> Option<(&str, usize)>;
    fn name(&self) -> &str;
}

struct AobDirect<'a> {
    name: &'a str,
    aobs: &'a [&'a str],
}

impl Aob for AobDirect<'_> {
    // Find the position of a matching pattern directly.
    fn find(&self, pe_file: &PeFile) -> Option<(&str, usize)> {
        self.aobs.iter().find_map(|aob| {
            let needle = into_needle(aob);

            pe_file
                .section_headers()
                .into_iter()
                .filter_map(|sh| {
                    Some((sh.VirtualAddress as usize, pe_file.get_section_bytes(sh).ok()?))
                })
                .find_map(|(base, bytes)| {
                    naive_search(bytes, &needle).map(|r| (self.name, r + base))
                })
        })
    }

    fn name(&self) -> &str {
        self.name
    }
}

struct AobIndirect<'a> {
    name: &'a str,
    aobs: &'a [&'a str],
    offset: usize,
}

impl Aob for AobIndirect<'_> {
    // Find the position of a matching pattern, and read a u32 value at an offset from there.
    // E.g. in "48 8b 0D aa bb cc dd" would yield the value "aa bb cc dd".
    fn find(&self, pe_file: &PeFile) -> Option<(&str, usize)> {
        self.aobs.iter().find_map(|aob| {
            let needle = into_needle(aob);

            pe_file
                .section_headers()
                .into_iter()
                .filter_map(|sh| pe_file.get_section_bytes(sh).ok())
                .find_map(|bytes| {
                    naive_search(bytes, &needle).map(|r| {
                        let r = u32::from_le_bytes(
                            (&bytes[r + self.offset..r + self.offset + 4]).try_into().unwrap(),
                        );
                        (self.name, r as usize)
                    })
                })
        })
    }

    fn name(&self) -> &str {
        self.name
    }
}

struct AobIndirectTwice<'a> {
    name: &'a str,
    aobs: &'a [&'a str],
    offset_from_pattern: usize,
    offset_from_offset: usize,
}

impl Aob for AobIndirectTwice<'_> {
    // Find the position of a matching pattern, read a u32 value at an offset from there,
    // interpret that as an offset from the pattern's position and add another offset from there.
    fn find(&self, pe_file: &PeFile) -> Option<(&str, usize)> {
        self.aobs.iter().find_map(|aob| {
            let needle = into_needle(aob);

            pe_file
                .section_headers()
                .into_iter()
                .filter_map(|sh| {
                    Some((sh.VirtualAddress as usize, pe_file.get_section_bytes(sh).ok()?))
                })
                .find_map(|(base, bytes)| {
                    let offset = naive_search(bytes, &needle)?;
                    let val = u32::from_le_bytes(
                        bytes[offset + self.offset_from_pattern
                            ..offset + self.offset_from_pattern + 4]
                            .try_into()
                            .unwrap(),
                    ) as usize;
                    let addr = val + self.offset_from_offset + offset + base;

                    Some((self.name, addr))
                })
        })
    }

    fn name(&self) -> &str {
        self.name
    }
}

pub fn aob_direct<'a>(name: &'a str, aobs: &'a [&'a str]) -> Box<dyn Aob + 'a> {
    Box::new(AobDirect { name, aobs })
}

pub fn aob_indirect<'a>(name: &'a str, aobs: &'a [&'a str], offset: usize) -> Box<dyn Aob + 'a> {
    Box::new(AobIndirect { name, aobs, offset })
}

pub fn aob_indirect_twice<'a>(
    name: &'a str,
    aobs: &'a [&'a str],
    offset_from_pattern: usize,
    offset_from_offset: usize,
) -> Box<dyn Aob + 'a> {
    Box::new(AobIndirectTwice { name, aobs, offset_from_pattern, offset_from_offset })
}

fn find_aobs<'a>(pe_file: &PeFile, aobs: &'a [Box<dyn Aob>]) -> Vec<(&'a str, usize)> {
    aobs.iter().filter_map(|aob| aob.find(pe_file)).collect::<Vec<_>>()
}

// Codegen routine
//

/// Generate the `BaseAddresses` struct.
fn codegen_base_addresses_struct(aobs: &[Box<dyn Aob>]) -> String {
    let names = aobs.iter().map(|a| a.name()).collect::<Vec<_>>();

    let mut generated = String::new();

    generated.push_str("// **********************************\n");
    generated.push_str("// *** AUTOGENERATED, DO NOT EDIT ***\n");
    generated.push_str("// **********************************\n");

    generated.push_str("#[derive(Debug)]\n");
    generated.push_str("pub struct BaseAddresses {\n");

    for name in &names {
        generated.push_str(&format!("    pub {}: usize,\n", AsSnakeCase(name)));
    }

    generated.push_str("}\n\n");
    generated.push_str("impl BaseAddresses {\n");
    generated.push_str("    pub fn with_module_base_addr(self, base: usize) -> BaseAddresses {\n");
    generated.push_str("        BaseAddresses {\n");

    for name in &names {
        generated.push_str(&format!(
            "            {}: self.{} + base,\n",
            AsSnakeCase(name),
            AsSnakeCase(name)
        ));
    }
    generated.push_str("        }\n    }\n}\n\n");
    generated
}

/// Generate `BaseAddresses` instances.
fn codegen_base_addresses_instances(ver: &Version, base_addresses: &[(&str, usize)]) -> String {
    use std::fmt::Write;
    let mut string = base_addresses.iter().fold(
        format!(
            "pub const BASE_ADDRESSES_{}_{:02}_{}: BaseAddresses = BaseAddresses {{\n",
            ver.0, ver.1, ver.2
        ),
        |mut o, (name, offset)| {
            writeln!(o, "    {}: 0x{:x},", AsSnakeCase(name), offset).unwrap();
            o
        },
    );
    string.push_str("};\n\n");
    string
}

/// Generate the `Version` enum and `From<Version> for BaseAddresses`.
fn codegen_version_enum(ver: &[VersionData]) -> String {
    use std::fmt::Write;
    let mut string = String::new();

    // pub enum Version

    string.push_str("#[derive(Clone, Copy)]\n");
    string.push_str("pub enum Version {\n");

    for v in ver {
        writeln!(string, "    V{}_{:02}_{},", v.version.0, v.version.1, v.version.2).unwrap();
    }

    string.push_str("}\n\n");

    // impl From<(u32, u32, u32)> for Version

    string.push_str("impl From<(u32, u32, u32)> for Version {\n");
    string.push_str("    fn from(v: (u32, u32, u32)) -> Self {\n");
    string.push_str("        match v {\n");

    for v in ver {
        let Version(maj, min, patch) = v.version;
        writeln!(
            string,
            "            ({maj}, {min}, {patch}) => Version::V{maj}_{min:02}_{patch},"
        )
        .unwrap();
    }

    string.push_str("            (maj, min, patch) => {\n");
    string.push_str(
        "                log::error!(\"Unrecognized version {maj}.{min:02}.{patch}\");\n",
    );
    string.push_str("                panic!()\n");
    string.push_str("            }\n");
    string.push_str("        }\n");
    string.push_str("    }\n");
    string.push_str("}\n\n");

    // impl From<Version> for (u32, u32, u32)

    string.push_str("impl From<Version> for (u32, u32, u32) {\n");
    string.push_str("    fn from(v: Version) -> Self {\n");
    string.push_str("        match v {\n");

    for v in ver {
        let Version(maj, min, patch) = v.version;
        writeln!(
            string,
            "            Version::V{maj}_{min:02}_{patch} => ({maj}, {min}, {patch}),"
        )
        .unwrap();
    }

    string.push_str("        }\n");
    string.push_str("    }\n");
    string.push_str("}\n\n");

    // impl From<Version> for BaseAddresses

    string.push_str("impl From<Version> for BaseAddresses {\n");
    string.push_str("    fn from(v: Version) -> Self {\n");
    string.push_str("        match v {\n");

    for v in ver {
        let Version(maj, min, patch) = v.version;
        let stem = format!("{maj}_{min:02}_{patch}");
        writeln!(string, "            Version::V{stem} => BASE_ADDRESSES_{stem},").unwrap();
    }

    string.push_str("        }\n");
    string.push_str("    }\n");
    string.push_str("}\n\n");

    string
}

pub fn codegen_base_addresses(
    codegen_path: PathBuf,
    patches_paths: impl Iterator<Item = PathBuf>,
    aobs: &[Box<dyn Aob>],
) {
    let mut processed_versions: HashSet<Version> = HashSet::new();

    let mut version_data = patches_paths
        .filter(|p| p.exists())
        .filter_map(|exe| {
            let file_map = FileMap::open(&exe).unwrap();
            let pe_file = PeFile::from_bytes(&file_map).unwrap();

            let version = pe_file
                .resources()
                .unwrap()
                .version_info()
                .unwrap()
                .fixed()
                .unwrap()
                .dwProductVersion;
            let version = Version(version.Major as u32, version.Minor as u32, version.Patch as u32);

            if processed_versions.contains(&version) {
                None
            } else {
                let exe = exe.canonicalize().unwrap();
                println!("\nVERSION {}: {:?}", version.to_fromsoft_string(), exe);

                let aobs = find_aobs(&pe_file, aobs);
                processed_versions.insert(version);
                Some(VersionData { version, aobs })
            }
        })
        .collect::<Vec<_>>();

    version_data.sort_by_key(|vd| vd.version);

    let mut codegen = codegen_base_addresses_struct(aobs);
    codegen.push_str(&codegen_version_enum(&version_data));

    let codegen = version_data.iter().fold(codegen, |mut o, i| {
        o.push_str(&codegen_base_addresses_instances(&i.version, &i.aobs));
        o
    });

    File::create(codegen_path).unwrap().write_all(codegen.as_bytes()).unwrap();
}
