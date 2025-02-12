use crate::lattices::reachingdefslattice::LocIdx;
use crate::lattices::Lattice;
use crate::utils::lifter::ValSize;

#[derive(Default, PartialEq, Eq, Clone, PartialOrd, Debug)]
pub struct X86RegsLattice<T: Lattice + Clone> {
    pub rax: T,
    pub rbx: T,
    pub rcx: T,
    pub rdx: T,
    pub rdi: T,
    pub rsi: T,
    pub rsp: T,
    pub rbp: T,
    pub r8: T,
    pub r9: T,
    pub r10: T,
    pub r11: T,
    pub r12: T,
    pub r13: T,
    pub r14: T,
    pub r15: T,
    pub zf: T,
}

impl<T: Lattice + Clone> X86RegsLattice<T> {
    pub fn get(&self, index: &u8, size: &ValSize) -> T {
        if let ValSize::SizeOther = size {
            return Default::default();
        }
        match index {
            0 => self.rax.clone(),
            1 => self.rcx.clone(),
            2 => self.rdx.clone(),
            3 => self.rbx.clone(),
            4 => self.rsp.clone(),
            5 => self.rbp.clone(),
            6 => self.rsi.clone(),
            7 => self.rdi.clone(),
            8 => self.r8.clone(),
            9 => self.r9.clone(),
            10 => self.r10.clone(),
            11 => self.r11.clone(),
            12 => self.r12.clone(),
            13 => self.r13.clone(),
            14 => self.r14.clone(),
            15 => self.r15.clone(),
            16 => self.zf.clone(),
            _ => panic!("Unknown register: index = {:?}", index),
        }
    }

    pub fn set(&mut self, index: &u8, size: &ValSize, value: T) -> () {
        if let ValSize::SizeOther = size {
            return;
        }
        match index {
            0 => self.rax = value,
            1 => self.rcx = value,
            2 => self.rdx = value,
            3 => self.rbx = value,
            4 => self.rsp = value,
            5 => self.rbp = value,
            6 => self.rsi = value,
            7 => self.rdi = value,
            8 => self.r8 = value,
            9 => self.r9 = value,
            10 => self.r10 = value,
            11 => self.r11 = value,
            12 => self.r12 = value,
            13 => self.r13 = value,
            14 => self.r14 = value,
            15 => self.r15 = value,
            16 => self.zf = value,
            _ => panic!("Unknown register: index = {:?}", index),
        }
    }

    pub fn clear_regs(&mut self) -> () {
        self.rax = Default::default();
        self.rcx = Default::default();
        self.rdx = Default::default();
        self.rbx = Default::default();
        self.rbp = Default::default();
        self.rsi = Default::default();
        self.rdi = Default::default();

        self.r8 = Default::default();
        self.r9 = Default::default();
        self.r10 = Default::default();
        self.r11 = Default::default();
        self.r12 = Default::default();
        self.r13 = Default::default();
        self.r14 = Default::default();
        self.r15 = Default::default();
        self.zf = Default::default();
    }

    pub fn clear_caller_save_regs(&mut self) {
        // x86-64 calling convention: rax, rcx, rdx, rsi, rdi, r8, r9, r10, r11 must be saved by
        // the caller (are clobbered by the callee), so their states become unknown after calls.
        //
        // TODO: get calling convention from program's target ABI; on Windows, rsi and rdi are
        // callee-save. The below is thus sound but conservative (and possibly
        // false-positive-producing) on Windows.
        self.rax = Default::default();
        self.rcx = Default::default();
        self.rdx = Default::default();
        self.rsi = Default::default();
        self.rdi = Default::default();

        self.r8 = Default::default();
        self.r9 = Default::default();
        self.r10 = Default::default();
        self.r11 = Default::default();
        self.zf = Default::default();
    }

    pub fn show(&self) -> () {
        println!("State = ");
        if self.rax != Default::default() {
            println!("rax = {:?}", self.rax)
        }
        if self.rcx != Default::default() {
            println!("rcx = {:?}", self.rcx)
        }
        if self.rdx != Default::default() {
            println!("rdx = {:?}", self.rdx)
        }
        if self.rbx != Default::default() {
            println!("rbx = {:?}", self.rbx)
        }
        if self.rbp != Default::default() {
            println!("rbp = {:?}", self.rbp)
        }
        if self.rsi != Default::default() {
            println!("rsi = {:?}", self.rsi)
        }
        if self.rdi != Default::default() {
            println!("rdi = {:?}", self.rdi)
        }
        if self.r8 != Default::default() {
            println!("r8 = {:?}", self.r8)
        }
        if self.r9 != Default::default() {
            println!("r9 = {:?}", self.r9)
        }
        if self.r10 != Default::default() {
            println!("r10 = {:?}", self.r10)
        }
        if self.r11 != Default::default() {
            println!("r11 = {:?}", self.r11)
        }
        if self.r12 != Default::default() {
            println!("r12 = {:?}", self.r12)
        }
        if self.r13 != Default::default() {
            println!("r13 = {:?}", self.r13)
        }
        if self.r14 != Default::default() {
            println!("r14 = {:?}", self.r14)
        }
        if self.r15 != Default::default() {
            println!("r15 = {:?}", self.r15)
        }
        if self.zf != Default::default() {
            println!("zf = {:?}", self.zf)
        }
    }
}

