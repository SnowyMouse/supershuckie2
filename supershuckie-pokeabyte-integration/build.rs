fn main() {
    build_shared_memory_impl();
}

#[cfg(target_os = "macos")]
fn build_shared_memory_impl() {
    let mut build = cc::Build::new();

    build.file("src/shared_memory/macos.c");
    build.compile("shared_memory_macos");
}
