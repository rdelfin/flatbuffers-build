// Example crate

#[allow(warnings)]
#[rustfmt::skip]
mod gen_flatbuffers {
    // for a cleaner code, you could do this in a separate module
    include!(concat!(env!("OUT_DIR"), "/flatbuffers/mod.rs"));
}
use gen_flatbuffers::my_game::sample::{Monster, MonsterArgs, MonsterT, Vec3, WeaponT};

fn main() -> anyhow::Result<()> {
    // Use builder style:
    // Writing a monster object to encoded bytes
    let mut builder = flatbuffers::FlatBufferBuilder::with_capacity(1024);
    let m = Monster::create(
        &mut builder,
        &MonsterArgs {
            pos: Some(&Vec3::new(0.1, 0.2, 0.3)),
            ..Default::default()
        },
    );
    builder.finish(m, None);
    let encoded_data = builder.finished_data();

    // Reading the object back
    let monster = flatbuffers::root::<Monster>(encoded_data)?;
    println!("Read back monster (created builder style): {monster:?}");

    // Use Object style:
    let mut builder = flatbuffers::FlatBufferBuilder::with_capacity(1024);
    let sword: WeaponT = WeaponT {
        name: Some("sword".to_string()),
        damage: 420,
    };
    let m: MonsterT = MonsterT {
        name: Some("Le MonsterT".to_string()),
        weapons: vec![sword].into(),
        ..Default::default()
    };

    let monster_offset = m.pack(&mut builder);
    builder.finish(monster_offset, None);
    let encoded_data = builder.finished_data();

    // Reading the object back
    let monster = flatbuffers::root::<Monster>(encoded_data)?;
    println!("Read back monster (create with object api): {monster:?}");

    Ok(())
}
