fn main() -> nih_plug_xtask::Result<()> {
    // Skip the first arg (program path) since main_with_args expects only the command args
    nih_plug_xtask::main_with_args("Skipper", std::env::args().skip(1))
}
