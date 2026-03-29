use std::collections::BTreeMap;

pub fn binaries() -> BTreeMap<&'static str, &'static [u8]> {
    BTreeMap::from([
        (
            "init.elf",
            include_bytes!("templates/binaries/init.elf.bin").as_slice(),
        ),
        (
            "probe.elf",
            include_bytes!("templates/binaries/probe.elf.bin").as_slice(),
        ),
        (
            "console.elf",
            include_bytes!("templates/binaries/console.elf.bin").as_slice(),
        ),
    ])
}
