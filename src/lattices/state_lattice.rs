use std::cmp::Ordering;
use std::fmt::Debug;
use std::collections::HashMap;
use std::convert::{TryFrom, From};
use std::hash::Hash;
use crate::utils::lifter::{MemArgs, MemArg, ValSize};
use crate::lattices::reaching_defs_lattice::LocIdx;
use crate::lattices::{Semilattice, Lattice};
use crate::utils::lifter::{Binopcode, Value};
use crate::utils::ir_utils::{get_imm_offset, is_rsp};

use self::X86Regs::*;

pub trait VarState {
    type Var;
    fn get(&self, index: &Value) -> Option<Self::Var>;
    fn set(&mut self, index: &Value, v: Self::Var) -> ();
    fn set_to_bot(&mut self, index: &Value) -> ();
    fn on_call(&mut self) -> ();
    fn adjust_stack_offset(&mut self, opcode: &Binopcode, dst: &Value, src1: &Value, src2: &Value);
}

#[derive(PartialEq, Clone, Eq, Debug, Copy, Hash)]
pub enum X86Regs {
    Rax,
    Rcx,
    Rdx,
    Rbx,
    Rsp,
    Rbp,
    Rsi,
    Rdi,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    Zf,
}

struct X86RegsIterator {
    current_reg: Option<X86Regs>
}

impl X86Regs {
    fn iter() -> X86RegsIterator {
        X86RegsIterator { current_reg: Some(Rax) }
    }
}

impl Iterator for X86RegsIterator {
    type Item = X86Regs;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current_reg {
            None => None,
            Some(reg) => {
                match reg {
                    Rax => {
                        self.current_reg = Some(Rcx);
                        return Some(Rax);
                    }
                    Rcx => {
                        self.current_reg = Some(Rdx);
                        return Some(Rcx);
                    }
                    Rdx => {
                        self.current_reg = Some(Rbx);
                        return Some(Rdx);
                    }
                    Rbx => {
                        self.current_reg = Some(Rsp);
                        return Some(Rbx);
                    }
                    Rsp => {
                        self.current_reg = Some(Rbp);
                        return Some(Rsp);
                    }
                    Rbp => {
                        self.current_reg = Some(Rsi);
                        return Some(Rbp);
                    }
                    Rsi => {
                        self.current_reg = Some(Rdi);
                        return Some(Rsi);
                    }
                    Rdi => {
                        self.current_reg = Some(R8);
                        return Some(Rdi);
                    }
                    R8 => {
                        self.current_reg = Some(R9);
                        return Some(R8);
                    }
                    R9 => {
                        self.current_reg = Some(R10);
                        return Some(R9);
                    }
                    R10 => {
                        self.current_reg = Some(R11);
                        return Some(R10);
                    }
                    R11 => {
                        self.current_reg = Some(R12);
                        return Some(R11);
                    }
                    R12 => {
                        self.current_reg = Some(R13);
                        return Some(R12);
                    }
                    R13 => {
                        self.current_reg = Some(R14);
                        return Some(R13);
                    }
                    R14 => {
                        self.current_reg = Some(R15);
                        return Some(R14);
                    }
                    R15 => {
                        self.current_reg = Some(Zf);
                        return Some(R15);
                    }
                    Zf => {
                        self.current_reg = None;
                        return Some(Zf);
                    }
                }
            }
        }
    }
}

impl TryFrom<&u8> for X86Regs {
    type Error = std::string::String;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Rax),
            1 => Ok(Rcx),
            2 => Ok(Rdx),
            3 => Ok(Rbx),
            4 => Ok(Rsp),
            5 => Ok(Rbp),
            6 => Ok(Rsi),
            7 => Ok(Rdi),
            8 => Ok(R8),
            9 => Ok(R9),
            10 => Ok(R10),
            11 => Ok(R11),
            12 => Ok(R12),
            13 => Ok(R13),
            14 => Ok(R14),
            15 => Ok(R15),
            16 => Ok(Zf),
            _ => Err(format!("Unknown register: index = {:?}", value)),
        }
    }
}

impl From<X86Regs> for u8 {
    fn from(value: X86Regs) -> Self {
        value as u8
    }
}


#[derive(PartialEq, Eq, Clone, Debug)]
pub struct VarSlot<T> {
    pub size: u32,
    pub value: T,
}

