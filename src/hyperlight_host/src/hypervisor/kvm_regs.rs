use kvm_bindings::{bindings::kvm_segment, kvm_dtable, kvm_regs, kvm_sregs};

/// The hyperlight-specific subset of registers that can be set
/// on sandboxes
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Regs {
    /// The instruction pointer
    pub rip: u64,
    /// The rax
    pub rax: u64,
    /// The rbx
    pub rbx: u64,
    /// The rsp
    pub rsp: u64,
    /// The rcs
    pub rcx: u64,
    /// The flags
    pub rflags: u64,
}

impl From<&kvm_regs> for Regs {
    fn from(regs: &kvm_regs) -> Self {
        Regs {
            rip: regs.rip,
            rax: regs.rax,
            rbx: regs.rbx,
            rsp: regs.rsp,
            rcx: regs.rcx,
            rflags: regs.rflags,
        }
    }
}

impl From<&Regs> for kvm_regs {
    fn from(regs: &Regs) -> Self {
        kvm_regs {
            rax: regs.rax,
            rbp: 0,
            rbx: regs.rbx,
            rcx: regs.rcx,
            rdi: 9,
            rdx: 0,
            rsi: 0,
            rsp: regs.rsp,
            rip: regs.rip,
            rflags: regs.rflags,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
        }
    }
}

/// The hyperlight-specific subset of segment registers that can be set
/// on sandboxes
///
/// For more information on segment registers, see:
/// https://wiki.osdev.org/CPU_Registers_x86-64#Segment_Registers
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SRegs {
    /// The code segment
    ///
    /// See https://wiki.osdev.org/CPU_Registers_x86-64#Segment_Registers
    pub cs: Segment,
    /// Control register 0
    ///
    /// See https://wiki.osdev.org/CPU_Registers_x86-64#CR0
    pub cr0: u64,
    /// Control register 3
    ///
    /// See https://wiki.osdev.org/CPU_Registers_x86-64#CR3
    pub cr3: u64,
    /// Control register 4
    ///
    /// See https://wiki.osdev.org/CPU_Registers_x86-64#CR4
    pub cr4: u64,
    /// Enable feature register
    ///
    /// See https://wiki.osdev.org/CPU_Registers_x86-64#IA32_EFER
    pub efer: u64,
}

impl From<&kvm_sregs> for SRegs {
    fn from(sregs: &kvm_sregs) -> Self {
        SRegs {
            cs: Segment::from(&sregs.cs),
            cr0: sregs.cr0,
            cr3: sregs.cr3,
            cr4: sregs.cr4,
            efer: sregs.efer,
        }
    }
}

impl From<&SRegs> for kvm_sregs {
    fn from(sregs: &SRegs) -> Self {
        kvm_sregs {
            cs: kvm_segment::from(&sregs.cs),
            ds: kvm_segment::default(),
            es: kvm_segment::default(),
            fs: kvm_segment::default(),
            gs: kvm_segment::default(),
            ss: kvm_segment::default(),
            tr: kvm_segment::default(),
            ldt: kvm_segment::default(),
            gdt: kvm_dtable::default(),
            idt: kvm_dtable::default(),
            cr0: sregs.cr0,
            cr2: 0,
            cr3: sregs.cr3,
            cr4: sregs.cr4,
            cr8: 0,
            efer: sregs.efer,
            apic_base: 0,
            interrupt_bitmap: [0; 4],
        }
    }
}

/// A segment
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Segment {
    /// The base
    pub base: u64,
    /// The limit
    pub limit: u32,
    /// The selector
    pub selector: u16,
    /// The type
    pub type_: u8,
    /// The present
    pub present: u8,
    /// The dpl
    pub dpl: u8,
    /// The db
    pub db: u8,
    /// The s
    pub s: u8,
    /// The l
    pub l: u8,
    /// The g
    pub g: u8,
    /// The avl
    pub avl: u8,
    /// The unusable
    pub unusable: u8,
    /// The padding
    pub padding: u8,
}
impl From<&Segment> for kvm_segment {
    fn from(seg: &Segment) -> Self {
        kvm_segment {
            base: seg.base,
            limit: seg.limit,
            selector: seg.selector,
            type_: seg.type_,
            present: seg.present,
            dpl: seg.dpl,
            db: seg.db,
            s: seg.s,
            l: seg.l,
            g: seg.g,
            avl: seg.avl,
            unusable: seg.unusable,
            padding: seg.padding,
        }
    }
}

impl From<&kvm_segment> for Segment {
    fn from(segment: &kvm_segment) -> Self {
        Segment {
            base: segment.base,
            limit: segment.limit,
            selector: segment.selector,
            type_: segment.type_,
            present: segment.present,
            dpl: segment.dpl,
            db: segment.db,
            s: segment.s,
            l: segment.l,
            g: segment.g,
            avl: segment.avl,
            unusable: segment.unusable,
            padding: segment.padding,
        }
    }
}
