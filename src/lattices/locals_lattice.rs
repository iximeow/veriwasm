use crate::lattices::{Semilattice, Lattice, VariableState};
pub use crate::lattices::VarState;
use crate::lattices::X86RegsLattice;
use crate::lattices::reaching_defs_lattice::LocIdx;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::default::Default;

#[derive(PartialEq, Clone, Eq, Debug)]
pub enum InitData {
    VarSet(HashSet<VarSlot>),
    Constant
}

impl PartialOrd for InitData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (InitData::Constant, InitData::Constant) => Some(Ordering::Equal),
            (InitData::VarSet(xs), InitData::VarSet(ys)) => {
                if xs == ys {
                    Some(Ordering::Equal)
                } else if xs.is_superset(ys) {
                    Some(Ordering::Greater)
                } else if ys.is_superset(xs) {
                    Some(Ordering::Less)
                } else {
                    None
                }
            }
            (InitData::VarSet(_), InitData::Constant) => Some(Ordering::Greater), // TODO
            (InitData::Constant, InitData::VarSet(_)) => Some(Ordering::Less) // TODO
        }
    }
}

impl Semilattice for InitData {
    fn meet(&self, other: &Self, _loc: &LocIdx) -> Self {
        match (self, other) {
            (InitData::Constant, _) => other,
            (_, InitData::Constant) => self,
            (InitData::VarSet(xs), InitData::VarSet(ys)) => {
                InitData::VarSet(xs.union(ys).collect())
            }
        }
    }
}

#[derive(Eq, PartialEq, PartialOrd, Clone, Debug)]
pub struct LocalsLattice {
    v: VariableState<InitData>
}

impl Default for LocalsLattice {
    fn default() -> Self {
        LocalsLattice {
            v: VariableState {
                regs: X86RegsLattice {
                    rax: X86Regs::Rax,
                    rbx: X86Regs::Rbx,
                    rcx: X86Regs::Rcx,
                    rdx: X86Regs::Rdx,
                    rdi: X86Regs::Rdi,
                    rsi: X86Regs::Rsi,
                    rsp: X86Regs::Rsp,
                    rbp: X86Regs::Rbp,
                    r8: X86Regs::R8,
                    r9: X86Regs::R9,
                    r10: X86Regs::R10,
                    r11: X86Regs::R11,
                    r12: X86Regs::R12,
                    r13: X86Regs::R13,
                    r14: X86Regs::R14,
                    r15: X86Regs::R15,
                    zf: X86Regs::Zf
                },
                stack: Default::default()
            }
        }
    }
}

impl Semilattice for LocalsLattice {
    fn meet(&self, other: &Self, _loc_idx: &LocIdx) -> Self {
        todo!()
        // VariableState {
        //     regs: X86RegsLattice {
        //         rax: X86Regs::Rax,
        //         rbx: X86Regs::Rbx,
        //         rcx: X86Regs::Rcx,
        //         rdx: X86Regs::Rdx,
        //         rdi: X86Regs::Rdi,
        //         rsi: X86Regs::Rsi,
        //         rsp: X86Regs::Rsp,
        //         rbp: X86Regs::Rbp,
        //         r8: X86Regs::R8,
        //         r9: X86Regs::R9,
        //         r10: X86Regs::R10,
        //         r11: X86Regs::R11,
        //         r12: X86Regs::R12,
        //         r13: X86Regs::R13,
        //         r14: X86Regs::R14,
        //         r15: X86Regs::R15,
        //         zf: X86Regs::Zf
        //     },
        //     stack: Default::default()
        // }
    }
}

impl Lattice for LocalsLattice {}


// #[derive(PartialEq, Eq, Clone, Debug)]
// pub struct StackSlot<T: Lattice + Clone> {
//     pub size: u32,
//     pub value: T,
// }

// impl<T: Lattice + Clone> PartialOrd for StackSlot<T> {
//     fn partial_cmp(&self, other: &StackSlot<T>) -> Option<Ordering> {
//         if self.size != other.size {
//             None
//         } else {
//             self.value.partial_cmp(&other.value)
//         }
//     }
// }

