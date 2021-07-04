use std::convert::TryFrom;
use std::mem::discriminant;

use crate::ir::types::*;
use crate::loaders::utils::VW_Metadata;
use yaxpeax_arch::{AddressBase, Arch, LengthedInstruction};
use yaxpeax_core::analyses::control_flow::VW_CFG;
use yaxpeax_core::arch::x86_64::analyses::data_flow::Location;
use yaxpeax_core::arch::InstructionSpan;
use yaxpeax_core::data::{Direction, ValueLocations};
use yaxpeax_core::memory::repr::process::ModuleData;
use yaxpeax_x86::long_mode::Opcode::*;
use yaxpeax_x86::long_mode::{register_class, Arch as AMD64, Opcode, Operand, RegSpec, Instruction as X64Instruction};
use X86Regs::*;

pub fn valsize(num: u32) -> ValSize {
    ValSize::try_from_bits(num).unwrap()
}

pub fn mk_value_i64(num: i64) -> Value {
    Value::Imm(ImmType::Signed, ValSize::Size64, num)
}

fn get_reg_size(reg: yaxpeax_x86::long_mode::RegSpec) -> ValSize {
    let size = match reg.class() {
        register_class::Q => ValSize::Size64,
        register_class::D => ValSize::Size32,
        register_class::W => ValSize::Size16,
        register_class::B => ValSize::Size8,
        register_class::RB => ValSize::Size8,
        register_class::RIP => panic!("Write to RIP: {:?}", reg.class()),
        register_class::EIP => panic!("Write to EIP: {:?}", reg.class()),
        register_class::X => ValSize::Size128,
        register_class::Y => ValSize::Size256,
        register_class::Z => ValSize::Size512,
        _ => panic!("Unknown register bank: {:?}", reg.class()),
    };
    return size;
}

fn convert_reg(reg: yaxpeax_x86::long_mode::RegSpec) -> Value {
    let (num, size) = match (reg.num(), reg.class()) {
        (n, register_class::Q) => (n, ValSize::Size64),
        (n, register_class::D) => (n, ValSize::Size32),
        (n, register_class::W) => (n, ValSize::Size16),
        (n, register_class::B) => (n, ValSize::Size8),
        (n, register_class::RB) => (n, ValSize::Size8),
        (_, register_class::RIP) => panic!("Write to RIP: {:?}", reg.class()),
        (_, register_class::EIP) => panic!("Write to EIP: {:?}", reg.class()),
        (n, register_class::X) => (n + ValSize::fp_offset(), ValSize::Size128),
        (n, register_class::Y) => (n + ValSize::fp_offset(), ValSize::Size256),
        (n, register_class::Z) => (n + ValSize::fp_offset(), ValSize::Size512),
        _ => panic!("Unknown register bank: {:?}", reg.class()),
    };
    Value::Reg(X86Regs::try_from(num).unwrap(), size)
}

fn convert_memarg_reg(reg: yaxpeax_x86::long_mode::RegSpec) -> MemArg {
    let size = match reg.class() {
        register_class::Q => ValSize::Size64,
        register_class::D => ValSize::Size32,
        register_class::W => ValSize::Size16,
        register_class::B => ValSize::Size8,
        _ => panic!("Unknown register bank: {:?}", reg.class()),
    };
    MemArg::Reg(X86Regs::try_from(reg.num()).unwrap(), size)
}

