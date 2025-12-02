use core::{mem::size_of, ptr};

use crate::{
    alloc::string::String,
    auxv_reader::AuxvReader,
    elf,
    fs::File,
    header::{TLS_DTV_OFFSET, TLS_MODULE_OFFSET},
    ld_so::{Tcb, TcbExt},
    sys,
};

#[no_mangle]
pub unsafe extern "C" fn __relibc_internal_init_tls(auxv: *const usize) {
    let mut auxv_reader = AuxvReader::new(auxv);
    let mut file = File::open(c_str!("/scheme/memory/temporary")).unwrap();

    let tls_template = auxv_reader.get_tls_template().unwrap();

    let tcb_size = (size_of::<Tcb<TcbExt>>() + 15) & !15;
    let tls_size = (tls_template.tls_len + 15) & !15;

    let tcb_mem = file
        .mmap(
            (tcb_size + tls_size) as sys::size_t,
            (sys::PROT_READ | sys::PROT_WRITE) as _,
        )
        .unwrap();

    let tcb = tcb_mem as *mut Tcb<TcbExt>;
    let static_tls = tcb.add(1) as *mut u8;

    ptr::copy_nonoverlapping(tls_template.tls_ptr, static_tls, tls_template.tls_len);

    (*tcb).tcb_ptr = tcb;
    (*tcb).tcb_len = tcb_size;
    (*tcb).tls_end = static_tls.add(tls_template.tls_len);
    (*tcb).dtv = tcb.add(1) as *mut ();
    (*tcb).dtv_len = 1;
    (*tcb).platform_specific.stack_base = ptr::null_mut();
    (*tcb).platform_specific.stack_size = 0;
    (*tcb).platform_specific.tls_dtv = (*tcb).dtv;
    (*tcb).platform_specific.tls_dtv_len = (*tcb).dtv_len;
    (*tcb).platform_specific.tls_static_base = static_tls;

    let dtv = (*tcb).dtv as *mut elf::TlsDtv;
    (*dtv).gen = 1;
    (*dtv).num = 1;

    let dtv_slot = dtv.add(1) as *mut elf::TlsModule;
    (*dtv_slot).module_id = 1;
    (*dtv_slot).pointer = static_tls;

    redox_rt::tcb_activate(tcb);
}
