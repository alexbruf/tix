//! wasm-bindgen glue: exports `run(argv, cwd) -> u32` (TIX-4).

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run(argv: Vec<String>, cwd: String) -> u32 {
    tix_io::run(&argv, &cwd)
}