fn convert_operand(op: yaxpeax_x86::long_mode::Operand, memsize: ValSize) -> Value {
    match op {
        Operand::ImmediateI8(imm) => Value::Imm(ImmType::Signed, ValSize::Size8, imm as i64),
        Operand::ImmediateU8(imm) => Value::Imm(ImmType::Unsigned, ValSize::Size8, imm as i64),
        Operand::ImmediateI16(imm) => Value::Imm(ImmType::Signed, ValSize::Size16, imm as i64),
        Operand::ImmediateU16(imm) => Value::Imm(ImmType::Unsigned, ValSize::Size16, imm as i64),
        Operand::ImmediateU32(imm) => Value::Imm(ImmType::Unsigned, ValSize::Size32, imm as i64),
        Operand::ImmediateI32(imm) => Value::Imm(ImmType::Signed, ValSize::Size32, imm as i64),
        Operand::ImmediateU64(imm) => Value::Imm(ImmType::Unsigned, ValSize::Size64, imm as i64),
        Operand::ImmediateI64(imm) => Value::Imm(ImmType::Signed, ValSize::Size64, imm as i64),
        Operand::Register(reg) => convert_reg(reg),
        //u32 and u64 are address sizes
        Operand::DisplacementU32(imm) => Value::Mem(
            memsize,
            MemArgs::Mem1Arg(MemArg::Imm(ImmType::Unsigned, ValSize::Size32, imm as i64)),
        ), //mem[c]
        Operand::DisplacementU64(imm) => Value::Mem(
            memsize,
            MemArgs::Mem1Arg(MemArg::Imm(ImmType::Unsigned, ValSize::Size64, imm as i64)),
        ), //mem[c]
        Operand::RegDeref(reg) if reg == RegSpec::rip() => Value::RIPConst,
        Operand::RegDeref(reg) => Value::Mem(memsize, MemArgs::Mem1Arg(convert_memarg_reg(reg))), // mem[reg]
        Operand::RegDisp(reg, _) if reg == RegSpec::rip() => Value::RIPConst,
        Operand::RegDisp(reg, imm) => Value::Mem(
            memsize,
            MemArgs::Mem2Args(
                convert_memarg_reg(reg),
                MemArg::Imm(ImmType::Signed, ValSize::Size32, imm as i64),
            ),
        ), //mem[reg + c]
        Operand::RegIndexBase(reg1, reg2) => Value::Mem(
            memsize,
            MemArgs::Mem2Args(convert_memarg_reg(reg1), convert_memarg_reg(reg2)),
        ), // mem[reg1 + reg2]
        Operand::RegIndexBaseDisp(reg1, reg2, imm) => Value::Mem(
            memsize,
            MemArgs::Mem3Args(
                convert_memarg_reg(reg1),
                convert_memarg_reg(reg2),
                MemArg::Imm(ImmType::Signed, ValSize::Size32, imm as i64),
            ),
        ), //mem[reg1 + reg2 + c]
        Operand::RegScale(_, _) => panic!("Memory operations with scaling prohibited"), // mem[reg * c]
        Operand::RegScaleDisp(_, _, _) => panic!("Memory operations with scaling prohibited"), //mem[reg*c1 + c2]
        Operand::RegIndexBaseScale(reg1, reg2, scale) =>
        //mem[reg1 + reg2*c]
        {
            if scale == 1 {
                Value::Mem(
                    memsize,
                    MemArgs::Mem2Args(convert_memarg_reg(reg1), convert_memarg_reg(reg2)),
                )
            } else {
                Value::Mem(
                    memsize,
                    MemArgs::MemScale(
                        convert_memarg_reg(reg1),
                        convert_memarg_reg(reg2),
                        MemArg::Imm(ImmType::Signed, ValSize::Size32, scale as i64),
                    ),
                )
            }
        }
        Operand::RegIndexBaseScaleDisp(reg1, reg2, scale, imm) => {
            assert_eq!(scale, 1);
            Value::Mem(
                memsize,
                MemArgs::Mem3Args(
                    convert_memarg_reg(reg1),
                    convert_memarg_reg(reg2),
                    MemArg::Imm(ImmType::Signed, ValSize::Size32, imm as i64),
                ),
            )
        } //mem[reg1 + reg2*c1 + c2]
        Operand::Nothing => panic!("Nothing Operand?"),
        op => {
            panic!("Unhandled operand {}", op);
        }
    }
}

// fn get_sources(instr: &yaxpeax_x86::long_mode::Instruction) -> Vec<Value> {
//     match instr.operand_count() {
//         0 => vec![],
//         1 => vec![convert_operand(instr.operand(0), ValSize::Size32)],
//         2 => vec![
//             convert_operand(instr.operand(0), ValSize::Size32),
//             convert_operand(instr.operand(1), ValSize::Size32),
//         ],
//         3 => vec![
//             convert_operand(instr.operand(0), ValSize::Size32),
//             convert_operand(instr.operand(1), ValSize::Size32),
//             convert_operand(instr.operand(2), ValSize::Size32),
//         ],
//         4 => vec![
//             convert_operand(instr.operand(0), ValSize::Size32),
//             convert_operand(instr.operand(1), ValSize::Size32),
//             convert_operand(instr.operand(2), ValSize::Size32),
//             convert_operand(instr.operand(3), ValSize::Size32),
//         ],
//         _ => panic!("Too many arguments?"),
//     }
// }

// fn clear_dst(instr: &yaxpeax_x86::long_mode::Instruction) -> Vec<Stmt> {
//     let uses_vec = <AMD64 as ValueLocations>::decompose(instr);
//     let writes_to_zf = uses_vec.iter().any(|(loc, dir)| match (loc, dir) {
//         (Some(Location::ZF), Direction::Write) => true,
//         _ => false,
//     });
//     let writes_to_cf = uses_vec.iter().any(|(loc, dir)| match (loc, dir) {
//         (Some(Location::CF), Direction::Write) => true,
//         _ => false,
//     });
//     let srcs: Vec<Value> = get_sources(instr);
//     let mut stmts: Vec<Stmt> = Vec::new();

//     stmts.push(Stmt::Clear(
//         convert_operand(instr.operand(0), ValSize::Size8),
//         srcs.clone(),
//     ));
//     if writes_to_zf {
//         stmts.push(Stmt::Clear(Value::Reg(Zf, ValSize::Size8), srcs.clone()));
//     };
//     if writes_to_cf {
//         stmts.push(Stmt::Clear(Value::Reg(Cf, ValSize::Size8), srcs));
//     };
//     stmts
// }

// Generic handling for unknown opcodes.
// fn generic_clear(instr: &yaxpeax_x86::long_mode::Instruction) -> Vec<Stmt> {
//     let uses_vec = <AMD64 as ValueLocations>::decompose(instr);
//     let writes_to_zf = uses_vec.iter().any(|(loc, dir)| match (loc, dir) {
//         (Some(Location::ZF), Direction::Write) => true,
//         _ => false,
//     });
//     let writes_to_cf = uses_vec.iter().any(|(loc, dir)| match (loc, dir) {
//         (Some(Location::CF), Direction::Write) => true,
//         _ => false,
//     });
//     let mut stmts = vec![];

//     for (loc, dir) in uses_vec {
//         match (loc, dir) {
//             (Some(Location::Register(reg)), Direction::Write) => {
//                 stmts.push(Stmt::Clear(convert_reg(reg), vec![]));
//             }
//             _ => {}
//         }
//     }
//     // TODO: dangerous
//     if writes_to_zf {
//         stmts.push(Stmt::Clear(Value::Reg(Zf, ValSize::Size8), vec![]));
//     }
//     if writes_to_cf {
//         stmts.push(Stmt::Clear(Value::Reg(Cf, ValSize::Size8), vec![]));
//     }

