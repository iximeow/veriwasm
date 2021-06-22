use std::path::Path;
use yaxpeax_core::memory::reader;
use yaxpeax_core::memory::repr::process::{
    ELFExport, ELFImport, ELFSymbol, ModuleData, ModuleInfo,
};
use yaxpeax_core::memory::repr::FileRepr;

use crate::loaders::utils::*;
use crate::utils::utils::deconstruct_elf;
use std::env;
use std::fs;
use wasmtime::*;
use yaxpeax_arch::Arch;
use yaxpeax_core::goblin::Object;
use yaxpeax_core::memory::repr::process::Segment;
use yaxpeax_core::memory::MemoryRepr;
use yaxpeax_x86::long_mode::Arch as AMD64;

//yaxpeax doesnt load .o files correctly, so this code
// manually adds memory regions corresponding to ELF sections
// (yaxpeax does this by segments, but .o files may not have segments)
fn fixup_object_file(program: &mut ModuleData, obj: &[u8]) {
    // let elf = program.module_info().unwrap();
    let elf = match Object::parse(obj) {
        Ok(obj @ Object::Elf(_)) => match obj {
            Object::Elf(elf) => elf,
            _ => panic!(),
        },
        _ => panic!(),
    };

    for section in elf.section_headers.iter() {
        if section.sh_name == 0 {
            continue;
        }
        //Load data for section
        let mut section_data = vec![0; section.sh_size as usize];
        for idx in 0..section.sh_size {
            section_data[idx as usize] = obj[(section.sh_offset + idx) as usize];
        }
        //add as segment
        let new_section = Segment {
            start: section.sh_addr as usize, // virtual addr
            data: section_data,
            name: elf
                .shdr_strtab
                .get(section.sh_name)
                .unwrap()
                .unwrap()
                .to_string(),
        };
        program.segments.push(new_section);
    }
}

pub fn load_wasmtime_program(path: &str) -> ModuleData {
    let buffer = fs::read(path).expect("Something went wrong reading the file");
    let store: Store<()> = Store::default();
    // Deserialize wasmtime module
    let module = unsafe { Module::deserialize(store.engine(), buffer).unwrap() };
    let obj = module.obj();
    // let types = module.types();
    // println!("{:?}", types);

    match ModuleData::load_from(&obj, path.to_string()) {
        Some(mut program) => {
            fixup_object_file(&mut program, &obj);
            program
        } //{ FileRepr::Executable(data) }
        None => {
            panic!("function:{} is not a valid path", path)
        }
    }
}

pub fn load_wasmtime_metadata(program: &ModuleData) -> VW_Metadata {
    let (_, sections, entrypoint, imports, exports, symbols) = deconstruct_elf(program);

    // unimplemented!();

    // let guest_table_0 = get_symbol_addr(symbols, "guest_table_0").unwrap();
    // let lucet_tables = get_symbol_addr(symbols, "lucet_tables").unwrap();
    // let lucet_probestack = get_symbol_addr(symbols, "lucet_probestack").unwrap();
    // println!(
    //     "guest_table_0 = {:x} lucet_tables = {:x} probestack = {:x}",
    //     guest_table_0, lucet_tables, lucet_probestack
    // );
    VW_Metadata {
        guest_table_0: 0,
        lucet_tables: 0,
        lucet_probestack: 0,
    }
}

// We do not need to check handwritten trampoline functions
pub fn is_valid_wasmtime_func_name(name: &String) -> bool {
    // true
    !name.starts_with("_trampoline")
}

pub fn get_wasmtime_func_signatures() -> FuncSignatures {
    unimplemented!();
}
