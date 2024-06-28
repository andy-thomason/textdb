# textdb

A super fast query library for columnar text files.

Sorted text files are human readable and can operate as fast as a binary
database such as a KV store.

If your data is largely static, a single memory mapped text file is usually
the simplest and fastest way to hold a table of data up to the 1TB region and beyond.

You can combine TSV with JSON to make structured storage and add bloom filters
and an index to work with unsorted data and secondary keys.

