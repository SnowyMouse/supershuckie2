fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").expect("no target OS set");
    match target_os.as_str() {
        "macos" => build_shared_memory_macos(),
        "linux" => build_shared_memory_linux(),
        unknown => unimplemented!("Poke-A-Byte integration for target_os '{unknown}' is not implemented")
    }
}

fn build_shared_memory_macos() {
    let mut build = cc::Build::new();
    build.file("src/shared_memory/macos.c");
    build.compile("pokeabyte_integration_shared_memory_macos");
}

fn build_shared_memory_linux() {
    let mut build = cc::Build::new();
    build.file("src/shared_memory/linux.c");
    build.compile("pokeabyte_integration_shared_memory_linux");
}
