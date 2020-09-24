//! # Storage
//!
//! Generic `Storage` trait that can be implemented for different
//! specific storage backends.
use std::result;

/// Result with error set to `failure::Error`
pub type Result<T> = result::Result<T, failure::Error>;

/// Generic trait that exposes a very simple key/value CRUD API for data storage.
///
/// This trait can be easily implemented for any specific storage
/// backend solution (databases, volatile memory, flat files, etc.)
pub trait Storage {
    /// Get a value from the storage give a key
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Put a value in the storage
    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()>;

    /// Delete a value from the storage
    fn delete(&self, key: &[u8]) -> Result<()>;

    /// Create an iterator over all the keys that start with the given prefix
    fn prefix_iterator<'a, 'b: 'a>(&'a self, prefix: &'b [u8]) -> Result<StorageIterator<'a>>;
}

/// Iterator over key-value pairs
pub type StorageIterator<'a> = Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a>;
