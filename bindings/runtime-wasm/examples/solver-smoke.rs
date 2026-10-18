//! The native side of the executed WebAssembly solver comparison. It shares its complete input
//! and output projection with the feature-gated WASM request, and never reads a config from disk.

fn main() {
    match rnl_runtime_wasm::solver_test() {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