//     stmts
// }

// Captures all register and flag sources 
// TODO: Memory?
fn get_sources(instr: &X64Instruction) -> Vec<Value> {
    let uses_vec = <AMD64 as ValueLocations>::decompose(instr);
    let mut sources = Vec::new();
    for (loc, dir) in uses_vec {
        match (loc, dir) {
            (Some(Location::Register(reg)), Direction::Read) => {
                sources.push(convert_reg(reg));
            }
            (Some(Location::ZF), Direction::Read) => {
                sources.push(Value::Reg(Zf, ValSize::Size8));
            }
            (Some(Location::CF), Direction::Read) => {
                sources.push(Value::Reg(Cf, ValSize::Size8));
            }
            _ => {}
        }
    }
    return sources;
}

// Captures all register and flag destinations
// TODO: Memory?
fn get_destinations(instr: &X64Instruction) -> Vec<Value> {
    let uses_vec = <AMD64 as ValueLocations>::decompose(instr);
    let mut destinations = Vec::new();
    for (loc, dir) in uses_vec {
        match (loc, dir) {
            (Some(Location::Register(reg)), Direction::Write) => {
                destinations.push(convert_reg(reg));
            }
            (Some(Location::ZF), Direction::Write) => {
                destinations.push(Value::Reg(Zf, ValSize::Size8));
            }
            (Some(Location::CF), Direction::Write) => {
                destinations.push(Value::Reg(Cf, ValSize::Size8));
            }
            _ => {}
        }
    }
    return destinations;
}

// fn clear_dst(instr: &yaxpeax_x86::long_mode::Instruction) -> Vec<Stmt> {
//     let uses_vec = <AMD64 as ValueLocations>::decompose(instr);
//     let srcs: Vec<Value> = get_sources(&uses_vec);
//     let mut stmts: Vec<Stmt> = Vec::new();
//     stmts.push(Stmt::Clear(convert_operand(dst), sources.clone()));

//     stmts.push(Stmt::Clear(
//         convert_operand(instr.operand(0), ValSize::Size8),
//         srcs.clone(),
//     ));
//     if writes_to_zf {
//         stmts.push(Stmt::Clear(Value::Reg(Zf, ValSize::Size8), srcs.clone()));
//     };
//     if writes_to_cf {
//         stmts.push(Stmt::Clear(Value::Reg(Cf, ValSize::Size8), srcs));
//     };
//     stmts
// }

fn generic_clear(instr: &X64Instruction) -> Vec<Stmt> {
    // let uses_vec = <AMD64 as ValueLocations>::decompose(instr);
    let mut stmts = vec![];
    let sources = get_sources(&instr);
    let dsts = get_destinations(&instr);

    for dst in dsts{
        stmts.push(Stmt::Clear(dst, sources.clone()));
    }

    // for (loc, dir) in uses_vec {
    //     match (loc, dir) {
    //         (Some(Location::Register(reg)), Direction::Write) => {
    //             stmts.push(Stmt::Clear(convert_reg(reg), sources));
    //         }
    //         (Some(Location::ZF), Direction::Write) => {
    //             stmts.push(Stmt::Clear(Value::Reg(Zf, ValSize::Size8), sources));
    //         }
    //         (Some(Location::CF), Direction::Write) => {
    //             stmts.push(Stmt::Clear(Value::Reg(Cf, ValSize::Size8), sources));
    //         }
    //         _ => {}
    //     }
    // }
    stmts
}

fn get_operand_size(op: &yaxpeax_x86::long_mode::Operand) -> Option<ValSize> {
    match op {
        Operand::ImmediateI8(_) | Operand::ImmediateU8(_) => Some(ValSize::Size8),
        Operand::ImmediateI16(_) | Operand::ImmediateU16(_) => Some(ValSize::Size16),
        Operand::ImmediateU32(_) | Operand::ImmediateI32(_) => Some(ValSize::Size32),
        Operand::ImmediateU64(_) | Operand::ImmediateI64(_) => Some(ValSize::Size64),
        Operand::Register(reg) => Some(get_reg_size(*reg)),
        //u32 and u64 are address sizes
        Operand::DisplacementU32(_)
        | Operand::DisplacementU64(_)
        | Operand::RegDeref(_)
        | Operand::RegDisp(_, _)
        | Operand::RegIndexBase(_, _)
        | Operand::RegIndexBaseDisp(_, _, _)
        | Operand::RegScale(_, _)
        | Operand::RegScaleDisp(_, _, _)
        | Operand::RegIndexBaseScale(_, _, _)
        | Operand::RegIndexBaseScaleDisp(_, _, _, _)
        | Operand::Nothing => None,
        op => {
            panic!("unsupported operand size: {}", op);
        }
    }
}

fn set_from_flags(operand: Operand, flags: Vec<X86Regs>) -> Stmt {
    Stmt::Clear(
        convert_operand(operand, ValSize::Size8),
        flags
            .iter()
            .map(|flag| Value::Reg(*flag, ValSize::Size8))
            .collect(),
    )
}

fn unop(opcode: Unopcode, instr: &yaxpeax_x86::long_mode::Instruction) -> Stmt {
    let memsize = match (
        get_operand_size(&instr.operand(0)),
        get_operand_size(&instr.operand(1)),
    ) {
        (None, None) => panic!("Two Memory Args?"),
        (Some(x), None) => x,
        (None, Some(x)) => x,
        (Some(x), Some(_y)) => x,
    };
    Stmt::Unop(
        opcode,
        convert_operand(instr.operand(0), memsize),
        convert_operand(instr.operand(1), memsize),
    )
}