impl<T: Lattice + Clone> Lattice for X86RegsLattice<T> {
    fn meet(&self, other: &Self, loc_idx: &LocIdx) -> Self {
        X86RegsLattice {
            rax: self.rax.meet(&other.rax, loc_idx),
            rbx: self.rbx.meet(&other.rbx, loc_idx),
            rcx: self.rcx.meet(&other.rcx, loc_idx),
            rdx: self.rdx.meet(&other.rdx, loc_idx),
            rdi: self.rdi.meet(&other.rdi, loc_idx),
            rsi: self.rsi.meet(&other.rsi, loc_idx),
            rsp: self.rsp.meet(&other.rsp, loc_idx),
            rbp: self.rbp.meet(&other.rbp, loc_idx),
            r8: self.r8.meet(&other.r8, loc_idx),
            r9: self.r9.meet(&other.r9, loc_idx),
            r10: self.r10.meet(&other.r10, loc_idx),
            r11: self.r11.meet(&other.r11, loc_idx),
            r12: self.r12.meet(&other.r12, loc_idx),
            r13: self.r13.meet(&other.r13, loc_idx),
            r14: self.r14.meet(&other.r14, loc_idx),
            r15: self.r15.meet(&other.r15, loc_idx),
            zf: self.zf.meet(&other.zf, loc_idx),
        }
    }
}

#[test]
fn regs_lattice_test() {
    use crate::lattices::BooleanLattice;

    let r1 = X86RegsLattice {
        rax: BooleanLattice { v: false },
        rbx: BooleanLattice { v: false },
        rcx: BooleanLattice { v: false },
        rdx: BooleanLattice { v: false },
        rdi: BooleanLattice { v: false },
        rsi: BooleanLattice { v: false },
        rsp: BooleanLattice { v: false },
        rbp: BooleanLattice { v: false },
        r8: BooleanLattice { v: false },
        r9: BooleanLattice { v: false },
        r10: BooleanLattice { v: false },
        r11: BooleanLattice { v: false },
        r12: BooleanLattice { v: false },
        r13: BooleanLattice { v: false },
        r14: BooleanLattice { v: false },
        r15: BooleanLattice { v: false },
        zf: BooleanLattice { v: false },
    };

    let r2 = X86RegsLattice {
        rax: BooleanLattice { v: true },
        rbx: BooleanLattice { v: false },
        rcx: BooleanLattice { v: false },
        rdx: BooleanLattice { v: false },
        rdi: BooleanLattice { v: false },
        rsi: BooleanLattice { v: false },
        rsp: BooleanLattice { v: false },
        rbp: BooleanLattice { v: false },
        r8: BooleanLattice { v: false },
        r9: BooleanLattice { v: false },
        r10: BooleanLattice { v: false },
        r11: BooleanLattice { v: false },
        r12: BooleanLattice { v: false },
        r13: BooleanLattice { v: false },
        r14: BooleanLattice { v: false },
        r15: BooleanLattice { v: false },
        zf: BooleanLattice { v: false },
    };

    let r3 = X86RegsLattice {
        rax: BooleanLattice { v: false },
        rbx: BooleanLattice { v: true },
        rcx: BooleanLattice { v: false },
        rdx: BooleanLattice { v: false },
        rdi: BooleanLattice { v: false },
        rsi: BooleanLattice { v: false },
        rsp: BooleanLattice { v: false },
        rbp: BooleanLattice { v: false },
        r8: BooleanLattice { v: false },
        r9: BooleanLattice { v: false },
        r10: BooleanLattice { v: false },
        r11: BooleanLattice { v: false },
        r12: BooleanLattice { v: false },
        r13: BooleanLattice { v: false },
        r14: BooleanLattice { v: false },
        r15: BooleanLattice { v: false },
        zf: BooleanLattice { v: false },
    };

    assert_eq!(r2.rax > r2.rbx, true);
    assert_eq!(r2.rax < r2.rbx, false);
    assert_eq!(r2.rax.gt(&r2.rbx), true);
    assert_eq!(r2.rbx == r2.rdi, true);

    assert_eq!(r1 < r2, true);
    assert_eq!(r1 <= r2, true);

    assert_eq!(r2 < r3, false);
    assert_eq!(r2 <= r3, false);

    assert_eq!(r2.meet(&r3, &LocIdx { addr: 0, idx: 0 }) == r1, true);
    assert_eq!(r1.meet(&r2, &LocIdx { addr: 0, idx: 0 }) == r1, true);
}
