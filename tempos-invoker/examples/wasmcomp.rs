use wasmer::{Module, Store};
use wasmer_compiler_llvm::LLVM;

fn main() -> anyhow::Result<()> {
    println!("Compiling...");

    let wasm_bytes = std::fs::read("apps/vpn/target/wasm32-unknown-unknown/release/vpn.wasm")?;
    // let store = Store::new(Cranelift::default());
    let store = Store::new(LLVM::default());
    let module = Module::new(&store, &wasm_bytes).unwrap();

    module.serialize_to_file("final.so")?;

    Ok(())
}
