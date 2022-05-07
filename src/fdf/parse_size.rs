use anyhow;
use lazy_static::lazy_static;
use regex::Regex;

pub fn parse_size_string(s: &str) -> anyhow::Result<u64> {
    lazy_static! {
        static ref RE: Regex = Regex::new("^([0-9.]+)\\s*([kmg])?$").unwrap();
    }
    let lows = s.to_ascii_lowercase();
    let captures = RE
        .captures(&lows)
        .ok_or_else(|| anyhow::anyhow!("Invalid size string: {}", s))?;
    let size_str = captures
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("Invalid size string: {}", s))?
        .as_str();
    let size: f64 = size_str
        .parse::<f64>()
        .map_err(|_| anyhow::anyhow!("Invalid number: {}", size_str))?;
    let multiplier = captures.get(2).map_or("", |m| m.as_str());
    Ok(match multiplier {
        "k" => size * 1024f64,
        "m" => size * 1048576f64,
        "g" => size * 1073741824f64,
        _ => size,
    } as u64)
}
