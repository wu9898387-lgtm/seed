fn main() {
    // Force the tiny Core crate into the link while keeping the baseline free
    // from CLI formatting and argument-parsing dependencies.
    std::hint::black_box(seed_core::PROTOCOL_VERSION);
    std::hint::black_box(seed_core::SpaceKind::Direct as u8);
}
