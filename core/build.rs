use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    generate_off_gcd_set(&out_dir);
    generate_attack_types_map(&out_dir);
    generate_alacrity_buffs_map(&out_dir);
    generate_discipline_abilities_map(&out_dir);

    println!("cargo:rerun-if-changed=data/off_gcd.json");
    println!("cargo:rerun-if-changed=data/attack_types.csv");
    println!("cargo:rerun-if-changed=data/alacrity_abilities.csv");
    println!("cargo:rerun-if-changed=data/discipline_unique_abilities.csv");
}

fn generate_off_gcd_set(out_dir: &str) {
    let json = fs::read_to_string("data/off_gcd.json").expect("failed to read off_gcd.json");

    // Simple JSON object parse: extract all numeric keys
    let mut ids: Vec<i64> = json
        .split('"')
        .enumerate()
        .filter_map(|(i, s)| if i % 2 == 1 { s.parse::<i64>().ok() } else { None })
        .collect();
    ids.sort_unstable();
    ids.dedup();

    let path = Path::new(out_dir).join("off_gcd_abilities.rs");
    let mut file = BufWriter::new(fs::File::create(&path).unwrap());

    let mut builder = phf_codegen::Set::new();
    for id in &ids {
        builder.entry(*id);
    }

    writeln!(file, "pub static OFF_GCD_ABILITIES: phf::Set<i64> = {};", builder.build()).unwrap();
}

fn generate_attack_types_map(out_dir: &str) {
    let csv = fs::read_to_string("data/attack_types.csv").expect("failed to read attack_types.csv");

    // BTreeMap for deterministic output (sorted by key)
    let mut entries = BTreeMap::new();
    for line in csv.lines().skip(1) {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 2 {
            continue;
        }
        let id: i64 = match fields[0].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let attack_type = fields[1].trim();
        if matches!(attack_type, "" | "None" | "God") {
            continue;
        }
        entries.entry(id).or_insert(attack_type.to_string());
    }

    let path = Path::new(out_dir).join("attack_types.rs");
    let mut file = BufWriter::new(fs::File::create(&path).unwrap());

    let mut builder = phf_codegen::Map::new();
    let quoted: Vec<_> = entries.iter().map(|(id, at)| (*id, format!("\"{}\"", at))).collect();
    for (id, at) in &quoted {
        builder.entry(*id, at);
    }

    writeln!(file, "pub static ATTACK_TYPES: phf::Map<i64, &'static str> = {};", builder.build())
        .unwrap();
}

fn generate_alacrity_buffs_map(out_dir: &str) {
    let csv = fs::read_to_string("data/alacrity_abilities.csv")
        .expect("failed to read alacrity_abilities.csv");

    // BTreeMap for deterministic output (sorted by key)
    let mut entries = BTreeMap::new();
    for line in csv.lines().skip(1) {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 5 {
            continue;
        }
        let id: i64 = match fields[0].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let amount: f32 = match fields[2].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let is_stack = fields[3].trim().eq_ignore_ascii_case("true");
        let duration_secs: f32 = match fields[4].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        entries.insert(
            id,
            format!(
                "AlacrityBuff {{ amount: {amount}f32, is_stack: {is_stack}, duration_secs: {duration_secs}f32 }}"
            ),
        );
    }

    let path = Path::new(out_dir).join("alacrity_buffs.rs");
    let mut file = BufWriter::new(fs::File::create(&path).unwrap());

    let mut builder = phf_codegen::Map::new();
    for (id, buff) in &entries {
        builder.entry(*id, buff);
    }

    writeln!(file, "pub static ALACRITY_BUFFS: phf::Map<i64, AlacrityBuff> = {};", builder.build())
        .unwrap();
}

fn generate_discipline_abilities_map(out_dir: &str) {
    let csv = fs::read_to_string("data/discipline_unique_abilities.csv")
        .expect("failed to read discipline_unique_abilities.csv");

    // Ability ID → discipline GUID. BTreeMap for deterministic output (sorted by key)
    let mut entries = BTreeMap::new();
    for line in csv.lines().skip(1) {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 4 {
            continue;
        }
        let discipline_id: i64 = match fields[1].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ability_id: i64 = match fields[3].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        entries.insert(ability_id, discipline_id);
    }

    let path = Path::new(out_dir).join("discipline_abilities.rs");
    let mut file = BufWriter::new(fs::File::create(&path).unwrap());

    let mut builder = phf_codegen::Map::new();
    let values: Vec<_> = entries.iter().map(|(id, disc)| (*id, format!("{disc}i64"))).collect();
    for (id, disc) in &values {
        builder.entry(*id, disc);
    }

    writeln!(
        file,
        "pub static DISCIPLINE_ABILITIES: phf::Map<i64, i64> = {};",
        builder.build()
    )
    .unwrap();
}
