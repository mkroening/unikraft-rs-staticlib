#![no_std]
#![warn(rust_2018_idioms)]

#[macro_export]
macro_rules! can_run_this {
    () => {};
}

#[cfg(any(feature = "kvm", feature = "linuxu"))]
mod entry {
    extern "C" {
        #[cfg_attr(feature = "kvm", link_name = "_libkvmplat_entry")]
        #[cfg_attr(feature = "linuxu", link_name = "_liblinuxuplat_start")]
        fn entry() -> !;
    }

    core::arch::global_asm!(
        ".global _unikraft_rs_start",
        "_unikraft_rs_start:",
        "jmp {entry}",
        "ud2",
        entry = sym entry,
    );
}
