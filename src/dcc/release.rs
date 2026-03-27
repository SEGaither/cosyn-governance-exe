use crate::dcc::types::RuntimeControl;

/// Derive all terminal DCC fields and return whether release is permitted.
/// This is the ONLY path to release. No alternate paths exist.
///
/// Derivation chain:
/// 1. derive_pass_basis (from structural + semantic)
/// 2. derive_reasoning_permitted (from all DCC fields)
/// 3. derive_release_pass (from reasoning_permitted)
///
/// Returns ctrl.release_pass after derivation.
pub fn derive_release(ctrl: &mut RuntimeControl) -> bool {
    ctrl.derive_pass_basis();
    ctrl.derive_release_pass(); // internally calls derive_reasoning_permitted
    ctrl.release_pass
}
