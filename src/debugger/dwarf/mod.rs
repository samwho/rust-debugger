use crate::result::Result;
use gimli::constants::{DW_AT_high_pc, DW_AT_low_pc, DW_AT_name, DW_TAG_subprogram};
use gimli::read::AttributeValue;
use object::{Object, ObjectSection};
use std::collections::HashMap;
use std::{borrow, fs::File, path::PathBuf};

#[derive(Debug, Clone)]
pub struct LineInfo {
    pub path: PathBuf,
    pub line: u64,
    pub column: u64,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub low_pc: u64,
    pub high_pc: u64,
}

#[derive(Debug, Clone)]
pub struct DebugInfo {
    symbols: HashMap<String, Symbol>,
    pc_to_line: HashMap<u64, LineInfo>,
}

impl DebugInfo {
    pub fn new(file: File) -> Result<Self> {
        let mut pc_to_line: HashMap<u64, LineInfo> = HashMap::new();
        let mut symbols: HashMap<String, Symbol> = HashMap::new();

        let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };
        let object = object::File::parse(&*mmap).unwrap();
        let endian = if object.is_little_endian() {
            gimli::RunTimeEndian::Little
        } else {
            gimli::RunTimeEndian::Big
        };

        // Load a section and return as `Cow<[u8]>`.
        let load_section = |id: gimli::SectionId| -> Result<borrow::Cow<[u8]>> {
            match object.section_by_name(id.name()) {
                Some(ref section) => Ok(section
                    .uncompressed_data()
                    .unwrap_or(borrow::Cow::Borrowed(&[][..]))),
                None => Ok(borrow::Cow::Borrowed(&[][..])),
            }
        };
        // Load a supplementary section. We don't have a supplementary object file,
        // so always return an empty slice.
        let load_section_sup = |_| Ok(borrow::Cow::Borrowed(&[][..]));

        // Load all of the sections.
        let dwarf_cow = gimli::Dwarf::load(&load_section, &load_section_sup)?;

        // Borrow a `Cow<[u8]>` to create an `EndianSlice`.
        let borrow_section: &dyn for<'a> Fn(
            &'a borrow::Cow<[u8]>,
        )
            -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
            &|section| gimli::EndianSlice::new(&*section, endian);

        // Create `EndianSlice`s for all of the sections.
        let dwarf = dwarf_cow.borrow(&borrow_section);

        // Iterate over the compilation units.
        let mut iter = dwarf.units();
        while let Some(header) = iter.next()? {
            println!(
                "Line number info for unit at <.debug_info+0x{:x}>",
                header.offset().0
            );
            let unit = dwarf.unit(header)?;

            // Get the line program for the compilation unit.
            if let Some(program) = unit.line_program.clone() {
                let comp_dir = if let Some(ref dir) = unit.comp_dir {
                    PathBuf::from(dir.to_string_lossy().into_owned())
                } else {
                    PathBuf::new()
                };

                // Iterate over the line program rows.
                let mut rows = program.rows();
                while let Some((header, row)) = rows.next_row()? {
                    if row.end_sequence() {
                        // End of sequence indicates a possible gap in addresses.
                        println!("{:x} end-sequence", row.address());
                    } else {
                        // Determine the path. Real applications should cache this for performance.
                        let mut path = PathBuf::new();
                        if let Some(file) = row.file(header) {
                            path = comp_dir.clone();
                            if let Some(dir) = file.directory(header) {
                                path.push(
                                    dwarf.attr_string(&unit, dir)?.to_string_lossy().as_ref(),
                                );
                            }
                            path.push(
                                dwarf
                                    .attr_string(&unit, file.path_name())?
                                    .to_string_lossy()
                                    .as_ref(),
                            );
                        }

                        // Determine line/column. DWARF line/column is never 0, so we use that
                        // but other applications may want to display this differently.
                        let line = row.line().unwrap_or(0);
                        let column = match row.column() {
                            gimli::ColumnType::LeftEdge => 0,
                            gimli::ColumnType::Column(x) => x,
                        };

                        pc_to_line.insert(row.address(), LineInfo { path, line, column });
                    }
                }
            }

            let mut entries = unit.entries();
            while let Some((_, entry)) = entries.next_dfs()? {
                match entry.tag() {
                    DW_TAG_subprogram => {
                        let name = match entry.attr(DW_AT_name)? {
                            Some(name) => name
                                .string_value(&dwarf.debug_str)
                                .map(|ds| ds.to_string())
                                .unwrap()?,
                            None => continue,
                        };

                        let low_pc = match entry.attr_value(DW_AT_low_pc)? {
                            Some(AttributeValue::Addr(low_pc)) => low_pc,
                            _ => continue,
                        };

                        let high_pc = match entry.attr_value(DW_AT_high_pc)? {
                            Some(AttributeValue::Udata(high_pc)) => high_pc,
                            _ => continue,
                        };

                        symbols.insert(
                            name.to_string(),
                            Symbol {
                                name: name.to_string(),
                                low_pc,
                                high_pc,
                            },
                        );
                    }
                    _ => {}
                }
            }
        }

        Ok(DebugInfo {
            pc_to_line,
            symbols,
        })
    }

    pub fn symbols(&self) -> &HashMap<String, Symbol> {
        &self.symbols
    }
}
