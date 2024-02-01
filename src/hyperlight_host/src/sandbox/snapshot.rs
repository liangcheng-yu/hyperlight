/// A snapshot of the internal state of a `Sandbox` implementation
pub struct Snapshot {
    pub(super) mem: Vec<u8>,
}

impl Snapshot {
    /// Get a reference to the snapshot of memory
    pub fn get_mem(&self) -> &Vec<u8> {
        &self.mem
    }
}