fn unop_w_memsize(
    opcode: Unopcode,
    instr: &yaxpeax_x86::long_mode::Instruction,
    memsize: ValSize,
) -> Stmt {
    Stmt::Unop(
        opcode,
        convert_operand(instr.operand(0), memsize),
        convert_operand(instr.operand(1), memsize),
    )
}

fn binop(opcode: Binopcode, instr: &yaxpeax_x86::long_mode::Instruction) -> Stmt {
    let memsize = match (
        get_operand_size(&instr.operand(0)),
        get_operand_size(&instr.operand(1)),
    ) {
        (None, None) => panic!("Two Memory Args?"),
        (Some(x), None) => x,
        (None, Some(x)) => x,
        (Some(x), Some(_y)) => x,
    };
    // if two operands than dst is src1
    if instr.operand_count() == 2 {
        Stmt::Binop(
            opcode,
            convert_operand(instr.operand(0), memsize),
            convert_operand(instr.operand(0), memsize),
            convert_operand(instr.operand(1), memsize),
        )
    } else {
        Stmt::Binop(
            opcode,
            convert_operand(instr.operand(0), memsize),
            convert_operand(instr.operand(1), memsize),
            convert_operand(instr.operand(2), memsize),
        )
    }
}

fn branch(instr: &yaxpeax_x86::long_mode::Instruction) -> Stmt {
    Stmt::Branch(
        instr.opcode(),
        convert_operand(instr.operand(0), ValSize::Size64),
    )
}

fn call(instr: &yaxpeax_x86::long_mode::Instruction, _metadata: &VW_Metadata) -> Stmt {
    let dst = convert_operand(instr.operand(0), ValSize::Size64);
    Stmt::Call(dst)
}

fn lea(instr: &yaxpeax_x86::long_mode::Instruction, addr: &Addr) -> Vec<Stmt> {
    let dst = instr.operand(0);
    let src1 = instr.operand(1);
    if let Operand::RegDisp(reg, disp) = src1 {
        if reg == RegSpec::rip() {
            //addr + instruction length + displacement
            let length = 0u64.wrapping_offset(instr.len()).to_linear();
            let target = (*addr as i64) + (length as i64) + (disp as i64);
            return vec![Stmt::Unop(
                Unopcode::Mov,
                convert_operand(dst.clone(), get_operand_size(&dst).unwrap()),
                Value::Imm(ImmType::Signed, ValSize::Size64, target),
            )];
        }
    }
    match convert_operand(src1, get_operand_size(&dst).unwrap()) {
        Value::Mem(_, memargs) => match memargs {
            MemArgs::Mem1Arg(arg) => match arg {
                MemArg::Imm(_, _, _val) => vec![unop(Unopcode::Mov, instr)],
                _ => generic_clear(instr),//clear_dst(instr),
            },
            _ => generic_clear(instr),//clear_dst(instr),
        },
        _ => panic!("Illegal lea"),
    }
}

