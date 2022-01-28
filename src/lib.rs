//! Rust bindings for Unidata's [libnetcdf](https://www.unidata.ucar.edu/software/netcdf/)
//!
//! This crate allows one to store and retrieve multi-dimensional arrays from a
//! `netCDF` supported format, which can be a `netCDF` file, a subset of `hdf5` files,
//! or from a DAP url.
//!
//!
//! `netCDF` files are self-contained, they contain metadata about the data contained in them.
//! See the [`CF Conventions`](https://cfconventions.org/) for conventions used for climate and
//! forecast models.
//!
//! To explore the documentation, see the `Functions` section, in particular
//! `open()`, `create()`, and `append()`.
//!
//! For more information see:
//! * [The official introduction to `netCDF`](https://www.unidata.ucar.edu/software/netcdf/docs/netcdf_introduction.html)
//! * [The `netCDF-c` repository](https://github.com/Unidata/netcdf-c)
//!
//! # Examples
//!
//! How to read a variable from a file:
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Open the file `simple_xy.nc`:
//! let file = netcdf::open("simple_xy.nc")?;
//!
//! // Get the variable in this file with the name "data"
//! let var = &file.variable("data").expect("Could not find variable 'data'");
//!
//! // Read a single datapoint from the variable as a numeric type
//! let data_i32 = var.value::<i32>(None)?;
//! let data_f32 : f32 = var.value(None)?;
//!
//! // If your variable is multi-dimensional you might want to get a certain index
//! let data_i32 = var.value::<i32>(Some(&[40, 0, 0]))?;
//!
//! // You can use `values()` to get all the data from the variable.
//! // Passing `None` as both arguments will give you the entire slice
//! # #[cfg(feature = "ndarray")]
//! let data = var.values::<i32>(None, None)?;
//!
//! // A subset can also be selected, the following will extract the slice at
//! // `(40, 0, 0)` and get a dataset of size `100, 100` from this
//! # #[cfg(feature = "ndarray")]
//! let data = var.values::<i32>(Some(&[40, 0 ,0]), Some(&[1, 100, 100]))?;
//! # Ok(()) }
//! ```
//!
//! How to create a new file and write to it:
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a new file with default settings
//! let mut file = netcdf::create("crabs.nc")?;
//!
//! // We must create a dimension which corresponds to our data
//! file.add_dimension("ncrabs", 10)?;
//! // These dimensions can also be unlimited and will be resized when writing
//! file.add_unlimited_dimension("time")?;
//!
//! // A variable can now be declared, and must be created from the dimension names.
//! let mut var = file.add_variable::<i32>(
//!             "crab_coolness_level",
//!             &["time", "ncrabs"],
//! )?;
//! // Metadata can be added to the variable
//! var.add_attribute("units", "Kelvin");
//! var.add_attribute("add_offset", 273.15_f32);
//!
//! // Data can then be created and added to the variable
//! let data : Vec<i32> = vec![42; 10];
//! var.put_values(&data, Some(&[0, 0]), None);
//! // (This puts data at offset (0, 0) until all the data has been consumed)
//!
//! // Values can be added along the unlimited dimension, which
//! // resizes along the `time` axis
//! var.put_values(&data, Some(&[1, 0]), None);
//! # Ok(()) }
//! ```

#![warn(missing_docs)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::wildcard_imports)]

use lazy_static::lazy_static;
use netcdf_sys::nc_type;
use std::sync::Mutex;

pub mod attribute;
pub mod dimension;
pub mod error;
pub mod file;
pub mod group;
pub mod types;
pub mod variable;

pub use attribute::*;
pub use dimension::*;
pub use file::*;
pub use group::*;
pub use variable::*;

/// Open a netcdf file in create mode
///
/// Will create a `netCDF4` file and overwrite existing file
pub fn create<P>(name: P) -> error::Result<MutableFile>
where
    P: AsRef<std::path::Path>,
{
    RawFile::create_with(name.as_ref(), Options::NETCDF4)
}

/// Open a `netCDF` file in create mode with the given options
pub fn create_with<P>(name: P, options: Options) -> error::Result<MutableFile>
where
    P: AsRef<std::path::Path>,
{
    RawFile::create_with(name.as_ref(), options)
}

/// Open a `netCDF` file in append mode
pub fn append<P>(name: P) -> error::Result<MutableFile>
where
    P: AsRef<std::path::Path>,
{
    append_with(name, Options::default())
}

/// Open a `netCDF` file in append mode with the given options
pub fn append_with<P>(name: P, options: Options) -> error::Result<MutableFile>
where
    P: AsRef<std::path::Path>,
{
    RawFile::append_with(name.as_ref(), options)
}

/// Open a `netCDF` file in read mode
pub fn open<P>(name: P) -> error::Result<File>
where
    P: AsRef<std::path::Path>,
{
    open_with(name, Options::default())
}

/// Open a `netCDF` file in read mode with the given options
pub fn open_with<P>(name: P, options: Options) -> error::Result<File>
where
    P: AsRef<std::path::Path>,
{
    RawFile::open_with(name.as_ref(), options)
}

#[cfg(feature = "memory")]
/// Open a `netCDF` file from a buffer
pub fn open_mem<'a>(name: Option<&str>, mem: &'a [u8]) -> error::Result<MemFile<'a>> {
    RawFile::open_from_memory(name, mem)
}

lazy_static! {
    /// Use this when accessing `netCDF` functions
    pub(crate) static ref LOCK: Mutex<()> = Mutex::new(());
}

/// All functions should be wrapped in this locker. Disregarding this, expect
/// segfaults, especially on non-threadsafe hdf5 builds
pub(crate) fn with_lock<F: FnMut() -> nc_type>(mut f: F) -> nc_type {
    let _l = LOCK.lock().unwrap();
    f()
}

pub(crate) mod utils {
    use super::error;
    use netcdf_sys::{NC_EMAXNAME, NC_MAX_NAME};
    /// Use this function for short `netCDF` names to avoid the allocation
    /// for a `CString`
    pub(crate) fn short_name_to_bytes(name: &str) -> error::Result<[u8; NC_MAX_NAME as usize + 1]> {
        if name.len() > NC_MAX_NAME as _ {
            Err(NC_EMAXNAME.into())
        } else {
            let len = name.bytes().position(|x| x == 0).unwrap_or(name.len());
            let mut bytes = [0_u8; NC_MAX_NAME as usize + 1];
            bytes[..len].copy_from_slice(name.as_bytes());
            Ok(bytes)
        }
    }
}
