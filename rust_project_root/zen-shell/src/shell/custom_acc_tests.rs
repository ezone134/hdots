    use super::*;

    #[test]
    fn parses_custom_acc_json() {
        // shape emitted by gen_launcher_cache → $states2/custom_acc.json:
        // [{name,hex}] where hex is a lowercase hex string (no '#').
        let raw = r#"[{"name":"blue","hex":"89b4fa"},{"name":"mauve","hex":"cba6f7"}]"#;
        let list: Vec<CustomAcc> = serde_json::from_str(raw).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "blue");
        assert_eq!(list[0].hex, "89b4fa");
        assert_eq!(list[1].name, "mauve");
    }

    #[test]
    fn acc_hex_flag_detects_hash_and_bare_hex() {
        // first digit `#` → save chip
        assert!(Shell::acc_hex_flag("#ff8800\n"));
        assert!(Shell::acc_hex_flag("  #abc  "));
        // bare hex codes (no hash) also count as "has a hex code"
        assert!(Shell::acc_hex_flag("ff8800"));
        assert!(Shell::acc_hex_flag("abc"));
        assert!(Shell::acc_hex_flag("deadbeef"));
        // names / empty / junk → no chip
        assert!(!Shell::acc_hex_flag(""));
        assert!(!Shell::acc_hex_flag("   \n"));
        assert!(!Shell::acc_hex_flag("blue"));
        assert!(!Shell::acc_hex_flag("Accent 3"));
        assert!(!Shell::acc_hex_flag("ff88"));
    }