pub fn lift(
    instr: &yaxpeax_x86::long_mode::Instruction,
    addr: &Addr,
    metadata: &VW_Metadata,
) -> Vec<Stmt> {
    log::debug!("lift: addr 0x{:x} instr {:?}", addr, instr);
    let mut instrs = Vec::new();
    match instr.opcode() {
        Opcode::MOV => instrs.push(unop_w_memsize(Unopcode::Mov, instr, ValSize::Size32)),
        Opcode::MOVQ => instrs.push(unop_w_memsize(Unopcode::Mov, instr, ValSize::Size64)),
        Opcode::MOVZX_b |
        Opcode::MOVZX_w => instrs.push(unop(Unopcode::Mov, instr)),

        Opcode::MOVD  => instrs.push(unop_w_memsize(Unopcode::Mov, instr, ValSize::Size32)),
        Opcode::MOVSD => instrs.push(unop_w_memsize(Unopcode::Mov, instr, ValSize::Size64)),

        Opcode::MOVSX |
        Opcode::MOVSX_w |
        Opcode::MOVSX_b |
        Opcode::MOVSXD => instrs.push(unop(Unopcode::Movsx, instr)),

        Opcode::LEA => instrs.extend(lea(instr, addr)),

        Opcode::TEST => {
            let memsize = match (
                get_operand_size(&instr.operand(0)),
                get_operand_size(&instr.operand(1)),
            ) {
                (None, None) => panic!("Two Memory Args?"),
                (Some(x), None) => x,
                (None, Some(x)) => x,
                (Some(x), Some(_y)) => x,
            };
            instrs.push(Stmt::Binop(
                Binopcode::Test,
                Value::Reg(Zf, ValSize::Size8),
                convert_operand(instr.operand(0), memsize),
                convert_operand(instr.operand(1), memsize),
            ));
            instrs.push(Stmt::Binop(
                Binopcode::Test,
                Value::Reg(Cf, ValSize::Size8),
                convert_operand(instr.operand(0), memsize),
                convert_operand(instr.operand(1), memsize),
            ));
        }

        Opcode::UCOMISS
        | Opcode::UCOMISD
        | Opcode::CMP => {
            let memsize = match (
                get_operand_size(&instr.operand(0)),
                get_operand_size(&instr.operand(1)),
            ) {
                (None, None) => panic!("Two Memory Args?"),
                (Some(x), None) => x,
                (None, Some(x)) => x,
                (Some(x), Some(_y)) => x,
            };
            instrs.push(Stmt::Binop(
                Binopcode::Cmp,
                Value::Reg(Zf, ValSize::Size8),
                convert_operand(instr.operand(0), memsize),
                convert_operand(instr.operand(1), memsize),
            ));
            instrs.push(Stmt::Binop(
                Binopcode::Cmp,
                Value::Reg(Cf, ValSize::Size8),
                convert_operand(instr.operand(0), memsize),
                convert_operand(instr.operand(1), memsize),
            ));
            instrs.push(Stmt::Binop(
                Binopcode::Cmp,
                Value::Reg(Pf, ValSize::Size8),
                convert_operand(instr.operand(0), memsize),
                convert_operand(instr.operand(1), memsize),
            ));
            instrs.push(Stmt::Binop(
                Binopcode::Cmp,
                Value::Reg(Sf, ValSize::Size8),
                convert_operand(instr.operand(0), memsize),
                convert_operand(instr.operand(1), memsize),
            ));
            instrs.push(Stmt::Binop(
                Binopcode::Cmp,
                Value::Reg(Of, ValSize::Size8),
                convert_operand(instr.operand(0), memsize),
                convert_operand(instr.operand(1), memsize),
            ));
        },

        Opcode::AND => {
            instrs.push(binop(Binopcode::And, instr));
            instrs.push(Stmt::Clear(
                Value::Reg(Zf, ValSize::Size8),
                get_sources(instr),
            ))
        }
        Opcode::ADD => {
            instrs.push(binop(Binopcode::Add, instr));
            instrs.push(Stmt::Clear(
                Value::Reg(Zf, ValSize::Size8),
                get_sources(instr),
            ))
        }
        Opcode::SUB => {
            instrs.push(binop(Binopcode::Sub, instr));
            instrs.push(Stmt::Clear(
                Value::Reg(Zf, ValSize::Size8),
                get_sources(instr),
            ))
        }
        Opcode::SHL => {
            instrs.push(binop(Binopcode::Shl, instr));
            instrs.push(Stmt::Clear(
                Value::Reg(Zf, ValSize::Size8),
                get_sources(instr),
            ))
        }

        Opcode::CMOVNB => {
            // Part of Spectre mitigation. Assume CMOV never happens (if it does, we just trap).

            /* nothing */
        }

        Opcode::UD2 => instrs.push(Stmt::Undefined),

        Opcode::RETURN => instrs.push(Stmt::Ret),

        Opcode::JMP => instrs.push(branch(instr)),
        Opcode::JO
        | Opcode::JNO
        | Opcode::JB
        | Opcode::JNB
        | Opcode::JZ
        | Opcode::JNZ
        | Opcode::JA
        | Opcode::JNA
        | Opcode::JS
        | Opcode::JNS
        | Opcode::JP
        | Opcode::JNP
        | Opcode::JL
        | Opcode::JGE
        | Opcode::JLE
        | Opcode::JG => instrs.push(branch(instr)),

        Opcode::CALL => instrs.push(call(instr, metadata)),

        Opcode::PUSH => {
            let width = instr.operand(0).width();
            assert_eq!(width, 8); //8 bytes
            instrs.push(Stmt::Binop(
                Binopcode::Sub,
                Value::Reg(Rsp, ValSize::Size64),
                Value::Reg(Rsp, ValSize::Size64),
                mk_value_i64(width.into()),
            ));
            instrs.push(Stmt::Unop(
                Unopcode::Mov,
                Value::Mem(
                    valsize((width * 8) as u32),
                    MemArgs::Mem1Arg(MemArg::Reg(Rsp, ValSize::Size64)),
                ),
                convert_operand(instr.operand(0), ValSize::Size64),
            ))
        }
        Opcode::POP => {
            let width = instr.operand(0).width();
            assert_eq!(width, 8); //8 bytes
            instrs.push(Stmt::Unop(
                Unopcode::Mov,
                convert_operand(instr.operand(0), ValSize::Size64),
                Value::Mem(
                    valsize((width * 8) as u32),
                    MemArgs::Mem1Arg(MemArg::Reg(Rsp, ValSize::Size64)),
                ),
            ));
            instrs.push(Stmt::Binop(
                Binopcode::Add,
                Value::Reg(Rsp, ValSize::Size64),
                Value::Reg(Rsp, ValSize::Size64),
                mk_value_i64(width.into()),
            ))
        }

        Opcode::NOP | Opcode::FILD | Opcode::STD | Opcode::CLD | Opcode::STI => (),
        Opcode::IDIV | Opcode::DIV => {
            // instrs.push(Stmt::Clear(Value::Reg(Zf, ValSize::Size8), vec![]));
            instrs.push(Stmt::Clear(Value::Reg(Rax, ValSize::Size64), vec![])); // clear RAX
            instrs.push(Stmt::Clear(Value::Reg(Rdx, ValSize::Size64), vec![])); // clear RDX
            instrs.push(Stmt::Clear(
                Value::Reg(Zf, ValSize::Size8),
                get_sources(instr),
            ));
        }

        Opcode::XORPS // TODO: do we need to generalize the size logic?
        | Opcode::XORPD
        | Opcode::XOR => {
            //XOR reg, reg => mov reg, 0
            if instr.operand_count() == 2 && instr.operand(0) == instr.operand(1) {
                instrs.push(Stmt::Unop(
                    Unopcode::Mov,
                    convert_operand(instr.operand(0), ValSize::Size64),
                    Value::Imm(ImmType::Signed, ValSize::Size64, 0),
                ));
                instrs.push(Stmt::Clear(
                    Value::Reg(Zf, ValSize::Size8),
                    get_sources(instr),
                ));
            } else {
                instrs.extend(generic_clear(instr));
                // instrs.extend(clear_dst(instr))
            }
        }

        // Opcode::CDQ | Opcode::CDQE => {
        //     // clear rax
        //     instrs.push(Stmt::Clear(Value::Reg(Rax, ValSize::Size64), vec![]));
        //     // clear rdx
        //     instrs.push(Stmt::Clear(Value::Reg(Rdx, ValSize::Size64), vec![]));
        // }

        SETG
        | SETLE => instrs.push(set_from_flags(instr.operand(0), vec![Zf, Sf, Of])),

        SETO
        | SETNO => instrs.push(set_from_flags(instr.operand(0), vec![Of])),

        SETS
        | SETNS => instrs.push(set_from_flags(instr.operand(0), vec![Sf])),

        SETGE
        | SETL => instrs.push(set_from_flags(instr.operand(0), vec![Sf, Of])),

        SETNZ
        | SETZ => instrs.push(set_from_flags(instr.operand(0), vec![Zf])),

        SETAE
        | SETB => instrs.push(set_from_flags(instr.operand(0), vec![Cf])),


        SETA
        | SETBE => instrs.push(set_from_flags(instr.operand(0), vec![Cf, Zf])),

        SETP
        | SETNP => instrs.push(set_from_flags(instr.operand(0), vec![Pf])),

        Opcode::BSF => {
            instrs.push(Stmt::Clear(
                Value::Reg(Zf, ValSize::Size8),
                vec![convert_operand(instr.operand(1), get_operand_size(&instr.operand(1)).unwrap())],
            ));
            instrs.push(Stmt::Clear(
                convert_operand(instr.operand(0), get_operand_size(&instr.operand(0)).unwrap()),
                vec![
                    convert_operand(instr.operand(0), get_operand_size(&instr.operand(0)).unwrap()),
                    convert_operand(instr.operand(1), get_operand_size(&instr.operand(1)).unwrap()),
                ],
            ));
        }
        Opcode::LZCNT => {
            instrs.push(Stmt::Clear(
                Value::Reg(Zf, ValSize::Size8),
                vec![convert_operand(instr.operand(1), get_operand_size(&instr.operand(1)).unwrap())],
            ));
            instrs.push(Stmt::Clear(
                convert_operand(instr.operand(0), get_operand_size(&instr.operand(0)).unwrap()),
                vec![
                    convert_operand(instr.operand(1), get_operand_size(&instr.operand(1)).unwrap()),
                ],
            ));
        }

        // TODO: is this right?
        Opcode::MOVSS => {
            instrs.push(unop_w_memsize(Unopcode::Mov, instr, ValSize::Size32));
        }
        Opcode::MOVAPS => {
            instrs.push(unop(Unopcode::Mov, instr));
        }
        Opcode::CVTSI2SS => {
            instrs.push(Stmt::Clear(
                convert_operand(instr.operand(0), get_operand_size(&instr.operand(0)).unwrap()),
                vec![
                    convert_operand(instr.operand(1), get_operand_size(&instr.operand(1)).unwrap()),
                ],
            ));
        }

        Opcode::OR
        | Opcode::SHR
        | Opcode::RCL
        | Opcode::RCR
        | Opcode::ROL
        | Opcode::ROR
        | Opcode::CMOVA
        | Opcode::CMOVB
        | Opcode::CMOVG
        | Opcode::CMOVGE
        | Opcode::CMOVL
        | Opcode::CMOVLE
        | Opcode::CMOVNA
        /* | Opcode::CMOVNB -- see above */
        | Opcode::CMOVNO
        | Opcode::CMOVNP
        | Opcode::CMOVNS
        | Opcode::CMOVNZ
        | Opcode::CMOVO
        | Opcode::CMOVP
        | Opcode::CMOVS
        | Opcode::CMOVZ
        | Opcode::SAR
        | Opcode::ADC
        | Opcode::ROUNDSS
        | Opcode::MUL
        | Opcode::IMUL
        | Opcode::POR
        | Opcode::PSHUFB
        | Opcode::PSHUFD
        | Opcode::PTEST
        | Opcode::PXOR
        | Opcode::ANDNPS
        | Opcode::CMPPD
        | Opcode::CMPPS
        | Opcode::ANDPS
        | Opcode::ORPS
        | Opcode::DIVSD
        | Opcode::MULSS
        | Opcode::ADDSD
        | Opcode::SUBSS
        | Opcode::ROUNDSD
        | Opcode::NOT
        | Opcode::POPCNT
        | Opcode::SUBSD
        | Opcode::MULSD
        | Opcode::DIVSS
        // | Opcode::LZCNT
        | Opcode::DIVPD
        | Opcode::DIVPS
        | Opcode::BLENDVPS
        | Opcode::BLENDVPD
        | Opcode::MAXPD
        | Opcode::MAXPS
        | Opcode::MAXSD
        | Opcode::MAXSS
        | Opcode::MINPD
        | Opcode::MINPS
        | Opcode::MINSD
        | Opcode::MINSS
        | Opcode::MULPD
        | Opcode::MULPS
        | Opcode::PMULLW
        | Opcode::PMULLD
        | Opcode::CVTDQ2PS
        | Opcode::CVTSD2SS
        | Opcode::CVTSI2SD
        | Opcode::CVTSS2SD
        | Opcode::CVTTSS2SI
        | Opcode::ADDPS
        | Opcode::ADDPD
        | Opcode::ADDSS
        | Opcode::PSLLW
        | Opcode::PSLLD
        | Opcode::PSLLQ
        | Opcode::PSRLW
        | Opcode::PSRLD
        | Opcode::PSRLQ
        | Opcode::PSRAW
        | Opcode::PSRAD
        | Opcode::PSUBB
        | Opcode::PSUBW
        | Opcode::PSUBD
        | Opcode::PSUBQ
        | Opcode::PSUBSB
        | Opcode::PSUBSW
        | Opcode::PSUBUSB
        | Opcode::PSUBUSW
        | Opcode::PUNPCKHBW
        | Opcode::PUNPCKHWD
        | Opcode::PUNPCKHDQ
        | Opcode::PUNPCKHQDQ
        | Opcode::PUNPCKLBW
        | Opcode::PUNPCKLWD
        | Opcode::PUNPCKLDQ
        | Opcode::PUNPCKLQDQ
        | Opcode::PACKSSWB
        | Opcode::PACKSSDW
        | Opcode::PADDB
        | Opcode::PADDD
        | Opcode::PADDQ
        | Opcode::PADDW
        | Opcode::PADDSB
        | Opcode::PADDSW
        | Opcode::PADDUSB
        | Opcode::PADDUSW
        | Opcode::PAND
        | Opcode::PANDN
        | Opcode::PAVGB
        | Opcode::PAVGW
        | Opcode::PCMPEQB
        | Opcode::PCMPEQD
        | Opcode::PCMPEQQ
        | Opcode::PCMPEQW
        | Opcode::PCMPGTB
        | Opcode::PCMPGTD
        | Opcode::PCMPGTQ
        | Opcode::PCMPGTW
        | Opcode::PEXTRB
        | Opcode::PEXTRW
        | Opcode::PINSRB
        | Opcode::PINSRW
        | Opcode::PMAXSB
        | Opcode::PMAXSW
        | Opcode::PMAXUB
        | Opcode::PMAXUD
        | Opcode::PMAXUW
        | Opcode::PMINSB
        | Opcode::PMINSD
        | Opcode::PMINSW
        | Opcode::PMINUB
        | Opcode::PMINUD
        | Opcode::PMINUW
        | Opcode::PMOVSXBW
        | Opcode::PMOVSXWD
        | Opcode::PMOVSXDQ
        | Opcode::PMOVZXBW
        | Opcode::PMOVZXWD
        | Opcode::PMOVZXDQ
        | Opcode::SQRTPD
        | Opcode::SQRTPS
        | Opcode::SQRTSD
        | Opcode::SQRTSS
        | Opcode::MOVLPS
        | Opcode::MOVLHPS
        | Opcode::MOVUPS
        | Opcode::SUBPD
        | Opcode::SUBPS
        | Opcode::TZCNT
        | Opcode::SBB
        | Opcode::BSR
        | Opcode::ANDPD
        | Opcode::ORPD => instrs.extend(generic_clear(instr)),/*instrs.extend(clear_dst(instr)),*/

        _ => unimplemented!(),/*instrs.extend(generic_clear(instr)),*/
    };
    instrs
}

