    use super::*;

    #[test]
    fn base_url_normalization() {
        assert_eq!(base_url(""), DEFAULT_BASE.to_string());
        assert_eq!(base_url("https://x/y/"), "https://x/y/".to_string());
        assert_eq!(base_url("https://x/y"), "https://x/y/".to_string());
    }

    #[test]
    fn region_at_picks_smallest_containing() {
        let m = Manifest {
            world: Meta {
                url: "world.bin".into(),
                sha256: "a".into(),
                size: 1,
            },
            cities: Meta {
                url: "cities.bin".into(),
                sha256: "b".into(),
                size: 1,
            },
            regions: BTreeMap::from([
                (
                    "AU".into(),
                    RegionMeta {
                        name: "Australia".into(),
                        bbox: [113.0, -44.0, 154.0, -10.0],
                        url: "countries/AU.bin".into(),
                        sha256: "c".into(),
                        size: 1,
                    },
                ),
                (
                    "JP".into(),
                    RegionMeta {
                        name: "Japan".into(),
                        bbox: [122.0, 24.0, 154.0, 46.0],
                        url: "countries/JP.bin".into(),
                        sha256: "d".into(),
                        size: 1,
                    },
                ),
            ]),
        };
        let (iso, r) = region_at(&m, 139.0, 35.0).expect("hit");
        assert_eq!(iso, "JP");
        assert_eq!(r.name, "Japan");
    }
