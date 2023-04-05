use boxcars::ParserBuilder;
use std::collections::HashSet;

fn main() {
    println!("Hello world");
    let data = include_bytes!("../../assets/replays/good/21a81.replay");
    let parsing = ParserBuilder::new(&data[..])
        .always_check_crc()
        .must_parse_network_data()
        .parse();
    let replay = parsing.unwrap();
    let frames = replay.network_frames.unwrap().frames;

    let player_actor_type = "TAGame.Default__PRI_TA";

    // println!("{:?}", replay.properties);
    // println!("{:?}", replay.objects);
    // println!("{:?}", replay.class_indices);
    // println!("{:?}", replay.names);
    // println!("{:?}", frames[0]);
    // println!("{:?}", frames[1]);
    // println!("{:?}", frames[4]);
    // println!("{:?}", frames[500]);
    // println!("{:?}", replay.objects);
    // println!("{:?}", replay.objects);
    // println!("{:?}", frames[6]);
    // println!("{:?}", replay.objects[38]);
    // println!("{:?}", replay.objects[52]);
    // println!("{:?}", replay.objects[55]);
    // println!("{:?}", replay.objects[210]);
    // println!("{:?}", replay.objects[225]);

    // println!("{:?}", frames[0]);
    let mut player_actor_ids = HashSet::new();

    for (index, frame) in frames.iter().enumerate() {
        for new_actor_info in frame.new_actors.iter() {
            if replay.objects[new_actor_info.object_id.0 as usize] == player_actor_type {
                player_actor_ids.insert(new_actor_info.actor_id);
                println!(
                    "Actor {:?} is associated with Object {:?}, which is a {:?} object, with ",
                    new_actor_info.actor_id,
                    new_actor_info.object_id,
                    replay.objects[new_actor_info.object_id.0 as usize],
                );
            }
        }

        for updated_attribute in frame.updated_actors.iter() {
            if player_actor_ids.contains(&updated_attribute.actor_id) {
                println!(
                    "{:?}, {:?}, attribute: {:?}",
                    updated_attribute.object_id,
                    replay.objects[updated_attribute.object_id.0 as usize],
                    updated_attribute.attribute
                );
            }
        }
    }
}
