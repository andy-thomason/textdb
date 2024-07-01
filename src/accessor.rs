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
