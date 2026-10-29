use super::*;
use sha2::{Digest, Sha256};

#[test]
fn capture_encoder_baseline() {
    let pcm: Vec<i16> = (0..32_123)
        .map(|i| ((i * 997) % 60_001 - 30_000) as i16)
        .collect();
    let mut encoder = LiveMp3Encoder::new();
    let cases = [
        ("empty", encoder.push_pcm(&[])),
        ("first", encoder.push_pcm(&pcm[..7000])),
        ("cross", encoder.push_pcm(&pcm[7000..25_000])),
        ("last", encoder.push_pcm(&pcm[25_000..])),
        ("finish", encoder.finish()),
        ("finish_again", encoder.finish()),
        ("after_finish", encoder.push_pcm(&[1])),
    ];
    for (name, result) in cases {
        match result {
            Ok(packets) => {
                println!("ORACLE {name} count={}", packets.len());
                for packet in packets {
                    println!(
                        "ORACLE {name} len={} sha={:x}",
                        packet.len(),
                        Sha256::digest(&packet)
                    );
                }
            }
            Err(error) => println!("ORACLE {name} error={error}"),
        }
    }
}