impl<T: PartialOrd> PartialOrd for VarSlot<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.size != other.size {
            None
        } else {
            self.value.partial_cmp(&other.value)
        }
    }
}

#[derive(PartialEq, Clone, Eq, Debug, Copy, Hash)]
pub enum VarIndex {
    Reg(X86Regs),
    Stack(i64)
}

// Currently implemented with hashmap, could also use a vector for a dense map
#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub struct VariableState<T> {
    pub stack_offset: i64,
    pub state: HashMap<VarIndex, VarSlot<T>>
}

// TODO(matt): does this get the edge cases right?
fn hashmap_lt<I: Eq + Hash, T: PartialOrd>(m1: &HashMap<I, T>, m2: &HashMap<I, T>) -> bool {
    for (k1, v1) in m1.iter() {
        if !m2.contains_key(k1) { // missing values are implicitly ⊥
            return false;
        } else {
            if m2.get(k1).unwrap() < v1 {
                return false;
            }
        }
    }
    true
}

impl<T: PartialOrd> PartialOrd for VariableState<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.stack_offset != other.stack_offset {
            None
        } else {
            if hashmap_lt(&self.state, &other.state) {
                Some(Ordering::Less)
            } else if hashmap_lt(&other.state, &self.state) {
                Some(Ordering::Greater)
            } else if self == other {
                Some(Ordering::Equal)
            } else {
                None
            }
        }
    }
}

impl<T: Semilattice + Clone> Semilattice for VariableState<T> {
    fn meet(&self, other: &Self, loc_idx: &LocIdx) -> Self {
        let mut newmap: HashMap<VarIndex, VarSlot<T>> = HashMap::new();
        for (var_index, v1) in self.state.iter() {
            match other.state.get(var_index) {
                Some(v2) => {
                    // TODO(matt): what if the sizes are different?
                    if v1.size == v2.size {
                        let new_v = v1.value.meet(&v2.value.clone(), loc_idx);
                        let newslot = VarSlot {
                            size: v1.size,
                            value: new_v,
                        };
                        newmap.insert(*var_index, newslot);
                    }
                }
                None => () // this means v2 = ⊥ so v1 ∧ v2 = ⊥
            }
        }
        VariableState {
            stack_offset: self.stack_offset,
            state: newmap,
        }
    }
}


impl<T: Semilattice + Default + Clone + Debug> Lattice for VariableState<T> { }

impl<T> VariableState<T> {
    pub fn update_stack_offset(&mut self, adjustment: i64) -> () {
        if (adjustment & 3) != 0 {
            panic!("Unsafe: Attempt to make stack not 4-byte aligned.");
        }
        self.stack_offset += adjustment;
    }

    pub fn update_stack_slot(&mut self, offset: i64, value: T, size: u32) -> () {
        //Check if 4 aligned
        if (offset & 3) != 0 {
            panic!("Unsafe: Attempt to store value on the stack on not 4-byte aligned address.");
        }
        if size > 8 {
            panic!("Store too large!");
        }
        //remove overlapping entries
        //if write is size 8: remove next slot (offset + 4) if one exists
        if size == 8 {
            self.state.remove(&VarIndex::Stack(self.stack_offset + offset + 4));
        }

        // if next slot back (offset-4) is size 8, remove it
        if let Some(x) = self.state.get(&VarIndex::Stack(self.stack_offset + offset - 4)) {
            if x.size == 8 {
                self.state.remove(&VarIndex::Stack(self.stack_offset + offset - 4));
            }
        }

        //if value is default, just delete entry map.remove(offset)
        self.state.insert(
            VarIndex::Stack(self.stack_offset + offset),
            VarSlot {
                size,
                value,
            },
        );
    }

    pub fn update_reg_slot(&mut self, index: &u8, size: &ValSize, value: T) -> () {
        if let ValSize::SizeOther = size {
            return;
        }
        let reg_index = match X86Regs::try_from(index) {
            Err(err) => panic!(err),
            Ok(reg) => reg,
        };
        self.state.insert(
            VarIndex::Reg(reg_index),
            VarSlot {
                size: todo!(),
                value
            }
        );
    }
}

