#[macro_use]
extern crate nom;
use nom::{le_f32, le_f64, le_u32, le_u64};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::str;

struct ElementRecordMetadata {
    init: f64,   // The start time (in s) of the epoch represented
    intlen: f64, // The length of the interval represented
    rsize: f64,  // The size of the record in units of 8 bytes (a double)
    n: f64,      // The number of records contained here
}

named!(take8char<&[u8] ,String>,
    do_parse!(
        stuff: take!(8) >>
        (str::from_utf8(stuff).unwrap().trim().to_owned())
    )
);

named!(take_locifn<&[u8] ,String>,
    do_parse!(
        stuff: take!(60) >>
        (str::from_utf8(stuff).unwrap().trim().to_owned())
    )
);

named!(take_ftpstr<&[u8] ,String>,
    do_parse!(
        stuff: take!(28) >>
        (str::from_utf8(stuff).unwrap().trim().to_owned())
    )
);

// Structs and their parsers live below (and one after each other)

// The header has a specific structure which stores where data exist.
#[derive(Debug)]
struct Header {
    file_architecture: String, // LOCIDW [8]
    n_double_precision: u32,   // ND
    n_integers: u32,           // NI
    internal_name: String,     // LOCIFN [60]
    first_summary_block: u32,  // FWARD
    last_summary_block: u32,   // BWARD
    first_free_address: u32,   // FREE
    numeric_format: String,    // LOCFMT [8]
    integrity_string: String,  // FTPSTR [28]
}

named!(parse_header<&[u8], Header>,
  do_parse!(
    arch: take8char >>
    nd: le_u32 >>
    ni: le_u32 >>
    locifn: take_locifn >>
    first_sum_blk: le_u32 >>
    last_sum_blk: le_u32 >>
    first_free_addr: le_u32 >>
    locfmt: take8char >>
    take!(603) >> // Skipping 603 zeros
    // chksum: take_ftpstr >>
    take!(28) >>
    take!(297) >> // Skipping 297 zeros
    (Header{
        file_architecture: arch,
        n_double_precision: nd,
        n_integers: ni,
        internal_name: locifn,
        first_summary_block: first_sum_blk,
        last_summary_block: last_sum_blk,
        first_free_address: first_free_addr,
        numeric_format: locfmt,
        integrity_string: "DUMMY".to_owned(),
    })
  )
);

// The comment records are followed by summary records.
// Blocks of summary records are chained like a linked list with each block
// having a header section that carries the chain information. These values are
// integers but they are stored as doubles
#[derive(Debug)]
struct SummaryRecordBlockHeader {
    next_summary_record_blk: f64, // pointer to next SR 1Kb block (1 indexed)
    prev_summary_record_blk: f64, // pointer to previous SR block (1 indexed)
    n_summaries: f64,             // number of element summaries here
}

// Parse summary record block header
named!(parse_srbh<&[u8], SummaryRecordBlockHeader>,
  do_parse!(
    next_summary_record_blk: le_f64 >>
    prev_summary_record_blk: le_f64 >>
    n_summaries: le_f64 >>
    (SummaryRecordBlockHeader{
        next_summary_record_blk,
        prev_summary_record_blk,
        n_summaries,
    })
  )
);

#[derive(Debug)]
struct Summary {
    begin_second: f64, // initial epoch, as seconds from J2000
    end_second: f64,   // final epoch, as seconds from J2000
    target_id: u32,    // target identifier
    center_id: u32,    // center identifier
    frame_id: u32,     // frame identifier (we handle 1 - J2000 - only)
    data_type: u32,    // data type identifier (we handle II or III)
    start_i: u32,      // index (8 byte blocks) where segment data starts
    end_i: u32,        // index (8 byte blocks) where segment data ends
}

// Parse summary record block header
named!(parse_summary<&[u8], Summary>,
  do_parse!(
      begin_second: le_f64 >>
      end_second: le_f64 >>
      target_id: le_u32 >>
      center_id: le_u32 >>
      frame_id: le_u32 >>
      data_type: le_u32 >>
      start_i: le_u32 >>
      end_i: le_u32 >>
      (Summary{
          begin_second,
          end_second,
          target_id,
          center_id,
          frame_id,
          data_type,
          start_i,
          end_i,
      })
  )
);

