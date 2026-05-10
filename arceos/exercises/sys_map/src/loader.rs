use alloc::vec;
use alloc::vec::Vec;
use axerrno::AxError;
use axhal::mem::{MemoryAddr, VirtAddr, PAGE_SIZE_4K};
use axhal::paging::MappingFlags;
use axmm::AddrSpace;
use std::fs::File;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::{self, Read};

use elf::abi::{PT_INTERP, PT_LOAD};
use elf::endian::AnyEndian;
use elf::parse::ParseAt;
use elf::segment::ProgramHeader;
use elf::segment::SegmentTable;
use elf::ElfBytes;

const ELF_HEAD_BUF_SIZE: usize = 256;
const MAPFILE_BIN: &[u8] = include_bytes!("../../../payload/mapfile_c/mapfile");

pub fn load_user_app(fname: &str, uspace: &mut AddrSpace) -> io::Result<usize> {
    if let Ok(mut file) = File::open(fname) {
        let (phdrs, entry, _, _) = load_elf_phdrs(&mut file)?;
        return load_user_elf(
            &mut |offset, buf| {
                file.seek(SeekFrom::Start(offset))?;
                file.read_exact(buf)?;
                Ok(())
            },
            &phdrs,
            entry,
            uspace,
        );
    }

    if fname == "/sbin/mapfile" {
        let (phdrs, entry, _, _) = parse_elf_from_bytes(MAPFILE_BIN)?;
        return load_user_elf(
            &mut |offset, buf| {
                let start = offset as usize;
                let end = start + buf.len();
                buf.copy_from_slice(&MAPFILE_BIN[start..end]);
                Ok(())
            },
            &phdrs,
            entry,
            uspace,
        );
    }

    Err(io::Error::from(AxError::NotFound))
}

fn load_user_elf<F>(
    read_range: &mut F,
    phdrs: &[ProgramHeader],
    entry: usize,
    uspace: &mut AddrSpace,
) -> io::Result<usize>
where
    F: FnMut(u64, &mut [u8]) -> io::Result<()>,
{
    for phdr in phdrs {
        ax_println!(
            "phdr: offset: {:#X}=>{:#X} size: {:#X}=>{:#X}",
            phdr.p_offset,
            phdr.p_vaddr,
            phdr.p_filesz,
            phdr.p_memsz
        );

        let vaddr = VirtAddr::from(phdr.p_vaddr as usize).align_down_4k();
        let vaddr_end = VirtAddr::from((phdr.p_vaddr + phdr.p_memsz) as usize).align_up_4k();

        ax_println!("{:#x} - {:#x}", vaddr, vaddr_end);
        uspace.map_alloc(
            vaddr,
            vaddr_end - vaddr,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER,
            true,
        )?;

        let mut data = vec![0u8; phdr.p_memsz as usize];
        let filesz = phdr.p_filesz as usize;
        read_range(phdr.p_offset, &mut data[..filesz])?;
        uspace.write(VirtAddr::from(phdr.p_vaddr as usize), &data)?;
    }

    Ok(entry)
}

fn load_elf_phdrs(file: &mut File) -> io::Result<(Vec<ProgramHeader>, usize, usize, usize)> {
    let mut buf: [u8; ELF_HEAD_BUF_SIZE] = [0; ELF_HEAD_BUF_SIZE];
    file.read(&mut buf)?;

    let ehdr = ElfBytes::<AnyEndian>::parse_elf_header(&buf[..]).unwrap();
    info!("e_entry: {:#X}", ehdr.e_entry);

    let phnum = ehdr.e_phnum as usize;
    // Validate phentsize before trying to read the table so that we can error early for corrupted files
    let entsize = ProgramHeader::validate_entsize(ehdr.class, ehdr.e_phentsize as usize).unwrap();
    let size = entsize.checked_mul(phnum).unwrap();
    assert!(size > 0 && size <= PAGE_SIZE_4K);
    let phoff = ehdr.e_phoff;
    let mut buf = alloc::vec![0u8; size];
    let _ = file.seek(SeekFrom::Start(phoff));
    file.read(&mut buf)?;
    let phdrs = SegmentTable::new(ehdr.endianness, ehdr.class, &buf[..]);

    let phdrs: Vec<ProgramHeader> = phdrs
        .iter()
        .filter(|phdr| phdr.p_type == PT_LOAD || phdr.p_type == PT_INTERP)
        .collect();
    Ok((
        phdrs,
        ehdr.e_entry as usize,
        ehdr.e_phoff as usize,
        ehdr.e_phnum as usize,
    ))
}

fn parse_elf_from_bytes(data: &[u8]) -> io::Result<(Vec<ProgramHeader>, usize, usize, usize)> {
    let head = data
        .get(..ELF_HEAD_BUF_SIZE)
        .ok_or_else(|| io::Error::from(AxError::UnexpectedEof))?;
    let ehdr = ElfBytes::<AnyEndian>::parse_elf_header(head)
        .map_err(|_| io::Error::from(AxError::InvalidData))?;
    info!("e_entry: {:#X}", ehdr.e_entry);

    let phnum = ehdr.e_phnum as usize;
    let entsize = ProgramHeader::validate_entsize(ehdr.class, ehdr.e_phentsize as usize)
        .map_err(|_| io::Error::from(AxError::InvalidData))?;
    let size = entsize
        .checked_mul(phnum)
        .ok_or_else(|| io::Error::from(AxError::InvalidData))?;
    assert!(size > 0 && size <= PAGE_SIZE_4K);
    let phoff = ehdr.e_phoff as usize;
    let phbuf = data
        .get(phoff..phoff + size)
        .ok_or_else(|| io::Error::from(AxError::UnexpectedEof))?;
    let phdrs = SegmentTable::new(ehdr.endianness, ehdr.class, phbuf);

    let phdrs: Vec<ProgramHeader> = phdrs
        .iter()
        .filter(|phdr| phdr.p_type == PT_LOAD || phdr.p_type == PT_INTERP)
        .collect();
    Ok((
        phdrs,
        ehdr.e_entry as usize,
        ehdr.e_phoff as usize,
        ehdr.e_phnum as usize,
    ))
}