fn parse_probestack_arg<'a>(
    instrs: BlockInstrs<'a>,
    metadata: &VW_Metadata,
) -> IResult<'a, (u64, StmtResult)> {
    let (rest, (addr, move_instr)) = parse_single_instr(instrs, metadata)?;
    if move_instr.len() != 1 {
        return Err(ParseErr::Error(instrs));
    }
    if let Stmt::Unop(Unopcode::Mov, Value::Reg(Rax, ValSize::Size32), Value::Imm(_, _, x)) =
        move_instr[0]
    {
        return Ok((rest, (x as u64, (addr, move_instr))));
    }
    Err(ParseErr::Error(instrs))
}

fn parse_probestack_call<'a>(
    instrs: BlockInstrs<'a>,
    metadata: &VW_Metadata,
) -> IResult<'a, StmtResult> {
    let (rest, (addr, call_instr)) = parse_single_instr(instrs, metadata)?;
    if call_instr.len() != 1 {
        return Err(ParseErr::Error(instrs));
    }
    if let Stmt::Call(Value::Imm(_, _, offset)) = call_instr[0] {
        if 5 + offset + (addr as i64) == metadata.lucet_probestack as i64 {
            return Ok((rest, (addr, call_instr)));
        }
    }
    Err(ParseErr::Error(instrs))
}

