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

