use generic_storage::{
    models::Person,
    serializers::{Borsh, Json, Wincode},
    storage::{
        transcribe_borsh_to_json, transcribe_json_to_wincode, transcribe_wincode_to_borsh, Storage,
    },
};

fn main() {
    let person = Person::new("André", 30);

    //borsh
    let mut borsh_storage = Storage::<Person, Borsh>::new();
    borsh_storage.save(&person).unwrap();

    let loaded = borsh_storage.load().unwrap();
    println!(
        "[Borsh]   loaded: {:?}  ({} bytes)",
        loaded,
        borsh_storage.byte_len().unwrap()
    );

    //wincode
    let mut wincode_storage = Storage::<Person, Wincode>::new();
    wincode_storage.save(&person).unwrap();

    let loaded = wincode_storage.load().unwrap();
    println!(
        "[Wincode] loaded: {:?}  ({} bytes)",
        loaded,
        wincode_storage.byte_len().unwrap()
    );

    //json
    let mut json_storage = Storage::<Person, Json>::new();
    json_storage.save(&person).unwrap();

    let loaded = json_storage.load().unwrap();
    println!(
        "[JSON]    loaded: {:?}  ({} bytes)",
        loaded,
        json_storage.byte_len().unwrap()
    );
    println!("[JSON]    raw:    {}", json_storage.as_json_str().unwrap());

    //bonus
    println!("\n── transcription demo ──────────────────────────────────────────");

    let mut dst_json = Storage::<Person, Json>::new();
    let mut dst_wincode = Storage::<Person, Wincode>::new();
    let mut dst_borsh = Storage::<Person, Borsh>::new();

    transcribe_borsh_to_json(&borsh_storage, &mut dst_json).unwrap();
    transcribe_json_to_wincode(&dst_json, &mut dst_wincode).unwrap();
    transcribe_wincode_to_borsh(&dst_wincode, &mut dst_borsh).unwrap();

    let round_tripped = dst_borsh.load().unwrap();
    println!("Borsh → JSON → Wincode → Borsh: {:?}", round_tripped);
    assert_eq!(person, round_tripped, "round-trip must be lossless");
    println!("Round-trip assertion passed.");
}
