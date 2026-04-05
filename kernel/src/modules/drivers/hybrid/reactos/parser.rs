use alloc::vec::Vec;

use super::{
    NtImportBinding, NtSymbol, NtSymbolTable, PeImageInfo, PeImportDescriptor, PeImportName,
    PeLoadError, PeRelocationBlock, PeSectionInfo, DOS_SIGNATURE, MACHINE_AMD64, PE_SIGNATURE,
};

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, PeLoadError> {
    let end = offset.saturating_add(2);
    let slice = bytes.get(offset..end).ok_or(PeLoadError::Truncated)?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, PeLoadError> {
    let end = offset.saturating_add(4);
    let slice = bytes.get(offset..end).ok_or(PeLoadError::Truncated)?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, PeLoadError> {
    let end = offset.saturating_add(8);
    let slice = bytes.get(offset..end).ok_or(PeLoadError::Truncated)?;
    Ok(u64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

pub fn parse_pe_image(image: &[u8]) -> Result<PeImageInfo, PeLoadError> {
    if read_u16(image, 0)? != DOS_SIGNATURE {
        return Err(PeLoadError::InvalidDosSignature);
    }

    let pe_offset = read_u32(image, 0x3C)? as usize;
    if read_u32(image, pe_offset)? != PE_SIGNATURE {
        return Err(PeLoadError::InvalidPeSignature);
    }

    let file_header = pe_offset + 4;
    let machine = read_u16(image, file_header)?;
    if machine != MACHINE_AMD64 {
        return Err(PeLoadError::UnsupportedMachine);
    }

    let number_of_sections = read_u16(image, file_header + 2)?;
    let optional_header_size = read_u16(image, file_header + 16)? as usize;
    let optional_header_offset = file_header + 20;

    let magic = read_u16(image, optional_header_offset)?;
    if magic != 0x20B {
        return Err(PeLoadError::InvalidOptionalHeader);
    }

    let entry_rva = read_u32(image, optional_header_offset + 16)?;
    let image_base = read_u64(image, optional_header_offset + 24)?;
    let size_of_image = read_u32(image, optional_header_offset + 56)?;
    let size_of_headers = read_u32(image, optional_header_offset + 60)?;
    let data_dir = optional_header_offset + 112;
    let import_directory_rva = read_u32(image, data_dir + 8)?;
    let import_directory_size = read_u32(image, data_dir + 12)?;
    let relocation_directory_rva = read_u32(image, data_dir + 40)?;
    let relocation_directory_size = read_u32(image, data_dir + 44)?;

    let section_table = optional_header_offset + optional_header_size;
    let mut sections = Vec::with_capacity(number_of_sections as usize);
    for i in 0..number_of_sections as usize {
        let base = section_table + i * 40;
        let _name = image
            .get(base..base + 8)
            .ok_or(PeLoadError::InvalidSectionTable)?;
        let virtual_size = read_u32(image, base + 8)?;
        let virtual_address = read_u32(image, base + 12)?;
        let raw_data_size = read_u32(image, base + 16)?;
        let raw_data_ptr = read_u32(image, base + 20)?;
        let characteristics = read_u32(image, base + 36)?;

        sections.push(PeSectionInfo {
            virtual_address,
            virtual_size,
            raw_data_ptr,
            raw_data_size,
            characteristics,
        });
    }

    Ok(PeImageInfo {
        machine,
        image_base,
        entry_rva,
        size_of_image,
        size_of_headers,
        number_of_sections,
        sections,
        import_directory_rva,
        import_directory_size,
        relocation_directory_rva,
        relocation_directory_size,
    })
}

fn rva_to_file_offset(info: &PeImageInfo, rva: u32) -> Option<usize> {
    for section in &info.sections {
        let section_start = section.virtual_address;
        let section_end = section
            .virtual_address
            .saturating_add(section.virtual_size.max(section.raw_data_size));
        if rva >= section_start && rva < section_end {
            let delta = rva.saturating_sub(section_start);
            return Some(section.raw_data_ptr.saturating_add(delta) as usize);
        }
    }

    if rva < info.size_of_headers {
        Some(rva as usize)
    } else {
        None
    }
}

pub fn parse_import_directory(
    image: &[u8],
    info: &PeImageInfo,
) -> Result<Vec<PeImportDescriptor>, PeLoadError> {
    if info.import_directory_rva == 0 || info.import_directory_size == 0 {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let mut cursor = rva_to_file_offset(info, info.import_directory_rva).ok_or(PeLoadError::Truncated)?;
    let end = cursor.saturating_add(info.import_directory_size as usize);

    while cursor.saturating_add(20) <= image.len() && cursor < end {
        let original_first_thunk = read_u32(image, cursor)?;
        let _time_date_stamp = read_u32(image, cursor + 4)?;
        let _forwarder_chain = read_u32(image, cursor + 8)?;
        let name_rva = read_u32(image, cursor + 12)?;
        let first_thunk_rva = read_u32(image, cursor + 16)?;

        if original_first_thunk == 0 && name_rva == 0 && first_thunk_rva == 0 {
            break;
        }

        out.push(PeImportDescriptor {
            name_rva,
            first_thunk_rva,
        });
        cursor = cursor.saturating_add(20);
    }

    Ok(out)
}

pub fn parse_relocation_blocks(
    image: &[u8],
    info: &PeImageInfo,
) -> Result<Vec<PeRelocationBlock>, PeLoadError> {
    if info.relocation_directory_rva == 0 || info.relocation_directory_size == 0 {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let mut cursor =
        rva_to_file_offset(info, info.relocation_directory_rva).ok_or(PeLoadError::Truncated)?;
    let end = cursor.saturating_add(info.relocation_directory_size as usize);

    while cursor.saturating_add(8) <= image.len() && cursor < end {
        let page_rva = read_u32(image, cursor)?;
        let block_size = read_u32(image, cursor + 4)?;
        if page_rva == 0 && block_size == 0 {
            break;
        }
        if block_size < 8 {
            return Err(PeLoadError::Truncated);
        }
        let entries_bytes = block_size - 8;
        let entry_count = (entries_bytes / 2) as u16;

        out.push(PeRelocationBlock {
            page_rva,
            block_size,
            entry_count,
        });

        cursor = cursor.saturating_add(block_size as usize);
    }

    Ok(out)
}

pub fn bind_imports_with_symbol_table(
    imports: &[PeImportDescriptor],
    symbol_table: &NtSymbolTable,
) -> Vec<NtImportBinding> {
    let fallback_symbols: [NtSymbol; 10] = [
        NtSymbol::IoCreateDevice,
        NtSymbol::IoDeleteDevice,
        NtSymbol::IoCallDriver,
        NtSymbol::IoCompleteRequest,
        NtSymbol::KeAcquireSpinLock,
        NtSymbol::KeReleaseSpinLock,
        NtSymbol::ExAllocatePool2,
        NtSymbol::ExFreePool,
        NtSymbol::MmMapIoSpace,
        NtSymbol::MmUnmapIoSpace,
    ];

    let mut out = Vec::new();
    for (idx, import) in imports.iter().enumerate() {
        let choose = (import.first_thunk_rva as usize) % fallback_symbols.len();
        let symbol = fallback_symbols[choose];
        if let Some(address) = symbol_table.resolve(symbol) {
            out.push(NtImportBinding {
                descriptor_index: idx,
                symbol,
                address,
            });
        }
    }
    out
}

fn read_c_string(image: &[u8], file_offset: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut idx = file_offset;
    while idx < image.len() {
        let b = image[idx];
        if b == 0 {
            break;
        }
        out.push(b);
        idx = idx.saturating_add(1);
    }
    out
}

pub fn parse_import_names(
    image: &[u8],
    info: &PeImageInfo,
    imports: &[PeImportDescriptor],
) -> Vec<PeImportName> {
    let mut out = Vec::new();
    for (idx, import) in imports.iter().enumerate() {
        let Some(dll_off) = rva_to_file_offset(info, import.name_rva) else {
            continue;
        };
        let dll = read_c_string(image, dll_off);

        let Some(mut thunk_off) = rva_to_file_offset(info, import.first_thunk_rva) else {
            continue;
        };
        let mut thunk_index = 0u32;
        loop {
            if thunk_off.saturating_add(8) > image.len() {
                break;
            }
            let thunk = u64::from_le_bytes([
                image[thunk_off],
                image[thunk_off + 1],
                image[thunk_off + 2],
                image[thunk_off + 3],
                image[thunk_off + 4],
                image[thunk_off + 5],
                image[thunk_off + 6],
                image[thunk_off + 7],
            ]);
            if thunk == 0 {
                break;
            }

            let is_ordinal = (thunk & (1u64 << 63)) != 0;
            if !is_ordinal {
                let hint_name_rva = (thunk & 0x7FFF_FFFF_FFFF_FFFF) as u32;
                if let Some(name_off) = rva_to_file_offset(info, hint_name_rva.saturating_add(2)) {
                    let name = read_c_string(image, name_off);
                    out.push(PeImportName {
                        dll: dll.clone(),
                        name,
                        descriptor_index: idx,
                        thunk_index,
                    });
                }
            }

            thunk_off = thunk_off.saturating_add(8);
            thunk_index = thunk_index.saturating_add(1);
        }
    }
    out
}