#[derive(Debug, PartialEq, Clone)]
struct Body {
    name: String,
    naif_id: i16, // Some NAIF ID may be negative, esp. for spacecraft
    gm: f64,
}

named!(seek_bodies, take_until_and_consume!("Bodies included:\0\0"));
named!(til_next_null, take_until_and_consume!("\0"));
named!(til_next_open_parens, ws!(take_until_and_consume!("(")));
named!(til_next_close_parens, ws!(take_until_and_consume!(")")));

named!(seek_to_gms<&[u8], &[u8]>,
    take_until_and_consume!("Sun/GM(I)")
);

named!(seek_to_end_of_comment<&[u8], &[u8]>,
    take_until_and_consume!("\04")
);

named!(parse_each_gm<&[u8], &[u8]>,
    do_parse!(
        many0!(tag!("Sun/GM(I)")) >>
        take_until_and_consume!("GM") >>
        fullline: til_next_null >>
        (fullline)
    )
);

named!(parse_all_body_hdr<&[u8],(Vec<Body>, &[u8])>,
    many_till!(parse_body_hdr, tag!("\0\0"))
);

named!(parse_body_hdr<&[u8], Body>,
  do_parse!(    // the parser takes a byte array as input, and returns an A struct
    many0!(tag!("\0")) >> // Remove any leading nulls
    name: til_next_open_parens >>
    naif_id : til_next_close_parens >>
    (Body{gm: -1.0, name: str::from_utf8(name).unwrap().trim().to_owned(),
        naif_id: str::from_utf8(naif_id).unwrap().parse::<i16>().unwrap()})
  )
);

named!(get_next_float<&[u8], f64>,
    do_parse!(
        val: le_f64 >>
        (val)
    )
);

fn record_positions(block: usize) -> (usize, usize) {
    let block_size: usize = 1024;
    let start_byte = (block as usize - 1) * block_size;
    (start_byte, start_byte + block_size)
}

