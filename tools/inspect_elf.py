from elftools.elf.elffile import ELFFile
p='boot/initramfs/usr/lib/aethercore/probe-linked.elf'
with open(p,'rb') as f:
    elf=ELFFile(f)
    print('Entry: 0x%x'%elf.header['e_entry'])
    print('Type:',elf.header['e_type'])
    print('Machine:',elf.header['e_machine'])
    print('ELF Class:', elf.elfclass)
    print('Endianness:', elf.little_endian and 'Little' or 'Big')
    for i,ph in enumerate(elf.iter_segments()):
        print('PH',i,ph.header.p_type,hex(ph.header.p_vaddr),ph.header.p_filesz,ph.header.p_memsz)