fn parse_probestack_suffix<'a>(
    instrs: BlockInstrs<'a>,
    metadata: &VW_Metadata,
) -> IResult<'a, StmtResult> {
    let (rest, (addr, sub_instr)) = parse_single_instr(instrs, metadata)?;
    if sub_instr.len() != 1 {
        return Err(ParseErr::Error(instrs));
    }
    if let Stmt::Binop(
        Binopcode::Sub,
        Value::Reg(Rsp, ValSize::Size64),
        Value::Reg(Rax, ValSize::Size64),
        _,
    ) = sub_instr[0]
    {
        return Ok((rest, (addr, sub_instr)));
    }
    Err(ParseErr::Error(instrs))
}

fn parse_probestack<'a>(
    instrs: BlockInstrs<'a>,
    metadata: &VW_Metadata,
) -> IResult<'a, StmtResult> {
    let (rest, (probestack_arg, (addr, mov_instr))) = parse_probestack_arg(instrs, metadata)?;
    let (rest, (_, call_instr)) = parse_probestack_call(rest, metadata)?;
    let (rest, (_, suffix_instr)) = parse_probestack_suffix(rest, metadata)?;
    let mut stmts = Vec::new();
    stmts.extend(mov_instr);
    stmts.push(Stmt::ProbeStack(probestack_arg));
    Ok((rest, (addr, stmts)))
}