impl<T: Default + Clone> VariableState<T> {
    pub fn get_stack_slot(&self, offset: i64, size: u32) -> T {
        if !(size == 4 || size == 8) {
            panic!("Load wrong size! size = {:?}", size);
        }

        match self.state.get(&VarIndex::Stack(self.stack_offset + offset)) {
            Some(stack_slot) => {
                if stack_slot.size == size {
                    stack_slot.value.clone()
                } else {
                    Default::default()
                }
            }
            None => Default::default(),
        }
    }

    pub fn get_var_slot(&self, index: &VarIndex) -> T {
        if let Some(slot) = self.state.get(index) {
            slot.value
        } else {
            Default::default()
        }
    }

    pub fn get_reg_slot(&self, index: &u8, size: &ValSize) -> T {
        if let ValSize::SizeOther = size {
            return Default::default();
        }
        let reg_index = match X86Regs::try_from(index) {
            Err(err) => panic!(err),
            Ok(reg) => reg,
        };
        match self.state.get(&VarIndex::Reg(reg_index)) {
            Some(reg_slot) => {
                reg_slot.value.clone() // TODO(matt): deal with size
            }
            None => Default::default(),
        }
    }
}

impl<T: Lattice + Clone> VarState for VariableState<T> {
    type Var = T;

    fn get(&self, index: &Value) -> Option<Self::Var> {
        match index {
            Value::Mem(memsize, memargs) => match memargs {
                MemArgs::Mem1Arg(arg) => {
                    if let MemArg::Reg(regnum, _) = arg {
                        if *regnum == u8::from(Rsp) {
                            return Some(self.get_stack_slot(0, memsize.to_u32() / 8));
                        }
                    }
                    None
                }
                MemArgs::Mem2Args(arg1, arg2) => {
                    if let MemArg::Reg(regnum, _) = arg1 {
                        if *regnum == u8::from(Rsp) {
                            if let MemArg::Imm(_, _, offset) = arg2 {
                                return Some(self.get_stack_slot(*offset, memsize.to_u32() / 8));
                            }
                        }
                    }
                    None
                }
                _ => None,
            },
            Value::Reg(regnum, s2) => Some(self.get_reg_slot(regnum, s2)),
            Value::Imm(_, _, _) => None,
        }
    }

    fn set(&mut self, index: &Value, value: Self::Var) -> () {
        match index {
            Value::Mem(memsize, memargs) => match memargs {
                MemArgs::Mem1Arg(arg) => {
                    if let MemArg::Reg(regnum, _) = arg {
                        if *regnum == u8::from(Rsp) {
                            self.update_stack_slot(0, value, memsize.to_u32() / 8)
                        }
                    }
                }
                MemArgs::Mem2Args(arg1, arg2) => {
                    if let MemArg::Reg(regnum, _) = arg1 {
                        if *regnum == u8::from(Rsp) {
                            if let MemArg::Imm(_, _, offset) = arg2 {
                                self.update_stack_slot(*offset, value, memsize.to_u32() / 8)
                            }
                        }
                    }
                }
                _ => (),
            },
            Value::Reg(regnum, s2) => {
                if let ValSize::SizeOther = s2 {
                    return;
                } else {
                    self.update_reg_slot(regnum, s2, value)
                }
            }
            Value::Imm(_, _, _) => panic!("Trying to write to an immediate value"),
        }
    }

    fn set_to_bot(&mut self, index: &Value) -> () {
        self.set(index, Default::default()) // TODO(matt): redo this with index function
    }

    // TODO(matt): when is this used and is it right?
    fn on_call(&mut self) -> () {
        for reg in X86Regs::iter() {
            self.state.remove(&VarIndex::Reg(reg));
        }
    }

    fn adjust_stack_offset(&mut self, opcode: &Binopcode, dst: &Value, src1: &Value, src2: &Value) {
        if is_rsp(dst) {
            if is_rsp(src1) {
                let adjustment = get_imm_offset(src2);
                //println!("opcode = {:?} {:?} = {:?} {:?} {:?}", opcode, dst, src1, src2, adjustment);
                match opcode {
                    Binopcode::Add => self.update_stack_offset(adjustment),
                    Binopcode::Sub => self.update_stack_offset(-adjustment),
                    _ => panic!("Illegal RSP write"),
                }
            } else {
                panic!("Illegal RSP write")
            }
        }
    }
}
