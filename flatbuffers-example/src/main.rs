#[allow(warnings)]
mod generated;

use generated::my_game::sample::{Monster, MonsterArgs, Vec3};

fn main() -> anyhow::Result<()> {
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
    println!("Read back monster: {monster:?}");

    Ok(())
}
