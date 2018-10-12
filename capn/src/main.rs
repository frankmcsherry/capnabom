#![feature(test, duration_float)]
extern crate test;
extern crate capnp;
pub mod foo_capnp {
    include!(concat!(env!("OUT_DIR"), "/foo_capnp.rs"));
}

extern crate memmap;
use std::fs::File;
use std::io::{BufReader, BufRead, BufWriter};
use std::env;
use std::time::Instant;

pub fn encode_capn(in_filename: &str, out_filename: &str) {
    // Can't encode direclty into capnproto because we don't know the size in advance, so use an
    // intermediate Vec<String>
    let mut lines: Vec<String> = Vec::new();
    let in_file = File::open(in_filename).unwrap();
    for line in BufReader::new(in_file).lines() {
        lines.push(line.unwrap());
    }
    let mut builder = capnp::message::Builder::new_default();
    {
        let msg = builder.init_root::<foo_capnp::dictionary::Builder>();
        let mut words = msg.init_words(lines.len() as u32);
        for (i, line) in lines.iter().enumerate() {
            words.set(i as u32, line);
        }
    }
    let out_file = File::create(out_filename).unwrap();
    capnp::serialize::write_message(&mut BufWriter::new(out_file), &builder).unwrap();
}

pub fn encode_capn_lean(lines: &Vec<String>, bytes: &mut Vec<u8>) {
    let mut builder = capnp::message::Builder::new_default();
    {
        let msg = builder.init_root::<foo_capnp::dictionary::Builder>();
        let mut words = msg.init_words(lines.len() as u32);
        for (i, line) in lines.iter().enumerate() {
            words.set(i as u32, line);
        }
    }
    capnp::serialize::write_message(bytes, &builder).unwrap();
}

fn decode_capn<R>(in_filename: &str, then: impl FnOnce(capnp::text_list::Reader) -> R) -> R {
    let in_file = File::open(in_filename).unwrap();
    let mmap = unsafe { memmap::Mmap::map(&in_file).unwrap() };
    let reader = capnp::serialize::read_message_from_words(
        unsafe { capnp::Word::bytes_to_words(&mmap[..]) },
        *capnp::message::ReaderOptions::new().traversal_limit_in_words(1000000000)
    ).unwrap();
    let msg = reader.get_root::<foo_capnp::dictionary::Reader>().unwrap();
    let words = msg.get_words().unwrap();
    then(words)
}

fn decode_capn_lean<R>(bytes: &mut [u8], then: impl FnOnce(capnp::text_list::Reader) -> R) -> R {
    let reader = capnp::serialize::read_message_from_words(
        unsafe { capnp::Word::bytes_to_words(bytes) },
        *capnp::message::ReaderOptions::new().traversal_limit_in_words(1000000000)
    ).unwrap();
    let msg = reader.get_root::<foo_capnp::dictionary::Reader>().unwrap();
    let words = msg.get_words().unwrap();
    then(words)
}

fn byte_sum(s: &str) -> u32 {
    let mut res: u32 = 0;
    for b in s.bytes() {
        res = res.wrapping_add(b as u32);
    }
    res
}

pub fn decode_capn_and_get_nth_byte_sum(in_filename: &str, n: usize) -> u32 {
    decode_capn(in_filename, |words| {
        let word = words.get(n as u32).unwrap();
        byte_sum(word)
    })
}

pub fn decode_capn_and_get_all_byte_sum(in_filename: &str) -> u32 {
    decode_capn(in_filename, |words| {
        let mut res: u32 = 0;
        for word in words {
            let word = word.unwrap();
            res = res.wrapping_add(byte_sum(word));
            //test::black_box(word);
        }
        res
    })
}

pub fn test_encode_pure(in_filename: &str) {
    let mut lines: Vec<String> = Vec::new();
    let in_file = File::open("/tmp/manywords").unwrap();
    for line in BufReader::new(in_file).lines() {
        lines.push(line.unwrap());
    }
    // since cargo bench wants to run 300 times which is way too slow
    let start = Instant::now();
    for _ in 0..10 {
        let mut builder = capnp::message::Builder::new_default();
        {
            let msg = builder.init_root::<foo_capnp::dictionary::Builder>();
            let mut words = msg.init_words(lines.len() as u32);
            for (i, line) in lines.iter().enumerate() {
                words.set(i as u32, line);
            }
        }
        //let mut out: Vec<u8> = Vec::new();
        let out_file = File::create("/dev/null").unwrap();
        let mut out = BufWriter::new(out_file);
        capnp::serialize::write_message(&mut out, &builder).unwrap();
        test::black_box(&out);
    }
    println!("{}", start.elapsed().as_float_secs());
}

fn main() {
    let mode = env::args().nth(1).unwrap();
    let in_filename = env::args().nth(2).unwrap();
    match &mode[..] {
        "encode" => encode_capn(&in_filename, &env::args().nth(3).unwrap()),
        "decode-nth" => println!("{}", decode_capn_and_get_nth_byte_sum(&in_filename, env::args().nth(3).unwrap().parse::<usize>().unwrap())),
        "decode-all" => println!("{}", decode_capn_and_get_all_byte_sum(&in_filename)),
        "encode-pure" => test_encode_pure(&in_filename),
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
        b.iter(|| encode_capn("/tmp/manywords", "/dev/null"));
    }

    #[bench]
    fn bench_decode_10000th(b: &mut Bencher) {
        b.iter(|| decode_capn_and_get_nth_byte_sum("/tmp/manywords-encoded-capn", 10000))
    }

    #[bench]
    fn bench_decode_all(b: &mut Bencher) {
        b.iter(|| decode_capn_and_get_all_byte_sum("/tmp/manywords-encoded-capn"))
    }

    #[bench]
    fn bench_encode_lean(b: &mut Bencher) {
        let mut lines: Vec<String> = Vec::new();
        let in_file = File::open("/tmp/manywords").unwrap();
        for line in BufReader::new(in_file).lines() {
            lines.push(line.unwrap());
        }
        let mut buffer = Vec::new();
        b.iter(|| {
            buffer.clear();
            encode_capn_lean(&lines, &mut buffer);
        });
    }

    #[bench]
    fn bench_decode_10000th_lean(b: &mut Bencher) {
        let mut lines: Vec<String> = Vec::new();
        let in_file = File::open("/tmp/manywords").unwrap();
        for line in BufReader::new(in_file).lines() {
            lines.push(line.unwrap());
        }
        let mut buffer = Vec::new();
        encode_capn_lean(&lines, &mut buffer);

        b.iter(|| {
            decode_capn_lean(&mut buffer, |words| byte_sum(words.get(10000).unwrap()))
        });
    }

    #[bench]
    fn bench_decode_all_lean(b: &mut Bencher) {
        let mut lines: Vec<String> = Vec::new();
        let in_file = File::open("/tmp/manywords").unwrap();
        for line in BufReader::new(in_file).lines() {
            lines.push(line.unwrap());
        }
        let mut buffer = Vec::new();
        encode_capn_lean(&lines, &mut buffer);

        b.iter(|| {
            decode_capn_lean(&mut buffer, |words| {
                let mut res: u32 = 0;
                for word in words {
                    let word = word.unwrap();
                    res = res.wrapping_add(byte_sum(word));
                    //test::black_box(word);
                }
                res
            })
        });
    }
}



