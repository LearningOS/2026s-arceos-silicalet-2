use axhal::paging::MappingFlags;
use axhal::mem::{PAGE_SIZE_4K, phys_to_virt};
use axmm::AddrSpace;
use crate::VM_ENTRY;

const SKERNEL2: [u8; 16] = [
    0xf3, 0x25, 0x40, 0xf1, // csrr a1, mhartid
    0x03, 0x35, 0x00, 0x04, // ld   a0, 64(zero)
    0x93, 0x08, 0x80, 0x00, // li   a7, 8
    0x73, 0x00, 0x00, 0x00, // ecall
];

pub fn load_vm_image(uspace: &mut AddrSpace) {
    uspace.map_alloc(VM_ENTRY.into(), PAGE_SIZE_4K, MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER, true).unwrap();

    let (paddr, _, _) = uspace
        .page_table()
        .query(VM_ENTRY.into())
        .unwrap_or_else(|_| panic!("Mapping failed for segment: {:#x}", VM_ENTRY));

    unsafe {
        core::ptr::copy_nonoverlapping(
            SKERNEL2.as_ptr(),
            phys_to_virt(paddr).as_mut_ptr(),
            SKERNEL2.len(),
        );
    }
}