// //Currently implemented with hashmap, could also use a vector for a dense map
// #[derive(Eq, Clone, Debug)]
// pub struct StackLattice<T: Lattice + Clone> {
//     pub offset: i64,
//     pub map: HashMap<i64, StackSlot<T>>,
// }

// impl<T: Lattice + Clone> StackLattice<T> {
//     pub fn update(&mut self, offset: i64, value: T, size: u32) -> () {
//         //Check if 4 aligned
//         if (offset & 3) != 0 {
//             panic!("Unsafe: Attempt to store value on the stack on not 4-byte aligned address.");
//         }
//         if size > 8 {
//             panic!("Store too large!");
//         }
//         //remove overlapping entries
//         //if write is size 8: remove next slot (offset + 4) if one exists
//         if size == 8 {
//             self.map.remove(&(self.offset + offset + 4));
//         }

//         // if next slot back (offset-4) is size 8, remove it
//         if let Some(x) = self.map.get(&(self.offset + offset - 4)) {
//             if x.size == 8 {
//                 self.map.remove(&(self.offset + offset - 4));
//             }
//         }

//         //if value is default, just delete entry map.remove(offset)
//         if value == Default::default() {
//             self.map.remove(&(self.offset + offset));
//         } else {
//             self.map.insert(
//                 self.offset + offset,
//                 StackSlot {
//                     size: size,
//                     value: value,
//                 },
//             );
//         }
//     }

//     pub fn get(&self, offset: i64, size: u32) -> T {
//         if !(size == 4 || size == 8) {
//             panic!("Load wrong size! size = {:?}", size);
//         }

//         match self.map.get(&(self.offset + offset)) {
//             Some(stack_slot) => {
//                 if stack_slot.size == size {
//                     stack_slot.value.clone()
//                 } else {
//                     Default::default()
//                 }
//             }
//             None => Default::default(),
//         }
//     }

//     pub fn update_stack_offset(&mut self, adjustment: i64) -> () {
//         if (adjustment & 3) != 0 {
//             panic!("Unsafe: Attempt to make stack not 4-byte aligned.");
//         }
//         self.offset += adjustment;
//     }
// }

// //check if StackLattice s1 is less than StackLattice s2
// fn hashmap_le<T: Lattice + Clone>(s1: &StackLattice<T>, s2: &StackLattice<T>) -> bool {
//     for (k1, v1) in s1.map.iter() {
//         if !s2.map.contains_key(k1) {
//             return false;
//         } else {
//             if s2.map.get(k1).unwrap() < v1 {
//                 return false;
//             } else {
//             }
//         }
//     }
//     true
// }

// impl<T: Lattice + Clone> PartialOrd for StackLattice<T> {
//     fn partial_cmp(&self, other: &StackLattice<T>) -> Option<Ordering> {
//         if self.offset != other.offset {
//             None
//         } else {
//             if hashmap_le(self, other) {
//                 Some(Ordering::Less)
//             } else if hashmap_le(other, self) {
//                 Some(Ordering::Greater)
//             } else if self == other {
//                 Some(Ordering::Equal)
//             } else {
//                 None
//             }
//         }
//     }
// }

// impl<T: Lattice + Clone> PartialEq for StackLattice<T> {
//     fn eq(&self, other: &StackLattice<T>) -> bool {
//         (self.map == other.map) && (self.offset == other.offset)
//     }
// }

// //assumes that stack offset is equal in both stack lattices
// impl<T: Lattice + Clone> Lattice for StackLattice<T> {
//     fn meet(&self, other: &Self, loc_idx: &LocIdx) -> Self {
//         let mut newmap: HashMap<i64, StackSlot<T>> = HashMap::new();
//         for (k, v1) in self.map.iter() {
//             match other.map.get(k) {
//                 Some(v2) => {
//                     if v1.size == v2.size {
//                         let new_v = v1.value.meet(&v2.value.clone(), loc_idx);
//                         if new_v != Default::default() {
//                             let newslot = StackSlot {
//                                 size: v1.size,
//                                 value: new_v,
//                             };
//                             newmap.insert(*k, newslot);
//                         }
//                     }
//                 }
//                 None => (),
//             }
//         }

