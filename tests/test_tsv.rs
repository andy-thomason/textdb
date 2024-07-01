use textdb::{accessor, maps, Table};

#[test]
fn test_tsv_text() {
    type Res<'a> = Result<Vec<&'a str>, std::str::Utf8Error>;
    let accessor = accessor::TsvText::<0>::default();

    let map = maps::SafeMemoryMap::from_str("A\nB\nC");
    let textdb = Table::new(map, accessor);
    assert_eq!(textdb.keys().collect::<Res>().unwrap(), ["A", "B", "C"]);
    assert_eq!(textdb.cols(0).collect::<Res>().unwrap(), ["A", "B", "C"]);
    assert_eq!(textdb.cols(1).collect::<Res>().unwrap(), ["", "", ""]);

    assert!(textdb.is_sorted().unwrap());

    let map = maps::SafeMemoryMap::from_str("A\nB\nC\n");
    let textdb = Table::new(map, accessor);
    assert!(textdb.is_sorted().unwrap());
    assert_eq!(textdb.keys().collect::<Res>().unwrap(), ["A", "B", "C"]);
    assert_eq!(textdb.cols(1).collect::<Res>().unwrap(), ["", "", ""]);

    let map = maps::SafeMemoryMap::from_str("B\nA\nC");
    let textdb = Table::new(map, accessor);
    assert_eq!(textdb.cols(1).collect::<Res>().unwrap(), ["", "", ""]);

    assert!(!textdb.is_sorted().unwrap());
    assert_eq!(textdb.keys().collect::<Res>().unwrap(), ["B", "A", "C"]);
    assert_eq!(textdb.cols(0).collect::<Res>().unwrap(), ["B", "A", "C"]);
    assert_eq!(textdb.cols(1).collect::<Res>().unwrap(), ["", "", ""]);

    let map = maps::SafeMemoryMap::from_str("A\t1\nB\t2\nC\t3");
    let textdb = Table::new(map, accessor);

    assert!(textdb.is_sorted().unwrap());
    assert_eq!(textdb.keys().collect::<Res>().unwrap(), ["A", "B", "C"]);
    assert_eq!(textdb.cols(0).collect::<Res>().unwrap(), ["A", "B", "C"]);
    assert_eq!(textdb.cols(1).collect::<Res>().unwrap(), ["1", "2", "3"]);
}
#[test]

fn test_tsv_parse() {
    // Not sorted by text
    let accessor = accessor::TsvText::<0>::default();
    let map = maps::SafeMemoryMap::from_str("10\n0100\n");
    let textdb = Table::new(map, accessor);
    assert!(!textdb.is_sorted().unwrap());

    // Sorted by number
    let accessor = accessor::TsvParse::<u32, 0>::default();
    let map = maps::SafeMemoryMap::from_str("10\n0100\n");
    let textdb = Table::new(map, accessor);
    assert!(textdb.is_sorted().unwrap());
}

#[test]
fn test_get_matching_lines_a() {
    let text = "A\nB\nC\nC\nD\nE\nF\nF\nF\nF\nF\nG\nH\nI\nJ\nK\nL\n";
    let textdb = Table::text_tsv_from_str(text);

    assert!(textdb.is_sorted().unwrap());

    assert_eq!(textdb.get_matching_lines("F".as_bytes()).count(), 5);
    assert_eq!(textdb.get_matching_lines("C".as_bytes()).count(), 2);
}

#[test]
fn test_get_matching_lines_b() {
    let text = "F\nF\nF\nF\nF\n";
    let textdb = Table::text_tsv_from_str(text);

    assert!(textdb.is_sorted().unwrap());

    assert_eq!(textdb.get_matching_lines("F".as_bytes()).count(), 5);
}

#[test]
fn test_get_matching_lines_1() {
    let kv = [(6, 6), (10, 1), (113, 2), (113, 5), (129, 3), (140, 0), (168, 7), (205, 9), (211, 8), (215, 4)];
    let text = kv.iter().map(|(k, v)| format!("{k}\t{v}")).collect::<Vec<_>>().join("\n");
    let accessor = accessor::TsvParse::<u8, 0>::default();
    let map = maps::SafeMemoryMap::from_str(&text);
    let textdb = Table::new(map, accessor);

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
        let textdb = Table::new(map, accessor);

        assert!(textdb.is_sorted().unwrap());

        for (k, _v) in kv {
            // println!("finding k={k}");
            for line in textdb.get_matching_lines(&k) {
                assert_eq!(line.col(0).unwrap().parse::<u8>().unwrap(), k);
                // assert_eq!(line.col(1).unwrap().parse::<u32>().unwrap(), v);
                // println!("{k} {v} {line}");
            }
        }
    }

}
