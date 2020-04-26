use libc::user_regs_struct;

#[derive(Clone, Default, Debug)]
pub struct Registers {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub orig_rax: u64,
    pub rip: u64,
    pub cs: u64,
    pub eflags: u64,
    pub rsp: u64,
    pub ss: u64,
    pub fs_base: u64,
    pub gs_base: u64,
    pub ds: u64,
    pub es: u64,
    pub fs: u64,
    pub gs: u64,
}

impl From<user_regs_struct> for Registers {
    fn from(r: user_regs_struct) -> Self {
        Registers {
            r15: r.r15,
            r14: r.r14,
            r13: r.r13,
            r12: r.r12,
            rbp: r.rbp,
            rbx: r.rbx,
            r11: r.r11,
            r10: r.r10,
            r9: r.r9,
            r8: r.r8,
            rax: r.rax,
            rcx: r.rcx,
            rdx: r.rdx,
            rsi: r.rsi,
            rdi: r.rdi,
            orig_rax: r.orig_rax,
            rip: r.rip,
            cs: r.cs,
            eflags: r.eflags,
            rsp: r.rsp,
            ss: r.ss,
            fs_base: r.fs_base,
            gs_base: r.gs_base,
            ds: r.ds,
            es: r.es,
            fs: r.fs,
            gs: r.gs,
        }
    }
}

impl From<Registers> for user_regs_struct {
    fn from(r: Registers) -> Self {
        user_regs_struct {
            r15: r.r15,
            r14: r.r14,
            r13: r.r13,
            r12: r.r12,
            rbp: r.rbp,
            rbx: r.rbx,
            r11: r.r11,
            r10: r.r10,
            r9: r.r9,
            r8: r.r8,
            rax: r.rax,
            rcx: r.rcx,
            rdx: r.rdx,
            rsi: r.rsi,
            rdi: r.rdi,
            orig_rax: r.orig_rax,
            rip: r.rip,
            cs: r.cs,
            eflags: r.eflags,
            rsp: r.rsp,
            ss: r.ss,
            fs_base: r.fs_base,
            gs_base: r.gs_base,
            ds: r.ds,
            es: r.es,
            fs: r.fs,
            gs: r.gs,
        }
    }
}
