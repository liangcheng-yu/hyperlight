use super::sandbox::ReusableSandbox;
use crate::sandbox::mem_mgr::MemMgr;
use anyhow::Result;

pub(crate) trait RestoreSandbox: ReusableSandbox + MemMgr {
    /// Restore the Sandbox's state
    fn restore_state(&mut self) -> Result<()> {
        if self.needs_state_reset() {
            let mem_mgr = self.get_mem_mgr_mut();
            mem_mgr.restore_state()?;
            if !mem_mgr.run_from_process_memory {
                // TODO: Call specific Hypervisor `reset_RSP` function.
            }
            self.set_needs_state_reset(false);
        }
        Ok(())
    }
    // ^^^ Note: In C#, we have two functions:
    // - `RestoreState`, and
    // - `ResetState`.
    //
    // ... where `ResetState` is a conditional `RestoreState`
    // that only happens if the sandbox has the `recycleAfterRun`
    // property set to `true`. In Rust, we don't need that, because
    // the `recycleAfterRun` state is abstracted away by the different
    // `Sandbox` states (i.e., `OneShot` (non-recyclable) vs.
    // `Reusable` (recyclable)), so having only `restore_state` suffices.
}