//         StackLattice {
//             offset: self.offset,
//             map: newmap,
//         }
//     }
// }

// impl<T: Lattice + Clone> Default for StackLattice<T> {
//     fn default() -> Self {
//         StackLattice {
//             offset: 0,
//             map: HashMap::new(),
//         }
//     }
// }

// #[test]
// fn stack_lattice_test_eq() {
//     use crate::lattices::BooleanLattice;
//     let mut x1: StackLattice<BooleanLattice> = Default::default();
//     let mut x2: StackLattice<BooleanLattice> = Default::default();
//     assert_eq!(x1 == x2, true);

//     //check equality with adjusted stack
//     x1.update_stack_offset(4);
//     x2.update_stack_offset(4);
//     assert_eq!(x1 == x2, true);

//     //check inequality of different stack adjustments
//     x1.update_stack_offset(4);
//     x2.update_stack_offset(8);
//     assert_eq!(x1 == x2, false);
//     x1.update_stack_offset(4);
//     assert_eq!(x1 == x2, true);

//     let y1 = BooleanLattice { v: false };
//     let y2 = BooleanLattice { v: false };
//     let y3 = BooleanLattice { v: true };

//     //check equality with entries added
//     x1.update(4, y1, 4);
//     //adding a false does nothing
//     assert_eq!(x1 == x2, true);

//     x2.update(4, y2, 4);
//     assert_eq!(x1 == x2, true);

//     //check that different sizes break equality
//     x1.update(20, y3, 4);
//     x2.update(20, y3, 8);
//     assert_eq!(x1 != x2, true);

//     assert_eq!(x1.get(20, 4) == y3, true);
//     // should be false if we access with wrong size
//     assert_eq!(x1.get(20, 8) == y3, false);
//     assert_eq!(x1.get(20, 8) == y1, true);

//     //empty entry should return default
//     assert_eq!(x1.get(64, 8) == y1, true);
// }

// #[test]
// fn stack_lattice_test_ord() {
//     use crate::lattices::BooleanLattice;
//     let mut x1: StackLattice<BooleanLattice> = Default::default();
//     let mut x2: StackLattice<BooleanLattice> = Default::default();
//     let y1 = BooleanLattice { v: true };
//     let y2 = BooleanLattice { v: true };

//     //check 1 entry vs 0
//     x1.update(4, y1, 4);
//     assert_eq!(x1 == x2, false);
//     assert_eq!(x1 > x2, true);
//     assert_eq!(x1 < x2, false);

//     //check 2 entry vs 1
//     x1.update(8, y2, 4);
//     x2.update(4, y1, 4);
//     assert_eq!(x1 == x2, false);
//     assert_eq!(x1 > x2, true);
//     assert_eq!(x1 < x2, false);

//     //check meet of 1 entry vs 2
//     assert_eq!(x1.meet(&x2, &LocIdx { addr: 0, idx: 0 }) == x2, true);
// }

// #[test]
// fn stack_lattice_test_overlapping_entries() {
//     use crate::lattices::BooleanLattice;
//     let mut x1: StackLattice<BooleanLattice> = Default::default();
//     let mut x2: StackLattice<BooleanLattice> = Default::default();
//     let y1 = BooleanLattice { v: true };
//     let y2 = BooleanLattice { v: true };
//     let y3 = BooleanLattice { v: true };

//     //overlapping entries
//     x1.update_stack_offset(16);
//     x2.update_stack_offset(16);
//     x1.update(0, y2, 8);
//     x1.update(4, y1, 4);
//     x2.update(4, y3, 4);
//     print!("{:?} {:?}", x1, x2);
//     assert_eq!(x1 == x2, true);
// }
