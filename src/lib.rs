#![doc = include_str!("../README.md")]

pub mod maps;
pub mod accessor;

use accessor::{Accessor, TsvText};
use maps::{MemoryMap, SafeMemoryMap};



/// A table of a memory mapped text database.
pub struct Table<Map: MemoryMap, Access : Accessor> {
    accessor: Access,
    map: Map,
}

/// A line from a memory mapped text database.
pub struct Line<'a, Map: MemoryMap, Access : Accessor> {
    textdb: &'a Table<Map, Access>,
    line: &'a [u8],
}

impl Table<SafeMemoryMap, TsvText> {
    // Make a table from an owned string.
    pub fn text_tsv_from_string(text: String) -> Self {
        let accessor = accessor::TsvText::<0>::default();
        let map = maps::SafeMemoryMap::from_string(text);
        Table::new(map, accessor)
    }

    // Make a table from a string reference.
    pub fn text_tsv_from_str<S : AsRef<str>>(text: S) -> Self {
        let accessor = accessor::TsvText::<0>::default();
        let map = maps::SafeMemoryMap::from_str(text.as_ref());
        Table::new(map, accessor)
    }
}


impl<Access : Accessor, Map: MemoryMap> Table<Map, Access> {
    /// Make a new memory mapped text database.
    pub fn new(map: Map, accessor: Access) -> Self {
        Self {
            map,
            accessor
        }
    }

    /// Return true if the database is sorted.
    /// Note: On large files (> 1TB) this may take some time to run.
    pub fn is_sorted(&self) -> anyhow::Result<bool> {
        let bytes = self.map.bytes();
        let mut iter = bytes.split(|b| *b == b'\n');
        let mut prev_line = iter.next().unwrap_or_default();
        for line in iter {
            if self.accessor.compare_lines(prev_line, line) == std::cmp::Ordering::Greater {
                return Ok(false);
            }
            prev_line = line;
        }
        Ok(true)
    }

    /// Get all the keys as strings.
    /// Note: On large files (> 1TB) this may take some time to run.
    pub fn keys(&self) -> impl Iterator<Item=Result<&str, std::str::Utf8Error>> {
        self.map.bytes().split(|b| *b == b'\n').map(|line| {
            std::str::from_utf8(self.accessor.key(line))
        })
    }

    /// Get one column as strings.
    pub fn cols(&self, i: usize) -> impl Iterator<Item=Result<&str, std::str::Utf8Error>> {
        self.map.bytes().split(|b| *b == b'\n').map(move |line| {
            std::str::from_utf8(self.accessor.col(line, i))
        })
    }

    /// Get a whole line between min and max which contains pos.
    fn find_line_at(bytes: &[u8], min: usize, max: usize, pos: usize) -> (usize, usize, &[u8]) {
        let start = bytes[min..pos].iter().rposition(|b| *b == b'\n').map(|p| min + p + 1).unwrap_or(min);
        let end = bytes[pos..max].iter().position(|b| *b == b'\n').map(|p| pos + p + 1).unwrap_or(max);
        assert!(start >= min);
        assert!(end <= max);
        assert!(end >= start);

        // Trim the newline.
        let line_end = if end != 0 && bytes[end-1] == b'\n' { end-1 } else { end };
        let line = &bytes[start..line_end];
        (start, end, line)
    }

    /// Return an iterator over all matching lines for a certain key.
    pub fn get_matching_lines(&self, key: &Access::KeyType) -> impl Iterator<Item=Line<Map, Access>> {
        let bytes = self.map.bytes();

        // Always the start of a line.
        let mut min = 0;

        // Always the end of a line (not counting the newline).
        let mut max = bytes.len();
        loop {
            let mid = min + (max - min) / 2;
            let (start, end, line) = Self::find_line_at(bytes, min, max, mid);

            #[cfg(test)]
            {
                let cmp = self.accessor.compare_key(line, key);
                let range = std::str::from_utf8(&bytes[min..max]).unwrap();
                println!("min={min} mid={mid} max={max} line ={:?} cmp={cmp:?} r={range:?}", std::str::from_utf8(line).unwrap());
            }

            match self.accessor.compare_key(line, key) {
                // line < key: 
                std::cmp::Ordering::Less => {
                    // Ensure forward progress by moving min up one line.
                    assert!(min != end);
                    min = end;
                }
                std::cmp::Ordering::Equal => {
                    let (_start, end, line) = Self::find_line_at(bytes, min, max, min);
                    #[cfg(test)]
                    {
                        assert_eq!(start, min);
                        let range = std::str::from_utf8(&bytes[min..max]).unwrap();
                        let cmp = self.accessor.compare_key(line, key);
                        println!("=min min={min} mid={mid} max={max} line ={:?} cmp={cmp:?} r={range:?}", std::str::from_utf8(line).unwrap());
                    }
                    let mut min_is_equal = false;
                    match self.accessor.compare_key(line, key) {
                        std::cmp::Ordering::Less => {
                            assert!(min != end);
                            min = end;
                        }
                        std::cmp::Ordering::Equal => {
                            min_is_equal = true;
                        }
                        std::cmp::Ordering::Greater => {
                            // Not sorted!
                            max = min;
                            break;
                        }
                    }

                    let (start, _end, line) = Self::find_line_at(bytes, min, max, max-1);
                    #[cfg(test)]
                    {
                        assert_eq!(_end, max);
                        let range = std::str::from_utf8(&bytes[min..max]).unwrap();
                        let cmp = self.accessor.compare_key(line, key);
                        println!("=max min={min} mid={mid} max={max} line ={:?} cmp={cmp:?} r={range:?}", std::str::from_utf8(line).unwrap());
                    }
                    match self.accessor.compare_key(line, key) {
                        std::cmp::Ordering::Less => {
                            // Not sorted!
                            max = min;
                            break;
                        }
                        std::cmp::Ordering::Equal => {
                            if min_is_equal {
                                // Sucess, both min and max are equal.
                                // Trim the range.
                                max = if max != 0 && bytes[max-1] == b'\n' { max-1 } else { max };
                                break;
                            }
                        }
                        std::cmp::Ordering::Greater => {
                            // Ensure forward progress by moving max down one.
                            assert!(max != start);
                            max = start;
                        }
                    }
                }
                std::cmp::Ordering::Greater => {
                    assert!(max != start);
                    max = start;
                }
            }
        }

        bytes[min..max].split(|b| *b == b'\n').map(|line| {
            Line {
                textdb: self,
                line,
            }
        })
    }
}

impl<'a, Access : Accessor, Map: MemoryMap> Line<'a, Map, Access> {
    /// Get the key of this line as a string.
    pub fn key(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.textdb.accessor.key(self.line))
    }

    /// Get a column of this line as a string.
    pub fn col(&self, i: usize) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.textdb.accessor.col(self.line, i))
    }

    pub fn line(&self) ->  Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.line)
    }
}


