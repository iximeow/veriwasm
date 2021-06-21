use crate::analyses::AbstractAnalyzer;
use crate::utils::lifter::Value;
use crate::lattices::reachingdefslattice::LocIdx;
use crate::lattices::localslattice::*;
use crate::utils::lifter::{Binopcode, Stmt};


// discovering function arguments by what's used uninitialized

pub struct LocalsAnalyzer {}

impl LocalsAnalyzer {
    fn aeval_val(&self, state: &LocalsLattice, value: &Value) -> InitData {
        state.get(value).unwrap_or(Uninit)
    }

    // if all values are initialized then the value is initialized
    fn aeval_vals(&self, state: &LocalsLattice, values: &Vec<Value>) -> InitData {
        values.iter().fold(Init, |acc, value| -> InitData {
            if (acc == Init) && (self.aeval_val(state, value) == Init) {
                Init
            } else {
                Uninit
            }})
    }
}

impl AbstractAnalyzer<LocalsLattice> for LocalsAnalyzer {
    fn init_state(&self) -> LocalsLattice {
        LocalsLattice::default()
    }

    fn aexec(&self, in_state: &mut LocalsLattice, ir_instr: &Stmt, _loc_idx: &LocIdx) -> () {
        match ir_instr {
            Stmt::Clear(dst, srcs) => {
                in_state.set(dst, self.aeval_vals(in_state, srcs))
            }
            Stmt::Unop(_, dst, src) => {
                in_state.set(dst, self.aeval_val(in_state, src))
            }
            Stmt::Binop(_, dst, src1, src2) => {
                let dst_val = if (self.aeval_val(in_state, src1) == Init) && (self.aeval_val(in_state, src2) == Init) {
                    Init
                } else {
                    Uninit
                };
                in_state.set(dst, dst_val)
            }
            Stmt::Call(_) => (), // TODO
            _ => ()
        }
    }

    fn process_branch(
        &self,
        _irmap: &crate::utils::lifter::IRMap,
        in_state: &LocalsLattice,
        succ_addrs: &Vec<u64>,
        _addr: &u64,
    ) -> Vec<(u64, LocalsLattice)> {
        succ_addrs
            .into_iter()
            .map(|addr| (addr.clone(), in_state.clone()))
            .collect()
    }
}
