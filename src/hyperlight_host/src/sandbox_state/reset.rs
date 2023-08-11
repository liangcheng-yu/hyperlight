use crate::sandbox::{guest_mgr::GuestMgr, hypervisor::HypervisorWrapperMgr, mem_mgr::MemMgr};
use anyhow::Result;

use super::sandbox::Sandbox;

pub trait RestoreSandbox: MemMgr + GuestMgr + HypervisorWrapperMgr + Sandbox {
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
                let hv = self.get_hypervisor_wrapper_mut()
                    .get_hypervisor_mut()?;

                hv.reset_rsp(hv.orig_rsp()?)?;
            }
            self.set_needs_state_reset(false);
        }

        Ok(())
    }
}
