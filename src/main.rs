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
named!(parens<u8>, ws!(delimited!(tag!("("), factor, tag!(")"))));
named!(
    factor<u8>,
    alt!(map_res!(map_res!(ws!(digit), str::from_utf8), FromStr::from_str) | parens)
);
named!(take_hdr, take_until_and_consume!("DAF/SPK"));
named!(
    seek_krnl_name,
    take_until_and_consume!("JPL planetary and lunar ephmeris ")
);
named!(seek_krnl_date, take_until_and_consume!("Integrated "));
named!(seek_bodies, take_until_and_consume!("Bodies included:\0\0"));
// named!(body_name, )

#[derive(Debug, PartialEq)]
struct SPK<'a> {
    name: &'a [u8],
    date: &'a [u8],
    test: &'a [u8],
}

#[derive(Debug, PartialEq)]
struct Body<'a> {
    name: &'a [u8],
    naif_id: u8,
}

named!(parser<&[u8],SPK>,
  do_parse!(    // the parser takes a byte array as input, and returns an A struct
    take_hdr >> // check for header
    seek_krnl_name >> // consume until there
    dn: take!(5) >> // get the DE name
    seek_krnl_date >>
    date: consume_until_null >> // get the date time of file
    seek_bodies >>
    test : ws!(take_until!("(")) >>
    (SPK{name: dn, date: date, test: test})
  )
);

fn main() {
    let mut f = File::open("data/de436.bsp").expect("open");
    let mut buffer = vec![0; 0];
    // read the whole file
    f.read_to_end(&mut buffer).expect("to end");
    //println!("{:?}", buffer);
    //let pt = hdr(take7(&buffer).expect("wut?").1).expect("does not start with DAF/SPK");
    //let pt = take_hdr(&buffer).expect("not an SPK file");
    //println!("{:?}", pt.0);
    //let pt = seek_krnl_name(pt.0).expect("oops");
    //println!("{:?}", pt.0);
    match parser(&buffer) {
        Ok(spk_info) => {
            let spk = spk_info.1;
            println!("{:?}", str::from_utf8(spk.name).unwrap());
            println!("{:?}", str::from_utf8(spk.date).unwrap());
            println!("{:?}", str::from_utf8(spk.test).unwrap())
        }
        Err(err) => panic!("oh no: {:?}", err),
    }
}
