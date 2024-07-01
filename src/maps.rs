use std::path::Path;

#[cfg(feature="mmap")]
use {memmap2::Mmap, anyhow::Context, std::fs::File};

pub trait MemoryMap {
    fn bytes(&self) -> &[u8];
}


/// An unsafe, high performance memory map
/// Unsafe because someone else could come and truncate your file!
#[cfg(feature="mmap")]
pub struct UnsafeMemoryMap {
    mmap: Mmap,
}

/// A low performance memory map from an owned string.
pub struct SafeMemoryMap {
    mmap: Vec<u8>,
}

#[cfg(feature="mmap")]
impl UnsafeMemoryMap {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let file = File::open(path)
            .with_context(|| format!("Unable to open {path:?}"))?;

        // Safety: It is impossible to avoid segfaults
        // when using memory mapped files as the file may be truncated
        // Even if we test the size of the file, another process may truncate
        // it before we read the bytes.
        let mmap = unsafe { Mmap::map(&file)? };
        Ok(Self {
            mmap
        })
    }
}

#[cfg(feature="mmap")]
impl MemoryMap for UnsafeMemoryMap {
    fn bytes(&self) -> &[u8] {
        self.mmap.as_ref()
    }
}

impl SafeMemoryMap {
    /// Create a safe memory map from a file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let string = std::fs::read_to_string(path)?;
        Ok(Self::from_string(string))
    }

    /// Create a safe memory map from a reference to a string-like object.
    pub fn from_str<S : AsRef<[u8]>>(value: S) -> Self {
        let mmap = value.as_ref();
        let mut i = mmap.len();
        while i > 0 && mmap[i-1] == b'\n' {
            i -= 1;
        }
        let mmap = mmap[0..i].to_vec();
        Self {
            mmap
        }
    }

    /// Create a safe memory map from an owned string.
    pub fn from_string(value: String) -> Self {
        let mut mmap = value.into_bytes();
        let mut i = mmap.len();
        while i > 0 && mmap[i-1] == b'\n' {
            i -= 1;
        }
        mmap.truncate(i);
        Self {
            mmap
        }
    }
}

impl MemoryMap for SafeMemoryMap {
    fn bytes(&self) -> &[u8] {
        self.mmap.as_ref()
    }
}
