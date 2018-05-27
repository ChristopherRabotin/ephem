#[macro_use]
extern crate nom;
use std::io;
use std::io::prelude::*;
use std::fs::File;

use nom::{digit, IResult};

// Parser definition

use std::str;
use std::str::FromStr;

named!(consume_until_null, take_until_and_consume!("\0"));
// We parse any expr surrounded by parens, ignoring all whitespaces around those
named!(parens<u8>, ws!(delimited!(tag!("("), naif_id, tag!(")"))));
// named!(
//     naif_id<u8>,
//     alt!(map_res!(map_res!(ws!(digit), str::from_utf8), FromStr::from_str) | parens)
// );
named!(
    naif_id<u8>,
    map_res!(map_res!(ws!(digit), str::from_utf8), FromStr::from_str)
);
named!(take_hdr, take_until_and_consume!("DAF/SPK"));
named!(
    seek_krnl_name,
    take_until_and_consume!("JPL planetary and lunar ephmeris ")
);
named!(seek_krnl_date, take_until_and_consume!("Integrated "));
named!(
    seek_span,
    take_until_and_consume!("span covered by ephemeris:\0\0")
);
named!(seek_start_date, take_until_and_consume!("to"));
named!(seek_bodies, take_until_and_consume!("Bodies included:\0\0"));
named!(til_next_null, take_until_and_consume!("\0"));
named!(til_next_dnull, take_until_and_consume!("\0\0"));
named!(til_next_open_parens, ws!(take_until_and_consume!("(")));
named!(til_next_close_parens, ws!(take_until_and_consume!(")")));
named!(
    naif_id_str,
    ws!(delimited!(tag!("("), take_until!("("), tag!(")")))
);
//named!(body_info, tuple!(body_name, naif_id));

#[derive(Debug, PartialEq)]
struct SPK<'a> {
    name: &'a str,
    date: &'a str,
    start_date: &'a str,
    end_date: &'a str,
}

#[derive(Debug, PartialEq)]
struct Body<'a> {
    name: &'a str,
    naif_id: i16, // Some NAIF ID may be negative, esp. for spacecraft
}

named!(parse_all_body_hdr<&[u8],(Vec<Body>, &[u8])>,
    many_till!(parse_body_hdr, tag!("\0\0"))
);

named!(parse_body_hdr<&[u8],Body>,
  do_parse!(    // the parser takes a byte array as input, and returns an A struct
    many0!(tag!("\0")) >> // Remove any leading nulls
    name: til_next_open_parens >>
    naif_id : til_next_close_parens >>
    (Body{name: str::from_utf8(name).unwrap().trim(),
        naif_id: str::from_utf8(naif_id).unwrap().parse::<i16>().unwrap()})
  )
);

named!(parse_meta<&[u8],SPK>,
  do_parse!(    // the parser takes a byte array as input, and returns an A struct
    take_hdr >> // check for header
    seek_krnl_name >> // consume until there
    dn: take!(5) >> // get the DE name
    seek_krnl_date >>
    date: consume_until_null >> // get the date time of file
    seek_span >>
    start_date: seek_start_date >>
    end_date: ws!(til_next_null) >>
    seek_bodies >> // Advance buffer until the body header for the next parser
    (SPK{name: str::from_utf8(dn).unwrap(), date: str::from_utf8(date).unwrap(),
        start_date: str::from_utf8(start_date).unwrap(),
        end_date: str::from_utf8(end_date).unwrap()})
  )
);

fn main() {
    let mut f = File::open("data/de436.bsp").expect("open");
    let mut buffer = vec![0; 0];
    f.read_to_end(&mut buffer).expect("to end");
    match parse_meta(&buffer) {
        Ok(spk_info) => {
            println!("{:?}", spk_info.1);
            match parse_all_body_hdr(spk_info.0) {
                Ok(body_hdrs) => println!("{:?}", (body_hdrs.1).0),
                Err(_) => panic!("failed"),
            }
        }
        Err(err) => panic!("oh no: {}"),
    }
}
