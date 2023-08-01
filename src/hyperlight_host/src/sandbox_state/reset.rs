use crate::sandbox::{guest_mgr::GuestMgr, mem_mgr::MemMgr};
use anyhow::Result;

use super::sandbox::Sandbox;

pub(crate) trait RestoreSandbox: MemMgr + GuestMgr + Sandbox {
    /// Reset the Sandbox's state
    fn reset_state(&mut self) -> Result<()> {
        if self.get_num_runs() > 0 && !self.is_reusable() {
            anyhow::bail!("You must use a ReusableSandbox if you need to call a function in the guest more than once");
        }

        if self.is_reusable() {
            self.restore_state()?;
        }

        self.increase_num_runs();

        Ok(())
    }

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
}
