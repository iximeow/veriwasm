use std::path::Path;
use yaxpeax_core::memory::reader;
use yaxpeax_core::memory::repr::process::{
    ELFExport, ELFImport, ELFSymbol, ModuleData, ModuleInfo,
};
use yaxpeax_core::memory::repr::FileRepr;

use crate::loaders::utils::VW_Metadata;
use std::env;
use std::fs;
use wasmtime::*;
use yaxpeax_arch::Arch;
use yaxpeax_core::memory::MemoryRepr;
use yaxpeax_x86::long_mode::Arch as AMD64;

pub fn load_wasmtime_program(path: &str) -> ModuleData {
    let buffer = fs::read(path).expect("Something went wrong reading the file");
    let store: Store<()> = Store::default();
    // Deserialize wasmtime module
    let module = unsafe { Module::deserialize(store.engine(), buffer).unwrap() };
    let obj = module.obj();

    match ModuleData::load_from(&obj, path.to_string()) {
        Some(program) => program, //{ FileRepr::Executable(data) }
        None => {
            panic!("function:{} is not a valid path", path)
        }
    }
}

pub fn load_wasmtime_metadata(program: &ModuleData) -> VW_Metadata {
    // let program = load_program(binpath);

    // grab some details from the binary and panic if it's not what we expected
    let (_, _sections, _entrypoint, _imports, _exports, symbols) =
        match (program as &dyn MemoryRepr<<AMD64 as Arch>::Address>).module_info() {
            Some(ModuleInfo::ELF(isa, _, _, sections, entry, _, imports, exports, symbols)) => {
                (isa, sections, entry, imports, exports, symbols)
            }
            Some(other) => {
                panic!("Module isn't an elf, but is a {:?}?", other);
            }
            None => {
                panic!("Module doesn't appear to be a binary yaxpeax understands");
            }
        };

    unimplemented!();

    // let guest_table_0 = get_symbol_addr(symbols, "guest_table_0").unwrap();
    // let lucet_tables = get_symbol_addr(symbols, "lucet_tables").unwrap();
    // let lucet_probestack = get_symbol_addr(symbols, "lucet_probestack").unwrap();
    // println!(
    //     "guest_table_0 = {:x} lucet_tables = {:x} probestack = {:x}",
    //     guest_table_0, lucet_tables, lucet_probestack
    // );
    // VW_Metadata {
    //     guest_table_0: guest_table_0,
    //     lucet_tables: lucet_tables,
    //     lucet_probestack: lucet_probestack,
    // }
}


// Currently no need to filter wasmtime functions
pub fn is_valid_wasmtime_func_name(name: &String) -> bool {
    true
}


// pub fn load_wasmtime_metadata(program: &ModuleData) -> VW_Metadata {
//     unimplemented!()
// }

// fn deserialize_module(path: &String) -> Module {

//     let buffer = fs::read(path)
//     .expect("Something went wrong reading the file");

//     // Configure the initial compilation environment, creating the global
//     // `Store` structure. Note that you can also tweak configuration settings
//     // with a `Config` and an `Engine` if desired.
//     println!("Initializing...");
//     let mut store: Store<()> = Store::default();

//     // Compile the wasm binary into an in-memory instance of a `Module`. Note
//     // that this is `unsafe` because it is our responsibility for guaranteeing
//     // that these bytes are valid precompiled module bytes. We know that from
//     // the structure of this example program.
//     println!("Deserialize module...");
//     let module = unsafe { Module::deserialize(store.engine(), buffer).unwrap() };

//     // Next we poke around a bit to extract the `run` function from the module.
//     // println!("Extracting export...");
//     // let run = instance.get_typed_func::<(), (), _>(&mut store, "run")?;

//     // And last but not least we can call it!
//     // println!("Calling export...");
//     // run.call(&mut store, ())?;

//     println!("Done.");
//     module
// }

// fn main() {
//     println!("Hello, world!");
//     let args: Vec<String> = env::args().collect();

//     let filename = &args[1];
//     let module = deserialize_module(filename);
//     let imports = module.imports();

//     for import in imports{
//         println!("Import: {:?}", import);
//     }
//     // println!("Imports: {:?}", imports);
//     let exports = module.exports();
//     for export in exports{
//         println!("Export: {:?}", export);
//     }
//     // println!("Exports: {:?}", exports);
//     let obj = module.obj();//.artifacts.obj
//     println!("Ok, now I'm really done!");
// }
