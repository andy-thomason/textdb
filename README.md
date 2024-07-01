# TextDb

A read-only database for TSV files.

## Rationale

Sorted text files are human readable and can operate as fast as a binary
database such as a KV store.

If your data is largely static, a single memory mapped text file is usually
the simplest and fastest way to hold a table of data up to the 1TB region and beyond.

You can combine TSV with JSON to make structured storage and add bloom filters
and an index to work with unsorted data and secondary keys.

It is easy to generate a TSV (tab separaed values) file with columns using
command line utilities such as 'ls', 'grep', 'sed' and 'awk'
or with Rust code using 'writeln!'.

### Generation example
```ignore
    for i in 0..100 {
        println!("{i}\t{}", i*100);
    }
```

TSV files are used in the biomedical world as an alternative
to the ambiguous CSV file which may contain quoted text in different
formats.

A sorted TSV file makes an excellent database, but when they get to
terabyte size, querying them becomes difficult with standard tools.

They may be slightly bigger than a RocksDB, SLED or MDBX database,
but they can be surprisingly fast once the disc cache has warmed up.

### Simple example using strings

Given a sorted database, which could be very large, the
function 'get_matching_lines' fill return
an iterator over the range of items equal to the key.

```rust
    use textdb::Table;

    let text = "A\nB\nC\nC\nD\nE\nF\nF\nF\nF\nF\nG\nH\nI\nJ\nK\nL\n";
    let textdb = Table::text_tsv_from_str(text);

    assert!(textdb.is_sorted().unwrap());

    assert_eq!(textdb.get_matching_lines("F".as_bytes()).count(), 5);
    assert_eq!(textdb.get_matching_lines("C".as_bytes()).count(), 2);
```

### Simple example using integer keys

Note that they key may be a string or any Rust type that implements
'FromStr'.

```rust
    use textdb::Table;

    let kv = [(6, 6), (10, 1), (113, 2), (113, 5), (129, 3), (140, 0), (168, 7), (205, 9), (211, 8), (215, 4)];
    let text = kv.iter().map(|(k, v)| format!("{k}\t{v}")).collect::<Vec<_>>().join("\n");
    let accessor = accessor::TsvParse::<u8, 0>::default();
    let map = maps::SafeMemoryMap::from_str(&text);
    let textdb = Table::new(map, accessor);

    // This would not be true for text order sorting.
    assert!(textdb.is_sorted().unwrap());

    assert_eq!(textdb.get_matching_lines(&113).count(), 2);
```
