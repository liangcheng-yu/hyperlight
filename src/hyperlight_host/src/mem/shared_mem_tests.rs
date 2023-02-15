use super::{ptr_offset::Offset, shared_mem::SharedMemory};
use anyhow::{bail, Result};
use std::clone::Clone;
use std::cmp::PartialEq;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::mem::size_of;

/// A function that knows how to read data of type `T` from a
/// `SharedMemory` at a specified offset
pub(crate) type ReaderFn<T> = dyn Fn(&SharedMemory, Offset) -> Result<T>;
/// A function that knows how to write data of type `T` from a
/// `SharedMemory` at a specified offset.
pub(crate) type WriterFn<T> = dyn Fn(&mut SharedMemory, Offset, T) -> Result<()>;

/// Run the standard suite of tests for a specified type `U` to write to
/// a `SharedMemory` and a specified type `T` to read back out of
/// the same `SharedMemory`.
///
/// It's possible to write one type and read a different type so you
/// can write tests involving different type combinations. For example,
/// this function is designed such that you can write a `u64` and read the
/// 8 `u8`s that make up that `u64` back out.
///
/// Regardless of which types you choose, they must be `Clone`able,
/// `Debug`able, and you must be able to check if `T`, the one returned
/// by the `reader`, is equal to `U`, the one accepted by the writer.
pub(crate) fn read_write_test_suite<T, U>(
    mem_size: usize,
    initial_val: U,
    reader: Box<ReaderFn<T>>,
    writer: Box<WriterFn<U>>,
) -> Result<()>
where
    T: PartialEq + Debug + Clone + TryFrom<U>,
    U: Debug + Clone,
{
    let test_read = |mem_size, offset| {
        let sm = SharedMemory::new(mem_size)?;
        (reader)(&sm, offset)
    };

    let test_write = |mem_size, offset, val| {
        let mut sm = SharedMemory::new(mem_size)?;
        (writer)(&mut sm, offset, val)
    };

    let test_write_read = |mem_size, offset: Offset, initial_val: U| {
        let mut sm = SharedMemory::new(mem_size)?;
        writer(&mut sm, offset, initial_val.clone())?;
        let ret_val = reader(&sm, offset)?;

        let initial_val_as_t = T::try_from(initial_val.clone())
            .map_err(|_| anyhow::anyhow!("cannot convert types"))?;
        if initial_val_as_t == ret_val {
            Ok(())
        } else {
            bail!(
                "(mem_size: {}, offset: {}, val: {:?}), actual returned val = {:?}",
                mem_size,
                u64::from(offset),
                initial_val,
                ret_val,
            );
        }
    };

    // write the value to the start of memory, then read it back
    test_write_read(mem_size, Offset::zero(), initial_val.clone())?;
    // write the value to the end of memory then read it back
    test_write_read(
        mem_size,
        Offset::try_from(mem_size - size_of::<T>())?,
        initial_val.clone(),
    )?;
    // write the value to the middle of memory, then read it back
    test_write_read(
        mem_size,
        Offset::try_from(mem_size / 2)?,
        initial_val.clone(),
    )?;
    // read a value from the memory at an invalid offset.
    swap_res(test_write_read(
        mem_size,
        Offset::try_from(mem_size * 2)?,
        initial_val.clone(),
    ))?;
    // write the value to the memory at an invalid offset.
    swap_res(test_write(
        mem_size,
        Offset::try_from(mem_size * 2)?,
        initial_val.clone(),
    ))?;
    // read a value from the memory beyond the end of the memory.
    swap_res(test_read(mem_size, Offset::try_from(mem_size)?))?;
    // write the value to the memory beyond the end of the memory.
    swap_res(test_write(
        mem_size,
        Offset::try_from(mem_size)?,
        initial_val,
    ))?;
    Ok(())
}

/// Swaps a result's status. If it was passed as an `Ok`, it will be returned
/// as an `Err` with a hard-coded error message. If it was passed as an `Err`,
/// it will be returned as an `Ok(_)`.
fn swap_res<T>(r: Result<T>) -> Result<()> {
    match r {
        Ok(_) => bail!("result was expected to be an error, but wasn't"),
        Err(_) => Ok(()),
    }
}
