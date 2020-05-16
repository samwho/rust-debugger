use crate::debugger::Subordinate;
use crate::Result;

const AT_NULL: usize = 0;
const AT_PHDR: usize = 3;
const AT_PHENT: usize = 4;
const AT_PHNUM: usize = 5;
const AT_PAGESZ: usize = 6;
const AT_BASE: usize = 7;
const AT_FLAGS: usize = 8;
const AT_ENTRY: usize = 9;
const AT_UID: usize = 11;
const AT_EUID: usize = 12;
const AT_GID: usize = 13;
const AT_EGID: usize = 14;
const AT_PLATFORM: usize = 15;
const AT_HWCAP: usize = 16;
const AT_CLKTCK: usize = 17;
const AT_DCACHEBSIZE: usize = 19;
const AT_ICACHEBSIZE: usize = 20;
const AT_UCACHEBSIZE: usize = 21;
const AT_IGNOREPPC: usize = 22;
const AT_SECURE: usize = 23;
const AT_BASE_PLATFORM: usize = 24;
const AT_RANDOM: usize = 25;
const AT_HWCAP2: usize = 26;
const AT_EXECFN: usize = 31;
const AT_SYSINFO_EHDR: usize = 33;
const AT_L1I_CACHESIZE: usize = 40;
const AT_L1I_CACHEGEOMETRY: usize = 41;
const AT_L1D_CACHESIZE: usize = 42;
const AT_L1D_CACHEGEOMETRY: usize = 43;
const AT_L2_CACHESIZE: usize = 44;
const AT_L2_CACHEGEOMETRY: usize = 45;
const AT_L3_CACHESIZE: usize = 46;
const AT_L3_CACHEGEOMETRY: usize = 47;

#[derive(Debug)]
pub enum Entry {
    Null,
    ProgramHeaderAddr(usize),
    ProgramHeaderSize(usize),
    ProgramHeaderCount(usize),
    PageSize(usize),
    BaseAddr(usize),
    Flags(usize),
    EntryAddr(usize),
    UID(usize),
    EUID(usize),
    GID(usize),
    EGID(usize),
    PlatformAddr(usize),
    HwCap(usize),
    ClockTick(usize),
    DataCacheBlockSize(usize),
    InstructionCacheBlockSize(usize),
    UnifiedCacheBlockSize(usize),
    Secure(bool),
    BasePlatformAddr(usize),
    Random(usize),
    HwCap2(usize),
    ExecutableAddr(usize),
    SysinfoHeaderAddr(usize),
    L1InstructionCacheSize(usize),
    L1InstructionCacheGeometry(usize),
    L1DataCacheSize(usize),
    L1DataCacheGeometry(usize),
    L2InstructionCacheSize(usize),
    L2InstructionCacheGeometry(usize),
    L3InstructionCacheSize(usize),
    L3InstructionCacheGeometry(usize),

    Ignore,
    Unknown(usize, usize),
}

impl Entry {
    fn new(t: usize, value: usize) -> Self {
        match t {
            AT_NULL => Entry::Null,
            AT_PHDR => Entry::ProgramHeaderAddr(value),
            AT_PHENT => Entry::ProgramHeaderSize(value),
            AT_PHNUM => Entry::ProgramHeaderCount(value),
            AT_PAGESZ => Entry::PageSize(value),
            AT_BASE => Entry::BaseAddr(value),
            AT_FLAGS => Entry::Flags(value),
            AT_ENTRY => Entry::EntryAddr(value),
            AT_UID => Entry::UID(value),
            AT_EUID => Entry::EUID(value),
            AT_GID => Entry::GID(value),
            AT_EGID => Entry::EGID(value),
            AT_PLATFORM => Entry::PlatformAddr(value),
            AT_HWCAP => Entry::HwCap(value),
            AT_CLKTCK => Entry::ClockTick(value),
            AT_DCACHEBSIZE => Entry::DataCacheBlockSize(value),
            AT_ICACHEBSIZE => Entry::InstructionCacheBlockSize(value),
            AT_UCACHEBSIZE => Entry::UnifiedCacheBlockSize(value),
            AT_IGNOREPPC => Entry::Ignore,
            AT_SECURE => Entry::Secure(value != 0),
            AT_BASE_PLATFORM => Entry::BasePlatformAddr(value),
            AT_RANDOM => Entry::Random(value),
            AT_HWCAP2 => Entry::HwCap2(value),
            AT_EXECFN => Entry::ExecutableAddr(value),
            AT_SYSINFO_EHDR => Entry::SysinfoHeaderAddr(value),
            AT_L1I_CACHESIZE => Entry::L1InstructionCacheSize(value),
            AT_L1I_CACHEGEOMETRY => Entry::L1InstructionCacheGeometry(value),
            AT_L1D_CACHESIZE => Entry::L1DataCacheSize(value),
            AT_L1D_CACHEGEOMETRY => Entry::L1DataCacheGeometry(value),
            AT_L2_CACHESIZE => Entry::L2InstructionCacheSize(value),
            AT_L2_CACHEGEOMETRY => Entry::L2InstructionCacheGeometry(value),
            AT_L3_CACHESIZE => Entry::L3InstructionCacheSize(value),
            AT_L3_CACHEGEOMETRY => Entry::L3InstructionCacheGeometry(value),
            _ => Entry::Unknown(t, value),
        }
    }
}

pub fn read(subordinate: &Subordinate) -> Result<Vec<Entry>> {
    let mut auxv: Vec<Entry> = Vec::new();
    let regs = subordinate.registers();

    let mut addr = (regs.rsp + 8) as usize;

    // Skip past argv
    addr = advance_to_next_null_entry(subordinate, addr)?;
    // Skip past envp
    addr = advance_to_next_null_entry(subordinate, addr)?;

    loop {
        let aux_type = read_u64(subordinate, addr)?;
        if aux_type == 0 {
            break;
        }

        let aux_val = read_u64(subordinate, addr + 8)?;
        auxv.push(Entry::new(aux_type as usize, aux_val as usize));

        addr += 16;
    }

    Ok(auxv)
}

fn read_u64(subordinate: &Subordinate, addr: usize) -> Result<u64> {
    let mut buf = [0 as u8; 8];
    let bytes = subordinate.read_bytes(addr, 8)?;
    buf.clone_from_slice(&bytes[0..8]);
    Ok(u64::from_le_bytes(buf))
}

fn advance_to_next_null_entry(subordinate: &Subordinate, addr: usize) -> Result<usize> {
    let mut addr = addr;
    loop {
        let mut buf = [0 as u8; 8];
        let bytes = subordinate.read_bytes(addr, 8)?;
        buf.clone_from_slice(&bytes[0..8]);
        let val = usize::from_le_bytes(buf);
        addr += 8;
        if val == 0 {
            break;
        }
    }
    Ok(addr)
}
