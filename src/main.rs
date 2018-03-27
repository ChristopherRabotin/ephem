#[macro_use]
extern crate nom;
use std::io;
use std::io::prelude::*;
use std::fs::File;

use nom::{digit, IResult};

// Parser definition

use std::str;
use std::str::FromStr;

// We parse any expr surrounded by parens, ignoring all whitespaces around those
named!(take_hdr, take_until_and_consume!("DAF/SPK"));
named!(
    krnl_name_seek,
    take_until_and_consume!("JPL") //take_until_and_consume!("JPL planetary and lunar ephmeris")
);
named!(take5, take!(5));
//DE436

#[derive(Debug, PartialEq)]
struct SPK<'a> {
    name: &'a [u8],
}

named!(parser<&[u8],SPK>,
  do_parse!(    // the parser takes a byte array as input, and returns an A struct
    take_until_and_consume!("DAF/SPK")       >>      // check for header
    take_until_and_consume!("JPL planetary and lunar ephmeris ") >> // consume until there
    dn: take5       >>      // the return value of ret_int1, if it does not fail, will be stored
    (SPK{name: dn})          // the final tuple will be able to use the variable defined previously
  )
);

fn main() {
    let mut f = File::open("data/de436.bsp").expect("open");
    let mut buffer = vec![0; 3000];
    // read the whole file
    f.read(&mut buffer).expect("to end");
    //println!("{:?}", buffer);
    //let pt = hdr(take7(&buffer).expect("wut?").1).expect("does not start with DAF/SPK");
    //let pt = take_hdr(&buffer).expect("not an SPK file");
    //println!("{:?}", pt.0);
    //let pt = krnl_name_seek(pt.0).expect("oops");
    //println!("{:?}", pt.0);
    match parser(&buffer) {
        Ok(spk_info) => println!("{:?}", str::from_utf8(spk_info.1.name).unwrap()),
        Err(_) => panic!("oh no"),
    }
}
