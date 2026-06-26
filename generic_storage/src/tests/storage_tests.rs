#[cfg(test)]
mod tests {
    use crate::{
        error::StorageError,
        models::{Config, Person},
        serializers::{Borsh, Json, Wincode},
        storage::{
            transcribe_borsh_to_json, transcribe_json_to_wincode, transcribe_wincode_to_borsh,
            Storage,
        },
    };

    fn sample_person() -> Person {
        Person::new("André", 30)
    }

    //borsh
    #[test]
    fn borsh_round_trip_person() {
        let person = sample_person();
        let mut s = Storage::<Person, Borsh>::new();

        assert!(!s.has_data());
        s.save(&person).unwrap();
        assert!(s.has_data());

        let loaded = s.load().unwrap();
        assert_eq!(person, loaded);
    }

    #[test]
    fn borsh_round_trip_config() {
        let cfg = Config::sample();
        let mut s = Storage::<Config, Borsh>::new();
        s.save(&cfg).unwrap();
        assert_eq!(cfg, s.load().unwrap());
    }

    #[test]
    fn borsh_empty_returns_error() {
        let s = Storage::<Person, Borsh>::new();
        assert!(matches!(s.load(), Err(StorageError::Empty)));
    }

    //wincode
    #[test]
    fn wincode_round_trip_person() {
        let person = sample_person();
        let mut s = Storage::<Person, Wincode>::new();

        assert!(!s.has_data());
        s.save(&person).unwrap();
        assert!(s.has_data());

        assert_eq!(person, s.load().unwrap());
    }

    #[test]
    fn wincode_round_trip_config() {
        let cfg = Config::sample();
        let mut s = Storage::<Config, Wincode>::new();
        s.save(&cfg).unwrap();
        assert_eq!(cfg, s.load().unwrap());
    }

    #[test]
    fn wincode_empty_returns_error() {
        let s = Storage::<Person, Wincode>::new();
        assert!(matches!(s.load(), Err(StorageError::Empty)));
    }

    //json
    #[test]
    fn json_round_trip_person() {
        let person = sample_person();
        let mut s = Storage::<Person, Json>::new();

        assert!(!s.has_data());
        s.save(&person).unwrap();
        assert!(s.has_data());

        assert_eq!(person, s.load().unwrap());
    }

    #[test]
    fn json_round_trip_config() {
        let cfg = Config::sample();
        let mut s = Storage::<Config, Json>::new();
        s.save(&cfg).unwrap();
        assert_eq!(cfg, s.load().unwrap());
    }

    #[test]
    fn json_empty_returns_error() {
        let s = Storage::<Person, Json>::new();
        assert!(matches!(s.load(), Err(StorageError::Empty)));
    }

    #[test]
    fn json_bytes_are_valid_utf8() {
        let mut s = Storage::<Person, Json>::new();
        s.save(&sample_person()).unwrap();
        // as_json_str is a Json-only method — verifies bytes are human-readable
        assert!(s.as_json_str().is_some());
    }

    // ── Byte-size ordering (binary < JSON)

    #[test]
    fn binary_formats_smaller_than_json() {
        let person = sample_person();

        let mut borsh_s = Storage::<Person, Borsh>::new();
        let mut wincode_s = Storage::<Person, Wincode>::new();
        let mut json_s = Storage::<Person, Json>::new();

        borsh_s.save(&person).unwrap();
        wincode_s.save(&person).unwrap();
        json_s.save(&person).unwrap();

        // Binary formats should always be more compact than JSON for simple structs.
        assert!(borsh_s.byte_len().unwrap() < json_s.byte_len().unwrap());
        assert!(wincode_s.byte_len().unwrap() < json_s.byte_len().unwrap());
    }

    // ── Bonus: transcription

    #[test]
    fn full_transcription_chain_is_lossless() {
        let original = sample_person();

        let mut src_borsh = Storage::<Person, Borsh>::new();
        let mut via_json = Storage::<Person, Json>::new();
        let mut via_wincode = Storage::<Person, Wincode>::new();
        let mut back_borsh = Storage::<Person, Borsh>::new();

        src_borsh.save(&original).unwrap();
        transcribe_borsh_to_json(&src_borsh, &mut via_json).unwrap();
        transcribe_json_to_wincode(&via_json, &mut via_wincode).unwrap();
        transcribe_wincode_to_borsh(&via_wincode, &mut back_borsh).unwrap();

        assert_eq!(original, back_borsh.load().unwrap());
    }

    #[test]
    fn transcribe_fails_gracefully_on_empty_src() {
        let src = Storage::<Person, Borsh>::new();
        let mut dst = Storage::<Person, Json>::new();

        // transcribe from empty storage should propagate StorageError::Empty
        assert!(matches!(
            transcribe_borsh_to_json(&src, &mut dst),
            Err(StorageError::Empty)
        ));
    }

    // ── Overwrite semantics
    #[test]
    fn save_overwrites_previous_value() {
        let mut s = Storage::<Person, Json>::new();
        s.save(&Person::new("Alice", 25)).unwrap();
        s.save(&Person::new("Bob", 40)).unwrap();

        let loaded = s.load().unwrap();
        assert_eq!(loaded.name, "Bob");
        assert_eq!(loaded.age, 40);
    }
}
