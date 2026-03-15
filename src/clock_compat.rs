//! Override `clock_gettime` on static musl builds to fix a startup panic on old Synology
//! DSM kernels.
//!
//! Some DSM kernels (e.g. DS216play) return `EINVAL` for `CLOCK_BOOTTIME` even though
//! the kernel version nominally supports it (the vDSO implementation is incomplete).
//! Rust ≥ 1.76 uses `CLOCK_BOOTTIME` for `std::time::Instant`, causing an immediate
//! panic on the very first call — before any application code runs.
//!
//! By providing our own `clock_gettime` symbol the static linker picks it up *before*
//! the copy in the musl archive, so every caller — including libstd, ctrlc, and any
//! other dependency — goes through this wrapper automatically.
//!
//! When `CLOCK_BOOTTIME` fails with any error we transparently retry with
//! `CLOCK_MONOTONIC`.  All other clocks are passed through unchanged.
//!
//! This override is compiled only for `target_env = "musl"` (the Synology SPK target).
//! Native / glibc dev builds are completely unaffected.

#[cfg(all(target_os = "linux", target_env = "musl"))]
mod inner {
    const CLOCK_BOOTTIME: libc::clockid_t = 7;
    const CLOCK_MONOTONIC: libc::clockid_t = 1;

    /// Drop-in replacement for the musl `clock_gettime(3)`.
    ///
    /// Behaviour is identical to the standard implementation except that a failed
    /// `CLOCK_BOOTTIME` call is transparently retried as `CLOCK_MONOTONIC`.
    ///
    /// We call the raw `syscall()` wrapper instead of recursing into ourselves.
    #[no_mangle]
    pub unsafe extern "C" fn clock_gettime(
        clk_id: libc::clockid_t,
        tp: *mut libc::timespec,
    ) -> libc::c_int {
        let ret = libc::syscall(libc::SYS_clock_gettime, clk_id, tp);
        if ret < 0 && clk_id == CLOCK_BOOTTIME {
            return libc::syscall(libc::SYS_clock_gettime, CLOCK_MONOTONIC, tp) as libc::c_int;
        }
        ret as libc::c_int
    }
}
