use accessor::{Accessor, TsvText};
use maps::SafeMemoryMap;


pub trait MemoryMap {
    fn bytes(&self) -> &[u8];
}

pub mod maps {
    use std::{fs::File, path::Path};

    use anyhow::Context;
    use memmap2::Mmap;

    use crate::MemoryMap;

    /// An unsafe, high performance memory map
    /// Unsafe because someone else could come and truncate your file!
    pub struct UnsafeMemoryMap {
        mmap: Mmap,
    }

    /// A low performance memory map from an owned string.
    pub struct SafeMemoryMap {
        mmap: Vec<u8>,
    }

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
    
}

pub mod accessor {
    pub trait Accessor {
        const KEY_COLUMN : usize = 0;
        const SEPARATOR : u8 = b'\t';
        type KeyType : ?Sized;

        fn compare_lines(&self, line1: &[u8], line2: &[u8]) -> std::cmp::Ordering {
            let k1 = self.key(line1);
            let k2 = self.key(line2);
            k1.cmp(k2)
        }

        fn compare_key(&self, line: &[u8], k2: &Self::KeyType) -> std::cmp::Ordering;

        /// Iterator over keys as strings.
        fn key<'a>(&self, line: &'a [u8]) -> &'a [u8] {
            self.col(line, Self::KEY_COLUMN)
        }

        /// Iterator over a column as strings.
        fn col<'a>(&self, line: &'a [u8], i: usize)  -> &'a [u8] {
            let mut iter = line.split(|b| *b == Self::SEPARATOR);
            for _ in 0..i {
                if iter.next().is_none() {
                    return &[];
                }
            }

            if let Some(col) = iter.next() {
                col
            } else {
                &[]
            }
        }
    }
    
    #[derive(Default, Clone, Copy)]
    pub struct TsvText<const KEY_COL : usize = 0>;

    #[derive(Default, Clone, Copy)]
    pub struct TsvParse<Ty : std::str::FromStr, const KEY_COL : usize> {
        ty: std::marker::PhantomData<Ty>,
    }

    impl<const KEY_COL : usize> Accessor for TsvText<KEY_COL>
    {
        const KEY_COLUMN : usize = KEY_COL;
        type KeyType = [u8];
        
        fn compare_key(&self, line: &[u8], k2: &Self::KeyType) -> std::cmp::Ordering {
            let k1 = self.key(line);
            k1.cmp(k2)
        }
    }

    impl<Ty : std::cmp::Ord + std::str::FromStr + Default + std::fmt::Display, const KEY_COL: usize> Accessor for TsvParse<Ty, KEY_COL> {
        const KEY_COLUMN : usize = KEY_COL;
        type KeyType = Ty;
    
        fn compare_lines(&self, line1: &[u8], line2: &[u8]) -> std::cmp::Ordering {
            let k1 = std::str::from_utf8(self.key(line1)).unwrap_or_default();
            let k2 = std::str::from_utf8(self.key(line2)).unwrap_or_default();
            let k1 : Ty = k1.parse().unwrap_or_default();
            let k2 : Ty = k2.parse().unwrap_or_default();
            k1.cmp(&k2)
        }
        
        fn compare_key(&self, line: &[u8], k2: &Self::KeyType) -> std::cmp::Ordering {
            let k1 = std::str::from_utf8(self.key(line)).unwrap_or_default();
            let k1 : Ty = k1.parse().unwrap_or_default();
            k1.cmp(k2)
        }
    }

}

/// A memory mapped text database.
pub struct TextDb<Map: MemoryMap, Access : Accessor> {
    accessor: Access,
    map: Map,
}

/// A line from a memory mapped text database.
pub struct Line<'a, Map: MemoryMap, Access : Accessor> {
    textdb: &'a TextDb<Map, Access>,
    line: &'a [u8],
}

impl TextDb<SafeMemoryMap, TsvText> {
    pub fn text_tsv_from_string(text: String) -> Self {
        let accessor = accessor::TsvText::<0>::default();
        let map = maps::SafeMemoryMap::from_string(text);
        TextDb::new(map, accessor)
    }

    pub fn text_tsv_from_str<S : AsRef<str>>(text: S) -> Self {
        let accessor = accessor::TsvText::<0>::default();
        let map = maps::SafeMemoryMap::from_str(text.as_ref());
        TextDb::new(map, accessor)
    }
}


impl<Access : Accessor, Map: MemoryMap> TextDb<Map, Access> {
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

            let cmp = self.accessor.compare_key(line, key);

            #[cfg(tracing)]
            let range = std::str::from_utf8(&bytes[min..max]).unwrap();
            #[cfg(tracing)]
            println!("min={min} mid={mid} max={max} line ={:?} cmp={cmp:?} r={range:?}", std::str::from_utf8(line).unwrap());