fn main() {
    let summary_length = 2 * 8 + 4 * 6; // 2: NI; 6: ND; 8 = sizeof(double); 6 = sizeof(int);
    let summary_hdr_size = 3 * 8; // 2: NI; 6: ND; 8 = sizeof(double); 6 = sizeof(int);
    let summaries_per_record = (1024 - 8 * 3) / summary_length;
    let parse_bodies = false;
    let mut f = File::open("./data/de421.bsp").expect("open"); // This fails to read the comments with the GMs

    // let mut f = File::open("./data/de436s.bsp").expect("open");
    // let mut f = File::open("./data/de436.bsp").expect("open");
    let mut mutbuf = vec![0; 0];
    f.read_to_end(&mut mutbuf).expect("to end");
    let buffer = mutbuf.clone();
    let (rem, hdr) = parse_header(&buffer).expect("could not read header");
    println!("{:?}", hdr);
    if parse_bodies {
        // We've got that header, let's parse the comment to get the list of bodies (this might fail)
        let (rem, _) = seek_bodies(&rem).expect("could not seek until bodies");
        let (rem, body_hdrs) =
            parse_all_body_hdr(&rem).expect("could not parse comment with bodies");

        // Let put all this into a HashMap for quick access
        let mut data = HashMap::new();
        for body in &(body_hdrs.0) {
            data.insert(body.naif_id, body.to_owned());
        }
        let mut buf = seek_to_gms(rem).unwrap().0;
        let mut sun_mu = -1.0f64;
        loop {
            match parse_each_gm(buf) {
                Ok(one) => {
                    let mut p_id = "";
                    let mut mu = -1.0;
                    for (ino, item) in str::from_utf8(one.1)
                        .unwrap()
                        .split_whitespace()
                        .enumerate()
                    {
                        match ino {
                            0 => p_id = item,
                            2 => {
                                // In the terrible format of a FORTRAN memory dump,
                                // the useful information, although always in column
                                // three, actually sometimes has an extra null byte.
                                // This breaks the parser. So here we're checking if
                                // we're parsing the Earth barycenter or the Moon GM
                                // and if so, we'll parse the second column and do the
                                // math. So far, I have only seen those rows break.
                                if p_id == "M" || p_id == "B" {
                                    mu = sun_mu / (item.replace("D", "E").parse::<f64>().unwrap());
                                }
                            }
                            3 => {
                                if p_id != "M" && p_id != "B" {
                                    mu = item.replace("D", "E").parse::<f64>().unwrap();
                                    if p_id == "S" {
                                        sun_mu = mu;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    if mu > -1.0 {
                        // If it's an integer, update the appropriate value.
                        match p_id.parse::<i16>() {
                            Ok(p_id) => {
                                let naif_id = if p_id < 4 { p_id * 100 + 99 } else { p_id };
                                let mut cur_data = data.get(&naif_id).unwrap().to_owned();
                                cur_data.gm = mu;
                                data.insert(naif_id, cur_data.to_owned());
                                if p_id < 3 {
                                    // Venus exists as both "Venus" and "Venus Barycenter"
                                    let mut cur_data = data.get(&p_id).unwrap().to_owned();
                                    cur_data.gm = mu;
                                    data.insert(p_id, cur_data.to_owned());
                                }
                            }
                            Err(_) => {
                                // This ID has a name.
                                let naif_id = match p_id {
                                    "S" => {
                                        10 // Sun
                                    }
                                    "M" => {
                                        // Moon
                                        301
                                    }
                                    "B" => {
                                        // Earth barycenter
                                        3
                                    }
                                    _ => {
                                        println!("unknown body `GM{}`", p_id);
                                        -1
                                    }
                                };
                                if naif_id > -1 {
                                    let mut cur_data = data.get(&naif_id).unwrap().to_owned();
                                    cur_data.gm = mu;
                                    data.insert(naif_id, cur_data.to_owned());
                                }
                            }
                        }
                    } else {
                        break;
                    }
                    buf = one.0;
                }
                Err(_) => {
                    println!("done");
                    break;
                }
            }
        }
        for body in data.values() {
            println!("{:?}", body);
        }
    }
    // Seek until the end of the comment so that we skip all the asteroids.
    let rem = seek_to_end_of_comment(rem).unwrap().0;
    // And now parse the summaries
    let mut summary_block = hdr.first_summary_block as usize - 1;
    // Read all the summary blocks
    let mut all_summaries = Vec::with_capacity(15);
    while summary_block > 0 {
        // let start_byte = (hdr.first_summary_block as usize - 1) * block_size;
        // let end_byte = start_byte + block_size;
        let (start, end) = record_positions(hdr.first_summary_block as usize);
        let (mut rem_summary, summary_hdr) = parse_srbh(&buffer[start..end]).expect("ugh");
        println!("{:?}", summary_hdr);
        let (start_name, end_name) = record_positions(hdr.first_summary_block as usize + 1);
        let name_buffer = &buffer[start_name..end_name];
        for sno in 0..summary_hdr.n_summaries as u64 {
            summary_block = summary_hdr.next_summary_record_blk as usize;
            // Read the record from the summary we just read.
            let usno = sno as usize;
            let name = str::from_utf8(&name_buffer[40 * usno..(usno + 1) * 40])
                .expect("could not decode the name of this summary")
                .trim();
            println!("{:?}", name);
            let (next_rem_summary, summary) = parse_summary(rem_summary).expect("ugh");
            println!("{:?} => {}", summary, next_rem_summary.len());
            rem_summary = next_rem_summary;
            all_summaries.push(summary);
        }
    }
    println!("done with summaries ({})", all_summaries.len());
    // Read the first summary entry
    let (start_name, end_name) = record_positions(all_summaries[14].end_i as usize - 1);
    let mut array_buffer = &buffer[start_name..end_name];
    // Read through the whole array
    loop {
        let (rem_array, val) = get_next_float(array_buffer).expect("mm");
        println!("{:?}", val);
        if rem_array.len() == 0 {
            println!("done");
            break;
        }
        array_buffer = rem_array;
    }
}
