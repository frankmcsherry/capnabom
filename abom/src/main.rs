#![feature(test)]
extern crate abomonation;
extern crate memmap;
use std::fs::File;
use std::io::{BufReader, BufRead, BufWriter};
use std::env;

pub fn encode_abom(in_filename: &str, out_filename: &str) {
    // Can't encode direclty into capnproto because we don't know the size in advance, so use an
    // intermediate Vec<String>
    let mut lines: Vec<String> = Vec::new();
    let in_file = File::open(in_filename).unwrap();
    for line in BufReader::new(in_file).lines() {
        lines.push(line.unwrap());
    }
    let out_file = File::create(out_filename).unwrap();
    unsafe { abomonation::encode(&lines, &mut BufWriter::new(out_file)).unwrap(); }
}

fn decode_abom<R>(in_filename: &str, then: impl FnOnce(&Vec<String>) -> R) -> R {
    let in_file = File::open(in_filename).unwrap();
    let mut mmap = unsafe { memmap::MmapOptions::new().map_copy(&in_file).unwrap() };
    let (result, remaining) = unsafe { abomonation::decode::<Vec<String>>(&mut mmap[..]).unwrap() };
    assert_eq!(remaining.len(), 0);
    then(result)
}

fn byte_sum(s: &str) -> u32 {
    let mut res: u32 = 0;
    for b in s.bytes() {
        res = res.wrapping_add(b as u32);
    }
    res
}

pub fn decode_abom_and_get_nth_byte_sum(in_filename: &str, n: usize) -> u32 {
    decode_abom(in_filename, |words| {
        byte_sum(&words[n])
    })
}

pub fn decode_abom_and_get_all_byte_sum(in_filename: &str) -> u32 {
    decode_abom(in_filename, |words| {
        let mut res: u32 = 0;
        for word in words {
            res = res.wrapping_add(byte_sum(word));
        }
        res
    })
}

fn main() {
    let mode = env::args().nth(1).unwrap();
    let in_filename = env::args().nth(2).unwrap();
    match &mode[..] {
        "encode" => encode_abom(&in_filename, &env::args().nth(3).unwrap()),
        "decode-nth" => println!("{}", decode_abom_and_get_nth_byte_sum(&in_filename, env::args().nth(3).unwrap().parse::<usize>().unwrap())),
        "decode-all" => println!("{}", decode_abom_and_get_all_byte_sum(&in_filename)),
        _ => panic!("?")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate test;
    use self::test::Bencher;

    #[bench]
    fn bench_encode(b: &mut Bencher) {
        b.iter(|| encode_abom("/tmp/manywords", "/dev/null"));
    }

    #[bench]
    fn bench_decode_10000th(b: &mut Bencher) {
        b.iter(|| decode_abom_and_get_nth_byte_sum("/tmp/manywords-encoded-abom", 10000))
    }

    #[bench]
    fn bench_decode_all(b: &mut Bencher) {
        b.iter(|| decode_abom_and_get_all_byte_sum("/tmp/manywords-encoded-abom"))
    }
}