            match self.accessor.compare_key(line, key) {
                // line < key: 
                std::cmp::Ordering::Less => {
                    // Ensure forward progress by moving min up one line.
                    assert!(min != end);
                    min = end;
                }
                std::cmp::Ordering::Equal => {
                    let (start, end, line) = Self::find_line_at(bytes, min, max, min);
                    assert_eq!(start, min);
                    #[cfg(tracing)]
                    let range = std::str::from_utf8(&bytes[min..max]).unwrap();
                    #[cfg(tracing)]
                    println!("=min min={min} mid={mid} max={max} line ={:?} cmp={cmp:?} r={range:?}", std::str::from_utf8(line).unwrap());
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

                    let (start, end, line) = Self::find_line_at(bytes, min, max, max-1);
                    assert_eq!(end, max);
                    #[cfg(tracing)]
                    let range = std::str::from_utf8(&bytes[min..max]).unwrap();
                    #[cfg(tracing)]
                    println!("=max min={min} mid={mid} max={max} line ={:?} cmp={cmp:?} r={range:?}", std::str::from_utf8(line).unwrap());
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


#[cfg(test)]
mod test {
    use crate::{accessor, maps, TextDb};

    #[test]
    fn test_tsv_text() {
        type Res<'a> = Result<Vec<&'a str>, std::str::Utf8Error>;
        let accessor = accessor::TsvText::<0>::default();

        let map = maps::SafeMemoryMap::from_str("A\nB\nC");
        let textdb = TextDb::new(map, accessor);
        assert_eq!(textdb.keys().collect::<Res>().unwrap(), ["A", "B", "C"]);
        assert_eq!(textdb.cols(0).collect::<Res>().unwrap(), ["A", "B", "C"]);
        assert_eq!(textdb.cols(1).collect::<Res>().unwrap(), ["", "", ""]);

        assert!(textdb.is_sorted().unwrap());

        let map = maps::SafeMemoryMap::from_str("A\nB\nC\n");
        let textdb = TextDb::new(map, accessor);
        assert!(textdb.is_sorted().unwrap());
        assert_eq!(textdb.keys().collect::<Res>().unwrap(), ["A", "B", "C"]);
        assert_eq!(textdb.cols(1).collect::<Res>().unwrap(), ["", "", ""]);

        let map = maps::SafeMemoryMap::from_str("B\nA\nC");
        let textdb = TextDb::new(map, accessor);
        assert_eq!(textdb.cols(1).collect::<Res>().unwrap(), ["", "", ""]);

        assert!(!textdb.is_sorted().unwrap());
        assert_eq!(textdb.keys().collect::<Res>().unwrap(), ["B", "A", "C"]);
        assert_eq!(textdb.cols(0).collect::<Res>().unwrap(), ["B", "A", "C"]);
        assert_eq!(textdb.cols(1).collect::<Res>().unwrap(), ["", "", ""]);

        let map = maps::SafeMemoryMap::from_str("A\t1\nB\t2\nC\t3");
        let textdb = TextDb::new(map, accessor);

        assert!(!textdb.is_sorted().unwrap());
        assert_eq!(textdb.keys().collect::<Res>().unwrap(), ["A", "B", "C"]);
        assert_eq!(textdb.cols(0).collect::<Res>().unwrap(), ["A", "B", "C"]);
        assert_eq!(textdb.cols(1).collect::<Res>().unwrap(), ["1", "2", "3"]);
   }
   #[test]

    fn test_tsv_parse() {
        // Not sorted by text
        let accessor = accessor::TsvText::<0>::default();
        let map = maps::SafeMemoryMap::from_str("10\n0100\n");
        let textdb = TextDb::new(map, accessor);
        assert!(!textdb.is_sorted().unwrap());

        // Sorted by number
        let accessor = accessor::TsvParse::<u32, 0>::default();
        let map = maps::SafeMemoryMap::from_str("10\n0100\n");
        let textdb = TextDb::new(map, accessor);
        assert!(textdb.is_sorted().unwrap());
    }

    #[test]
    fn test_get_matching_lines_a() {
        let text = "A\nB\nC\nC\nD\nE\nF\nF\nF\nF\nF\nG\nH\nI\nJ\nK\nL\n";
        let textdb = TextDb::text_tsv_from_str(text);

        assert!(textdb.is_sorted().unwrap());

        assert_eq!(textdb.get_matching_lines("F".as_bytes()).count(), 5);
        assert_eq!(textdb.get_matching_lines("C".as_bytes()).count(), 2);
    }

    #[test]
    fn test_get_matching_lines_b() {
        let text = "F\nF\nF\nF\nF\n";
        let textdb = TextDb::text_tsv_from_str(text);

        assert!(textdb.is_sorted().unwrap());

        assert_eq!(textdb.get_matching_lines("F".as_bytes()).count(), 5);
    }

    #[test]
    fn test_get_matching_lines_1() {
        let kv = [(6, 6), (10, 1), (113, 2), (113, 5), (129, 3), (140, 0), (168, 7), (205, 9), (211, 8), (215, 4)];
        let text = kv.iter().map(|(k, v)| format!("{k}\t{v}")).collect::<Vec<_>>().join("\n");
        let accessor = accessor::TsvParse::<u8, 0>::default();
        let map = maps::SafeMemoryMap::from_str(&text);
        let textdb = TextDb::new(map, accessor);

        assert!(textdb.is_sorted().unwrap());

        assert_eq!(textdb.get_matching_lines(&113).count(), 2);
    }


    #[test]
    fn test_get_matching_lines_2() {
        for i in 0..1000 {
            let mut kv = (0..i).map(|i| {
                let key : u8 = rand::random();
                (key, i)
            }).collect::<Vec<_>>();
    
            kv.sort();
    
            let text = kv.iter().map(|(k, v)| format!("{k}\t{v}")).collect::<Vec<_>>().join("\n");
    
            // println!("{kv:?} {text}");
    
            let accessor = accessor::TsvParse::<u8, 0>::default();
            let map = maps::SafeMemoryMap::from_str(&text);
            let textdb = TextDb::new(map, accessor);
    
            assert!(textdb.is_sorted().unwrap());
    
            for (k, v) in kv {
                // println!("finding k={k}");
                for line in textdb.get_matching_lines(&k) {
                    assert_eq!(line.col(0).unwrap().parse::<u8>().unwrap(), k);
                    // assert_eq!(line.col(1).unwrap().parse::<u32>().unwrap(), v);
                    // println!("{k} {v} {line}");
                }
            }
        }

    }
}
