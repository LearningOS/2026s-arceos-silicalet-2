#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#![feature(asm_const)]
#![feature(riscv_ext_intrinsics)]

#[cfg(feature = "axstd")]
extern crate axstd as std;
extern crate alloc;
#[macro_use]
extern crate axlog;

mod task;
mod vcpu;
mod regs;
mod csrs;
mod sbi;
mod loader;

use vcpu::VmCpuRegisters;
use riscv::register::{scause, sstatus, stval};
use csrs::defs::hstatus;
use tock_registers::LocalRegisterCopy;
use csrs::{RiscvCsrTrait, CSR};
use vcpu::_run_guest;
use sbi::{BaseFunction, SbiMessage, SBI_ERR_NOT_SUPPORTED};
use loader::load_vm_image;
use axhal::mem::{PhysAddr, PAGE_SIZE_4K, phys_to_virt};
use axhal::paging::MappingFlags;
use axmm::AddrSpace;
use crate::regs::GprIndex;
use crate::regs::GprIndex::{A0, A1};

const VM_ENTRY: usize = 0x8020_0000;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    ax_println!("Hypervisor ...");

    // A new address space for vm.
    let mut uspace = axmm::new_user_aspace().unwrap();

    // Load vm binary into address space.
    load_vm_image(&mut uspace);

    // Setup context to prepare to enter guest mode.
    let mut ctx = VmCpuRegisters::default();
    prepare_guest_context(&mut ctx);

    // Setup pagetable for 2nd address mapping.
    let ept_root = uspace.page_table_root();
    prepare_vm_pgtable(ept_root);

    // Kick off vm and wait for it to exit.
    while !run_guest(&mut ctx, &mut uspace) {
    }

    panic!("Hypervisor ok!");
}

fn prepare_vm_pgtable(ept_root: PhysAddr) {
    let hgatp = 8usize << 60 | usize::from(ept_root) >> 12;
    unsafe {
        core::arch::asm!(
            "csrw hgatp, {hgatp}",
            hgatp = in(reg) hgatp,
        );
        core::arch::riscv64::hfence_gvma_all();
    }
}

fn run_guest(ctx: &mut VmCpuRegisters, uspace: &mut AddrSpace) -> bool {
    unsafe {
        _run_guest(ctx);
    }
    vmexit_handler(ctx, uspace)
}

fn read_insn(uspace: &AddrSpace, gpa: usize) -> u32 {
    let off = gpa & 0xFFF;
    let (paddr, _, _) = uspace.page_table().query((gpa & !0xFFF).into()).unwrap();
    unsafe { core::ptr::read_volatile(phys_to_virt(paddr).as_usize().wrapping_add(off) as *const u32) }
}

const MHARTID: usize = 0xF14;

#[allow(unreachable_code)]
fn vmexit_handler(ctx: &mut VmCpuRegisters, uspace: &mut AddrSpace) -> bool {
    use scause::{Exception, Trap};

    let scause = scause::read();
    match scause.cause() {
        Trap::Exception(Exception::VirtualSupervisorEnvCall) => {
            let sbi_msg = SbiMessage::from_regs(ctx.guest_regs.gprs.a_regs()).ok();
            ax_println!("VmExit Reason: VSuperEcall: {:?}", sbi_msg);
            if let Some(msg) = sbi_msg {
                let done = handle_sbi(ctx, msg);
                if !done {
                    ctx.guest_regs.sepc += 4;
                }
                return done;
            }
            panic!("bad sbi message!");
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            let insn = read_insn(uspace, ctx.guest_regs.sepc);
            let rd = ((insn >> 7) & 0x1F) as u32;
            let csr = (insn >> 20) as usize;
            let funct3 = (insn >> 12) & 0x7;
            if funct3 == 2 && csr == MHARTID {
                ctx.guest_regs.gprs.set_reg(GprIndex::from_raw(rd).unwrap(), 0x1234);
                ctx.guest_regs.sepc += 4;
                return false;
            }
            panic!("Unhandled CSR: insn {:#x} csr {:#x} rd {} funct3 {}", insn, csr, rd, funct3);
        }
        Trap::Exception(Exception::LoadGuestPageFault) => {
            let addr = stval::read();
            let page = addr & !0xFFF;
            uspace.map_alloc(page.into(), PAGE_SIZE_4K, MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER, true).unwrap();
            let (paddr, _, _) = uspace.page_table().query(page.into()).unwrap();
            unsafe {
                phys_to_virt(paddr).as_mut_ptr().add(0x40).cast::<u64>().write_volatile(0x6688u64);
            }
            false
        }
        Trap::Exception(Exception::StoreGuestPageFault) => {
            let addr = stval::read();
            let page = addr & !0xFFF;
            uspace.map_alloc(page.into(), PAGE_SIZE_4K, MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER, true).unwrap();
            false
        }
        _ => {
            panic!(
                "Unhandled trap: {:?}, sepc: {:#x}, stval: {:#x}",
                scause.cause(),
                ctx.guest_regs.sepc,
                stval::read()
            );
        }
    }
}

fn handle_sbi(ctx: &mut VmCpuRegisters, msg: SbiMessage) -> bool {
    match msg {
        SbiMessage::Reset(_) => {
            let a0 = ctx.guest_regs.gprs.reg(A0);
            let a1 = ctx.guest_regs.gprs.reg(A1);
            ax_println!("a0 = {:#x}, a1 = {:#x}", a0, a1);
            assert_eq!(a0, 0x6688);
            assert_eq!(a1, 0x1234);
            ax_println!("Shutdown vm normally!");
            true
        }
        SbiMessage::PutChar(c) => {
            ax_print!("{}", c as u8 as char);
            false
        }
        SbiMessage::SetTimer(_) => {
            ctx.guest_regs.gprs.set_reg(A0, 0);
            false
        }
        SbiMessage::Base(f) => {
            handle_sbi_base(ctx, f);
            false
        }
        _ => {
            ctx.guest_regs.gprs.set_reg(A0, SBI_ERR_NOT_SUPPORTED as usize);
            false
        }
    }
}

fn handle_sbi_base(ctx: &mut VmCpuRegisters, f: BaseFunction) {
    use BaseFunction::*;
    let (err, val) = match f {
        GetSepcificationVersion => (0, 0x02000000usize),
        GetImplementationID => (0, 0),
        GetImplementationVersion => (0, 1),
        ProbeSbiExtension(_) => (0, 0),
        GetMachineVendorID => (0, 0),
        GetMachineArchitectureID => (0, 0),
        GetMachineImplementationID => (0, 0),
    };
    ctx.guest_regs.gprs.set_reg(A0, err);
    ctx.guest_regs.gprs.set_reg(A1, val);
}

fn prepare_guest_context(ctx: &mut VmCpuRegisters) {
    // Set hstatus
    let mut hstatus = LocalRegisterCopy::<usize, hstatus::Register>::new(
        riscv::register::hstatus::read().bits(),
    );
    // Set Guest bit in order to return to guest mode.
    hstatus.modify(hstatus::spv::Guest);
    // Set SPVP bit in order to accessing VS-mode memory from HS-mode.
    hstatus.modify(hstatus::spvp::Supervisor);
    CSR.hstatus.write_value(hstatus.get());
    ctx.guest_regs.hstatus = hstatus.get();

    // Set sstatus in guest mode.
    let mut sstatus = sstatus::read();
    sstatus.set_spp(sstatus::SPP::Supervisor);
    ctx.guest_regs.sstatus = sstatus.bits();
    // Return to entry to start vm.
    ctx.guest_regs.sepc = VM_ENTRY;
}
