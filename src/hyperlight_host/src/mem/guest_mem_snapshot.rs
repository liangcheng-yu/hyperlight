use anyhow::Result;

use super::guest_mem::GuestMemory;

/// A wrapper around a `GuestMemory` reference and a snapshot
/// of the memory therein
pub struct GuestMemorySnapshot {
    snapshot: Vec<u8>,
    gm: GuestMemory,
}

impl GuestMemorySnapshot {
    /// Take a snapshot of the memory in `gm`, then create a new instance
    /// of `Self` with the snapshot stored therein.
    pub fn new(gm: GuestMemory) -> Result<Self> {
        // TODO: Track dirty pages instead of copying entire memory
        let snapshot = gm.copy_all_to_vec()?;
        Ok(Self { gm, snapshot })
    }

    /// Take another snapshot of the internally-stored `GuestMemory`,
    /// then store it internally.
    pub fn replace_snapshot(&mut self) -> Result<()> {
        let new_snapshot = self.gm.copy_all_to_vec()?;
        self.snapshot = new_snapshot;
        Ok(())
    }

    /// Copy the memory from the internally-stored memory snapshot
    /// into the internally-stored `GuestMemory`
    pub fn restore_from_snapshot(&mut self) -> Result<()> {
        self.gm.copy_from_slice(self.snapshot.as_slice(), 0)
    }
}

#[cfg(test)]
mod tests {
    use crate::mem::guest_mem::GuestMemory;

    #[test]
    fn restore_replace() {
        let data1 = vec![b'a', b'b', b'c'];
        let data2 = data1.iter().map(|b| b + 1).collect::<Vec<u8>>();
        let mut gm = GuestMemory::new(data1.len()).unwrap();
        gm.copy_from_slice(data1.as_slice(), 0).unwrap();
        let mut snap = super::GuestMemorySnapshot::new(gm.clone()).unwrap();
        {
            // after the first snapshot is taken, make sure gm has the equivalent
            // of data1
            assert_eq!(data1, gm.copy_all_to_vec().unwrap());
        }

        {
            // modify gm with data2 rather than data1 and restore from
            // snapshot. we should have the equivalent of data1 again
            gm.copy_from_slice(data2.as_slice(), 0).unwrap();
            assert_eq!(data2, gm.copy_all_to_vec().unwrap());
            snap.restore_from_snapshot().unwrap();
            assert_eq!(data1, gm.copy_all_to_vec().unwrap());
        }
        {
            // modify gm with data2, then retake the snapshot and restore
            // from the new snapshot. we should have the equivalent of data2
            gm.copy_from_slice(data2.as_slice(), 0).unwrap();
            assert_eq!(data2, gm.copy_all_to_vec().unwrap());
            snap.replace_snapshot().unwrap();
            assert_eq!(data2, gm.copy_all_to_vec().unwrap());
            snap.restore_from_snapshot().unwrap();
            assert_eq!(data2, gm.copy_all_to_vec().unwrap());
        }
    }
}
