//! Module containing a Storage generic trait that can be implemented for different specific storage
//! backends.

use crate::error::Result;

/// This is a generic trait that exposes a very simple key/value CRUD API for data storage.
/// This trait can be easily implemented for any specific storage backend solution (databases,
/// volatile memory, flat files, etc.)
pub trait Storage<ConnData, Key, Value> {

    /// Storage object constructor.
    /// `connection_data` can be used for passing credentials, urls, paths, etc. down to the storage
    /// backend.
    fn new(connection_data: ConnData) -> Result<Box<Self>>;

    /// Create / update entries in the storage, identified by a key.
    fn put(&mut self, key: Key, value: Value) -> Result<()>;

    /// Retrieve an entry from the storage, identified by its key.
    fn get(&self, key: Key) -> Result<Option<Value>>;

    /// Delete an entry from the storage, identified by its key.
    fn delete(&mut self, key: Key) -> Result<()>;

}