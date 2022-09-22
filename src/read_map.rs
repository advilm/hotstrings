use std::{
    fs::File,
    io::{self, BufRead},
};

pub fn read_map(path: &str) -> io::Result<Vec<(String, String)>> {
    let mut map: Vec<(String, String)> = Vec::new();
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);
    for line in reader.lines() {
        if let Some((key, val)) = line?.split_once("::") {
            map.push((key.into(), val.into()));
        }
    }
    Ok(map)
}