// returns (addr, operand(0), operand(1))
fn parse_bsf<'a>(instrs: BlockInstrs<'a>) -> IResult<'a, (Addr, Value, Value)> {
    if let Some(((addr, instr), rest)) = instrs.split_first() {
        if let Opcode::BSF = instr.opcode() {
            return Ok((
                rest,
                (
                    *addr,
                    convert_operand(
                        instr.operand(0),
                        get_operand_size(&instr.operand(0)).unwrap(),
                    ),
                    convert_operand(
                        instr.operand(1),
                        get_operand_size(&instr.operand(1)).unwrap(),
                    ),
                ),
            ));
        }
    }
    Err(ParseErr::Incomplete)
}

// returns src of the cmove (dst must be the same)
fn parse_cmovez<'a>(instrs: BlockInstrs<'a>, bsf_dst: &Value) -> IResult<'a, (Addr, Value)> {
    if let Some(((addr, instr), rest)) = instrs.split_first() {
        if let Opcode::CMOVZ = instr.opcode() {
            let mov_dst = convert_operand(
                instr.operand(0),
                get_operand_size(&instr.operand(0)).unwrap(),
            );
            if let (Value::Reg(bsf_dst_reg, _), Value::Reg(mov_dst_reg, _)) = (bsf_dst, mov_dst) {
                if *bsf_dst_reg == mov_dst_reg {
                    return Ok((
                        rest,
                        (
                            *addr,
                            convert_operand(
                                instr.operand(1),
                                get_operand_size(&instr.operand(1)).unwrap(),
                            ),
                        ),
                    ));
                }
            }
        }
    }
    Err(ParseErr::Error(instrs))
}

fn parse_bsf_cmove<'a>(instrs: BlockInstrs<'a>, metadata: &VW_Metadata) -> IResult<'a, StmtResult> {
    let (rest, (addr, bsf_dst, bsf_src)) = parse_bsf(instrs)?;
    let (rest, (_addr, mov_src)) = parse_cmovez(rest, &bsf_dst)?;
    let mut stmts = Vec::new();
    stmts.push(Stmt::Clear(
        Value::Reg(Zf, ValSize::Size8),
        vec![bsf_src.clone()],
    ));
    stmts.push(Stmt::Clear(bsf_dst, vec![bsf_src, mov_src]));
    Ok((rest, (addr, stmts)))
}

fn parse_single_instr<'a>(
    instrs: BlockInstrs<'a>,
    metadata: &VW_Metadata,
) -> IResult<'a, StmtResult> {
    if let Some(((addr, instr), rest)) = instrs.split_first() {
        Ok((rest, (*addr, lift(instr, addr, metadata))))
    } else {
        Err(ParseErr::Incomplete)
    }
}

fn parse_instr<'a>(instrs: BlockInstrs<'a>, metadata: &VW_Metadata) -> IResult<'a, StmtResult> {
    if let Ok((rest, stmt)) = parse_bsf_cmove(instrs, metadata) {
        Ok((rest, stmt))
    } else if let Ok((rest, stmt)) = parse_probestack(instrs, metadata) {
        Ok((rest, stmt))
    } else {
        parse_single_instr(instrs, metadata)
    }
}

fn parse_instrs<'a>(instrs: BlockInstrs, metadata: &VW_Metadata) -> Vec<(Addr, Vec<Stmt>)> {
    let mut block_ir: Vec<(Addr, Vec<Stmt>)> = Vec::new();
    let mut rest = instrs;
    while let Ok((more, (addr, stmts))) = parse_instr(rest, metadata) {
        rest = more;
        if stmts.len() == 1 {
            if let Stmt::Branch(Opcode::JMP, _) = stmts[0] {
                // Don't continue past an unconditional jump --
                // Cranelift's new backend embeds constants in the
                // code stream at points (e.g. jump tables) and we
                // should not disassemble them as code.
                block_ir.push((addr, stmts));
                break;
            }
        }
        block_ir.push((addr, stmts));
    }
    block_ir
}

pub fn lift_cfg(program: &ModuleData, cfg: &VW_CFG, metadata: &VW_Metadata) -> IRMap {
    let mut irmap = IRMap::new();
    let g = &cfg.graph;
    for block_addr in g.nodes() {
        let block = cfg.get_block(block_addr);

        let instrs_vec: Vec<(u64, yaxpeax_x86::long_mode::Instruction)> = program
            .instructions_spanning(<AMD64 as Arch>::Decoder::default(), block.start, block.end)
            .collect();
        let instrs = instrs_vec.as_slice();
        let block_ir = parse_instrs(instrs, &metadata);

        irmap.insert(block_addr, block_ir);
    }
    irmap
}

// TODO: baby version of nom, resolve crate incompatibilities later

type IResult<'a, O> = Result<(BlockInstrs<'a>, O), ParseErr<BlockInstrs<'a>>>;
type StmtResult = (Addr, Vec<Stmt>);
type Addr = u64;

enum ParseErr<E> {
    Incomplete, // input too short
    Error(E),   // recoverable
    Failure(E), // unrecoverable
}

type BlockInstrs<'a> = &'a [(Addr, yaxpeax_x86::long_mode::Instruction)];
