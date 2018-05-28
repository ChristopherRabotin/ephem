#[macro_use]
extern crate nom;
use std::io::prelude::*;
use std::fs::File;
use std::collections::HashMap;
use std::str;

named!(consume_until_null, take_until_and_consume!("\0"));
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
named!(til_next_open_parens, ws!(take_until_and_consume!("(")));
named!(til_next_close_parens, ws!(take_until_and_consume!(")")));

#[derive(Debug, PartialEq)]
struct SPK<'a> {
    name: &'a str,
    date: &'a str,
    start_date: &'a str,
    end_date: &'a str,
}

#[derive(Debug, PartialEq, Clone)]
struct Body {
    name: String,
    naif_id: i16, // Some NAIF ID may be negative, esp. for spacecraft
    gm: f64,
}

named!(seek_to_gms<&[u8], &[u8]>,
    take_until_and_consume!("Sun/GM(I)")
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

named!(parse_body_hdr<&[u8],Body>,
  do_parse!(    // the parser takes a byte array as input, and returns an A struct
    many0!(tag!("\0")) >> // Remove any leading nulls
    name: til_next_open_parens >>
    naif_id : til_next_close_parens >>
    (Body{gm: -1.0, name: str::from_utf8(name).unwrap().trim().to_owned(),
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
    let mut data: HashMap<i16, Body>;
    data = HashMap::new();
    match parse_meta(&buffer) {
        Ok(spk_info) => {
            println!("{:?}", spk_info.1);
            match parse_all_body_hdr(spk_info.0) {
                Ok(body_hdrs) => {
                    // Let put all this into a HashMap for quick access
                    for body in &((body_hdrs.1).0) {
                        data.insert(body.naif_id, body.to_owned());
                    }
                    println!("{:?}", data);
                    let mut buf = seek_to_gms(body_hdrs.0).unwrap().0;
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
                                        3 => {
                                            mu = item.replace("D", "E").parse::<f64>().unwrap();
                                        }
                                        _ => {}
                                    }
                                }
                                if mu > -1.0 {
                                    // If it's an integer, update the appropriate value.
                                    match p_id.parse::<i16>() {
                                        Ok(p_id) => {
                                            let naif_id =
                                                if p_id < 4 { p_id * 100 + 99 } else { p_id };
                                            let mut cur_data =
                                                data.get(&naif_id).unwrap().to_owned();
                                            cur_data.gm = mu;
                                            data.insert(naif_id, cur_data.to_owned());
                                            if p_id < 3 {
                                                // Venus exists as both "Venus" and "Venus Barycenter"
                                                let mut cur_data =
                                                    data.get(&p_id).unwrap().to_owned();
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
                                                let mut cur_data =
                                                    data.get(&naif_id).unwrap().to_owned();
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
                    println!("\n{:?}", data);
                }
                Err(_) => panic!("failed"),
            }
        }
        Err(_) => panic!("oh no: {}"),
    }
}
